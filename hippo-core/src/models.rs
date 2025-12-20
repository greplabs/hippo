//! Core data models for Hippo
//!
//! Everything in Hippo is a `Memory` - a file, folder, or derived artifact
//! that can be searched, tagged, and connected to other memories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for any memory
pub type MemoryId = Uuid;

/// A Memory represents any indexed item in Hippo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub path: PathBuf,
    pub source: Source,
    pub kind: MemoryKind,
    pub metadata: MemoryMetadata,
    pub tags: Vec<Tag>,
    pub embedding_id: Option<String>, // Reference to vector in Qdrant
    pub connections: Vec<Connection>,
    pub is_favorite: bool, // User starred/favorited this file
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

impl Memory {
    pub fn new(path: PathBuf, source: Source, kind: MemoryKind) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            path,
            source,
            kind,
            metadata: MemoryMetadata::default(),
            tags: Vec::new(),
            embedding_id: None,
            connections: Vec::new(),
            is_favorite: false,
            created_at: now,
            modified_at: now,
            indexed_at: now,
        }
    }
}

/// The type/category of a memory
/// The type/category of a memory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryKind {
    // Media
    Image {
        width: u32,
        height: u32,
        format: String,
    },
    Video {
        duration_ms: u64,
        format: String,
    }, // Changed to ms as u64
    Audio {
        duration_ms: u64,
        format: String,
    }, // Changed to ms as u64

    // Documents
    Document {
        format: DocumentFormat,
        page_count: Option<u32>,
    },
    Spreadsheet {
        sheet_count: u32,
    },
    Presentation {
        slide_count: u32,
    },

    // Code
    Code {
        language: String,
        lines: u32,
    },

    // Data
    Archive {
        item_count: u32,
    },
    Database,

    // Other
    Folder,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DocumentFormat {
    Pdf,
    Word,
    Markdown,
    PlainText,
    Html,
    Rtf,
    Other(String),
}

/// Where a memory originates from
/// Where a memory originates from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Source {
    Local { root_path: PathBuf },
    GoogleDrive { account_id: String },
    ICloud { account_id: String },
    Dropbox { account_id: String },
    OneDrive { account_id: String },
    S3 { bucket: String, region: String },
    Custom { name: String }, // Simplified - config stored elsewhere
}

impl Source {
    pub fn display_name(&self) -> &str {
        match self {
            Source::Local { .. } => "Local",
            Source::GoogleDrive { .. } => "Google Drive",
            Source::ICloud { .. } => "iCloud",
            Source::Dropbox { .. } => "Dropbox",
            Source::OneDrive { .. } => "OneDrive",
            Source::S3 { .. } => "Amazon S3",
            Source::Custom { name } => name,
        }
    }

    pub fn icon_name(&self) -> &str {
        match self {
            Source::Local { .. } => "device",
            Source::GoogleDrive { .. } => "google-drive",
            Source::ICloud { .. } => "apple",
            Source::Dropbox { .. } => "dropbox",
            Source::OneDrive { .. } => "onedrive",
            Source::S3 { .. } => "aws",
            Source::Custom { .. } => "cloud",
        }
    }
}

/// Rich metadata extracted from files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMetadata {
    // Common
    pub title: Option<String>,
    pub description: Option<String>,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub hash: Option<String>, // Content hash for deduplication

    // Image/Video specific
    pub exif: Option<ExifData>,
    pub dimensions: Option<(u32, u32)>,
    pub duration: Option<f64>,

    // Location
    pub location: Option<GeoLocation>,

    // People (face clusters, not recognition)
    pub face_cluster_ids: Vec<String>,

    // Document specific
    pub text_preview: Option<String>, // First ~500 chars
    pub word_count: Option<u32>,

    // Code specific
    pub code_info: Option<CodeInfo>,

    // AI-generated
    pub ai_summary: Option<String>,
    pub ai_tags: Vec<String>,
    pub scene_tags: Vec<String>, // beach, city, food, etc.
    pub ai_caption: Option<String>, // Vision model generated caption for images

    // Custom fields
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub focal_length: Option<f32>,
    pub aperture: Option<f32>,
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>,
    pub taken_at: Option<DateTime<Utc>>,
    pub gps: Option<GeoLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub place_name: Option<String>, // Reverse geocoded
    pub city: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeInfo {
    pub language: String,
    pub lines_of_code: u32,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub functions: Vec<FunctionInfo>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

/// A tag that can be applied to memories
/// A tag that can be applied to memories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub name: String,
    pub source: TagSource,
    pub confidence: Option<u8>, // 0-100 percentage for AI tags
}

impl Tag {
    pub fn user(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: TagSource::User,
            confidence: None,
        }
    }

    pub fn ai(name: impl Into<String>, confidence: u8) -> Self {
        Self {
            name: name.into(),
            source: TagSource::Ai,
            confidence: Some(confidence.min(100)),
        }
    }

    pub fn system(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: TagSource::System,
            confidence: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TagSource {
    User,     // Manually added
    Ai,       // AI-suggested and accepted
    System,   // Auto-derived (file type, folder name, etc.)
    Imported, // From file metadata
}

/// A connection between two memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub target_id: MemoryId,
    pub kind: ConnectionKind,
    pub strength: f32, // 0.0 - 1.0
    pub bidirectional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionKind {
    // Explicit
    SameFolder,
    SameAlbum,
    LinkedInDocument,

    // Code-specific
    Imports,
    ImportedBy,
    References,

    // AI-derived
    SimilarContent, // High vector similarity
    SameEvent,      // Temporal + location clustering
    SamePerson,     // Face cluster match
    SameProject,    // Inferred project grouping

    // User-defined
    Custom(String),
}

/// A cluster of related memories (album, project, event)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: Uuid,
    pub name: String,
    pub kind: ClusterKind,
    pub memory_ids: Vec<MemoryId>,
    pub cover_memory_id: Option<MemoryId>,
    pub auto_generated: bool,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClusterKind {
    Album,
    Project,
    Event,
    Person,
    Location,
    Custom(String),
}

/// Search query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,   // Semantic search text
    pub tags: Vec<TagFilter>,   // Tag filters
    pub sources: Vec<Source>,   // Filter by source
    pub kinds: Vec<MemoryKind>, // Filter by type
    pub date_range: Option<DateRange>,
    pub location: Option<LocationFilter>,
    pub sort: SortOrder,
    pub limit: usize,
    pub offset: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            tags: Vec::new(),
            sources: Vec::new(),
            kinds: Vec::new(),
            date_range: None,
            location: None,
            sort: SortOrder::Relevance,
            limit: 50,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagFilter {
    pub tag: String,
    pub mode: TagFilterMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagFilterMode {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationFilter {
    pub latitude: f64,
    pub longitude: f64,
    pub radius_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Relevance,
    DateNewest,
    DateOldest,
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub memories: Vec<MemorySearchResult>,
    pub total_count: usize,
    pub suggested_tags: Vec<String>,
    pub clusters: Vec<Cluster>, // Related clusters
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub memory: Memory,
    pub score: f32,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub field: String,
    pub snippet: String,
}

/// Configuration for a source connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub source: Source,
    pub enabled: bool,
    pub sync_interval_secs: u64,
    pub last_sync: Option<DateTime<Utc>>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

/// Statistics about the Hippo index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_memories: u64,
    pub by_kind: HashMap<String, u64>,
    pub by_source: HashMap<String, u64>,
    pub total_size_bytes: u64,
    pub index_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub memory_count: usize,
    pub source_count: usize,
    pub tag_count: usize,
    pub cluster_count: usize,
}
