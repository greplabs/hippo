# Hippo - Project Documentation

## Overview

Hippo ("The Memory That Never Forgets") is a local-first, cross-platform file organizer built with Rust + Tauri 2. It indexes files from local folders, extracts metadata, and provides fast search with semantic vector capabilities, AI-powered analysis, and real-time file watching.

**Architecture**: Rust core library + Tauri 2 desktop app with standalone HTML/JS UI (no build step)

**Current Version**: v1.2.0 | **Latest Session**: 15 | **Latest PR**: #84

---

## Architecture

```
hippo/
├── Cargo.toml                    # Workspace config
├── hippo-core/                   # Core Rust library
│   └── src/
│       ├── lib.rs                # Main Hippo struct & public API
│       ├── models.rs             # All data types (Memory, Tag, Source, etc.)
│       ├── error.rs              # HippoError enum
│       ├── indexer/              # File discovery & metadata extraction
│       │   ├── mod.rs            # Orchestration, progress, batch processing
│       │   ├── extractors.rs     # EXIF, document metadata, file stats
│       │   └── code_parser.rs    # AST parsing (Rust/Python/JS/Go)
│       ├── storage/mod.rs        # SQLite + Qdrant hybrid storage
│       ├── search/               # Hybrid search (text + semantic + fuzzy)
│       │   ├── mod.rs
│       │   └── advanced_filter.rs
│       ├── embeddings/mod.rs     # Vector embeddings (Ollama/OpenAI/fallback)
│       ├── ollama/mod.rs         # Ollama AI integration
│       ├── watcher/mod.rs        # Real-time file system monitoring
│       ├── thumbnails/mod.rs     # Image/video/PDF thumbnail generation
│       ├── qdrant/               # Vector database management
│       │   ├── mod.rs
│       │   └── manager.rs
│       ├── scheduler/mod.rs      # Scheduled auto-indexing
│       ├── duplicates/mod.rs     # Hash-based duplicate detection
│       ├── graph/mod.rs          # Knowledge graph (partial)
│       └── sources/mod.rs        # Source connectors
├── hippo-cli/                    # CLI application
│   └── src/
│       ├── main.rs               # CLI commands (chomp, sniff, remember, etc.)
│       └── tui/                  # Interactive terminal UI (ratatui + crossterm)
│           ├── mod.rs
│           └── widgets.rs
└── hippo-tauri/                  # Desktop application
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json # Permissions
    ├── src/main.rs               # Tauri IPC commands
    └── ui/dist/index.html        # Complete standalone UI (no build step)
```

---

## Core Modules

### Indexer (`hippo-core/src/indexer/`)
- File discovery via `walkdir` with 70+ supported extensions
- Parallel batch processing with configurable size
- Progress tracking with ETA calculation
- EXIF extraction, code parsing, audio/video duration
- Smart re-indexing (skip unchanged files via mtime comparison)
- Skip patterns: `.git`, `node_modules`, `.venv`, `__pycache__`, `target`, `build`, `dist`
- Fast mode (default): skips Ollama embeddings for 100x faster indexing

### Storage (`hippo-core/src/storage/mod.rs`)
- SQLite with FTS5, WAL mode, 1GB cache, 1GB mmap
- Qdrant vector database integration with SQLite fallback
- Denormalized search columns for fast queries
- Mutex poisoning recovery via `get_db()` helper
- Tables: `memories`, `sources`, `tags`, `embeddings`, `clusters`, `saved_searches`, `search_history`

### Search (`hippo-core/src/search/`)
- **Text**: FTS5 with BM25 ranking (title=10, filename=8, tags=7, content=5)
- **Semantic**: Vector similarity via Qdrant/SQLite
- **Hybrid**: Weighted combination (semantic 0.7 + keyword 0.3)
- **Fuzzy**: Levenshtein distance with optimized single-row algorithm
- **Natural Language**: Parses queries like "photos from last week"
- **Operators**: AND, OR, NOT, "quoted phrases", prefix queries, column-specific
- Paginated search with cursor-based pagination and total count

### Embeddings (`hippo-core/src/embeddings/mod.rs`)
- **Ollama**: `nomic-embed-text` (768-dim, recommended)
- **OpenAI**: `text-embedding-ada-002` (1536-dim)
- **Hash Fallback**: Deterministic for offline use
- LRU cache: 5000 entries, 1-hour TTL

### Ollama (`hippo-core/src/ollama/mod.rs`)
- Embeddings, text generation, streaming chat, document analysis
- Code analysis, image captioning (llava), RAG
- Default model: `gemma2:2b`, fast tagging: `qwen2:0.5b`
- Cancellation token support for streaming

