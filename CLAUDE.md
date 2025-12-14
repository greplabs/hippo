Hippo - Complete Project Summary
Overview
Hippo 🦛 ("The Memory That Never Forgets") is a local-first, cross-platform file organizer built with Rust + Tauri 2. It indexes files from local folders, extracts metadata, and provides fast search with filtering capabilities.

Architecture
hippo/
├── Cargo.toml                    # Workspace config
├── hippo-core/                   # Core Rust library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Main Hippo struct & public API
│       ├── models.rs             # All data types (Memory, Tag, Source, etc.)
│       ├── error.rs              # HippoError enum
│       ├── indexer/
│       │   ├── mod.rs            # File discovery & background worker
│       │   ├── extractors.rs     # EXIF, document, code metadata extraction
│       │   └── code_parser.rs    # AST parsing for Rust/Python/JS/Go
│       ├── embeddings/
│       │   └── mod.rs            # ONNX embedding stubs (not implemented)
│       ├── storage/
│       │   └── mod.rs            # SQLite storage layer
│       ├── search/
│       │   └── mod.rs            # Search engine with filtering
│       ├── graph/
│       │   └── mod.rs            # Knowledge graph (stub)
│       └── sources/
│           └── mod.rs            # Cloud connector stubs
│
└── hippo-tauri/                  # Desktop application
    ├── Cargo.toml
    ├── tauri.conf.json           # Tauri config (withGlobalTauri: true)
    ├── capabilities/
    │   └── default.json          # Permissions (core, dialog, shell)
    ├── icons/                    # App icons (placeholder PNGs)
    ├── src/
    │   └── main.rs               # Tauri commands (IPC handlers)
    └── ui/
        └── dist/
            └── index.html        # Complete standalone UI (no build step)

Key Data Types (hippo-core/src/models.rs)
rust// Core indexed item
struct Memory {
    id: MemoryId (Uuid),
    path: PathBuf,
    source: Source,
    kind: MemoryKind,
    metadata: MemoryMetadata,
    tags: Vec<Tag>,
    embedding_id: Option<String>,
    connections: Vec<Connection>,
    created_at, modified_at, indexed_at: DateTime<Utc>,
}

// File type variants
enum MemoryKind {
    Image { width, height, format },
    Video { duration_ms: u64, format },
    Audio { duration_ms: u64, format },
    Document { format, page_count },
    Code { language, lines },
    Spreadsheet, Presentation, Archive, Database, Folder, Unknown
}

// Where files come from
enum Source {
    Local { root_path: PathBuf },
    GoogleDrive { account_id },  // stub
    ICloud, Dropbox, OneDrive, S3, Custom { name }  // stubs
}

// Search with filters
struct SearchQuery {
    text: Option<String>,
    tags: Vec<TagFilter>,
    sources: Vec<Source>,
    kinds: Vec<MemoryKind>,
    date_range: Option<DateRange>,
    sort: SortOrder,
    limit: usize,
    offset: usize,
}

Tauri Commands (hippo-tauri/src/main.rs)
CommandParametersDescriptioninitialize-Create Hippo instancesearchquery, tagsSearch memoriesadd_sourcesourceType, pathAdd folder to indexremove_sourcepath, deleteFilesRemove source & optionally delete memoriesreindex_sourcepathRe-scan a folderget_sources-List configured sourcesget_stats-Get index statisticsget_tags-List all tags with countsadd_tagmemoryId, tagAdd tag to memoryreset_index-Delete all data and reinitializeopen_filepathOpen file with default appopen_in_finderpathReveal file in Finder/Explorerget_mind_mapmemoryId, depthGet knowledge graph (stub)

Working Features
Indexing

✅ Add local folders via native folder picker dialog
✅ Background indexing with progress (async worker)
✅ File discovery with 70+ supported extensions
✅ Parallel batch processing (rayon)
✅ EXIF extraction for images (camera, GPS, dimensions)
✅ Code parsing for Rust/Python/JS/Go (imports, exports, functions)
✅ Basic metadata extraction (size, dates, MIME type)

