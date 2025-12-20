//! Qdrant vector database integration for semantic search
//!
//! Provides vector storage and similarity search using Qdrant.
//! Falls back gracefully when Qdrant is unavailable.

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::embeddings::{IMAGE_EMBEDDING_DIM, TEXT_EMBEDDING_DIM, CODE_EMBEDDING_DIM};
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParamsBuilder, DeletePointsBuilder,
    PointId, Value as QdrantValue,
    vectors_config::Config, VectorsConfig,
};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Collection names for different embedding types
pub const COLLECTION_IMAGES: &str = "hippo_images";
pub const COLLECTION_TEXT: &str = "hippo_text";
pub const COLLECTION_CODE: &str = "hippo_code";

/// Qdrant storage wrapper for vector operations
pub struct QdrantStorage {
    client: Option<Qdrant>,
    url: String,
    available: Arc<RwLock<bool>>,
}

impl QdrantStorage {
    /// Create a new Qdrant storage connection
    pub async fn new(url: &str) -> Result<Self> {
        let mut storage = Self {
            client: None,
            url: url.to_string(),
            available: Arc::new(RwLock::new(false)),
        };

        // Try to connect
        storage.connect().await;

        Ok(storage)
    }

    /// Attempt to connect to Qdrant
    async fn connect(&mut self) {
        match Qdrant::from_url(&self.url).build() {
            Ok(client) => {
                // Test connection with health check
                match client.health_check().await {
                    Ok(_) => {
                        info!("Connected to Qdrant at {}", self.url);
                        self.client = Some(client);
                        *self.available.write().await = true;
                    }
                    Err(e) => {
                        warn!("Qdrant health check failed: {}. Vector search will be limited.", e);
                        *self.available.write().await = false;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create Qdrant client: {}. Vector search will be limited.", e);
                *self.available.write().await = false;
            }
        }
    }

    /// Check if Qdrant is available
    pub async fn is_available(&self) -> bool {
        *self.available.read().await
    }

    /// Ensure all required collections exist
    pub async fn ensure_collections(&self) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()), // Silently skip if not available
        };

        // Create collections if they don't exist
        let collections = [
            (COLLECTION_IMAGES, IMAGE_EMBEDDING_DIM),
            (COLLECTION_TEXT, TEXT_EMBEDDING_DIM),
            (COLLECTION_CODE, CODE_EMBEDDING_DIM),
        ];

        for (name, dim) in collections {
            if !self.collection_exists(name).await? {
                info!("Creating Qdrant collection: {} (dim={})", name, dim);

                let create_collection = CreateCollectionBuilder::new(name)
                    .vectors_config(VectorsConfig {
                        config: Some(Config::Params(
                            VectorParamsBuilder::new(dim as u64, Distance::Cosine).build()
                        ))
                    });

                client
                    .create_collection(create_collection)
                    .await
                    .map_err(|e| HippoError::Other(format!("Failed to create collection {}: {}", name, e)))?;
            }
        }

