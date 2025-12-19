//! Auto-organization with virtual file maps
//!
//! This module provides intelligent file organization without physically moving files.
//! It creates "virtual collections" - logical groupings based on:
//! - Content similarity (using embeddings)
//! - Temporal proximity (events, projects)
//! - Location clustering (places, trips)
//! - Face/person grouping
//! - Topic/category classification
//!
//! Files remain in their original locations; Hippo maintains pointers and metadata
//! for organization purposes.

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::storage::Storage;
use crate::embeddings::Embedder;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

/// A virtual collection that groups related files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualCollection {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub collection_type: CollectionType,
    pub memory_ids: Vec<MemoryId>,
    pub cover_id: Option<MemoryId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub auto_generated: bool,
    pub confidence: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CollectionType {
    /// Time-based grouping (e.g., "Photos from July 2024")
    Temporal { start: DateTime<Utc>, end: DateTime<Utc> },
    /// Location-based grouping (e.g., "New York Trip")
    Location { center_lat: f64, center_lon: f64, radius_km: f64 },
    /// Content similarity grouping (e.g., "Beach Photos")
    Topic { keywords: Vec<String> },
    /// Project-based grouping for code/documents
    Project { root_path: Option<PathBuf> },
    /// Person/face-based grouping
    Person { face_cluster_id: String },
    /// Custom user-defined collection
    Custom,
    /// Smart album based on multiple criteria
    SmartAlbum { rules: Vec<SmartRule> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmartRule {
    pub field: String,
    pub operator: RuleOperator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    Between,
    InList,
}

/// A file pointer that references a file's location and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePointer {
    pub memory_id: MemoryId,
    pub original_path: PathBuf,
    pub virtual_paths: Vec<VirtualPath>,
    pub collections: Vec<uuid::Uuid>,
    pub suggested_location: Option<PathBuf>,
    pub organization_score: f32,
}

/// A virtual path representing where a file "belongs" logically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualPath {
    pub path: PathBuf,
    pub reason: OrganizationReason,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrganizationReason {
    DateBased { date: DateTime<Utc> },
    LocationBased { place: String },
    ContentSimilarity { topic: String },
    ProjectMembership { project: String },
    UserDefined,
}

/// The file map that tracks virtual organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMap {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pointers: HashMap<MemoryId, FilePointer>,
    pub collections: HashMap<uuid::Uuid, VirtualCollection>,
    pub path_index: HashMap<String, Vec<MemoryId>>, // virtual path -> memories
}

impl Default for FileMap {
    fn default() -> Self {
        Self {
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            pointers: HashMap::new(),
            collections: HashMap::new(),
            path_index: HashMap::new(),
        }
    }
}

/// The auto-organization engine
pub struct Organizer {
    storage: Arc<Storage>,
    embedder: Arc<Embedder>,
    file_map: tokio::sync::RwLock<FileMap>,
    config: OrganizerConfig,
}

#[derive(Debug, Clone)]
pub struct OrganizerConfig {
    /// Minimum similarity score for content-based grouping
    pub similarity_threshold: f32,
    /// Time window for temporal clustering (in hours)
    pub temporal_window_hours: i64,
    /// Distance threshold for location clustering (in km)
    pub location_radius_km: f64,
    /// Minimum items for a collection to be created
    pub min_collection_size: usize,
    /// Enable automatic organization on new files
    pub auto_organize: bool,
    /// Date format for temporal organization
    pub date_format: String,
}

impl Default for OrganizerConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.75,
            temporal_window_hours: 24,
            location_radius_km: 50.0,
            min_collection_size: 3,
            auto_organize: true,
            date_format: "%Y/%m".to_string(),
        }
    }
}

impl Organizer {
    pub fn new(storage: Arc<Storage>, embedder: Arc<Embedder>) -> Self {
        Self {
            storage,
            embedder,
            file_map: tokio::sync::RwLock::new(FileMap::default()),
            config: OrganizerConfig::default(),
        }
    }

