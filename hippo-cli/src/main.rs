use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::Confirm;
use hippo_core::{Hippo, SearchQuery, Source, Tag, TagSource, TagFilter, TagFilterMode, ClaudeClient};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tabled::{Table, Tabled};

const HIPPO_ART: &str = r#"
    .-"""-.
   /        \
  /_        _\
 // \      / \\
 |\__\    /__/|
  \    ||    /
   \        /
    \  __  /   HIPPO
     '.__.'    The Memory That Never Forgets
"#;

#[derive(Parser)]
#[command(name = "hippo")]
#[command(author, version, about = "Hippo - The Memory That Never Forgets")]
#[command(after_help = "Examples:
  hippo chomp ~/Documents     Index a folder
  hippo sniff \"vacation\"      Search for files
  hippo remember              List recent files
  hippo twins                 Find duplicate files
  hippo brain                 AI auto-organize
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Chomp on some files - index a folder into Hippo's memory
    #[command(alias = "eat", alias = "index", alias = "add")]
    Chomp {
        /// Path to folder to index
        path: PathBuf,
    },

    /// Sniff around - search your memories
    #[command(alias = "search", alias = "find", alias = "s")]
    Sniff {
        /// Search query
        query: String,
        /// Filter by tags
        #[arg(short, long)]
        tags: Vec<String>,
        /// Limit results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Remember things - list your memories
    #[command(alias = "list", alias = "ls")]
    Remember {
        /// Number of memories to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Filter by type (image, video, audio, code, document)
        #[arg(short = 't', long)]
        kind: Option<String>,
    },

    /// Check your weight - show index statistics
    #[command(alias = "stats", alias = "info")]
    Weight,

    /// Gather the herd - list all sources (indexed folders)
    #[command(alias = "sources", alias = "folders")]
    Herd,

    /// Mark your territory - add tags to files
    #[command(alias = "tag")]
    Mark {
        /// File path or memory ID
        target: String,
        /// Tags to add
        tags: Vec<String>,
    },

    /// Find your twins - detect duplicate files
    #[command(alias = "duplicates", alias = "dupes")]
    Twins {
        /// Minimum file size to check (bytes)
        #[arg(short, long, default_value = "1024")]
        min_size: u64,
    },

    /// Use your brain - AI auto-organize and tag files
    #[command(alias = "ai", alias = "organize", alias = "auto")]
    Brain {
        /// Anthropic API key (or set ANTHROPIC_API_KEY env var)
        #[arg(short, long, env = "ANTHROPIC_API_KEY")]
        api_key: Option<String>,
        /// Only analyze, don't apply changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Take a big splash - refresh/reindex all sources
    #[command(alias = "refresh", alias = "reindex")]
    Splash,

    /// Stomp it out - remove a source from the index
    #[command(alias = "remove", alias = "rm")]
    Stomp {
        /// Path to remove
        path: PathBuf,
        /// Also delete the memories (not the actual files)
        #[arg(long)]
        delete_memories: bool,
    },

    /// Open wide - reveal file in finder/explorer
    #[command(alias = "open", alias = "reveal")]
    Yawn {
        /// File path or search query
        target: String,
    },

    /// Wade in the water - watch for file changes
    #[command(alias = "watch")]
    Wade {
        /// Paths to watch (defaults to all sources)
        paths: Vec<PathBuf>,
    },

    /// Hippo's home - show config and data locations
    #[command(alias = "config", alias = "home")]
    Den,

    /// Start fresh - reset the entire index
    #[command(alias = "reset", alias = "clear")]
    Forget {
        /// Skip confirmation
        #[arg(long)]
        force: bool,
    },
}

#[derive(Tabled)]
struct MemoryRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    kind: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Tags")]
    tags: String,
}

#[derive(Tabled)]
struct SourceRow {
    #[tabled(rename = "Path")]
    path: String,
    #[tabled(rename = "Files")]
    files: String,
}

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

fn print_hippo() {
    println!("{}", HIPPO_ART.bright_cyan());
}

fn print_header(text: &str) {
    println!("\n{} {}", "=>".bright_green(), text.bold());
}

fn print_success(text: &str) {
    println!("{} {}", "✓".bright_green(), text);
}

fn print_error(text: &str) {
    println!("{} {}", "✗".bright_red(), text);
}

fn print_info(text: &str) {
    println!("{} {}", "•".bright_blue(), text);
}

fn get_kind_string(kind: &hippo_core::MemoryKind) -> String {
    match kind {
        hippo_core::MemoryKind::Image { .. } => "Image".to_string(),
        hippo_core::MemoryKind::Video { .. } => "Video".to_string(),
        hippo_core::MemoryKind::Audio { .. } => "Audio".to_string(),
        hippo_core::MemoryKind::Code { language, .. } => format!("Code ({})", language),
        hippo_core::MemoryKind::Document { .. } => "Document".to_string(),
        hippo_core::MemoryKind::Spreadsheet { .. } => "Spreadsheet".to_string(),
        hippo_core::MemoryKind::Presentation { .. } => "Presentation".to_string(),
        hippo_core::MemoryKind::Archive { .. } => "Archive".to_string(),
        hippo_core::MemoryKind::Database => "Database".to_string(),
        hippo_core::MemoryKind::Folder => "Folder".to_string(),
        hippo_core::MemoryKind::Unknown => "File".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize Hippo
    let hippo = Hippo::new().await?;

    match cli.command {
        Commands::Chomp { path } => {
            print_header(&format!("Chomping on {}", path.display()));

            if !path.exists() {
                print_error(&format!("Path does not exist: {}", path.display()));
                return Ok(());
            }

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Indexing files...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let source = Source::Local { root_path: path.clone() };
            hippo.add_source(source).await?;

            pb.finish_with_message("Done!");
            print_success(&format!(
                "Added {} to Hippo's memory",
                path.display().to_string().bright_cyan()
            ));

            // Show stats
            let stats = hippo.stats().await?;
            print_info(&format!("Total memories: {}", stats.total_memories));
        }

        Commands::Sniff { query, tags, limit } => {
            print_header(&format!("Sniffing for \"{}\"", query.bright_yellow()));

            let search_query = SearchQuery {
                text: Some(query),
                tags: tags
                    .into_iter()
                    .map(|t| TagFilter { tag: t, mode: TagFilterMode::Include })
                    .collect(),
                limit,
                ..Default::default()
            };

            let results = hippo.search_advanced(search_query).await?;

            if results.memories.is_empty() {
                print_info("No memories found. Try a different search?");
            } else {
                println!(
                    "\n{} {} memories found:\n",
                    "Found".bright_green(),
                    results.memories.len()
                );

                let rows: Vec<MemoryRow> = results
                    .memories
                    .iter()
                    .map(|r| {
                        let mem = &r.memory;
                        let name = mem
                            .metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| {
                                mem.path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default()
                            });
                        let kind = get_kind_string(&mem.kind);
                        let size = format_bytes(mem.metadata.file_size);
                        let tags = mem
                            .tags
                            .iter()
                            .map(|t| t.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");

                        MemoryRow { name, kind, size, tags }
                    })
                    .collect();

                let table = Table::new(rows).to_string();
                println!("{}", table);
            }
        }

        Commands::Remember { limit, kind: _ } => {
            print_header("Remembering...");

            let search_query = SearchQuery {
                limit,
                ..Default::default()
            };

            let results = hippo.search_advanced(search_query).await?;

            if results.memories.is_empty() {
                print_info("No memories yet. Try 'hippo chomp <folder>' to add some!");
            } else {
                println!(
                    "\n{} {} memories:\n",
                    "Showing".bright_green(),
                    results.memories.len()
                );

                let rows: Vec<MemoryRow> = results
                    .memories
                    .iter()
                    .map(|r| {
                        let mem = &r.memory;
                        let name = mem
                            .metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| {
                                mem.path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default()
                            });
                        let kind = get_kind_string(&mem.kind);
                        let size = format_bytes(mem.metadata.file_size);
                        let tags = mem
                            .tags
                            .iter()
                            .map(|t| t.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");

                        MemoryRow { name, kind, size, tags }
                    })
                    .collect();

                let table = Table::new(rows).to_string();
                println!("{}", table);
            }
        }

        Commands::Weight => {
            print_hippo();
            print_header("Checking Hippo's weight...");

            let stats = hippo.stats().await?;
            let sources = hippo.list_sources().await?;

            println!("\n{}", "Index Statistics".bold().underline());
            println!(
                "  {} {}",
                "Total Memories:".bright_blue(),
                stats.total_memories.to_string().bright_yellow()
            );
            println!(
                "  {} {}",
                "Total Sources:".bright_blue(),
                sources.len().to_string().bright_yellow()
            );
        }

        Commands::Herd => {
            print_header("Gathering the herd...");

            let sources = hippo.list_sources().await?;

            if sources.is_empty() {
                print_info("No sources yet. Try 'hippo chomp <folder>' to add one!");
            } else {
                println!("\n{} sources:\n", sources.len());

                let rows: Vec<SourceRow> = sources
                    .iter()
                    .map(|s| {
                        let path = match &s.source {
                            Source::Local { root_path } => {
                                root_path.to_string_lossy().to_string()
                            }
                            _ => "Unknown".to_string(),
                        };
                        SourceRow {
                            path,
                            files: "-".to_string(),
                        }
                    })
                    .collect();

                let table = Table::new(rows).to_string();
                println!("{}", table);
            }
        }

        Commands::Mark { target, tags } => {
            print_header(&format!("Marking {} with tags", target.bright_cyan()));

            if tags.is_empty() {
                print_error("No tags provided. Usage: hippo mark <file> <tag1> <tag2> ...");
                return Ok(());
            }

            // Search for the file
            let results = hippo.search(&target).await?;

            if let Some(result) = results.memories.first() {
                for tag_name in &tags {
                    let tag = Tag {
                        name: tag_name.clone(),
                        source: TagSource::User,
                        confidence: None,
                    };
                    hippo.add_tag(result.memory.id, tag).await?;
                    print_success(&format!("Added tag: {}", tag_name.bright_yellow()));
                }
            } else {
                print_error(&format!("No file found matching: {}", target));
            }
        }

        Commands::Twins { min_size } => {
            print_header("Looking for twins (duplicates)...");

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Scanning and hashing files...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            // Use real hash-based duplicate detection
            let (duplicate_groups, summary) = hippo.find_duplicates(min_size).await?;

            pb.finish_and_clear();

            if duplicate_groups.is_empty() {
                print_success("No duplicate files found!");
                println!("\n  {} files scanned", summary.files_scanned);
            } else {
                // Print summary
                println!(
                    "\n{} {} duplicate groups ({} duplicate files):\n",
                    "Found".bright_yellow(),
                    summary.duplicate_groups,
                    summary.total_duplicates
                );

                println!(
                    "  {} {} wasted by duplicates\n",
                    "💾".bright_red(),
                    format_bytes(summary.wasted_bytes).bright_red()
                );

                // Print each group
                for (i, group) in duplicate_groups.iter().take(20).enumerate() {
                    println!(
                        "{} {} files × {} = {} wasted",
                        format!("Group {}:", i + 1).bright_cyan(),
                        group.memory_ids.len(),
                        format_bytes(group.size),
                        format_bytes(group.wasted_bytes()).bright_yellow()
                    );

                    // Show hash (truncated)
                    println!("  {} {}", "Hash:".dimmed(), &group.hash[..16]);

                    // List files
                    for path in &group.paths {
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());
                        println!("  - {}", name.bright_white());
                        println!("    {}", path.display().to_string().dimmed());
                    }
                    println!();
                }

                // Show summary if there are more groups
                if duplicate_groups.len() > 20 {
                    println!(
                        "  {} {} more groups not shown",
                        "...".dimmed(),
                        duplicate_groups.len() - 20
                    );
                }

                // Cleanup suggestion
                println!("\n{}", "Tip: Review duplicates carefully before deleting. Keep the file in the best location.".dimmed());
            }
        }

        Commands::Brain { api_key, dry_run } => {
            print_header("Activating Hippo's brain...");

            let api_key = api_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());

            if api_key.is_none() {
                print_error("No API key provided!");
                println!(
                    "  Set {} or use {}",
                    "ANTHROPIC_API_KEY".bright_yellow(),
                    "--api-key".bright_cyan()
                );
                return Ok(());
            }

            let api_key = api_key.unwrap();

            if dry_run {
                print_info("Dry run mode - no changes will be applied");
            }

            // Create Claude client
            let claude = ClaudeClient::new(api_key);

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Finding files to analyze...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            // Get files with few or no tags
            let results = hippo
                .search_advanced(SearchQuery {
                    limit: 100,
                    ..Default::default()
                })
                .await?;

            let untagged: Vec<_> = results
                .memories
                .iter()
                .filter(|r| r.memory.tags.len() < 3) // Files with less than 3 tags
                .collect();

            pb.finish_and_clear();

            if untagged.is_empty() {
                print_success("All files are well-tagged!");
                return Ok(());
            }

            println!(
                "\n{} {} files could use better tagging\n",
                "Found".bright_yellow(),
                untagged.len()
            );

            // Analyze files with Claude
            let analyze_count = std::cmp::min(10, untagged.len());
            println!("Analyzing {} files with Claude AI...\n", analyze_count);

            let mut total_tags_added = 0;

            for (i, r) in untagged.iter().take(analyze_count).enumerate() {
                let mem = &r.memory;
                let name = mem
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                print!(
                    "{} Analyzing {}... ",
                    format!("[{}/{}]", i + 1, analyze_count).dimmed(),
                    name.bright_cyan()
                );

                match claude.analyze_file(mem).await {
                    Ok(analysis) => {
                        if analysis.tags.is_empty() {
                            println!("{}", "no suggestions".dimmed());
                            continue;
                        }

                        let tag_names: Vec<String> = analysis.tags
                            .iter()
                            .map(|t| format!("{} ({}%)", t.name, t.confidence))
                            .collect();

                        println!("{}", tag_names.join(", ").bright_yellow());

                        // Show description if available
                        if let Some(desc) = &analysis.description {
                            println!("    {}", desc.dimmed());
                        }

                        if !dry_run {
                            for tag_suggestion in &analysis.tags {
                                if tag_suggestion.confidence >= 60 {
                                    let tag = tag_suggestion.to_tag();
                                    if hippo.add_tag(mem.id, tag).await.is_ok() {
                                        total_tags_added += 1;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} {}", "error:".bright_red(), e);
                    }
                }

                // Small delay between API calls
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            println!();
            if dry_run {
                print_info("Dry run complete - no changes were made");
            } else {
                print_success(&format!("Added {} tags to files!", total_tags_added));
            }
        }

        Commands::Splash => {
            print_header("Making a big splash - reindexing all sources...");

            let sources = hippo.list_sources().await?;

            if sources.is_empty() {
                print_info("No sources to refresh. Add one with 'hippo chomp <folder>'");
                return Ok(());
            }

            for source in &sources {
                if let Source::Local { root_path } = &source.source {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.green} {msg}")
                            .unwrap(),
                    );
                    pb.set_message(format!("Reindexing {}...", root_path.display()));
                    pb.enable_steady_tick(std::time::Duration::from_millis(100));

                    hippo.sync_source(&source.source).await?;

                    pb.finish_with_message(format!("Done: {}", root_path.display()));
                }
            }

            print_success("All sources refreshed!");
        }

        Commands::Stomp {
            path,
            delete_memories,
        } => {
            print_header(&format!("Stomping out {}", path.display()));

            if Confirm::new()
                .with_prompt("Are you sure you want to remove this source?")
                .default(false)
                .interact()?
            {
                let source = Source::Local { root_path: path };
                hippo.remove_source(&source, delete_memories).await?;
                print_success("Source removed!");
            } else {
                print_info("Cancelled");
            }
        }

        Commands::Yawn { target } => {
            // Search for the file
            let results = hippo.search(&target).await?;

            if let Some(result) = results.memories.first() {
                let path = &result.memory.path;
                print_info(&format!("Opening {}...", path.display()));

                #[cfg(target_os = "macos")]
                {
                    std::process::Command::new("open")
                        .arg("-R")
                        .arg(path)
                        .spawn()?;
                }

                #[cfg(target_os = "linux")]
                {
                    std::process::Command::new("xdg-open")
                        .arg(path.parent().unwrap_or(path))
                        .spawn()?;
                }

                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("explorer")
                        .arg("/select,")
                        .arg(path)
                        .spawn()?;
                }
            } else {
                print_error(&format!("No file found matching: {}", target));
            }
        }

        Commands::Wade { paths } => {
            print_header("Wading in the water - watching for changes...");

            let sources = hippo.list_sources().await?;
            let watch_paths: Vec<PathBuf> = if paths.is_empty() {
                sources
                    .iter()
                    .filter_map(|s| {
                        if let Source::Local { root_path } = &s.source {
                            Some(root_path.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                paths
            };

            if watch_paths.is_empty() {
                print_error("No paths to watch. Add a source first with 'hippo chomp <folder>'");
                return Ok(());
            }

            // Start watching each path
            let mut watched_count = 0;
            for path in &watch_paths {
                let source = Source::Local { root_path: path.clone() };
                match hippo.watch_source(&source).await {
                    Ok(_) => {
                        watched_count += 1;
                        println!("  {} {}", "✓".bright_green(), path.display().to_string().bright_cyan());
                    }
                    Err(e) => {
                        println!("  {} {} - {}", "✗".bright_red(), path.display(), e);
                    }
                }
            }

            if watched_count == 0 {
                print_error("Failed to watch any paths");
                return Ok(());
            }

            println!("\n{}", format!("Watching {} path(s) for changes...", watched_count).bright_green());
            println!("{}", "Press Ctrl+C to stop".dimmed());
            println!();

            // Set up Ctrl+C handler
            let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let r = running.clone();
            ctrlc::set_handler(move || {
                r.store(false, std::sync::atomic::Ordering::SeqCst);
            }).expect("Error setting Ctrl-C handler");

            // Keep running and show status
            let mut last_count = hippo.stats().await?.total_memories;
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // Check if anything changed
                let current_count = hippo.stats().await?.total_memories;
                if current_count != last_count {
                    let diff = current_count as i64 - last_count as i64;
                    if diff > 0 {
                        println!("  {} {} new file(s) indexed", "→".bright_blue(), diff);
                    } else {
                        println!("  {} {} file(s) removed", "→".bright_yellow(), -diff);
                    }
                    last_count = current_count;
                }
            }

            // Cleanup
            println!("\n{}", "Stopping watchers...".dimmed());
            hippo.unwatch_all().await?;
            print_success("Stopped watching");
        }

        Commands::Den => {
            print_hippo();
            print_header("Hippo's Den");

            let dirs = directories::ProjectDirs::from("", "", "Hippo")
                .expect("Failed to get project dirs");

            println!("\n{}", "Locations:".bold());
            println!(
                "  {} {}",
                "Data:".bright_blue(),
                dirs.data_dir().display()
            );
            println!(
                "  {} {}",
                "Config:".bright_blue(),
                dirs.config_dir().display()
            );
            println!(
                "  {} {}",
                "Cache:".bright_blue(),
                dirs.cache_dir().display()
            );

            println!("\n{}", "Database:".bold());
            println!(
                "  {}",
                dirs.data_dir().join("hippo.db").display()
            );
        }

        Commands::Forget { force } => {
            print_header("Forgetting everything...");

            let confirm = force
                || Confirm::new()
                    .with_prompt("This will delete ALL indexed data. Are you sure?")
                    .default(false)
                    .interact()?;

            if confirm {
                hippo.clear_all().await?;
                print_success("Index reset. Hippo's memory is now clean.");
            } else {
                print_info("Cancelled");
            }
        }
    }

    Ok(())
}
