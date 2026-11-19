//! Qdrant vector database integration for semantic search
//!
//! Provides vector storage and similarity search using Qdrant.
//! Falls back gracefully when Qdrant is unavailable.

pub mod manager;

pub use manager::{QdrantManager, QdrantStatus};

use crate::embeddings::{CODE_EMBEDDING_DIM, IMAGE_EMBEDDING_DIM, TEXT_EMBEDDING_DIM};
use crate::error::{HippoError, Result};
use crate::models::*;
use qdrant_client::qdrant::{
    vectors_config::Config, CreateCollectionBuilder, DeletePointsBuilder, Distance, PointId,
    PointStruct, SearchPointsBuilder, UpsertPointsBuilder, Value as QdrantValue,
    VectorParamsBuilder, VectorsConfig,
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

/// Minimum cosine score for a Qdrant hit to be returned. Below this, matches are
/// effectively noise (unrelated files) and must not be shown as "N% similar".
/// ~0.3 is a reasonable floor for nomic-embed-text.
pub const QDRANT_SCORE_THRESHOLD: f32 = 0.3;

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
                        warn!(
                            "Qdrant health check failed: {}. Vector search will be limited.",
                            e
                        );
                        *self.available.write().await = false;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to create Qdrant client: {}. Vector search will be limited.",
                    e
                );
                *self.available.write().await = false;
            }
        }
    }

    /// Check if Qdrant is available
    pub async fn is_available(&self) -> bool {
        *self.available.read().await
    }

    /// Ensure all required collections exist with the DEFAULT constant dims.
    ///
    /// The text collection dim must match the active embedder's runtime output
    /// dim. `TEXT_EMBEDDING_DIM` is kept in sync with the default local model
    /// (nomic → 768). When the active provider differs (e.g. OpenAI → 1536),
    /// callers should instead use [`ensure_collections_for_embedder`] so the
    /// collection is created at the real runtime dim. (storage/mod.rs currently
    /// calls this variant; see cross-cutting note in the PR.)
    pub async fn ensure_collections(&self) -> Result<()> {
        self.ensure_collections_with_dims(TEXT_EMBEDDING_DIM, IMAGE_EMBEDDING_DIM, CODE_EMBEDDING_DIM)
            .await
    }

    /// Ensure collections exist, deriving the TEXT dim from the embedder's
    /// actual runtime output dim rather than a hardcoded constant.
    pub async fn ensure_collections_for_embedder(
        &self,
        embedder: &crate::embeddings::Embedder,
    ) -> Result<()> {
        self.ensure_collections_with_dims(
            embedder.text_output_dim(),
            IMAGE_EMBEDDING_DIM,
            CODE_EMBEDDING_DIM,
        )
        .await
    }

    /// Ensure collections exist at the given dims, self-healing on mismatch.
    ///
    /// If a collection already exists but was created at a DIFFERENT dim (e.g.
    /// a stale 1536-dim `hippo_text` left over from the old OpenAI default), it
    /// is dropped and recreated at the correct dim. This prevents the silent
    /// "dimension mismatch → vectors never stored" failure.
    pub async fn ensure_collections_with_dims(
        &self,
        text_dim: usize,
        image_dim: usize,
        code_dim: usize,
    ) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()), // Silently skip if not available
        };

        let collections = [
            (COLLECTION_IMAGES, image_dim),
            (COLLECTION_TEXT, text_dim),
            (COLLECTION_CODE, code_dim),
        ];

        for (name, dim) in collections {
            if self.collection_exists(name).await? {
                // Validate the existing collection's dim matches what we expect.
                match self.collection_dim(name).await {
                    Some(existing) if existing as usize == dim => {
                        // Already correct — nothing to do.
                        continue;
                    }
                    Some(existing) => {
                        warn!(
                            "Qdrant collection {} has dim {} but embedder needs {}; recreating",
                            name, existing, dim
                        );
                        client.delete_collection(name).await.map_err(|e| {
                            HippoError::Other(format!(
                                "Failed to drop mismatched collection {}: {}",
                                name, e
                            ))
                        })?;
                    }
                    None => {
                        // Could not read dim; leave as-is to avoid data loss.
                        warn!("Could not read dim for collection {}; leaving as-is", name);
                        continue;
                    }
                }
            }

            info!("Creating Qdrant collection: {} (dim={})", name, dim);
            let create_collection =
                CreateCollectionBuilder::new(name).vectors_config(VectorsConfig {
                    config: Some(Config::Params(
                        VectorParamsBuilder::new(dim as u64, Distance::Cosine).build(),
                    )),
                });

            client
                .create_collection(create_collection)
                .await
                .map_err(|e| {
                    HippoError::Other(format!("Failed to create collection {}: {}", name, e))
                })?;
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

    /// Read the configured vector dimension of an existing collection, if any.
    async fn collection_dim(&self, name: &str) -> Option<u64> {
        let client = self.client.as_ref()?;
        let info = client.collection_info(name).await.ok()?;
        let params = info.result?.config?.params?;
        match params.vectors_config?.config? {
            Config::Params(vp) => Some(vp.size),
            // Named/sparse vector configs are not used by this app.
            _ => None,
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

        // A dimension mismatch here previously returned Ok(()) silently, which
        // meant the vector was NEVER stored (not in Qdrant, not in SQLite) and
        // semantic search returned nothing. That is a hard error now: the
        // collection must be (re)created at the embedder's runtime dim via
        // `ensure_collections_for_embedder` at startup.
        if embedding.len() != expected_dim {
            return Err(HippoError::Other(format!(
                "Embedding dimension mismatch for {}: got {}, collection '{}' expects {}. \
                 Recreate the collection at the embedder's runtime dim (startup validation).",
                memory_id,
                embedding.len(),
                collection,
                expected_dim
            )));
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
            HashMap::from([(
                "memory_id".to_string(),
                QdrantValue::from(memory_id.to_string()),
            )]),
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

        // A dim mismatch must error, not silently zero-pad: padding produces a
        // garbage query vector that returns meaningless near-zero-cosine hits.
        if query.len() != expected_dim {
            return Err(HippoError::Other(format!(
                "Query embedding dimension {} does not match collection '{}' dim {}",
                query.len(),
                collection,
                expected_dim
            )));
        }

        // Cut off near-zero-cosine junk so unrelated files are not surfaced as
        // low-percentage "matches". Tuned for nomic-embed-text cosine scores.
        let search_request = SearchPointsBuilder::new(collection, query, limit as u64)
            .with_payload(true)
            .score_threshold(QDRANT_SCORE_THRESHOLD);

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

    /// Retrieve embedding for a memory from Qdrant
    pub async fn get_embedding(
        &self,
        memory_id: MemoryId,
        kind: &MemoryKind,
    ) -> Result<Option<Vec<f32>>> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(None),
        };

        let collection = Self::get_collection_for_kind(kind);
        let point_id = PointId::from(memory_id.to_string());

        use qdrant_client::qdrant::GetPointsBuilder;
        let request = GetPointsBuilder::new(collection, vec![point_id])
            .with_vectors(true)
            .with_payload(true);

        match client.get_points(request).await {
            Ok(response) => {
                if let Some(point) = response.result.first() {
                    if let Some(vectors) = &point.vectors {
                        if let Some(qdrant_client::qdrant::vectors::VectorsOptions::Vector(v)) =
                            &vectors.vectors_options
                        {
                            return Ok(Some(v.data.clone()));
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => {
                debug!("Failed to get embedding from Qdrant: {}", e);
                Ok(None)
            }
        }
    }

    /// Upsert a single document chunk as its own point in `hippo_text`.
    ///
    /// The point id is the chunk id; the payload carries `parent_memory_id`,
    /// `ordinal`, and an `is_chunk` marker so retrieval can roll chunk hits up to
    /// their parent memory. Chunks always live in the text collection (768-dim),
    /// regardless of the parent memory's kind.
    pub async fn upsert_chunk(&self, chunk: &Chunk, embedding: Vec<f32>) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => {
                debug!("Qdrant not available, skipping chunk upsert");
                return Ok(());
            }
        };

        if embedding.len() != TEXT_EMBEDDING_DIM {
            return Err(HippoError::Other(format!(
                "Chunk embedding dimension mismatch for {}: got {}, collection '{}' expects {}",
                chunk.chunk_id,
                embedding.len(),
                COLLECTION_TEXT,
                TEXT_EMBEDDING_DIM
            )));
        }

        let point = PointStruct::new(
            chunk.chunk_id.clone(),
            embedding,
            HashMap::from([
                (
                    "parent_memory_id".to_string(),
                    QdrantValue::from(chunk.parent_memory_id.to_string()),
                ),
                ("ordinal".to_string(), QdrantValue::from(chunk.ordinal as i64)),
                ("is_chunk".to_string(), QdrantValue::from(true)),
            ]),
        );

        client
            .upsert_points(UpsertPointsBuilder::new(COLLECTION_TEXT, vec![point]).wait(true))
            .await
            .map_err(|e| HippoError::Other(format!("Failed to upsert chunk vector: {}", e)))?;

        debug!(
            "Upserted chunk {} (parent {}) to {}",
            chunk.chunk_id, chunk.parent_memory_id, COLLECTION_TEXT
        );
        Ok(())
    }

    /// Search `hippo_text` and return hits annotated with their parent memory id.
    ///
    /// Each hit is `(parent_memory_id, chunk_id, score)`. A chunk point carries
    /// `parent_memory_id` in its payload and its own id as the chunk id; a legacy
    /// whole-document point (no parent payload) is reported with its point id as
    /// the parent and `None` for the chunk id. This lets callers roll chunked and
    /// non-chunked memories up uniformly.
    pub async fn search_text_with_parents(
        &self,
        query: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<(MemoryId, Option<String>, f32)>> {
        use qdrant_client::qdrant::value::Kind;

        let client = match &self.client {
            Some(c) => c,
            None => {
                debug!("Qdrant not available for chunk search");
                return Ok(vec![]);
            }
        };

        if query.len() != TEXT_EMBEDDING_DIM {
            return Err(HippoError::Other(format!(
                "Query embedding dimension {} does not match collection '{}' dim {}",
                query.len(),
                COLLECTION_TEXT,
                TEXT_EMBEDDING_DIM
            )));
        }

        let search_request = SearchPointsBuilder::new(COLLECTION_TEXT, query, limit as u64)
            .with_payload(true)
            .score_threshold(QDRANT_SCORE_THRESHOLD);

        let results = client
            .search_points(search_request)
            .await
            .map_err(|e| HippoError::Other(format!("Qdrant chunk search failed: {}", e)))?;

        let mut hits: Vec<(MemoryId, Option<String>, f32)> = Vec::new();
        for point in results.result {
            let point_id = match point.id.as_ref().and_then(|id| id.point_id_options.as_ref()) {
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u)) => u.clone(),
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(n)) => n.to_string(),
                None => continue,
            };

            let parent_str = point.payload.get("parent_memory_id").and_then(|v| match &v.kind {
                Some(Kind::StringValue(s)) => Some(s.clone()),
                _ => None,
            });

            match parent_str {
                Some(parent) => {
                    if let Ok(pid) = Uuid::parse_str(&parent) {
                        hits.push((pid, Some(point_id), point.score));
                    }
                }
                None => {
                    // Legacy whole-document point: its id IS the memory id.
                    if let Ok(mid) = Uuid::parse_str(&point_id) {
                        hits.push((mid, None, point.score));
                    }
                }
            }
        }

        debug!("Qdrant chunk search returned {} hits", hits.len());
        Ok(hits)
    }

    /// Delete the given point ids from the `hippo_text` collection (used to tear
    /// down a memory's chunk vectors on delete / re-index).
    pub async fn delete_text_points(&self, ids: &[String]) -> Result<()> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };
        if ids.is_empty() {
            return Ok(());
        }

        let selector: Vec<PointId> = ids.iter().map(|s| PointId::from(s.clone())).collect();
        client
            .delete_points(
                DeletePointsBuilder::new(COLLECTION_TEXT)
                    .points(selector)
                    .wait(true),
            )
            .await
            .map_err(|e| HippoError::Other(format!("Failed to delete chunk vectors: {}", e)))?;

        debug!("Deleted {} chunk vectors from {}", ids.len(), COLLECTION_TEXT);
        Ok(())
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
                    .wait(true),
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
        let result = storage
            .search(vec![0.0; TEXT_EMBEDDING_DIM], None, 10)
            .await;
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
        assert_eq!(
            QdrantStorage::get_collection_from_str(Some("image")),
            COLLECTION_IMAGES
        );
        assert_eq!(
            QdrantStorage::get_collection_from_str(Some("Image")),
            COLLECTION_IMAGES
        );
        assert_eq!(
            QdrantStorage::get_collection_from_str(Some("code")),
            COLLECTION_CODE
        );
        assert_eq!(
            QdrantStorage::get_collection_from_str(Some("Code")),
            COLLECTION_CODE
        );
        assert_eq!(
            QdrantStorage::get_collection_from_str(Some("text")),
            COLLECTION_TEXT
        );
        assert_eq!(
            QdrantStorage::get_collection_from_str(None),
            COLLECTION_TEXT
        );
    }

    #[tokio::test]
    async fn test_delete_when_unavailable() {
        let storage = QdrantStorage::new("http://localhost:9999").await.unwrap();
        let result = storage
            .delete(
                uuid::Uuid::new_v4(),
                &MemoryKind::Image {
                    width: 100,
                    height: 100,
                    format: "jpg".to_string(),
                },
            )
            .await;

        // Should succeed silently when Qdrant is unavailable
        assert!(result.is_ok());
    }
}