Storage

✅ SQLite database (~/.local/share/Hippo/hippo.db or similar)
✅ JSON columns for flexible schema
✅ Tag counting and management
✅ Source configuration persistence

Search

✅ Text search (title, path, tags - substring matching)
✅ Tag filtering (include/exclude)
✅ Real-time search with debouncing (300ms)
✅ Client-side type filtering (Images, Videos, Audio, Code, Docs)
✅ Client-side sorting (date, name, size - asc/desc)

UI Features

✅ Grid and List view modes
✅ Type filter pills (All, Images, Videos, Audio, Code, Docs)
✅ Sort dropdown (Newest, Oldest, Name A-Z/Z-A, Size)
✅ Tag suggestions from search
✅ Tab key converts search text to tag filter
✅ Detail panel with file info
✅ Open file / Reveal in Finder buttons
✅ Keyboard shortcuts (⌘K to focus search, Esc to close)

Source Management

✅ Add folder (native dialog)
✅ Remove source (with memory deletion)
✅ Re-index source
✅ Reset entire index
✅ Auto-refresh during indexing (every 2s for 40s)


Not Yet Implemented
Embeddings & Semantic Search

ONNX models (CLIP for images, BGE for text, CodeBERT for code)
Qdrant vector database integration
Similarity search

Cloud Sources

Google Drive, iCloud, Dropbox OAuth flows
Cloud file syncing

Advanced Features

File watching (notify crate) - auto-detect changes
Image thumbnails
Duplicate detection
Face clustering
AI captioning (Claude API)
Knowledge graph visualization (D3.js)


Dependencies
hippo-core
tomltokio, serde, serde_json, uuid, chrono, thiserror, anyhow
rusqlite, qdrant-client (unused), ort (ONNX - stub)
walkdir, mime_guess, image, exif, rayon
directories, num_cpus, tracing
hippo-tauri
tomltauri 2.1, tauri-plugin-dialog 2, tauri-plugin-fs 2, tauri-plugin-shell 2
hippo-core, serde, serde_json, tokio, tracing, directories

Running the App
bashcd hippo-tauri
cargo run
```

Database location: `~/Library/Application Support/Hippo/hippo.db` (macOS)

---

## Key Files to Extend

| Feature | File(s) |
|---------|---------|
| Add new file types | `hippo-core/src/indexer/mod.rs` (SUPPORTED_EXTENSIONS) |
| Metadata extraction | `hippo-core/src/indexer/extractors.rs` |
| Code language support | `hippo-core/src/indexer/code_parser.rs` |
| Search logic | `hippo-core/src/search/mod.rs` |
| Storage schema | `hippo-core/src/storage/mod.rs` (init_schema) |
| New Tauri commands | `hippo-tauri/src/main.rs` |
| UI changes | `hippo-tauri/ui/dist/index.html` |
| Data models | `hippo-core/src/models.rs` |

---

## UI Structure (index.html)

The UI is a single HTML file with embedded JavaScript (no build step):
```
- State variables (memories, sources, tags, viewMode, sortBy, filterType, etc.)
- Icons object (SVG strings)
- Helper functions (formatBytes, formatDate, getTypeIcon, etc.)
- API functions (refreshData, handleSearch, addSource, removeSource, etc.)
- getDisplayedMemories() - applies filter + sort
- render() - builds entire HTML string and sets innerHTML
- Keyboard event listeners
- Initialization (calls invoke('initialize') then refreshData)

Next Steps to Consider

File watching - notify crate to auto-detect new/changed files
Thumbnails - Generate and cache image thumbnails
Favorites - Star important files
Dark mode - Theme toggle
Embeddings - Implement ONNX models for semantic search
Better code preview - Syntax highlighting in detail panel
Bulk operations - Multi-select and bulk tagging
Export/Import - Backup and restore index data



