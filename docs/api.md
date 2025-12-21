---
layout: default
title: API Reference
nav_order: 5
description: "Developer API documentation for Hippo - Core API, Tauri commands, and integration guide"
---

# API Reference
{: .no_toc }

Complete API documentation for developers integrating with Hippo
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Hippo provides three levels of API access:

1. **Rust Core API** (`hippo-core`) - Direct library usage in Rust applications
2. **Tauri Commands** (`hippo-tauri`) - IPC API for desktop app
3. **REST API** (`hippo-web`) - HTTP endpoints for web/mobile apps (coming soon)

---

## Rust Core API

The `hippo-core` library provides the foundational API for all Hippo functionality.

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hippo-core = { path = "../hippo-core" }  # or version from crates.io
tokio = { version = "1.40", features = ["full"] }
```

### Quick Start

```rust
use hippo_core::{Hippo, Source, SearchQuery, Tag};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Hippo with default config
    let hippo = Hippo::new().await?;

    // Add a local folder to index
    let source = Source::Local {
        root_path: PathBuf::from("/Users/you/Documents")
    };
    hippo.add_source(source).await?;

    // Search for files
    let results = hippo.search("vacation photos").await?;

    // Add tags
    for result in &results.memories {
        hippo.add_tag(result.memory.id, Tag::user("summer")).await?;
    }

    Ok(())
}
```

---

## Core Types

### Hippo

Main entry point for the library.

```rust
pub struct Hippo {
    // Internal fields...
}

impl Hippo {
    /// Create with default configuration
    pub async fn new() -> Result<Self>;

    /// Create with custom configuration
    pub async fn with_config(config: HippoConfig) -> Result<Self>;
}
```

**Example**:

```rust
// Default config
let hippo = Hippo::new().await?;

// Custom config
let config = HippoConfig {
    data_dir: PathBuf::from("/custom/path"),
    local_embeddings: true,
    ai_api_key: Some("sk-...".to_string()),
    qdrant_url: "http://localhost:6334".to_string(),
    indexing_parallelism: 8,
    auto_tag_enabled: false,
};
let hippo = Hippo::with_config(config).await?;
```

### HippoConfig

Configuration options for Hippo.

```rust
pub struct HippoConfig {
    /// Directory to store Hippo data
    pub data_dir: PathBuf,

    /// Whether to run embedding models locally
    pub local_embeddings: bool,

    /// API key for cloud AI services (optional)
    pub ai_api_key: Option<String>,

    /// Qdrant connection URL
    pub qdrant_url: String,

    /// Maximum concurrent indexing operations
    pub indexing_parallelism: usize,

    /// Whether to enable auto-tagging during indexing
    pub auto_tag_enabled: bool,
}
```

**Default values**:

```rust
HippoConfig {
    data_dir: ~/Library/Application Support/Hippo (macOS),
    local_embeddings: true,
    ai_api_key: None,
    qdrant_url: "http://localhost:6334",
    indexing_parallelism: num_cpus (max 8),
    auto_tag_enabled: false,
}
```

---

## Source Management API

### add_source

Add a folder to index.

```rust
pub async fn add_source(&self, source: Source) -> Result<()>
```

**Parameters**:
- `source`: Source to add (Local, GoogleDrive, etc.)

**Returns**: `Result<()>`

**Example**:

```rust
// Add local folder
let source = Source::Local {
    root_path: PathBuf::from("~/Documents")
};
hippo.add_source(source).await?;

// Cloud sources (stubs - not yet implemented)
let gdrive = Source::GoogleDrive {
    account_id: "user@gmail.com".to_string()
};
// hippo.add_source(gdrive).await?;  // Coming soon
```

### remove_source

Remove a source from the index.

```rust
pub async fn remove_source(&self, source: &Source, delete_memories: bool) -> Result<()>
```

**Parameters**:
- `source`: Source to remove
- `delete_memories`: Whether to delete indexed files

**Returns**: `Result<()>`

**Example**:

```rust
let source = Source::Local {
    root_path: PathBuf::from("~/Downloads")
};

// Remove source but keep memories
hippo.remove_source(&source, false).await?;

