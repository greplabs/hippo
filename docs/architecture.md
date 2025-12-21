---
layout: default
title: Architecture
nav_order: 6
description: "Deep dive into Hippo's architecture - project structure, modules, and how everything connects"
---

# Architecture Guide
{: .no_toc }

Deep dive into Hippo's architecture and internals
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Hippo is built as a modular Rust workspace with clear separation of concerns. The architecture prioritizes:

- **Performance**: Rust for core logic, parallel processing
- **Privacy**: All data stored locally, optional AI
- **Portability**: Cross-platform desktop, web, and CLI
- **Extensibility**: Plugin-ready module system

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         HIPPO ARCHITECTURE                       │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Tauri UI   │────▶│  hippo-tauri │────▶│  hippo-core  │
│  (HTML/JS)   │     │   (Commands) │     │    (Rust)    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                     ┌────────────────────────────┼────────────────┐
                     │                            │                │
                     ▼                            ▼                ▼
              ┌────────────┐              ┌────────────┐    ┌────────────┐
              │   SQLite   │              │   Ollama   │    │  Indexer   │
              │  (Storage) │              │    (AI)    │    │  (Files)   │
              └────────────┘              └────────────┘    └────────────┘
                     │                            │                │
                     │                            │                │
              ┌──────┴──────┐              ┌──────┴──────┐   ┌─────┴─────┐
              │  Memories   │              │  Embeddings │   │  Sources  │
              │    Tags     │              │     RAG     │   │  Watcher  │
              │   Search    │              │    Chat     │   │  Extract  │
              └─────────────┘              └─────────────┘   └───────────┘
```

---

## Workspace Structure

Hippo is organized as a Cargo workspace with 5 main crates:

```
hippo/
├── Cargo.toml                    # Workspace config
├── hippo-core/                   # Core Rust library ⭐
├── hippo-tauri/                  # Desktop app
├── hippo-cli/                    # Command-line tool
├── hippo-web/                    # Web server (REST API)
└── hippo-wasm/                   # WebAssembly bindings
```

### hippo-core

The heart of Hippo. A standalone Rust library that can be embedded in any application.

**Responsibilities**:
- File indexing and metadata extraction
- Storage (SQLite)
- Search engine
- Embedding generation
- AI integration
- File watching
- Thumbnail generation

**Key modules**:

```
hippo-core/src/
├── lib.rs                # Main Hippo struct & public API
├── models.rs             # All data types
├── error.rs              # Error handling
├── storage/
│   └── mod.rs            # SQLite storage layer
├── indexer/
│   ├── mod.rs            # File discovery & worker
│   ├── extractors.rs     # Metadata extraction (EXIF, etc.)
│   └── code_parser.rs    # AST parsing for code files
├── search/
│   └── mod.rs            # Search engine with filters
├── embeddings/
│   └── mod.rs            # ONNX embedding generation
├── thumbnails/
│   └── mod.rs            # Image/video thumbnail cache
├── ai/
│   └── mod.rs            # File analysis with AI
├── ollama/
│   └── mod.rs            # Local Ollama client
├── qdrant/
│   └── mod.rs            # Vector database manager
├── watcher/
│   └── mod.rs            # File system watching
├── duplicates/
│   └── mod.rs            # Duplicate detection
├── organization/
│   └── mod.rs            # Smart file organization
├── graph/
│   └── mod.rs            # Knowledge graph (stub)
└── sources/
    └── mod.rs            # Cloud connectors (stubs)
```

### hippo-tauri

Desktop application built with Tauri 2.0.

**Structure**:

```
hippo-tauri/
├── Cargo.toml
├── tauri.conf.json           # App config
├── capabilities/
│   └── default.json          # Permissions
├── icons/                    # App icons
├── src/
│   └── main.rs               # Tauri commands (IPC)
└── ui/
    └── dist/
        └── index.html        # Complete UI (no build step)
```

**Tauri Commands**:
- Source management (add, remove, list)
- Search with filters
- Tag management
- Statistics
- File operations (open, reveal)

### hippo-cli

Command-line interface with hippo-themed commands.

**Structure**:

```
hippo-cli/
├── Cargo.toml
└── src/
    └── main.rs               # CLI using clap