### Watcher (`hippo-core/src/watcher/mod.rs`)
- `notify` crate for platform-native file system events
- Auto re-indexing on create/modify, auto deletion on remove
- Debounced events, pause/resume, shutdown flag
- Skip patterns for `.git`, `node_modules`, etc.

### Thumbnails (`hippo-core/src/thumbnails/mod.rs`)
- Two-tier caching: memory LRU (2000 entries, 100MB) + disk (SHA256-named)
- Images, videos (ffmpeg), PDFs (pdfium), Office docs
- Smart cache invalidation based on file mtime

### Scheduler (`hippo-core/src/scheduler/mod.rs`)
- Background tokio task for periodic source re-indexing
- Configurable check interval (default: 300s) and per-source sync interval (default: 3600s)
- Tracks `last_sync` timestamp per source

### Qdrant (`hippo-core/src/qdrant/`)
- Managed process: auto-download, start, stop
- Collections: `hippo_text` (768-dim), `hippo_image` (512-dim), `hippo_code` (768-dim)
- Silent SQLite fallback on dimension mismatch

---

## Data Models

```rust
pub struct Memory {
    pub id: MemoryId,           // UUID
    pub path: PathBuf,
    pub source: Source,
    pub kind: MemoryKind,       // Image, Video, Audio, Document, Code, etc.
    pub metadata: MemoryMetadata,
    pub tags: Vec<Tag>,
    pub embedding_id: Option<String>,
    pub connections: Vec<Connection>,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

pub enum Source {
    Local { root_path: PathBuf },
    // Cloud stubs: GoogleDrive, ICloud, Dropbox, OneDrive, S3, Custom
}

pub struct Tag {
    pub name: String,
    pub kind: TagKind,          // User, System, AI
    pub confidence: u8,         // 0-100 for AI tags
    pub color: Option<String>,
    pub parent: Option<String>, // Hierarchical: "project/hippo"
}
```

---

## Tauri IPC Commands

| Command | Description |
|---------|-------------|
| `initialize` | Create Hippo instance, start Qdrant |
| `search` | Search memories with text and tags |
| `add_source` / `remove_source` | Manage indexed folders |
| `reindex_source` | Re-scan a folder |
| `get_sources` / `get_stats` / `get_tags` | Query state |
| `add_tag` / `remove_tag` | Tag management |
| `toggle_favorite` | Star/unstar a file |
| `get_thumbnail` | Get or generate thumbnail |
| `get_indexing_progress` | Current indexing status |
| `semantic_search` / `get_similar` | Vector search |
| `chat_with_ai` / `analyze_file` | AI features |
| `send_notification` | Desktop notification |
| `open_file` / `open_in_finder` | File actions |
| `save_search` / `list_saved_searches` / `delete_saved_search` / `use_saved_search` | Saved searches |
| `add_search_history` / `get_search_history` / `clear_search_history` | Search history |
| `get_recent_files` / `get_recently_modified` | Recent views |
| `batch_rename` | Bulk rename with templates |
| `search_paginated` | Paginated search with total count |
| `start_scheduler` / `get_scheduler_status` / `set_source_sync_interval` | Scheduler |
| `reset_index` | Delete all data and reinitialize |

---

## Development Guide

### Running

```bash
cargo tauri dev                    # Development (Tauri app)
cargo run -p hippo-cli -- --help   # CLI
cargo test --workspace             # All tests
cargo tauri build                  # Production build
```

### Key File Reference

| Feature | File(s) |
|---------|---------|
| File type support | `hippo-core/src/indexer/mod.rs` (SUPPORTED_EXTENSIONS) |
| Metadata extraction | `hippo-core/src/indexer/extractors.rs` |
| Code parsing | `hippo-core/src/indexer/code_parser.rs` |
| Search logic | `hippo-core/src/search/mod.rs` |
| Storage schema | `hippo-core/src/storage/mod.rs` (init_schema) |
| Vector search | `hippo-core/src/qdrant/mod.rs` |
| AI integration | `hippo-core/src/ollama/mod.rs` |
| File watching | `hippo-core/src/watcher/mod.rs` |
| Thumbnails | `hippo-core/src/thumbnails/mod.rs` |
| Scheduler | `hippo-core/src/scheduler/mod.rs` |
| Tauri commands | `hippo-tauri/src/main.rs` |
| UI | `hippo-tauri/ui/dist/index.html` |
| Data models | `hippo-core/src/models.rs` |
| TUI | `hippo-cli/src/tui/mod.rs`, `hippo-cli/src/tui/widgets.rs` |

### Adding a New Tauri Command

