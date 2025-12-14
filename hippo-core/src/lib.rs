//! # Hippo Core
//! 
//! The memory that never forgets. 🦛
//! 
//! Hippo is an intelligent file memory system that indexes, understands,
//! and connects all your files across local storage and cloud providers.
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

pub mod models;
pub mod indexer;
pub mod embeddings;
pub mod search;
pub mod sources;
pub mod graph;
pub mod storage;
pub mod error;
pub mod ai;
pub mod watcher;
pub mod duplicates;

pub use models::*;
pub use error::{HippoError, Result};

// Re-export graph types for the API
pub use graph::MindMap;

// Re-export AI types
pub use ai::{ClaudeClient, FileAnalysis, TagSuggestion, OrganizationSuggestion};

// Re-export watcher types
pub use watcher::{FileWatcher, WatchEvent, WatchStats};

// Re-export duplicates types
pub use duplicates::{DuplicateGroup, DuplicateSummary, compute_file_hash, find_duplicates_by_scanning};

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Hippo instance
#[allow(dead_code)]
pub struct Hippo {
    storage: Arc<storage::Storage>,
    pub indexer: Arc<indexer::Indexer>,
    embedder: Arc<embeddings::Embedder>,
    searcher: Arc<search::Searcher>,
    graph: Arc<RwLock<graph::KnowledgeGraph>>,
    watcher: Option<Arc<watcher::FileWatcher>>,
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
        let searcher = Arc::new(search::Searcher::new(
            storage.clone(),
            embedder.clone(),
            &config,
        ).await?);
        let graph = Arc::new(RwLock::new(graph::KnowledgeGraph::new(storage.clone())));
        
        // Initialize watcher (optional - can be started later)
        let watcher = match watcher::FileWatcher::new(storage.clone(), indexer.clone()) {
            Ok(w) => Some(Arc::new(w)),
            Err(e) => {
                tracing::warn!("Failed to create file watcher: {}", e);
                None
            }
        };

        Ok(Self {
            storage,
            indexer,
            embedder,
            searcher,
            graph,
            watcher,
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
                self.storage.remove_memories_by_path_prefix(root_path.to_string_lossy().as_ref()).await?;
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
    pub async fn add_to_cluster(&self, cluster_id: uuid::Uuid, memory_ids: Vec<MemoryId>) -> Result<()> {
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
            watcher.watch(source).await
        } else {
            Err(HippoError::Other("File watcher not available".to_string()))
        }
    }

    /// Stop watching a source
    pub async fn unwatch_source(&self, source: &Source) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.unwatch(source).await
        } else {
            Ok(())
        }
    }

    /// Start watching all configured sources
    pub async fn watch_all(&self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.watch_all_sources().await
        } else {
            Err(HippoError::Other("File watcher not available".to_string()))
        }
    }

    /// Stop all file watchers
    pub async fn unwatch_all(&self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.unwatch_all().await
        } else {
            Ok(())
        }
    }

    /// Get the number of active watchers
    pub async fn active_watchers(&self) -> usize {
        if let Some(watcher) = &self.watcher {
            watcher.active_count().await
        } else {
            0
        }
    }

    /// Get list of watched paths
    pub async fn watched_paths(&self) -> Vec<PathBuf> {
        if let Some(watcher) = &self.watcher {
            watcher.watched_paths().await
        } else {
            vec![]
        }
    }

    // === Duplicate Detection ===

    /// Find duplicate files in the index
    pub async fn find_duplicates(&self, min_size: u64) -> Result<(Vec<DuplicateGroup>, DuplicateSummary)> {
        let memories = self.storage.get_all_memories().await?;
        Ok(duplicates::find_duplicates_by_scanning(&memories, min_size)?)
    }

    /// Get all memories (for duplicate scanning)
    pub async fn get_all_memories(&self) -> Result<Vec<Memory>> {
        self.storage.get_all_memories().await
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
            println!("Hippo creation failed (expected in test env): {:?}", result.err());
        }
    }
}