```

**Commands**: See [CLI Guide](cli-guide)

### hippo-web

REST API server for web/mobile clients (coming soon).

**Planned endpoints**:
- `POST /api/search`
- `GET /api/memories/:id`
- `POST /api/sources`
- `GET /api/tags`

### hippo-wasm

WebAssembly bindings for browser integration (experimental).

---

## Core Modules Deep Dive

### Storage Layer

**File**: `hippo-core/src/storage/mod.rs`

**Technology**: SQLite with JSON columns for flexibility

**Schema**:

```sql
-- Indexed files
CREATE TABLE memories (
    id TEXT PRIMARY KEY,              -- UUID
    path TEXT NOT NULL UNIQUE,        -- Full file path
    source_json TEXT NOT NULL,        -- JSON: Source enum
    kind_json TEXT NOT NULL,          -- JSON: MemoryKind enum
    metadata_json TEXT NOT NULL,      -- JSON: All metadata
    tags_json TEXT,                   -- JSON: Array of tags
    embedding_id TEXT,                -- Qdrant point ID
    connections_json TEXT,            -- JSON: Related files
    created_at INTEGER NOT NULL,      -- Unix timestamp
    modified_at INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL
);

-- Full-text search index (FTS5)
CREATE VIRTUAL TABLE memories_fts USING fts5(
    id,
    title,
    path,
    tags
);

-- Indexed folders
CREATE TABLE sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_json TEXT NOT NULL UNIQUE, -- JSON: Source enum
    added_at INTEGER NOT NULL
);

-- Tag counts
CREATE TABLE tag_counts (
    name TEXT PRIMARY KEY,
    count INTEGER NOT NULL DEFAULT 0
);
```

**Why JSON columns?**
- Flexible schema for different file types
- Avoid complex joins
- Easy to serialize/deserialize Rust types
- SQLite has excellent JSON support

### Indexing Pipeline

**File**: `hippo-core/src/indexer/mod.rs`

**Flow**:

```
1. User adds source → Queue source for indexing
2. Background worker starts
3. Walk directory tree (walkdir crate)
4. Filter by supported extensions
5. Batch files (100 at a time)
6. Parallel processing (rayon)
   ├── Extract metadata
   ├── Generate thumbnail (if image/video)
   ├── Compute hash (for duplicates)
   └── Generate embedding (if enabled)
7. Store in database
8. Update FTS index
```

**Supported Extensions** (70+):

```rust
// In hippo-core/src/indexer/mod.rs
const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Images
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "heic", "heif",
    "raw", "cr2", "nef", "arw", "dng",

    // Videos
    "mp4", "mov", "avi", "mkv", "webm", "flv", "wmv", "m4v", "mpg", "mpeg",

    // Audio
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus",

    // Documents
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp",
    "txt", "md", "rtf", "tex",

    // Code
    "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "cpp", "c", "h",
    "cs", "php", "rb", "swift", "kt", "r", "scala", "sh", "bash", "zsh",
    "sql", "html", "css", "scss", "sass", "vue", "svelte",

    // Archives
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar",

    // Databases
    "db", "sqlite", "sqlite3",

    // Spreadsheets
    "csv", "tsv",
];
```

### Metadata Extraction

**File**: `hippo-core/src/indexer/extractors.rs`

Different extractors for different file types:

**Images (EXIF)**:
```rust
pub fn extract_image_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Use kamadak-exif crate
    // Extract: dimensions, camera, GPS, ISO, aperture, etc.
}
```

**Videos**:
```rust
pub fn extract_video_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Use symphonia crate for audio metadata
    // Extract: duration, resolution, codec
}
```

**Code**:
```rust
pub fn extract_code_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Use tree-sitter for AST parsing
    // Extract: language, imports, exports, functions, lines
}
```

**Documents (PDF)**:
```rust
pub fn extract_document_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Use pdf-extract crate
    // Extract: page count, author, title, text content
}
```

### Search Engine

**File**: `hippo-core/src/search/mod.rs`

**Multi-stage search**:

```
1. Text Search (SQLite FTS5)
   ├── Search title, path, tags
   └── Full-text index for speed

2. Tag Filtering
   ├── Include tags (AND)
   └── Exclude tags (NOT)

3. Type Filtering
   └── Filter by MemoryKind

4. Date Range Filtering
   └── Filter by modified_at

5. Source Filtering
   └── Filter by source path

6. Sorting
   ├── By relevance (FTS rank)
   ├── By date (newest/oldest)
   ├── By name (A-Z/Z-A)
   └── By size (largest/smallest)

7. Pagination
   └── LIMIT + OFFSET