// Remove source and delete memories
hippo.remove_source(&source, true).await?;
```

### list_sources

Get all configured sources.

```rust
pub async fn list_sources(&self) -> Result<Vec<SourceConfig>>
```

**Returns**: `Result<Vec<SourceConfig>>`

**Example**:

```rust
let sources = hippo.list_sources().await?;
for source in sources {
    println!("Source: {:?}", source.source);
}
```

### sync_source

Re-index a specific source.

```rust
pub async fn sync_source(&self, source: &Source) -> Result<()>
```

**Parameters**:
- `source`: Source to re-index

**Returns**: `Result<()>`

**Example**:

```rust
let source = Source::Local {
    root_path: PathBuf::from("~/Documents")
};
hippo.sync_source(&source).await?;
```

---

## Search API

### search

Simple text search.

```rust
pub async fn search(&self, query: &str) -> Result<SearchResults>
```

**Parameters**:
- `query`: Search text

**Returns**: `Result<SearchResults>`

**Example**:

```rust
let results = hippo.search("vacation photos").await?;

for result in &results.memories {
    println!("Found: {}", result.memory.path.display());
    println!("Score: {}", result.score);
}
```

### search_advanced

Advanced search with filters.

```rust
pub async fn search_advanced(&self, query: SearchQuery) -> Result<SearchResults>
```

**Parameters**:
- `query`: Detailed search query with filters

**Returns**: `Result<SearchResults>`

**Example**:

```rust
use hippo_core::{SearchQuery, TagFilter, TagFilterMode, SortOrder};

let query = SearchQuery {
    text: Some("vacation".to_string()),
    tags: vec![
        TagFilter {
            tag: "beach".to_string(),
            mode: TagFilterMode::Include,
        },
        TagFilter {
            tag: "work".to_string(),
            mode: TagFilterMode::Exclude,
        },
    ],
    sources: vec![],  // All sources
    kinds: vec![],    // All file types
    date_range: None,
    sort: SortOrder::DateDesc,
    limit: 100,
    offset: 0,
};

let results = hippo.search_advanced(query).await?;
```

### SearchQuery

Search query structure.

```rust
pub struct SearchQuery {
    /// Text to search for (optional)
    pub text: Option<String>,

    /// Tag filters to apply
    pub tags: Vec<TagFilter>,

    /// Filter by sources
    pub sources: Vec<Source>,

    /// Filter by file kinds
    pub kinds: Vec<MemoryKind>,

    /// Filter by date range
    pub date_range: Option<DateRange>,

    /// Sort order
    pub sort: SortOrder,

    /// Maximum results to return
    pub limit: usize,

    /// Result offset for pagination
    pub offset: usize,
}
```

### SearchResults

Search results structure.

```rust
pub struct SearchResults {
    /// Found memories with relevance scores
    pub memories: Vec<SearchResult>,

    /// Total count (before pagination)
    pub total_count: usize,

    /// Query execution time in milliseconds
    pub query_time_ms: u64,
}

pub struct SearchResult {
    /// The memory (file)
    pub memory: Memory,

    /// Relevance score (0.0 - 1.0)
    pub score: f32,
}
```

---

## Tagging API

### add_tag

Add a tag to a file.

```rust
pub async fn add_tag(&self, memory_id: MemoryId, tag: Tag) -> Result<()>
```

**Parameters**:
- `memory_id`: UUID of the memory
- `tag`: Tag to add

**Returns**: `Result<()>`

**Example**:

```rust
use uuid::Uuid;
use hippo_core::Tag;

let memory_id = Uuid::parse_str("...")?;

// Add user tag
hippo.add_tag(memory_id, Tag::user("important")).await?;

// Add AI tag with confidence
hippo.add_tag(memory_id, Tag::ai("beach", 0.95)).await?;
```

### remove_tag

Remove a tag from a file.

```rust
pub async fn remove_tag(&self, memory_id: MemoryId, tag_name: &str) -> Result<()>
```

**Parameters**:
- `memory_id`: UUID of the memory
- `tag_name`: Name of tag to remove

**Returns**: `Result<()>`

**Example**:

```rust
hippo.remove_tag(memory_id, "old-tag").await?;
```

### get_tags

Get all tags in the index.

```rust
pub async fn get_tags(&self) -> Result<Vec<TagCount>>
```

**Returns**: `Result<Vec<TagCount>>`

**Example**:

```rust
let tags = hippo.get_tags().await?;
for tag in tags {
    println!("{}: {} files", tag.name, tag.count);
}
```

### suggest_tags

Get tag suggestions based on text.

```rust
pub async fn suggest_tags(&self, text: &str) -> Result<Vec<String>>
```

**Parameters**:
- `text`: Text to analyze for tag suggestions

**Returns**: `Result<Vec<String>>`

**Example**:

```rust
let suggestions = hippo.suggest_tags("beach vacation photo").await?;
// Returns: ["beach", "vacation", "photo", "summer", ...]
```

---

## Duplicate Detection API

### find_duplicates

Find duplicate files by content hash.

```rust
pub async fn find_duplicates(&self, min_size: u64) -> Result<(Vec<DuplicateGroup>, DuplicateSummary)>
```

**Parameters**:
- `min_size`: Minimum file size to check (bytes)

**Returns**: `Result<(Vec<DuplicateGroup>, DuplicateSummary)>`

**Example**:

```rust
// Find duplicates > 1KB
let (groups, summary) = hippo.find_duplicates(1024).await?;

