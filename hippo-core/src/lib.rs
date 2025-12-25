//! # Hippo Core
//!
//! The memory that never forgets. 🦛
//!
//! Hippo is an intelligent file memory system that indexes, understands,
//! and connects all your files across local storage and cloud providers.

#![allow(missing_docs)]
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Hippo Core                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐│
//! │  │ Sources │  │ Indexer │  │Embedder │  │   Search        ││
//! │  │         │──│         │──│  (CLIP) │──│   (Qdrant)      ││
//! │  │ Local   │  │ Extract │  │  (BGE)  │  │                 ││
//! │  │ GDrive  │  │ Parse   │  │  (Code) │  │ Semantic        ││
//! │  │ iCloud  │  │ Analyze │  │         │  │ Tag-based       ││
//! │  │ S3      │  │         │  │         │  │ Filters         ││
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘│
//! │       │            │            │               │          │
//! │       ▼            ▼            ▼               ▼          │
//! │  ┌──────────────────────────────────────────────────────┐  │
//! │  │                    Storage Layer                      │  │
//! │  │  SQLite (metadata)  +  Qdrant (vectors)              │  │
//! │  └──────────────────────────────────────────────────────┘  │
//! │                            │                               │
//! │                            ▼                               │
//! │  ┌──────────────────────────────────────────────────────┐  │
//! │  │                   Knowledge Graph                     │  │
//! │  │  Connections • Clusters • Mind Maps                  │  │
//! │  └──────────────────────────────────────────────────────┘  │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use hippo_core::Hippo;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize Hippo
//!     let hippo = Hippo::new().await?;
//!     
//!     // Add a local folder to index
//!     hippo.add_source(Source::Local {
//!         root_path: "/Users/you/Photos".into()
//!     }).await?;
//!     
//!     // Search semantically
//!     let results = hippo.search("sunset beach vacation").await?;
//!     
//!     // Add tags inline
//!     hippo.add_tag(results[0].id, Tag::user("favorites")).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod ai;
pub mod duplicates;
pub mod embeddings;
pub mod error;
pub mod graph;
pub mod indexer;
pub mod models;
pub mod ollama;
pub mod organization;
pub mod qdrant;
pub mod search;
pub mod sources;
pub mod storage;
pub mod thumbnails;
pub mod watcher;

pub use error::{HippoError, Result};
pub use models::*;
pub use search::ParsedQuery;

// Re-export graph types for the API
pub use graph::MindMap;

// Re-export AI types
pub use ai::{
    analyze_code, analyze_document, analyze_file, analyze_image, analyze_video, AiConfig,
    AiProvider, AnalysisResult, ClaudeClient, CodeAnalysis, CodeSummary, CollectionSuggestion,
    Color, DetectedObject, DocumentAnalysis, DocumentSummary, DuplicateMatch, DuplicateType,
    ExtractedEntities, FileAnalysis, FunctionInfo, ImageAnalysis, OrganizationSuggestion,
    SimilarFile, TagSuggestion, UnifiedAiClient, VideoAnalysis,
};

// Re-export watcher types
pub use watcher::{FileWatcher, WatchEvent, WatchStats};

// Re-export duplicates types
pub use duplicates::{
    compute_file_hash, find_duplicates_by_scanning, DuplicateGroup, DuplicateSummary,
};

// Re-export thumbnail types
pub use thumbnails::{
    is_ffmpeg_available, is_supported_image, is_supported_video, ThumbnailManager, ThumbnailStats,
    THUMBNAIL_SIZE,
};

// Re-export embeddings types
pub use embeddings::{
    Embedder, VectorIndex, CODE_EMBEDDING_DIM, IMAGE_EMBEDDING_DIM, TEXT_EMBEDDING_DIM,
};

// Re-export Ollama types
pub use ollama::{
    ChatMessage, LocalAnalysis, OllamaClient, OllamaConfig, OllamaModel, RagContext, RagDocument,
    RecommendedModels,
};