```

**Performance**:
- SQLite FTS5 for full-text search (<50ms for 100K files)
- Indexes on frequently queried columns
- Prepared statements for reuse
- Connection pooling (single connection per Hippo instance)

### Embeddings & Vector Search

**File**: `hippo-core/src/embeddings/mod.rs`

**Models** (using ONNX Runtime):
- **Images**: CLIP (OpenAI) - 512 dimensions
- **Text**: BGE (BAAI) - 384 dimensions
- **Code**: CodeBERT (Microsoft) - 768 dimensions

**Pipeline**:

```
1. File indexed → Generate embedding
2. Store embedding in Qdrant
3. Semantic search:
   ├── Convert query to embedding
   ├── Search Qdrant for similar vectors
   └── Return matching files
```

**Status**: Partially implemented (stubs in place)

### Thumbnail Generation

**File**: `hippo-core/src/thumbnails/mod.rs`

**Images**:
```rust
// Resize to 256x256, maintain aspect ratio
// Save as JPEG with quality 85
// Cache at: ~/Library/Caches/Hippo/thumbnails/{hash}.jpg
```

**Videos**:
```rust
// Extract first frame using ffmpeg
// Resize to 256x256
// Fallback to icon if ffmpeg unavailable
```

**Cache invalidation**:
- Hash based on: file path + modified time
- Re-generate if file changes

### File Watching

**File**: `hippo-core/src/watcher/mod.rs`

**Technology**: `notify` crate (cross-platform)

**Events**:
- File created → Index new file
- File modified → Update metadata
- File deleted → Remove from index
- File renamed → Update path

**Debouncing**:
- Wait 1 second after last event
- Batch multiple events
- Avoid re-indexing on temp files

### AI Integration

**Files**:
- `hippo-core/src/ai/mod.rs` - Claude API client
- `hippo-core/src/ollama/mod.rs` - Ollama client

**Claude Features**:
- File analysis (describe content)
- Tag suggestions (with confidence)
- Organization suggestions
- Duplicate detection (semantic)

**Ollama Features**:
- Local inference (privacy)
- Fast responses (~1.5s with qwen2:0.5b)
- RAG (Retrieval-Augmented Generation)
- Chat with your files

### Duplicate Detection

**File**: `hippo-core/src/duplicates/mod.rs`

**Algorithm**:

```
1. Get all files > min_size
2. Group by exact file size
3. For each group (size > 1):
   ├── Compute SHA-256 hash
   └── Group by hash
4. Report groups with > 1 file
```

**Performance**:
- Parallel hashing (rayon)
- Progress reporting
- Skip small files (< 1KB default)

---

## Data Flow

### Indexing Flow

```
User Action: Add ~/Documents
    │
    ▼
Hippo::add_source()
    │
    ├─▶ Storage::add_source()        # Save to DB
    └─▶ Indexer::queue_source()      # Start indexing
            │
            ▼
        Walk directory
            │
            ├─▶ Filter by extension
            │
            ├─▶ Batch files (100)
            │
            └─▶ Parallel process (rayon)
                    │
                    ├─▶ Extract metadata
                    ├─▶ Generate thumbnail
                    ├─▶ Compute hash
                    ├─▶ Generate embedding
                    │
                    ▼
                Storage::add_memory()
                    │
                    ├─▶ Insert into memories table
                    ├─▶ Update FTS index
                    └─▶ Update tag counts
```

### Search Flow

```
User Query: "vacation photos"
    │
    ▼
Hippo::search()
    │
    ▼
SearchQuery {
    text: "vacation photos",
    tags: ["beach"],
    kinds: [Image],
    limit: 100
}
    │
    ▼
Searcher::search()
    │
    ├─▶ Build SQL query
    │   ├─▶ FTS for text
    │   ├─▶ WHERE for tags
    │   ├─▶ WHERE for kinds
    │   └─▶ ORDER BY + LIMIT
    │
    ├─▶ Execute query
    │
    ├─▶ Deserialize results
    │
    └─▶ Return SearchResults
            │
            ▼
        UI displays results
```

### Tagging Flow

```
User Action: Add tag "important"
    │
    ▼
Hippo::add_tag(memory_id, tag)
    │
    ▼
Storage::add_tag()
    │
    ├─▶ Get current tags from memory
    ├─▶ Append new tag
    ├─▶ Update memory.tags_json
    ├─▶ Update FTS index (for search)
    └─▶ Increment tag_counts.count
```

---

## Performance Optimizations

### Parallel Processing

```rust
// Use rayon for CPU-bound tasks
use rayon::prelude::*;

files.par_iter()
    .map(|file| extract_metadata(file))
    .collect()