println!("Found {} duplicate groups", summary.duplicate_groups);
println!("Total duplicates: {}", summary.total_duplicates);
println!("Wasted space: {} bytes", summary.wasted_bytes);

for group in groups {
    println!("Hash: {}", group.hash);
    println!("Size: {} bytes", group.size);
    for path in &group.paths {
        println!("  - {}", path.display());
    }
}
```

### DuplicateGroup

Group of duplicate files.

```rust
pub struct DuplicateGroup {
    /// SHA-256 hash of file contents
    pub hash: String,

    /// File size in bytes
    pub size: u64,

    /// Paths to duplicate files
    pub paths: Vec<PathBuf>,

    /// Memory IDs of duplicates
    pub memory_ids: Vec<MemoryId>,
}
```

---

## File Watching API

### watch_source

Start watching a source for changes.

```rust
pub async fn watch_source(&self, source: &Source) -> Result<()>
```

**Parameters**:
- `source`: Source to watch

**Returns**: `Result<()>`

**Example**:

```rust
let source = Source::Local {
    root_path: PathBuf::from("~/Documents")
};

hippo.watch_source(&source).await?;

// Watcher runs in background, updating index automatically
```

### unwatch_source

Stop watching a source.

```rust
pub async fn unwatch_source(&self, source: &Source) -> Result<()>
```

**Example**:

```rust
hippo.unwatch_source(&source).await?;
```

### unwatch_all

Stop all file watchers.

```rust
pub async fn unwatch_all(&self) -> Result<()>
```

**Example**:

```rust
hippo.unwatch_all().await?;
```

---

## Statistics API

### stats

Get index statistics.

```rust
pub async fn stats(&self) -> Result<IndexStats>
```

**Returns**: `Result<IndexStats>`

**Example**:

```rust
let stats = hippo.stats().await?;

println!("Total memories: {}", stats.total_memories);
println!("Total sources: {}", stats.total_sources);
println!("Total tags: {}", stats.total_tags);
println!("Database size: {} bytes", stats.db_size_bytes);
```

### IndexStats

Statistics structure.

```rust
pub struct IndexStats {
    pub total_memories: usize,
    pub total_sources: usize,
    pub total_tags: usize,
    pub db_size_bytes: u64,
    pub memories_by_kind: HashMap<String, usize>,
}
```

---

## AI Integration API

### Claude Client

Use Claude API for file analysis.

```rust
use hippo_core::ClaudeClient;

let client = ClaudeClient::new("your-api-key".to_string());

// Analyze a file
let analysis = client.analyze_file(&memory).await?;

println!("Description: {:?}", analysis.description);
println!("Tags: {:?}", analysis.tags);
```

### Ollama Client

Use local Ollama for AI features.

```rust
use hippo_core::OllamaClient;

let client = OllamaClient::new("http://localhost:11434");

// Check if Ollama is available
if client.is_available().await {
    let analysis = client.analyze_file_local(&memory).await?;
    println!("Tags: {:?}", analysis.suggested_tags);
}
```

---

## Tauri IPC Commands

Commands available in the Tauri desktop app via `window.__TAURI__.core.invoke()`.

### initialize

Initialize the Hippo instance.

```javascript
const status = await invoke('initialize');
// Returns: "Hippo initialized successfully"
```

### search

Search for files.

```javascript
const results = await invoke('search', {
  query: 'vacation',
  tags: ['beach', 'summer']
});

// Returns: SearchResults
console.log(results.memories.length);  // Number of results
```

### add_source

Add a folder to index.

```javascript
const status = await invoke('add_source', {
  sourceType: 'local',
  path: '/Users/john/Documents'
});