    pub fn with_config(mut self, config: OrganizerConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a memory to the organization system
    pub async fn add_memory(&self, memory: &Memory) -> Result<FilePointer> {
        let mut pointer = FilePointer {
            memory_id: memory.id,
            original_path: memory.path.clone(),
            virtual_paths: Vec::new(),
            collections: Vec::new(),
            suggested_location: None,
            organization_score: 0.0,
        };

        // Generate virtual paths based on different criteria
        pointer.virtual_paths.extend(self.generate_date_paths(memory));
        pointer.virtual_paths.extend(self.generate_location_paths(memory));
        pointer.virtual_paths.extend(self.generate_type_paths(memory));

        // Calculate suggested location
        pointer.suggested_location = self.suggest_location(memory, &pointer.virtual_paths);
        pointer.organization_score = self.calculate_organization_score(&pointer);

        // Store the pointer
        let mut map = self.file_map.write().await;
        map.pointers.insert(memory.id, pointer.clone());
        map.updated_at = Utc::now();

        // Update path index
        for vpath in &pointer.virtual_paths {
            let path_str = vpath.path.to_string_lossy().to_string();
            map.path_index
                .entry(path_str)
                .or_default()
                .push(memory.id);
        }

        Ok(pointer)
    }

    /// Generate date-based virtual paths
    fn generate_date_paths(&self, memory: &Memory) -> Vec<VirtualPath> {
        let mut paths = Vec::new();

        let date = memory.created_at;
        let formatted = date.format(&self.config.date_format).to_string();

        paths.push(VirtualPath {
            path: PathBuf::from(format!("By Date/{}", formatted)),
            reason: OrganizationReason::DateBased { date },
            confidence: 0.9,
        });

        // Add year grouping
        paths.push(VirtualPath {
            path: PathBuf::from(format!("By Year/{}", date.format("%Y"))),
            reason: OrganizationReason::DateBased { date },
            confidence: 0.85,
        });

        paths
    }

    /// Generate location-based virtual paths
    fn generate_location_paths(&self, memory: &Memory) -> Vec<VirtualPath> {
        let mut paths = Vec::new();

        if let Some(ref location) = memory.metadata.location {
            // Use place name if available
            if let Some(ref place) = location.place_name {
                paths.push(VirtualPath {
                    path: PathBuf::from(format!("By Location/{}", place)),
                    reason: OrganizationReason::LocationBased {
                        place: place.clone(),
                    },
                    confidence: 0.9,
                });
            }

            // Use city/country if available
            if let Some(ref city) = location.city {
                if let Some(ref country) = location.country {
                    paths.push(VirtualPath {
                        path: PathBuf::from(format!("By Location/{}/{}", country, city)),
                        reason: OrganizationReason::LocationBased {
                            place: format!("{}, {}", city, country),
                        },
                        confidence: 0.85,
                    });
                }
            }
        }

        paths
    }

    /// Generate type-based virtual paths
    fn generate_type_paths(&self, memory: &Memory) -> Vec<VirtualPath> {
        let mut paths = Vec::new();

        let type_name = match &memory.kind {
            MemoryKind::Image { format, .. } => format!("Images/{}", format.to_uppercase()),
            MemoryKind::Video { format, .. } => format!("Videos/{}", format.to_uppercase()),
            MemoryKind::Audio { format, .. } => format!("Audio/{}", format.to_uppercase()),
            MemoryKind::Document { format, .. } => {
                let fmt_name = match format {
                    DocumentFormat::Pdf => "PDF",
                    DocumentFormat::Word => "Word",
                    DocumentFormat::Markdown => "Markdown",
                    DocumentFormat::PlainText => "Text",
                    DocumentFormat::Html => "HTML",
                    DocumentFormat::Rtf => "RTF",
                    DocumentFormat::Other(s) => s.as_str(),
                };
                format!("Documents/{}", fmt_name)
            }
            MemoryKind::Code { language, .. } => format!("Code/{}", language),
            MemoryKind::Spreadsheet { .. } => "Spreadsheets".to_string(),
            MemoryKind::Presentation { .. } => "Presentations".to_string(),
            MemoryKind::Archive { .. } => "Archives".to_string(),
            MemoryKind::Database => "Databases".to_string(),
            MemoryKind::Folder => "Folders".to_string(),
            MemoryKind::Unknown => "Other".to_string(),
        };

        paths.push(VirtualPath {
            path: PathBuf::from(format!("By Type/{}", type_name)),
            reason: OrganizationReason::ContentSimilarity {
                topic: type_name.clone(),
            },
            confidence: 0.95,
        });

        paths
    }

    /// Suggest an ideal location for the file
    fn suggest_location(
        &self,
        _memory: &Memory,
        virtual_paths: &[VirtualPath],
    ) -> Option<PathBuf> {
        // Find the highest confidence virtual path
        virtual_paths
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|vp| vp.path.clone())
    }