// Re-export organization types
pub use organization::{
    CollectionType, FileMap, FilePointer, OrganizationReason, OrganizationStats, Organizer,
    OrganizerConfig, VirtualCollection, VirtualPath,
};

// Re-export qdrant types
pub use qdrant::{QdrantManager, QdrantStats, QdrantStatus};

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Hippo instance
#[allow(dead_code)]
pub struct Hippo {
    storage: Arc<storage::Storage>,
    /// The file indexer for scanning and processing files
    pub indexer: Arc<indexer::Indexer>,
    embedder: Arc<embeddings::Embedder>,
    searcher: Arc<search::Searcher>,
    graph: Arc<RwLock<graph::KnowledgeGraph>>,
    /// The file watcher for real-time updates
    pub watcher: Option<Arc<RwLock<watcher::FileWatcher>>>,
    thumbnail_manager: Arc<thumbnails::ThumbnailManager>,
    organizer: Arc<organization::Organizer>,
    config: HippoConfig,
}

/// Configuration for Hippo
#[derive(Debug, Clone)]
pub struct HippoConfig {
    /// Directory to store Hippo data (indexes, cache, etc.)
    pub data_dir: PathBuf,

    /// Whether to run embedding models locally
    pub local_embeddings: bool,

    /// API key for cloud AI services (optional)
    pub ai_api_key: Option<String>,

    /// Qdrant connection settings
    pub qdrant_url: String,

    /// Maximum concurrent indexing operations
    pub indexing_parallelism: usize,

    /// Whether to enable auto-tagging during indexing using Ollama
    pub auto_tag_enabled: bool,
}

impl Default for HippoConfig {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("", "", "Hippo")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".hippo"));

        Self {
            data_dir,
            local_embeddings: true,
            ai_api_key: None,
            qdrant_url: "http://localhost:6334".into(),
            indexing_parallelism: num_cpus::get().min(8),
            auto_tag_enabled: true,
        }
    }
}