        Ok(())
    }

    /// Check if a collection exists
    async fn collection_exists(&self, name: &str) -> Result<bool> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(false),
        };

        match client.collection_exists(name).await {
            Ok(exists) => Ok(exists),
            Err(_) => Ok(false),
        }
    }

    /// Get the appropriate collection name for a memory kind
    fn get_collection_for_kind(kind: &MemoryKind) -> &'static str {
        match kind {
            MemoryKind::Image { .. } => COLLECTION_IMAGES,
            MemoryKind::Code { .. } => COLLECTION_CODE,
            _ => COLLECTION_TEXT,
        }
    }

    /// Get the collection name from a kind string
    fn get_collection_from_str(kind: Option<&str>) -> &'static str {
        match kind {
            Some("image") | Some("Image") => COLLECTION_IMAGES,
            Some("code") | Some("Code") => COLLECTION_CODE,
            _ => COLLECTION_TEXT,
        }
    }

    /// Upsert a vector into Qdrant
    pub async fn upsert(
        &self,
        memory_id: MemoryId,
        embedding: Vec<f32>,
        kind: &MemoryKind,
    ) -> Result<()> {
        let _client = match &self.client {
            Some(c) => c,
            None => {
                debug!("Qdrant not available, skipping upsert");
                return Ok(());
            }
        };

        let collection = Self::get_collection_for_kind(kind);

        // Verify embedding dimension matches collection
        let expected_dim = match collection {
            COLLECTION_IMAGES => IMAGE_EMBEDDING_DIM,
            COLLECTION_CODE => CODE_EMBEDDING_DIM,
            _ => TEXT_EMBEDDING_DIM,
        };

        if embedding.len() != expected_dim {
            warn!(
                "Embedding dimension mismatch: got {}, expected {} for collection {}",
                embedding.len(),
                expected_dim,
                collection
            );
            // Pad or truncate embedding to match
            let mut adjusted = embedding;
            adjusted.resize(expected_dim, 0.0);
            return self.upsert_raw(memory_id, adjusted, collection).await;
        }

        self.upsert_raw(memory_id, embedding, collection).await
    }

    /// Raw upsert operation
    async fn upsert_raw(
        &self,
        memory_id: MemoryId,
        embedding: Vec<f32>,
        collection: &str,
    ) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };

        let point = PointStruct::new(
            memory_id.to_string(),
            embedding,
            HashMap::from([
                ("memory_id".to_string(), QdrantValue::from(memory_id.to_string())),
            ]),
        );

        client
            .upsert_points(UpsertPointsBuilder::new(collection, vec![point]).wait(true))
            .await
            .map_err(|e| HippoError::Other(format!("Failed to upsert vector: {}", e)))?;

        debug!("Upserted vector for {} to {}", memory_id, collection);
        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(
        &self,
        query: Vec<f32>,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        let client = match &self.client {
            Some(c) => c,
            None => {
                debug!("Qdrant not available for search");
                return Ok(vec![]);
            }
        };

        let collection = Self::get_collection_from_str(kind);

        // Verify and adjust query dimension
        let expected_dim = match collection {
            COLLECTION_IMAGES => IMAGE_EMBEDDING_DIM,
            COLLECTION_CODE => CODE_EMBEDDING_DIM,
            _ => TEXT_EMBEDDING_DIM,
        };

        let mut query_vec = query;
        if query_vec.len() != expected_dim {
            query_vec.resize(expected_dim, 0.0);
        }

        let search_request = SearchPointsBuilder::new(collection, query_vec, limit as u64)
            .with_payload(true);

        let results = client
            .search_points(search_request)
            .await
            .map_err(|e| HippoError::Other(format!("Qdrant search failed: {}", e)))?;

        let mut matches: Vec<(MemoryId, f32)> = Vec::new();

        for point in results.result {
            if let Some(id) = point.id {
                let uuid_str = match id.point_id_options {
                    Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u)) => u,
                    Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(n)) => n.to_string(),
                    None => continue,
                };

                if let Ok(memory_id) = Uuid::parse_str(&uuid_str) {
                    matches.push((memory_id, point.score));
                }
            }
        }

        debug!("Qdrant search returned {} matches", matches.len());
        Ok(matches)
    }

    /// Find similar items to a specific memory
    pub async fn find_similar(
        &self,
        memory_id: MemoryId,
        embedding: Vec<f32>,
        kind: &MemoryKind,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        let _collection = Self::get_collection_for_kind(kind);
        let kind_str = match kind {
            MemoryKind::Image { .. } => Some("image"),
            MemoryKind::Code { .. } => Some("code"),
            _ => None,
        };

        let mut results = self.search(embedding, kind_str, limit + 1).await?;

        // Remove the source memory from results
        results.retain(|(id, _)| *id != memory_id);
        results.truncate(limit);

        Ok(results)
    }

    /// Delete a vector from Qdrant
    pub async fn delete(&self, memory_id: MemoryId, kind: &MemoryKind) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };

        let collection = Self::get_collection_for_kind(kind);

        let points_selector = vec![PointId::from(memory_id.to_string())];

        client
            .delete_points(
                DeletePointsBuilder::new(collection)
                    .points(points_selector)
                    .wait(true)
            )
            .await
            .map_err(|e| HippoError::Other(format!("Failed to delete vector: {}", e)))?;

        debug!("Deleted vector for {} from {}", memory_id, collection);
        Ok(())
    }

    /// Get collection statistics
    pub async fn stats(&self) -> Result<QdrantStats> {
        let client = match &self.client {
            Some(c) => c,
            None => {
                return Ok(QdrantStats {
                    available: false,
                    collections: HashMap::new(),
                    total_vectors: 0,
                });
            }
        };

        let mut collections = HashMap::new();
        let mut total = 0u64;

        for name in [COLLECTION_IMAGES, COLLECTION_TEXT, COLLECTION_CODE] {
            if let Ok(info) = client.collection_info(name).await {
                if let Some(result) = info.result {
                    let count = result.points_count.unwrap_or(0);
                    collections.insert(name.to_string(), count);
                    total += count;
                }
            }
        }

        Ok(QdrantStats {
            available: true,
            collections,
            total_vectors: total,
        })
    }
}