    /// Calculate how well-organized a file is
    fn calculate_organization_score(&self, pointer: &FilePointer) -> f32 {
        let has_date = pointer
            .virtual_paths
            .iter()
            .any(|p| matches!(p.reason, OrganizationReason::DateBased { .. }));
        let has_location = pointer
            .virtual_paths
            .iter()
            .any(|p| matches!(p.reason, OrganizationReason::LocationBased { .. }));
        let has_type = pointer
            .virtual_paths
            .iter()
            .any(|p| matches!(p.reason, OrganizationReason::ContentSimilarity { .. }));
        let in_collections = !pointer.collections.is_empty();

        let mut score = 0.0f32;
        if has_date { score += 0.25; }
        if has_location { score += 0.25; }
        if has_type { score += 0.25; }
        if in_collections { score += 0.25; }

        score
    }

    /// Find files that match a virtual path pattern
    pub async fn find_by_virtual_path(&self, pattern: &str) -> Result<Vec<MemoryId>> {
        let map = self.file_map.read().await;
        let pattern_lower = pattern.to_lowercase();

        let mut results = Vec::new();
        for (path, ids) in &map.path_index {
            if path.to_lowercase().contains(&pattern_lower) {
                results.extend(ids.clone());
            }
        }

        // Deduplicate
        let unique: HashSet<_> = results.into_iter().collect();
        Ok(unique.into_iter().collect())
    }