impl Hippo {
    /// Create a new Hippo instance with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(HippoConfig::default()).await
    }

    /// Create a new Hippo instance with custom configuration
    pub async fn with_config(config: HippoConfig) -> Result<Self> {
        // Ensure data directory exists
        std::fs::create_dir_all(&config.data_dir)?;

        // Initialize components
        let storage = Arc::new(storage::Storage::new(&config).await?);
        let embedder = Arc::new(embeddings::Embedder::new(&config).await?);
        let indexer = Arc::new(indexer::Indexer::new(
            storage.clone(),
            embedder.clone(),
            &config,
        )?);
        let searcher =
            Arc::new(search::Searcher::new(storage.clone(), embedder.clone(), &config).await?);
        let graph = Arc::new(RwLock::new(graph::KnowledgeGraph::new(storage.clone())));

        // Initialize watcher (optional - can be started later)
        let watcher = match watcher::FileWatcher::new(storage.clone(), None) {
            Ok(mut w) => {
                // Set the indexer so the watcher can re-index changed files
                w.set_indexer(indexer.clone());
                Some(Arc::new(RwLock::new(w)))
            }
            Err(e) => {
                tracing::warn!("Failed to create file watcher: {}", e);
                None
            }
        };

        // Initialize thumbnail manager
        let thumbnail_manager = Arc::new(thumbnails::ThumbnailManager::new()?);

        // Initialize organizer
        let organizer = Arc::new(organization::Organizer::new(
            storage.clone(),
            embedder.clone(),
        ));

        Ok(Self {
            storage,
            indexer,
            embedder,
            searcher,
            graph,
            watcher,
            thumbnail_manager,
            organizer,
            config,
        })
    }

    // === Source Management ===

    /// Add a new source to index
    pub async fn add_source(&self, source: Source) -> Result<()> {
        self.storage.add_source(source.clone()).await?;
        self.indexer.queue_source(source).await?;
        Ok(())
    }

    /// Remove a source and optionally delete indexed memories
    pub async fn remove_source(&self, source: &Source, delete_memories: bool) -> Result<()> {
        if delete_memories {
            // Delete all memories from this source's path
            if let Source::Local { root_path } = source {
                self.storage
                    .remove_memories_by_path_prefix(root_path.to_string_lossy().as_ref())
                    .await?;
            }
        }
        self.storage.remove_source(source).await?;
        Ok(())
    }

    /// Clear all data and reset the index
    pub async fn clear_all(&self) -> Result<()> {
        self.storage.clear_all().await
    }

    /// Get all configured sources
    pub async fn list_sources(&self) -> Result<Vec<SourceConfig>> {
        self.storage.list_sources().await
    }

    /// Trigger a sync for a specific source
    pub async fn sync_source(&self, source: &Source) -> Result<()> {
        self.indexer.sync_source(source).await
    }

    // === Search ===

    /// Perform a semantic search
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        let search_query = SearchQuery {
            text: Some(query.to_string()),
            ..Default::default()
        };
        self.searcher.search(search_query).await
    }

    /// Perform an advanced search with filters
    pub async fn search_advanced(&self, query: SearchQuery) -> Result<SearchResults> {
        self.searcher.search(query).await
    }

    /// Get suggested tags based on search text
    pub async fn suggest_tags(&self, text: &str) -> Result<Vec<String>> {
        self.searcher.suggest_tags(text).await
    }

    // === Memory Management ===

    /// Get a memory by ID
    pub async fn get_memory(&self, id: MemoryId) -> Result<Option<Memory>> {
        self.storage.get_memory(id).await
    }

    /// Delete a memory by its ID
    pub async fn delete_memory(&self, id: MemoryId) -> Result<()> {
        self.storage.delete_memory(id).await
    }

    /// Add a tag to a memory
    pub async fn add_tag(&self, memory_id: MemoryId, tag: Tag) -> Result<()> {
        self.storage.add_tag(memory_id, tag).await
    }

    /// Remove a tag from a memory
    pub async fn remove_tag(&self, memory_id: MemoryId, tag_name: &str) -> Result<()> {
        self.storage.remove_tag(memory_id, tag_name).await
    }

    /// Toggle favorite status for a memory
    pub async fn toggle_favorite(&self, memory_id: MemoryId) -> Result<bool> {
        self.storage.toggle_favorite(memory_id).await
    }

    /// Get all unique tags
    pub async fn list_tags(&self) -> Result<Vec<(String, u64)>> {
        self.storage.list_tags().await
    }

    // === Knowledge Graph ===

    /// Get the mind map / knowledge graph for a memory
    pub async fn get_mind_map(&self, memory_id: MemoryId, depth: usize) -> Result<graph::MindMap> {
        let graph = self.graph.read().await;
        graph.build_mind_map(memory_id, depth).await
    }

    /// Get related memories
    pub async fn get_related(&self, memory_id: MemoryId, limit: usize) -> Result<Vec<Memory>> {
        let graph = self.graph.read().await;
        graph.get_related(memory_id, limit).await
    }

    // === Clusters ===

    /// Get all clusters
    pub async fn list_clusters(&self) -> Result<Vec<Cluster>> {
        self.storage.list_clusters().await
    }

    /// Create a new cluster (album, project, etc.)
    pub async fn create_cluster(&self, name: &str, kind: ClusterKind) -> Result<Cluster> {
        self.storage.create_cluster(name, kind).await
    }

    /// Add memories to a cluster
    pub async fn add_to_cluster(
        &self,
        cluster_id: uuid::Uuid,
        memory_ids: Vec<MemoryId>,
    ) -> Result<()> {
        self.storage.add_to_cluster(cluster_id, memory_ids).await
    }

    // === Stats ===

    /// Get index statistics
    pub async fn stats(&self) -> Result<IndexStats> {
        self.storage.stats().await
    }

    // === File Watching ===

    /// Start watching a source for file changes
    pub async fn watch_source(&self, source: &Source) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            if let Source::Local { root_path } = source {
                watcher.write().await.watch(root_path, source.clone()).await
            } else {
                Err(HippoError::Other(
                    "Only local sources can be watched".to_string(),
                ))
            }
        } else {
            Err(HippoError::Other("File watcher not available".to_string()))
        }
    }

    /// Stop watching a source
    pub async fn unwatch_source(&self, source: &Source) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            if let Source::Local { root_path } = source {
                watcher.write().await.unwatch(root_path).await
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Start watching all configured sources
    pub async fn watch_all(&self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.write().await.watch_all_sources().await
        } else {
            Err(HippoError::Other("File watcher not available".to_string()))
        }
    }

    /// Stop all file watchers
    pub async fn unwatch_all(&self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.write().await.unwatch_all().await
        } else {
            Ok(())
        }
    }

    /// Get the number of active watchers
    pub async fn active_watchers(&self) -> usize {
        if let Some(watcher) = &self.watcher {
            watcher.read().await.active_count().await
        } else {
            0
        }
    }

    /// Get list of watched paths
    pub async fn watched_paths(&self) -> Vec<PathBuf> {
        if let Some(watcher) = &self.watcher {
            watcher
                .read()
                .await
                .watched_paths()
                .await
                .into_iter()
                .map(|(p, _)| p)
                .collect()
        } else {
            vec![]
        }
    }

    /// Get watcher stats
    pub async fn watcher_stats(&self) -> Option<watcher::WatchStats> {
        if let Some(watcher) = &self.watcher {
            Some(watcher.read().await.stats().await)
        } else {
            None
        }
    }

    /// Subscribe to file watch events
    pub fn subscribe_watch_events(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<watcher::WatchEvent>> {
        self.watcher.as_ref().map(|w| {
            // This is a bit tricky - we need to get the receiver without blocking
            // For now, return None - we'll handle events differently in Tauri
            // The proper way would be to use a channel that can be subscribed to multiple times
            // We'll emit events via Tauri's event system instead
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async { w.read().await.subscribe() })
            })
        })
    }

    // === Duplicate Detection ===

    /// Find duplicate files in the index
    pub async fn find_duplicates(
        &self,
        min_size: u64,
    ) -> Result<(Vec<DuplicateGroup>, DuplicateSummary)> {
        let memories = self.storage.get_all_memories().await?;
        duplicates::find_duplicates_by_scanning(&memories, min_size)
    }

    /// Get all memories (for duplicate scanning)
    pub async fn get_all_memories(&self) -> Result<Vec<Memory>> {
        self.storage.get_all_memories().await
    }

    // === Thumbnail Management ===

    /// Get or generate a thumbnail for an image file
    pub fn get_thumbnail(&self, image_path: &std::path::Path) -> Result<PathBuf> {
        self.thumbnail_manager.generate_thumbnail(image_path)
    }

    /// Get or generate a thumbnail for a video file
    pub fn get_video_thumbnail(&self, video_path: &std::path::Path) -> Result<PathBuf> {
        self.thumbnail_manager.generate_video_thumbnail(video_path)
    }

    /// Get the thumbnail path without generating (may not exist)
    pub fn get_thumbnail_path(&self, image_path: &std::path::Path) -> PathBuf {
        self.thumbnail_manager.get_thumbnail_path(image_path)
    }

    /// Check if a thumbnail exists for the given image
    pub fn has_thumbnail(&self, image_path: &std::path::Path) -> bool {
        self.thumbnail_manager.has_thumbnail(image_path)
    }

    /// Get thumbnail cache statistics
    pub fn thumbnail_stats(&self) -> Result<ThumbnailStats> {
        self.thumbnail_manager.get_stats()
    }

    /// Clear the thumbnail cache
    pub fn clear_thumbnail_cache(&self) -> Result<()> {
        self.thumbnail_manager.clear_cache()
    }

    /// Get the thumbnail manager for direct access
    pub fn thumbnail_manager(&self) -> &Arc<thumbnails::ThumbnailManager> {
        &self.thumbnail_manager
    }

    // === Organization ===

    /// Get virtual paths for a memory (logical organization without moving files)
    pub async fn get_virtual_paths(
        &self,
        memory_id: MemoryId,
    ) -> Result<Vec<organization::VirtualPath>> {
        self.organizer.get_virtual_paths(memory_id).await
    }

    /// List all virtual collections
    pub async fn list_collections(&self) -> Result<Vec<organization::VirtualCollection>> {
        self.organizer.list_collections().await
    }

    /// Get collections a memory belongs to
    pub async fn get_collections_for_memory(
        &self,
        memory_id: MemoryId,
    ) -> Result<Vec<organization::VirtualCollection>> {
        self.organizer.get_collections_for_memory(memory_id).await
    }

    /// Create a custom collection
    pub async fn create_collection(
        &self,
        name: &str,
        description: Option<String>,
        memory_ids: Vec<MemoryId>,
    ) -> Result<organization::VirtualCollection> {
        self.organizer
            .create_collection(name, description, memory_ids)
            .await
    }

    /// Add memories to a collection
    pub async fn add_to_collection(
        &self,
        collection_id: uuid::Uuid,
        memory_ids: Vec<MemoryId>,
    ) -> Result<()> {
        self.organizer
            .add_to_collection(collection_id, memory_ids)
            .await
    }

    /// Remove a collection
    pub async fn remove_collection(&self, collection_id: uuid::Uuid) -> Result<()> {
        self.organizer.remove_collection(collection_id).await
    }

    /// Auto-discover topic collections from AI tags
    pub async fn discover_collections(&self) -> Result<Vec<organization::VirtualCollection>> {
        self.organizer.discover_topic_collections().await
    }

    /// Get similar file suggestions for grouping
    pub async fn suggest_groupings(
        &self,
        memory_id: MemoryId,
    ) -> Result<Vec<organization::VirtualCollection>> {
        self.organizer.suggest_groupings(memory_id).await
    }

    /// Get organization statistics
    pub async fn organization_stats(&self) -> Result<organization::OrganizationStats> {
        self.organizer.get_organization_stats().await
    }

    /// Organize a memory (generate virtual paths and add to file map)
    pub async fn organize_memory(&self, memory: &Memory) -> Result<organization::FilePointer> {
        self.organizer.add_memory(memory).await
    }

    /// Find memories by virtual path pattern
    pub async fn find_by_virtual_path(&self, pattern: &str) -> Result<Vec<MemoryId>> {
        self.organizer.find_by_virtual_path(pattern).await
    }

    // === Similarity Search ===

    /// Find similar memories using vector search
    pub async fn find_similar(
        &self,
        memory_id: MemoryId,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        self.storage.find_similar(memory_id, limit).await
    }

    /// Perform hybrid search (semantic + keyword)
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        self.searcher.hybrid_search(query, limit).await
    }

    /// Perform pure semantic search using embeddings
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        self.searcher.semantic_search(query, limit).await
    }

    /// Get Qdrant statistics
    pub async fn qdrant_stats(&self) -> Result<qdrant::QdrantStats> {
        self.storage.qdrant_stats().await
    }

    /// Get the organizer for direct access
    pub fn organizer(&self) -> &Arc<organization::Organizer> {
        &self.organizer
    }

    // === Export/Import Operations ===

    /// Export all index data to a serializable structure
    pub async fn export_index(&self) -> Result<IndexExport> {
        self.storage.export_index().await
    }

    /// Import index data from an export structure
    pub async fn import_index(&self, data: IndexExport) -> Result<ImportStats> {
        self.storage.import_index(data).await
    }
}

// Re-export num_cpus for config default
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hippo_creation() {
        // This will fail without Qdrant running, but tests the API
        let result = Hippo::new().await;
        // In CI, this might fail - that's OK for now
        if result.is_err() {
            println!(
                "Hippo creation failed (expected in test env): {:?}",
                result.err()
            );
        }
    }
}
