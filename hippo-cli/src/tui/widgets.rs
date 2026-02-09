//! Custom widgets for the Hippo TUI

use hippo_core::{Memory, MemoryKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

/// Format bytes into human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get a short icon/label for a MemoryKind
fn kind_icon(kind: &MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Image { .. } => "[IMG]",
        MemoryKind::Video { .. } => "[VID]",
        MemoryKind::Audio { .. } => "[AUD]",
        MemoryKind::Code { .. } => "[COD]",
        MemoryKind::Document { .. } => "[DOC]",
        MemoryKind::Spreadsheet { .. } => "[XLS]",
        MemoryKind::Presentation { .. } => "[PPT]",
        MemoryKind::Archive { .. } => "[ZIP]",
        MemoryKind::Database => "[DB ]",
        MemoryKind::Folder => "[DIR]",
        MemoryKind::Unknown => "[   ]",
    }
}

fn kind_color(kind: &MemoryKind) -> Color {
    match kind {
        MemoryKind::Image { .. } => Color::Magenta,
        MemoryKind::Video { .. } => Color::Red,
        MemoryKind::Audio { .. } => Color::Green,
        MemoryKind::Code { .. } => Color::Cyan,
        MemoryKind::Document { .. } => Color::Blue,
        MemoryKind::Spreadsheet { .. } => Color::Yellow,
        MemoryKind::Presentation { .. } => Color::LightRed,
        MemoryKind::Archive { .. } => Color::DarkGray,
        MemoryKind::Database => Color::LightBlue,
        MemoryKind::Folder => Color::Yellow,
        MemoryKind::Unknown => Color::DarkGray,
    }
}

fn kind_label(kind: &MemoryKind) -> String {
    match kind {
        MemoryKind::Image {
            format,
            width,
            height,
            ..
        } => {
            format!("Image ({}) {}x{}", format, width, height)
        }
        MemoryKind::Video {
            format,
            duration_ms,
            ..
        } => {
            let secs = duration_ms / 1000;
            let mins = secs / 60;
            let secs = secs % 60;
            format!("Video ({}) {}:{:02}", format, mins, secs)
        }
        MemoryKind::Audio {
            format,
            duration_ms,
            ..
        } => {
            let secs = duration_ms / 1000;
            let mins = secs / 60;
            let secs = secs % 60;
            format!("Audio ({}) {}:{:02}", format, mins, secs)
        }
        MemoryKind::Code {
            language, lines, ..
        } => {
            format!("Code ({}) {} lines", language, lines)
        }
        MemoryKind::Document {
            format, page_count, ..
        } => {
            let pages = page_count
                .map(|p| format!(", {} pages", p))
                .unwrap_or_default();
            format!("Document ({:?}{})", format, pages)
        }
        MemoryKind::Spreadsheet { sheet_count, .. } => {
            format!("Spreadsheet ({} sheets)", sheet_count)
        }
        MemoryKind::Presentation { slide_count, .. } => {
            format!("Presentation ({} slides)", slide_count)
        }
        MemoryKind::Archive { item_count, .. } => {
            format!("Archive ({} items)", item_count)
        }
        MemoryKind::Database => "Database".to_string(),
        MemoryKind::Folder => "Folder".to_string(),
        MemoryKind::Unknown => "Unknown".to_string(),
    }
}

// ===== SearchInput Widget =====

pub struct SearchInput<'a> {
    query: &'a str,
    is_active: bool,
}

impl<'a> SearchInput<'a> {
    pub fn new(query: &'a str, is_active: bool) -> Self {
        Self { query, is_active }
    }

    pub fn render(&self) -> Paragraph<'a> {
        let display_text = if self.query.is_empty() && !self.is_active {
            Span::styled(
                "Type / to search...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::styled(self.query, Style::default().fg(Color::White))
        };

        Paragraph::new(Line::from(display_text))
    }
}

// ===== FileList Widget =====

pub struct FileList<'a> {
    files: &'a [Memory],
    is_active: bool,
}

impl<'a> FileList<'a> {
    pub fn new(files: &'a [Memory], is_active: bool) -> Self {
        Self { files, is_active }
    }

