//! Terminal User Interface for Hippo
//!
//! A full-featured TUI built with ratatui for browsing, searching,
//! and managing indexed files directly in the terminal.

pub mod widgets;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hippo_core::{Hippo, Memory, SearchQuery};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;
use widgets::{DetailPanel, FileList, SearchInput, TagCloud};

/// Run the TUI application
pub async fn run(hippo: &Hippo) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Load initial data
    load_data(hippo, &mut app).await;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => {
                        if handle_normal_key(key, &mut app, hippo).await {
                            break;
                        }
                    }
                    InputMode::Editing => {
                        handle_editing_key(key, &mut app, hippo).await;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Sidebar,
    FileList,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Files,
    Favorites,
    Tags,
    Duplicates,
}

impl AppTab {
    fn titles() -> &'static [&'static str] {
        &["Files", "Favorites", "Tags", "Duplicates"]
    }

    fn index(self) -> usize {
        match self {
            AppTab::Files => 0,
            AppTab::Favorites => 1,
            AppTab::Tags => 2,
            AppTab::Duplicates => 3,
        }
    }
}

struct App {
    input_mode: InputMode,
    active_panel: Panel,
    active_tab: AppTab,
    search_query: String,
    cursor_position: usize,
    files: Vec<Memory>,
    file_list_state: ListState,
    sources: Vec<String>,
    tags: Vec<(String, u64)>,
    total_count: usize,
    status_message: Option<String>,
    show_help: bool,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            input_mode: InputMode::Normal,
            active_panel: Panel::FileList,
            active_tab: AppTab::Files,
            search_query: String::new(),
            cursor_position: 0,
            files: Vec::new(),
            file_list_state: ListState::default(),
            sources: Vec::new(),
            tags: Vec::new(),
            total_count: 0,
            status_message: Some("Welcome to Hippo TUI! Press ? for help".to_string()),
            show_help: false,
            should_quit: false,
        }
    }

    fn selected_file(&self) -> Option<&Memory> {
        self.file_list_state
            .selected()
            .and_then(|i| self.files.get(i))
    }

    fn move_selection_down(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let i = match self.file_list_state.selected() {
            Some(i) => (i + 1).min(self.files.len() - 1),
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    fn move_selection_up(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let i = match self.file_list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    fn enter_search(&mut self) {
        self.input_mode = InputMode::Editing;
    }

    fn exit_search(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    fn insert_char(&mut self, c: char) {
        self.search_query.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.search_query.remove(self.cursor_position);
        }
    }

    fn move_cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.search_query.len() {
            self.cursor_position += 1;
        }
    }

    fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            AppTab::Files => AppTab::Favorites,
            AppTab::Favorites => AppTab::Tags,
            AppTab::Tags => AppTab::Duplicates,
            AppTab::Duplicates => AppTab::Files,
        };
    }

    fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            AppTab::Files => AppTab::Duplicates,
            AppTab::Favorites => AppTab::Files,
            AppTab::Tags => AppTab::Favorites,
            AppTab::Duplicates => AppTab::Tags,
        };
    }
}

async fn load_data(hippo: &Hippo, app: &mut App) {
    // Load sources
    if let Ok(sources) = hippo.list_sources().await {
        app.sources = sources
            .iter()
            .map(|sc| match &sc.source {
                hippo_core::Source::Local { root_path } => root_path.display().to_string(),
                other => format!("{:?}", other),
            })
            .collect();
    }

    // Load tags
    if let Ok(tags) = hippo.list_tags().await {
        app.tags = tags;
    }

    // Load files
    search_files(hippo, app).await;
}

async fn search_files(hippo: &Hippo, app: &mut App) {
    let query = if app.search_query.is_empty() {
        None
    } else {
        Some(app.search_query.clone())
    };

    let search_query = SearchQuery {
        text: query,
        tags: Vec::new(),
        sources: Vec::new(),
        kinds: Vec::new(),
        date_range: None,
        sort: hippo_core::SortOrder::DateNewest,
        limit: 100,
        offset: 0,
        ..Default::default()
    };

    match hippo.search_advanced(search_query).await {
        Ok(results) => {
            app.total_count = results.total_count;
            app.files = results.memories.into_iter().map(|r| r.memory).collect();
            if !app.files.is_empty() && app.file_list_state.selected().is_none() {
                app.file_list_state.select(Some(0));
            }
        }
        Err(e) => {
            app.status_message = Some(format!("Search error: {}", e));
        }
    }
}