1. Define in `hippo-tauri/src/main.rs`:
```rust
#[tauri::command]
async fn your_command(param: String, state: State<'_, AppState>) -> Result<T, String> {
    let hippo = state.hippo.read().await;
    let hippo = hippo.as_ref().ok_or("Not initialized")?;
    hippo.your_method(param).await.map_err(|e| e.to_string())
}
```
2. Register in `generate_handler![]`
3. Call from UI: `await window.__TAURI__.invoke('your_command', { param: 'value' })`

### Database Locations
- **macOS**: `~/Library/Application Support/Hippo/hippo.db`
- **Linux**: `~/.local/share/Hippo/hippo.db`
- **Thumbnails**: `~/.cache/Hippo/thumbnails/`
- **Qdrant**: `~/Library/Application Support/com.hippo.app/qdrant/`

---

## Working Features

### Core
- File indexing with 70+ extensions, parallel batch processing, smart re-indexing
- SQLite + Qdrant hybrid storage with FTS5 full-text search
- Text, semantic, hybrid, fuzzy search with BM25 ranking
- Real-time file watching with auto re-indexing
- Scheduled auto-indexing (background scheduler)
- Two-tier thumbnail caching (memory + disk)
- Hash-based duplicate detection

### UI
- Grid/List views, type filter pills, sort dropdown
- Dark mode with system preference detection
- Tag management (colors, hierarchical, bulk tagging)
- Favorites view, collections/albums, timeline view
- Duplicate file manager, advanced filters (size, date, dimensions)
- Bulk operations (delete, tag, rename, export to JSON/CSV)
- Saved searches (DB-backed), search history
- Recently added/modified views
- Paginated search with infinite scroll
- Drag & drop folders to add sources
- Detail panel with file info, code preview (Prism.js)
- Find similar files with similarity percentages
- Keyboard shortcuts (?, /, G, L, Cmd+K, Cmd+D, Cmd+R, Cmd+Shift+H)

### AI (via Ollama)
- Local embeddings (nomic-embed-text)
- Text generation, streaming chat, RAG
- Document/code analysis, image captioning (llava)
- Ultra-fast auto-tagging (qwen2:0.5b)

### Desktop
- System tray with show/hide
- Global hotkey (Cmd+Shift+H) to show window
- Desktop notifications (indexing complete)
- Drag & drop file/folder import

### CLI
| Command | Aliases | Description |
|---------|---------|-------------|
| `chomp` | eat, index, add | Index a folder |
| `sniff` | search, find, s | Search files |
| `remember` | list, ls | List memories |
| `weight` | stats, info | Show statistics |
| `herd` | sources, folders | List sources |
| `mark` | tag | Add tags |
| `twins` | duplicates, dupes | Find duplicates |
| `brain` | ai, organize | AI auto-organize |
| `splash` | refresh, reindex | Reindex all |
| `stomp` | remove, rm | Remove source |
| `yawn` | open, reveal | Open in Finder |
| `wade` | watch | Watch changes |
| `den` | config, home | Show config |
| `forget` | reset, clear | Clear all |
| `tui` / `ui` | - | Interactive TUI |

---

## Current State

### Latest: Session 15 (February 2026)

**Branch**: `feature/session15-desktop-experience` | **PR**: #84

**Session 15 features**: Global Hotkey (Cmd+Shift+H), Drag & Drop, Scheduled Auto-indexing

**Bug fixes applied** (Session 15 continuation):
- Fixed infinite scroll (`getElementById('content')` null)
- Fixed Find Similar Files wrong command name
- Fixed CSP for Prism.js code preview
- Fixed batch rename collision check
- Fixed search history remove persistence + stuck indexing overlay
- Scheduler: needs wiring to actual re-indexing (pending task #26)

### Release History
| Version | Date | Key Features |
|---------|------|-------------|
| v1.2.0 | Feb 2026 | TUI, notifications, FTS5, 8 UI features, session 14-15 features |
| v1.1.0 | Dec 2025 | Rich AI content analysis, ultra-fast auto-tagging |
| v1.0.0 | Dec 2025 | GA release, fast indexing, memory leak fixes, icon fix |

### Session Roadmap
| Session | Features | Status |
|---------|----------|--------|
| 14 | Saved Searches, Search History, Recent Views, Batch Rename, Paginated Search | Done |
| 15 | Drag & Drop, Global Hotkey, Scheduled Auto-indexing | Done |
| 16 | Knowledge Graph Visualization, Location Map | Planned |
| 17 | Smart Collections, Natural Language Queries | Planned |

### Merged PRs
#38-#84 (all merged through main)

---

## Not Yet Implemented
- Cloud sources (Google Drive, iCloud, Dropbox, S3)
- Knowledge graph visualization (D3.js)
- Location map for geotagged files
- Smart collections (AI-grouped)
- Natural language file operations
- Face clustering