/// Statistics about Qdrant storage
#[derive(Debug, Clone, serde::Serialize)]
pub struct QdrantStats {
    pub available: bool,
    pub collections: HashMap<String, u64>,
    pub total_vectors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_qdrant_unavailable() {
        // Test graceful handling when Qdrant is not running
        let storage = QdrantStorage::new("http://localhost:9999").await.unwrap();
        assert!(!storage.is_available().await);

        // Operations should not fail
        let result = storage.search(vec![0.0; TEXT_EMBEDDING_DIM], None, 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_stats_when_unavailable() {
        let storage = QdrantStorage::new("http://localhost:9999").await.unwrap();
        let stats = storage.stats().await.unwrap();

        assert!(!stats.available);
        assert_eq!(stats.total_vectors, 0);
        assert!(stats.collections.is_empty());
    }

    #[test]
    fn test_get_collection_for_kind() {
        assert_eq!(
            QdrantStorage::get_collection_for_kind(&MemoryKind::Image {
                width: 100,
                height: 100,
                format: "jpg".to_string()
            }),
            COLLECTION_IMAGES
        );

        assert_eq!(
            QdrantStorage::get_collection_for_kind(&MemoryKind::Code {
                language: "rust".to_string(),
                lines: 100
            }),
            COLLECTION_CODE
        );

        assert_eq!(
            QdrantStorage::get_collection_for_kind(&MemoryKind::Document {
                format: crate::models::DocumentFormat::Pdf,
                page_count: Some(10)
            }),
            COLLECTION_TEXT
        );
    }

    #[test]
    fn test_get_collection_from_str() {
        assert_eq!(QdrantStorage::get_collection_from_str(Some("image")), COLLECTION_IMAGES);
        assert_eq!(QdrantStorage::get_collection_from_str(Some("Image")), COLLECTION_IMAGES);
        assert_eq!(QdrantStorage::get_collection_from_str(Some("code")), COLLECTION_CODE);
        assert_eq!(QdrantStorage::get_collection_from_str(Some("Code")), COLLECTION_CODE);
        assert_eq!(QdrantStorage::get_collection_from_str(Some("text")), COLLECTION_TEXT);
        assert_eq!(QdrantStorage::get_collection_from_str(None), COLLECTION_TEXT);
    }

    #[tokio::test]
    async fn test_delete_when_unavailable() {
        let storage = QdrantStorage::new("http://localhost:9999").await.unwrap();
        let result = storage.delete(
            uuid::Uuid::new_v4(),
            &MemoryKind::Image {
                width: 100,
                height: 100,
                format: "jpg".to_string()
            }
        ).await;

        // Should succeed silently when Qdrant is unavailable
        assert!(result.is_ok());
    }
}