/// Returns true if the app should quit
async fn handle_normal_key(key: KeyEvent, app: &mut App, hippo: &Hippo) -> bool {
    // Help overlay takes priority
    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char('/') => {
            app.enter_search();
        }
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                Panel::Sidebar => Panel::FileList,
                Panel::FileList => Panel::Detail,
                Panel::Detail => Panel::Sidebar,
            };
        }
        KeyCode::BackTab => {
            app.active_panel = match app.active_panel {
                Panel::Sidebar => Panel::Detail,
                Panel::FileList => Panel::Sidebar,
                Panel::Detail => Panel::FileList,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.active_panel == Panel::FileList {
                app.move_selection_down();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.active_panel == Panel::FileList {
                app.move_selection_up();
            }
        }
        KeyCode::Char('1') => app.active_tab = AppTab::Files,
        KeyCode::Char('2') => app.active_tab = AppTab::Favorites,
        KeyCode::Char('3') => app.active_tab = AppTab::Tags,
        KeyCode::Char('4') => app.active_tab = AppTab::Duplicates,
        KeyCode::Char('n') => {
            app.next_tab();
        }
        KeyCode::Char('p') => {
            app.prev_tab();
        }
        KeyCode::Char('f') => {
            // Toggle favorite
            if let Some(mem) = app.selected_file() {
                let id = mem.id;
                match hippo.toggle_favorite(id).await {
                    Ok(is_fav) => {
                        let msg = if is_fav {
                            "Added to favorites"
                        } else {
                            "Removed from favorites"
                        };
                        app.status_message = Some(msg.to_string());
                        search_files(hippo, app).await;
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Open file
            if let Some(mem) = app.selected_file() {
                let path = mem.path.clone();
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").arg(&path).spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("explorer").arg(&path).spawn();
                }
                app.status_message = Some(format!("Opened: {}", path.display()));
            }
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            load_data(hippo, app).await;
            app.status_message = Some("Refreshed".to_string());
        }
        _ => {}
    }
    false
}

async fn handle_editing_key(key: KeyEvent, app: &mut App, hippo: &Hippo) {
    match key.code {
        KeyCode::Enter => {
            app.exit_search();
            search_files(hippo, app).await;
        }
        KeyCode::Esc => {
            app.exit_search();
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Home => {
            app.cursor_position = 0;
        }
        KeyCode::End => {
            app.cursor_position = app.search_query.len();
        }
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search bar
            Constraint::Length(3),  // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    render_search_bar(f, app, main_chunks[0]);
    render_tabs(f, app, main_chunks[1]);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(main_chunks[2]);

    render_sidebar(f, app, content_chunks[0]);
    render_file_list(f, app, content_chunks[1]);
    render_detail_panel(f, app, content_chunks[2]);
    render_status_bar(f, app, main_chunks[3]);

    if app.show_help {
        render_help_overlay(f, size);
    }
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let search_widget = SearchInput::new(&app.search_query, app.input_mode == InputMode::Editing);
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(" Search (press / to focus) ")
        .border_style(if app.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = search_block.inner(area);
    f.render_widget(search_block, area);

    let paragraph = search_widget.render();
    f.render_widget(paragraph, inner);

    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((inner.x + app.cursor_position as u16, inner.y));
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = AppTab::titles()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let num = format!("[{}] ", i + 1);
            Line::from(vec![
                Span::styled(num, Style::default().fg(Color::DarkGray)),
                Span::raw(*t),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" View "))
        .select(app.active_tab.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));

    f.render_widget(tabs, area);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Sidebar;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut items: Vec<ListItem> = Vec::new();

    // Sources section
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "-- Sources --",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )])));

    if app.sources.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for source in &app.sources {
            let display = if source.len() > 25 {
                format!("  ...{}", &source[source.len() - 22..])
            } else {
                format!("  {}", source)
            };
            items.push(ListItem::new(Span::styled(
                display,
                Style::default().fg(Color::White),
            )));
        }
    }

    items.push(ListItem::new("")); // spacer

    // Tags section
    let tag_cloud = TagCloud::new(&app.tags);
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "-- Tags --",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )])));

    for (tag_line, style) in tag_cloud.render_items() {
        items.push(ListItem::new(Span::styled(tag_line, style)));
    }

    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .title(" Sidebar ")
        .border_style(border_style);

    let sidebar_list = List::new(items).block(sidebar_block);

    f.render_widget(sidebar_list, area);
}

fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::FileList;
    let file_list_widget = FileList::new(&app.files, is_active);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Files ({}) ", app.files.len()))
        .border_style(if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (list_widget, mut state) = file_list_widget.render(app.file_list_state.clone());
    f.render_stateful_widget(list_widget, inner, &mut state);
}

fn render_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Detail;
    let detail = DetailPanel::new(app.selected_file(), is_active);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .border_style(if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = block.inner(area);
    f.render_widget(block, area);

    let paragraph = detail.render();
    f.render_widget(paragraph, inner);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_text = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "SEARCH",
    };

    let panel_text = match app.active_panel {
        Panel::Sidebar => "Sidebar",
        Panel::FileList => "Files",
        Panel::Detail => "Detail",
    };

    let status = app.status_message.as_deref().unwrap_or("Ready");

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_text),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {} ", panel_text),
            Style::default().fg(Color::Black).bg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::styled(status, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(
            format!(" {} memories ", app.total_count),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::styled(
            " ?=help q=quit /=search ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_bar = Paragraph::new(status_line);
    f.render_widget(status_bar, area);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let help_width = 60u16.min(area.width.saturating_sub(4));
    let help_height = 22u16.min(area.height.saturating_sub(4));
    let help_area = Rect::new(
        (area.width.saturating_sub(help_width)) / 2,
        (area.height.saturating_sub(help_height)) / 2,
        help_width,
        help_height,
    );

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line("q", "Quit"),
        help_line("/", "Focus search input"),
        help_line("Esc", "Exit search / close overlay"),
        help_line("Enter", "Open selected file"),
        help_line("Tab", "Switch panels (forward)"),
        help_line("Shift+Tab", "Switch panels (backward)"),
        help_line("j / Down", "Move selection down"),
        help_line("k / Up", "Move selection up"),
        help_line("f", "Toggle favorite"),
        help_line("1-4", "Switch to tab"),
        help_line("n / p", "Next / previous tab"),
        help_line("Ctrl+R", "Refresh data"),
        help_line("?", "Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .border_style(Style::default().fg(Color::Yellow));

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .wrap(Wrap { trim: true });

    f.render_widget(help_paragraph, help_area);
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<12}", key),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(desc),
    ])
}