// Returns: "Source added successfully"
```

### remove_source

Remove a source.

```javascript
const status = await invoke('remove_source', {
  path: '/Users/john/Downloads',
  deleteFiles: false  // Don't delete memories
});
```

### get_sources

List all sources.

```javascript
const sources = await invoke('get_sources');

// Returns: SourceConfig[]
sources.forEach(source => {
  console.log(source.source);
});
```

### get_stats

Get index statistics.

```javascript
const stats = await invoke('get_stats');

// Returns: IndexStats
console.log(`Total files: ${stats.total_memories}`);
```

### get_tags

List all tags.

```javascript
const tags = await invoke('get_tags');

// Returns: TagCount[]
tags.forEach(tag => {
  console.log(`${tag.name}: ${tag.count} files`);
});
```

### add_tag

Add tag to a file.

```javascript
const status = await invoke('add_tag', {
  memoryId: 'uuid-string',
  tag: 'important'
});
```

### bulk_add_tag

Add tag to multiple files.

```javascript
const status = await invoke('bulk_add_tag', {
  memoryIds: ['uuid1', 'uuid2', 'uuid3'],
  tag: 'archive'
});
```

### bulk_delete

Delete multiple files from index.

```javascript
const status = await invoke('bulk_delete', {
  memoryIds: ['uuid1', 'uuid2']
});
```

### open_file

Open file with default application.

```javascript
const status = await invoke('open_file', {
  path: '/Users/john/Documents/file.pdf'
});
```

### open_in_finder

Reveal file in Finder/Explorer.

```javascript
const status = await invoke('open_in_finder', {
  path: '/Users/john/Documents/file.pdf'
});
```

### reset_index

Clear all data and reset.

```javascript
const status = await invoke('reset_index');
```

---

## Data Models

### Memory

Core file/memory type.

```rust
pub struct Memory {
    /// Unique identifier
    pub id: MemoryId,  // UUID

    /// Full path to the file
    pub path: PathBuf,

    /// Source this memory came from
    pub source: Source,

    /// Kind of file (Image, Video, Code, etc.)
    pub kind: MemoryKind,

    /// File metadata
    pub metadata: MemoryMetadata,

    /// Tags associated with this memory
    pub tags: Vec<Tag>,

    /// Embedding ID in vector database (optional)
    pub embedding_id: Option<String>,

    /// Connections to other memories
    pub connections: Vec<Connection>,

    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}
```

### MemoryKind

File type variants.

```rust
pub enum MemoryKind {
    Image { width: u32, height: u32, format: String },
    Video { duration_ms: u64, format: String },
    Audio { duration_ms: u64, format: String },
    Document { format: String, page_count: Option<u32> },
    Code { language: String, lines: usize },
    Spreadsheet { format: String },
    Presentation { format: String },
    Archive { format: String },
    Database,
    Folder,
    Unknown,
}
```

### Source

Where files come from.

```rust
pub enum Source {
    Local { root_path: PathBuf },
    GoogleDrive { account_id: String },  // Stub
    ICloud { account_id: String },       // Stub
    Dropbox { account_id: String },      // Stub
    OneDrive { account_id: String },     // Stub
    S3 { bucket: String, region: String }, // Stub
    Custom { name: String },
}
```

### Tag

Tag with source and confidence.

```rust
pub struct Tag {
    pub name: String,
    pub source: TagSource,
    pub confidence: Option<f32>,
}

pub enum TagSource {
    User,      // Manually added
    Ai,        // Generated by AI
    System,    // Auto-generated by system
    Imported,  // Imported from external source
}
```

---

## Error Handling

Hippo uses the `Result<T, HippoError>` pattern.

```rust
pub type Result<T> = std::result::Result<T, HippoError>;

pub enum HippoError {
    Storage(String),
    Indexer(String),
    Search(String),
    Embeddings(String),
    Io(std::io::Error),
    // ... more variants
}
```

**Example**:

```rust
match hippo.search("query").await {
    Ok(results) => {
        println!("Found {} results", results.memories.len());
    }
    Err(HippoError::Search(msg)) => {
        eprintln!("Search failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

---

## Next Steps

- [Architecture Guide](architecture) - Understand how it works
- [CLI Guide](cli-guide) - Command-line usage
- [Desktop App Guide](desktop-app) - GUI features
- [Contributing](contributing) - Help improve Hippo

---

For questions or issues, visit [GitHub Issues](https://github.com/greplabs/hippo/issues).