    /// Create a temporal collection from memories in a time range
    pub async fn create_temporal_collection(
        &self,
        name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<VirtualCollection> {
        let map = self.file_map.read().await;

        let memory_ids: Vec<MemoryId> = map
            .pointers
            .iter()
            .filter(|(_, p)| {
                p.virtual_paths.iter().any(|vp| {
                    if let OrganizationReason::DateBased { date } = &vp.reason {
                        *date >= start && *date <= end
                    } else {
                        false
                    }
                })
            })
            .map(|(id, _)| *id)
            .collect();

        drop(map);

        let collection = VirtualCollection {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            description: Some(format!(
                "Files from {} to {}",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            )),
            collection_type: CollectionType::Temporal { start, end },
            memory_ids: memory_ids.clone(),
            cover_id: memory_ids.first().copied(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            auto_generated: true,
            confidence: 0.9,
            metadata: HashMap::new(),
        };

        let mut map = self.file_map.write().await;
        map.collections.insert(collection.id, collection.clone());

        // Update pointers
        for id in &collection.memory_ids {
            if let Some(pointer) = map.pointers.get_mut(id) {
                pointer.collections.push(collection.id);
            }
        }

        Ok(collection)
    }

    /// Auto-discover collections from content similarity
    pub async fn discover_topic_collections(&self) -> Result<Vec<VirtualCollection>> {
        let mut collections = Vec::new();

        // Group by AI-generated tags/scene tags
        let map = self.file_map.read().await;
        let mut tag_groups: HashMap<String, Vec<MemoryId>> = HashMap::new();

        for (id, pointer) in &map.pointers {
            if let Some(memory) = self.storage.get_memory(*id).await? {
                for tag in &memory.metadata.ai_tags {
                    tag_groups.entry(tag.clone()).or_default().push(*id);
                }
                for tag in &memory.metadata.scene_tags {
                    tag_groups.entry(tag.clone()).or_default().push(*id);
                }
            }
        }

        drop(map);

        // Create collections for groups that meet minimum size
        for (tag, ids) in tag_groups {
            if ids.len() >= self.config.min_collection_size {
                let collection = VirtualCollection {
                    id: uuid::Uuid::new_v4(),
                    name: format!("{} Collection", capitalize_first(&tag)),
                    description: Some(format!("Files tagged with '{}'", tag)),
                    collection_type: CollectionType::Topic {
                        keywords: vec![tag.clone()],
                    },
                    memory_ids: ids.clone(),
                    cover_id: ids.first().copied(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    auto_generated: true,
                    confidence: 0.8,
                    metadata: HashMap::new(),
                };

                let mut map = self.file_map.write().await;
                map.collections.insert(collection.id, collection.clone());
                drop(map);

                collections.push(collection);
            }
        }

        Ok(collections)
    }

    /// Find similar files and suggest groupings
    pub async fn suggest_groupings(&self, memory_id: MemoryId) -> Result<Vec<VirtualCollection>> {
        let similar = self.storage.find_similar(memory_id, 20).await?;
        let mut suggestions = Vec::new();

        if similar.len() >= self.config.min_collection_size {
            let ids: Vec<_> = similar.iter().map(|(id, _)| *id).collect();
            let collection = VirtualCollection {
                id: uuid::Uuid::new_v4(),
                name: "Similar Files".to_string(),
                description: Some("Files with similar content".to_string()),
                collection_type: CollectionType::Topic {
                    keywords: vec!["similar".to_string()],
                },
                memory_ids: ids,
                cover_id: Some(memory_id),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                auto_generated: true,
                confidence: 0.7,
                metadata: HashMap::new(),
            };
            suggestions.push(collection);
        }

        Ok(suggestions)
    }

    /// Get all virtual paths for a memory
    pub async fn get_virtual_paths(&self, memory_id: MemoryId) -> Result<Vec<VirtualPath>> {
        let map = self.file_map.read().await;
        Ok(map
            .pointers
            .get(&memory_id)
            .map(|p| p.virtual_paths.clone())
            .unwrap_or_default())
    }

    /// Get all collections a memory belongs to
    pub async fn get_collections_for_memory(
        &self,
        memory_id: MemoryId,
    ) -> Result<Vec<VirtualCollection>> {
        let map = self.file_map.read().await;

        let collection_ids = map
            .pointers
            .get(&memory_id)
            .map(|p| p.collections.clone())
            .unwrap_or_default();

        Ok(collection_ids
            .iter()
            .filter_map(|id| map.collections.get(id).cloned())
            .collect())
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<VirtualCollection>> {
        let map = self.file_map.read().await;
        Ok(map.collections.values().cloned().collect())
    }

    /// Create a custom collection
    pub async fn create_collection(
        &self,
        name: &str,
        description: Option<String>,
        memory_ids: Vec<MemoryId>,
    ) -> Result<VirtualCollection> {
        let collection = VirtualCollection {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            description,
            collection_type: CollectionType::Custom,
            memory_ids: memory_ids.clone(),
            cover_id: memory_ids.first().copied(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            auto_generated: false,
            confidence: 1.0,
            metadata: HashMap::new(),
        };

        let mut map = self.file_map.write().await;
        map.collections.insert(collection.id, collection.clone());

        // Update pointers
        for id in &collection.memory_ids {
            if let Some(pointer) = map.pointers.get_mut(id) {
                pointer.collections.push(collection.id);
            }
        }

        Ok(collection)
    }

    /// Add memories to an existing collection
    pub async fn add_to_collection(
        &self,
        collection_id: uuid::Uuid,
        memory_ids: Vec<MemoryId>,
    ) -> Result<()> {
        let mut map = self.file_map.write().await;

        if let Some(collection) = map.collections.get_mut(&collection_id) {
            for id in &memory_ids {
                if !collection.memory_ids.contains(id) {
                    collection.memory_ids.push(*id);
                }
            }
            collection.updated_at = Utc::now();

            // Update pointers
            for id in &memory_ids {
                if let Some(pointer) = map.pointers.get_mut(id) {
                    if !pointer.collections.contains(&collection_id) {
                        pointer.collections.push(collection_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove a collection
    pub async fn remove_collection(&self, collection_id: uuid::Uuid) -> Result<()> {
        let mut map = self.file_map.write().await;

        if let Some(collection) = map.collections.remove(&collection_id) {
            // Update pointers to remove collection reference
            for id in &collection.memory_ids {
                if let Some(pointer) = map.pointers.get_mut(id) {
                    pointer.collections.retain(|c| *c != collection_id);
                }
            }
        }

        Ok(())
    }

    /// Get organization summary/statistics
    pub async fn get_organization_stats(&self) -> Result<OrganizationStats> {
        let map = self.file_map.read().await;

        let total_files = map.pointers.len();
        let total_collections = map.collections.len();
        let auto_collections = map
            .collections
            .values()
            .filter(|c| c.auto_generated)
            .count();

        let avg_organization_score = if total_files > 0 {
            map.pointers.values().map(|p| p.organization_score).sum::<f32>() / total_files as f32
        } else {
            0.0
        };

        let files_in_collections = map
            .pointers
            .values()
            .filter(|p| !p.collections.is_empty())
            .count();

        Ok(OrganizationStats {
            total_files,
            total_collections,
            auto_generated_collections: auto_collections,
            custom_collections: total_collections - auto_collections,
            files_in_collections,
            average_organization_score: avg_organization_score,
            virtual_paths_count: map.path_index.len(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationStats {
    pub total_files: usize,
    pub total_collections: usize,
    pub auto_generated_collections: usize,
    pub custom_collections: usize,
    pub files_in_collections: usize,
    pub average_organization_score: f32,
    pub virtual_paths_count: usize,
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("beach"), "Beach");
        assert_eq!(capitalize_first("SUNSET"), "SUNSET");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    #[test]
    fn test_file_map_default() {
        let map = FileMap::default();
        assert_eq!(map.version, 1);
        assert!(map.pointers.is_empty());
        assert!(map.collections.is_empty());
        assert!(map.path_index.is_empty());
    }

    #[test]
    fn test_organizer_config_default() {
        let config = OrganizerConfig::default();
        assert_eq!(config.similarity_threshold, 0.75);
        assert_eq!(config.temporal_window_hours, 24);
        assert_eq!(config.location_radius_km, 50.0);
        assert_eq!(config.min_collection_size, 3);
        assert!(config.auto_organize);
        assert_eq!(config.date_format, "%Y/%m");
    }

    #[test]
    fn test_virtual_collection_serialization() {
        let collection = VirtualCollection {
            id: uuid::Uuid::new_v4(),
            name: "Test Collection".to_string(),
            description: Some("A test collection".to_string()),
            collection_type: CollectionType::Custom,
            memory_ids: vec![],
            cover_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            auto_generated: false,
            confidence: 1.0,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&collection).unwrap();
        let parsed: VirtualCollection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test Collection");
        assert_eq!(parsed.confidence, 1.0);
    }

    #[test]
    fn test_collection_type_serialization() {
        let temporal = CollectionType::Temporal {
            start: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap(),
        };
        let json = serde_json::to_string(&temporal).unwrap();
        assert!(json.contains("Temporal"));

        let location = CollectionType::Location {
            center_lat: 40.7128,
            center_lon: -74.0060,
            radius_km: 10.0,
        };
        let json = serde_json::to_string(&location).unwrap();
        assert!(json.contains("Location"));

        let topic = CollectionType::Topic {
            keywords: vec!["beach".to_string(), "sunset".to_string()],
        };
        let json = serde_json::to_string(&topic).unwrap();
        assert!(json.contains("Topic"));
    }

    #[test]
    fn test_virtual_path_serialization() {
        let vpath = VirtualPath {
            path: PathBuf::from("By Date/2024/01"),
            reason: OrganizationReason::DateBased {
                date: Utc::now(),
            },
            confidence: 0.9,
        };

        let json = serde_json::to_string(&vpath).unwrap();
        let parsed: VirtualPath = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.confidence, 0.9);
    }

    #[test]
    fn test_file_pointer_serialization() {
        let pointer = FilePointer {
            memory_id: uuid::Uuid::new_v4(),
            original_path: PathBuf::from("/Users/test/photo.jpg"),
            virtual_paths: vec![],
            collections: vec![],
            suggested_location: Some(PathBuf::from("By Date/2024/01")),
            organization_score: 0.5,
        };

        let json = serde_json::to_string(&pointer).unwrap();
        let parsed: FilePointer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.organization_score, 0.5);
    }

    #[test]
    fn test_smart_rule() {
        let rule = SmartRule {
            field: "extension".to_string(),
            operator: RuleOperator::Equals,
            value: "jpg".to_string(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let parsed: SmartRule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.field, "extension");
        assert_eq!(parsed.operator, RuleOperator::Equals);
    }

    #[test]
    fn test_organization_stats_serialization() {
        let stats = OrganizationStats {
            total_files: 100,
            total_collections: 5,
            auto_generated_collections: 3,
            custom_collections: 2,
            files_in_collections: 80,
            average_organization_score: 0.75,
            virtual_paths_count: 200,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: OrganizationStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_files, 100);
        assert_eq!(parsed.average_organization_score, 0.75);
    }
}