```

### Database Optimization

```sql
-- Indexes for fast lookups
CREATE INDEX idx_memories_path ON memories(path);
CREATE INDEX idx_memories_modified ON memories(modified_at);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE memories_fts USING fts5(...);

-- Prepared statements (reused)
let stmt = conn.prepare("SELECT * FROM memories WHERE id = ?")?;
```

### Caching

- **Thumbnails**: Disk cache with hash-based invalidation
- **Embeddings**: Stored in Qdrant, reused for search
- **Metadata**: SQLite is the cache

### Batching

- Index files in batches of 100
- Commit transactions in bulk
- Reduce I/O overhead

---

## Configuration

### Data Directories

**macOS**:
```
~/Library/Application Support/Hippo/    # Database & config
~/Library/Caches/Hippo/                 # Thumbnails & temp
```

**Linux**:
```
~/.local/share/Hippo/                   # Database & config
~/.cache/Hippo/                         # Thumbnails & temp
```

**Windows**:
```
%APPDATA%\Hippo\                        # Database & config
%LOCALAPPDATA%\Hippo\cache\             # Thumbnails & temp
```

### Environment Variables

```bash
# Claude API key (for AI features)
ANTHROPIC_API_KEY=sk-ant-...

# Ollama host (default: localhost:11434)
OLLAMA_HOST=http://localhost:11434

# Qdrant URL (default: localhost:6334)
QDRANT_URL=http://localhost:6334

# Log level (error, warn, info, debug, trace)
RUST_LOG=hippo=debug
```

---

## Extension Points

### Adding New File Types

**1. Add extension to supported list**:

```rust
// hippo-core/src/indexer/mod.rs
const SUPPORTED_EXTENSIONS: &[&str] = &[
    // ... existing
    "newext",
];
```

**2. Create metadata extractor**:

```rust
// hippo-core/src/indexer/extractors.rs
pub fn extract_newtype_metadata(path: &Path) -> Option<MemoryMetadata> {
    // Parse file and extract metadata
    Some(MemoryMetadata {
        title: Some("...".to_string()),
        // ...
    })
}
```

**3. Register extractor**:

```rust
// hippo-core/src/indexer/mod.rs
match extension {
    "newext" => extract_newtype_metadata(&path),
    // ...
}
```

### Adding New Tauri Commands

**1. Define command**:

```rust
// hippo-tauri/src/main.rs
#[tauri::command]
async fn my_command(state: State<'_, AppState>) -> Result<String, String> {
    // Implementation
    Ok("Success".to_string())
}
```

**2. Register command**:

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        initialize,
        search,
        my_command,  // Add here
    ])
```

**3. Call from UI**:

```javascript
const result = await invoke('my_command');
```

### Adding New AI Providers

**1. Create client**:

```rust
// hippo-core/src/ai/my_provider.rs
pub struct MyProviderClient {
    // ...
}

impl MyProviderClient {
    pub async fn analyze_file(&self, memory: &Memory) -> Result<AnalysisResult> {
        // Implementation
    }
}
```

**2. Register in AI module**:

```rust
// hippo-core/src/ai/mod.rs
pub enum AiProvider {
    Claude,
    Ollama,
    MyProvider,  // Add here
}
```

---

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific module
cargo test storage::tests

# Run with output
cargo test -- --nocapture
```

### Integration Tests

Located in each crate's `tests/` directory:

```
hippo-core/tests/
├── integration_test.rs
├── search_test.rs
└── indexer_test.rs
```

### Benchmarks

```bash
# Run benchmarks (requires nightly)
cargo +nightly bench
```

---

## Security Considerations

### Local-First Privacy

- **No telemetry**: Zero data sent to external servers
- **No tracking**: No analytics or usage metrics
- **No cloud required**: Works fully offline

### Optional Cloud AI

- **Explicit opt-in**: AI features require API key
- **User choice**: Claude (cloud) vs Ollama (local)
- **Data minimization**: Only file metadata sent, not content

### Database Security

- **Local SQLite**: No network exposure
- **File permissions**: Only user has access
- **No encryption**: Files not encrypted at rest (consider using encrypted disk)

---

## Next Steps

- [API Reference](api) - Developer documentation
- [CLI Guide](cli-guide) - Command-line usage
- [Desktop App Guide](desktop-app) - GUI features
- [Contributing](contributing) - Help improve Hippo

---

Questions? [Open an issue](https://github.com/greplabs/hippo/issues) on GitHub.
