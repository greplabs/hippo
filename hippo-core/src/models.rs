//! Core data models for Hippo
//!
//! Everything in Hippo is a `Memory` - a file, folder, or derived artifact
//! that can be searched, tagged, and connected to other memories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

// === Storage optimization constants ===
// These limits reduce database size by truncating large text fields

/// Maximum characters for AI-generated summary (200 chars = ~40 words)
pub const MAX_AI_SUMMARY_CHARS: usize = 200;

/// Maximum characters for AI-generated caption (150 chars = ~25 words)
pub const MAX_AI_CAPTION_CHARS: usize = 150;

/// Maximum characters for text preview (256 chars)
pub const MAX_TEXT_PREVIEW_CHARS: usize = 256;

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
    pub video_metadata: Option<VideoMetadata>,
    pub audio_metadata: Option<AudioMetadata>,

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
    pub scene_tags: Vec<String>,    // beach, city, food, etc.
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
    pub exposure_time: Option<f32>, // In seconds
    pub taken_at: Option<DateTime<Utc>>,
    pub gps: Option<GeoLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub codec: Option<String>,
    pub bitrate: Option<u64>, // bits per second
    pub framerate: Option<f32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u32>,
    pub audio_sample_rate: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub codec: Option<String>,
    pub bitrate: Option<u64>, // bits per second
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_number: Option<u32>,
    pub genre: Option<String>,
    pub year: Option<u32>,
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
/// Supports hierarchical tags with "/" separator (e.g., "project/hippo/frontend")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub name: String,
    pub source: TagSource,
    pub confidence: Option<u8>, // 0-100 percentage for AI tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>, // Parent tag name for hierarchical organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>, // Optional color for visual distinction
}

impl Tag {
    pub fn user(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let (parsed_name, parent) = Self::parse_hierarchical(&name_str);
        Self {
            name: parsed_name,
            source: TagSource::User,
            confidence: None,
            parent,
            color: None,
        }
    }

    pub fn ai(name: impl Into<String>, confidence: u8) -> Self {
        let name_str = name.into();
        let (parsed_name, parent) = Self::parse_hierarchical(&name_str);
        Self {
            name: parsed_name,
            source: TagSource::Ai,
            confidence: Some(confidence.min(100)),
            parent,
            color: None,
        }
    }

    pub fn system(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let (parsed_name, parent) = Self::parse_hierarchical(&name_str);
        Self {
            name: parsed_name,
            source: TagSource::System,
            confidence: None,
            parent,
            color: None,
        }
    }

    /// Parse a hierarchical tag path like "project/hippo/frontend"
    /// Returns (leaf_name, parent_path)
    fn parse_hierarchical(path: &str) -> (String, Option<String>) {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 1 {
            let leaf = parts.last().unwrap().to_string();
            let parent = parts[..parts.len() - 1].join("/");
            (leaf, Some(parent))
        } else {
            (path.to_string(), None)
        }
    }

    /// Get the full path of the tag (including parent hierarchy)
    pub fn full_path(&self) -> String {
        match &self.parent {
            Some(parent) => format!("{}/{}", parent, self.name),
            None => self.name.clone(),
        }
    }

    /// Check if this tag is a child of the given parent path
    pub fn is_child_of(&self, parent_path: &str) -> bool {
        match &self.parent {
            Some(parent) => parent == parent_path || parent.starts_with(&format!("{}/", parent_path)),
            None => false,
        }
    }

    /// Get the depth of this tag in the hierarchy (0 for root tags)
    pub fn depth(&self) -> usize {
        match &self.parent {
            Some(parent) => parent.matches('/').count() + 1,
            None => 0,
        }
    }

    /// Create a child tag under this tag
    pub fn child(&self, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: self.source.clone(),
            confidence: None,
            parent: Some(self.full_path()),
            color: None,
        }
    }

    /// Set a custom color for this tag
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
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

/// Export structure containing all index data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExport {
    pub version: String,
    pub export_date: DateTime<Utc>,
    pub memories: Vec<Memory>,
    pub sources: Vec<SourceConfig>,
    pub tags: Vec<(String, u64)>,
    pub clusters: Vec<Cluster>,
}

/// Import statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStats {
    pub memories_imported: usize,
    pub tags_imported: usize,
    pub sources_imported: usize,
    pub clusters_imported: usize,
    pub duplicates_skipped: usize,
    pub errors: Vec<String>,
}

/// Database vacuum statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacuumStats {
    pub size_before: u64,
    pub size_after: u64,
    pub bytes_reclaimed: u64,
}

impl VacuumStats {
    /// Get the percentage of space reclaimed
    pub fn reclaim_percentage(&self) -> f64 {
        if self.size_before == 0 {
            0.0
        } else {
            (self.bytes_reclaimed as f64 / self.size_before as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_simple() {
        let tag = Tag::user("simple");
        assert_eq!(tag.name, "simple");
        assert_eq!(tag.parent, None);
        assert_eq!(tag.full_path(), "simple");
        assert_eq!(tag.depth(), 0);
    }

    #[test]
    fn test_tag_hierarchical() {
        let tag = Tag::user("project/hippo/frontend");
        assert_eq!(tag.name, "frontend");
        assert_eq!(tag.parent, Some("project/hippo".to_string()));
        assert_eq!(tag.full_path(), "project/hippo/frontend");
        assert_eq!(tag.depth(), 2);
    }

    #[test]
    fn test_tag_is_child_of() {
        let tag = Tag::user("project/hippo/frontend");
        assert!(tag.is_child_of("project/hippo"));
        assert!(tag.is_child_of("project"));
        assert!(!tag.is_child_of("other"));
        assert!(!tag.is_child_of("frontend"));
    }

    #[test]
    fn test_tag_child_creation() {
        let parent = Tag::user("project");
        let child = parent.child("hippo");
        assert_eq!(child.name, "hippo");
        assert_eq!(child.parent, Some("project".to_string()));
        assert_eq!(child.full_path(), "project/hippo");

        let grandchild = child.child("frontend");
        assert_eq!(grandchild.name, "frontend");
        assert_eq!(grandchild.parent, Some("project/hippo".to_string()));
        assert_eq!(grandchild.full_path(), "project/hippo/frontend");
    }

    #[test]
    fn test_tag_with_color() {
        let tag = Tag::user("important").with_color("#ff0000");
        assert_eq!(tag.color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_tag_ai_hierarchical() {
        let tag = Tag::ai("category/subcategory/item", 85);
        assert_eq!(tag.name, "item");
        assert_eq!(tag.parent, Some("category/subcategory".to_string()));
        assert_eq!(tag.confidence, Some(85));
        assert_eq!(tag.source, TagSource::Ai);
    }
}