    pub fn render(&self, state: ListState) -> (List<'a>, ListState) {
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|mem| {
                let name = mem
                    .metadata
                    .title
                    .as_deref()
                    .unwrap_or_else(|| {
                        mem.path
                            .file_name()
                            .map(|n| n.to_str().unwrap_or("?"))
                            .unwrap_or("Unknown")
                    });

                let size = format_bytes(mem.metadata.file_size);
                let icon = kind_icon(&mem.kind);
                let fav = if mem.is_favorite { " *" } else { "" };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(kind_color(&mem.kind)),
                    ),
                    Span::styled(name.to_string(), Style::default().fg(Color::White)),
                    Span::styled(fav.to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("  {}", size),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let highlight_style = if self.is_active {
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(Color::DarkGray).fg(Color::Gray)
        };

        let list = List::new(items).highlight_style(highlight_style);

        (list, state)
    }
}

// ===== DetailPanel Widget =====

pub struct DetailPanel<'a> {
    memory: Option<&'a Memory>,
    #[allow(dead_code)]
    is_active: bool,
}

impl<'a> DetailPanel<'a> {
    pub fn new(memory: Option<&'a Memory>, is_active: bool) -> Self {
        Self { memory, is_active }
    }

    pub fn render(&self) -> Paragraph<'static> {
        match self.memory {
            None => Paragraph::new(Span::styled(
                "Select a file to view details",
                Style::default().fg(Color::DarkGray),
            )),
            Some(mem) => {
                let name = mem
                    .metadata
                    .title
                    .clone()
                    .unwrap_or_else(|| {
                        mem.path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string()
                    });

                let path = mem.path.display().to_string();
                let size = format_bytes(mem.metadata.file_size);
                let kind = kind_label(&mem.kind);
                let fav = if mem.is_favorite {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                };

                let tags_str = if mem.tags.is_empty() {
                    "(none)".to_string()
                } else {
                    mem.tags
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let created = mem.created_at.format("%Y-%m-%d %H:%M").to_string();
                let modified = mem.modified_at.format("%Y-%m-%d %H:%M").to_string();
                let indexed = mem.indexed_at.format("%Y-%m-%d %H:%M").to_string();

                let mut lines = vec![
                    Line::from(Span::styled(
                        name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    detail_line_owned("Type", kind),
                    detail_line_owned("Size", size),
                    detail_line_owned("Favorite", fav),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Path:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(path, Style::default().fg(Color::DarkGray))),
                    Line::from(""),
                    detail_line_owned("Tags", tags_str),
                    Line::from(""),
                    detail_line_owned("Created", created),
                    detail_line_owned("Modified", modified),
                    detail_line_owned("Indexed", indexed),
                ];

                if let Some(summary) = &mem.metadata.ai_summary {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "AI Summary:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(Span::styled(
                        summary.clone(),
                        Style::default().fg(Color::White),
                    )));
                }

                if let Some(preview) = &mem.metadata.text_preview {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Preview:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    let truncated: String = preview.chars().take(200).collect();
                    lines.push(Line::from(Span::styled(
                        truncated,
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Press f to toggle favorite, Enter to open",
                    Style::default().fg(Color::DarkGray),
                )));

                Paragraph::new(lines).wrap(Wrap { trim: true })
            }
        }
    }
}

fn detail_line_owned(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {}: ", label),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(value, Style::default().fg(Color::White)),
    ])
}

// ===== TagCloud Widget =====

pub struct TagCloud<'a> {
    tags: &'a [(String, u64)],
}

impl<'a> TagCloud<'a> {
    pub fn new(tags: &'a [(String, u64)]) -> Self {
        Self { tags }
    }

    pub fn render_items(&self) -> Vec<(String, Style)> {
        if self.tags.is_empty() {
            return vec![(
                "  (no tags)".to_string(),
                Style::default().fg(Color::DarkGray),
            )];
        }

        self.tags
            .iter()
            .take(20)
            .map(|(name, count)| {
                let color = if *count > 50 {
                    Color::Yellow
                } else if *count > 10 {
                    Color::Cyan
                } else if *count > 5 {
                    Color::Green
                } else {
                    Color::White
                };

                (
                    format!("  {} ({})", name, count),
                    Style::default().fg(color),
                )
            })
            .collect()
    }
}
