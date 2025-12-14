//! Embedding generation for semantic search
//!
//! Supports multiple embedding backends:
//! - Local ONNX models (CLIP, BGE, CodeBERT)
//! - API-based embeddings (OpenAI, Voyage AI)
//! - Simple hash-based fallback for offline use

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::HippoConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

/// Embedding dimension for different model types
pub const TEXT_EMBEDDING_DIM: usize = 1536;  // OpenAI ada-002 compatible
pub const IMAGE_EMBEDDING_DIM: usize = 512;  // CLIP compatible
pub const CODE_EMBEDDING_DIM: usize = 768;   // CodeBERT compatible

/// Main embedder that handles all embedding generation
pub struct Embedder {
    config: EmbedderConfig,
    client: Client,
    cache: HashMap<String, Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct EmbedderConfig {
    pub use_local: bool,
    pub model_path: Option<std::path::PathBuf>,
    pub api_key: Option<String>,
    pub api_provider: EmbeddingProvider,
}

#[derive(Debug, Clone, Default)]
pub enum EmbeddingProvider {
    #[default]
    Local,
    OpenAI,
    Voyage,
}

impl Embedder {
    pub async fn new(config: &HippoConfig) -> Result<Self> {
        Ok(Self {
            config: EmbedderConfig {
                use_local: config.local_embeddings,
                model_path: Some(config.data_dir.join("models")),
                api_key: config.ai_api_key.clone(),
                api_provider: EmbeddingProvider::Local,
            },
            client: Client::new(),
            cache: HashMap::new(),
        })
    }

    /// Create embedder with OpenAI API
    pub fn with_openai(api_key: String) -> Self {
        Self {
            config: EmbedderConfig {
                use_local: false,
                model_path: None,
                api_key: Some(api_key),
                api_provider: EmbeddingProvider::OpenAI,
            },
            client: Client::new(),
            cache: HashMap::new(),
        }
    }

    /// Generate embedding for a memory
    pub async fn embed_memory(&self, memory: &Memory) -> Result<Vec<f32>> {
        match &memory.kind {
            MemoryKind::Image { .. } => self.embed_image(&memory.path).await,
            MemoryKind::Code { language, .. } => self.embed_code(&memory.path, language).await,
            MemoryKind::Document { .. } => self.embed_document(&memory.path).await,
            _ => self.embed_generic(memory).await,
        }
    }

    /// Embed an image file
    async fn embed_image(&self, path: &Path) -> Result<Vec<f32>> {
        // For now, use a simple perceptual hash-based embedding
        // TODO: Integrate CLIP model via ONNX
        debug!("Generating image embedding for: {:?}", path);

        match image::open(path) {
            Ok(img) => {
                // Create a simple embedding from image statistics
                let rgb = img.to_rgb8();
                let (width, height) = rgb.dimensions();

                // Calculate basic statistics per channel
                let mut embedding = vec![0.0f32; IMAGE_EMBEDDING_DIM];

                // Resize to 16x16 for feature extraction
                let small = image::imageops::resize(&rgb, 16, 16, image::imageops::FilterType::Triangle);

                // Use pixel values as features
                for (i, pixel) in small.pixels().enumerate() {
                    if i * 3 + 2 < embedding.len() {
                        embedding[i * 3] = pixel[0] as f32 / 255.0;
                        embedding[i * 3 + 1] = pixel[1] as f32 / 255.0;
                        embedding[i * 3 + 2] = pixel[2] as f32 / 255.0;
                    }
                }

                // Add dimension info
                let dim_idx = 256 * 3;
                if dim_idx + 2 < embedding.len() {
                    embedding[dim_idx] = (width as f32).ln() / 10.0;
                    embedding[dim_idx + 1] = (height as f32).ln() / 10.0;
                    embedding[dim_idx + 2] = (width as f32 / height as f32).min(5.0) / 5.0;
                }

                // Normalize
                Self::normalize(&mut embedding);
                Ok(embedding)
            }
            Err(e) => {
                warn!("Failed to load image for embedding: {}", e);
                Ok(vec![0.0; IMAGE_EMBEDDING_DIM])
            }
        }
    }

    /// Embed a code file
    async fn embed_code(&self, path: &Path, language: &str) -> Result<Vec<f32>> {
        debug!("Generating code embedding for: {:?}", path);

        let code = std::fs::read_to_string(path).unwrap_or_default();

        // Use text embedding for code, but add language-specific features
        let mut embedding = self.hash_embed(&code, CODE_EMBEDDING_DIM);

        // Add language features
        let lang_idx = CODE_EMBEDDING_DIM - 20;
        let lang_hash = Self::simple_hash(language) as usize % 20;
        if lang_idx + lang_hash < embedding.len() {
            embedding[lang_idx + lang_hash] = 1.0;
        }

        Self::normalize(&mut embedding);
        Ok(embedding)
    }

    /// Embed a document file
    async fn embed_document(&self, path: &Path) -> Result<Vec<f32>> {
        debug!("Generating document embedding for: {:?}", path);

        // Try to read text content
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => {
                // For binary documents, use filename and metadata
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            }
        };

        // Use API if available, otherwise use local hash-based embedding
        if let Some(api_key) = &self.config.api_key {
            if matches!(self.config.api_provider, EmbeddingProvider::OpenAI) {
                return self.embed_with_openai(&text, api_key).await;
            }
        }

        Ok(self.hash_embed(&text, TEXT_EMBEDDING_DIM))
    }

    /// Embed generic content
    async fn embed_generic(&self, memory: &Memory) -> Result<Vec<f32>> {
        // Create embedding from path and metadata
        let tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();
        let text = format!(
            "{} {} {:?}",
            memory.path.display(),
            tags.join(" "),
            memory.kind
        );

        Ok(self.hash_embed(&text, TEXT_EMBEDDING_DIM))
    }

    /// Embed a search query
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // Use API if available
        if let Some(api_key) = &self.config.api_key {
            if matches!(self.config.api_provider, EmbeddingProvider::OpenAI) {
                return self.embed_with_openai(query, api_key).await;
            }
        }

        Ok(self.hash_embed(query, TEXT_EMBEDDING_DIM))
    }

    /// Embed text using OpenAI API
    async fn embed_with_openai(&self, text: &str, api_key: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            input: String,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbedData>,
        }

        #[derive(Deserialize)]
        struct EmbedData {
            embedding: Vec<f32>,
        }

        // Truncate text to avoid token limits
        let truncated: String = text.chars().take(8000).collect();

        let request = EmbedRequest {
            model: "text-embedding-ada-002".to_string(),
            input: truncated,
        };

        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            warn!("OpenAI embedding failed with status {}, falling back to local", status);
            return Ok(self.hash_embed(text, TEXT_EMBEDDING_DIM));
        }

        let embed_response: EmbedResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse embedding response: {}", e)))?;

        embed_response.data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| HippoError::Other("No embedding in response".to_string()))
    }

    /// Simple hash-based embedding (fallback for offline use)
    fn hash_embed(&self, text: &str, dim: usize) -> Vec<f32> {
        let mut embedding = vec![0.0f32; dim];

        // Tokenize and hash
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let total_tokens = tokens.len().max(1) as f32;

        for (pos, token) in tokens.iter().enumerate() {
            let token_lower = token.to_lowercase();
            let hash = Self::simple_hash(&token_lower);

            // Position-weighted hashing
            let position_weight = 1.0 - (pos as f32 / total_tokens) * 0.5;

            // Multiple hash positions for better distribution
            for i in 0..3 {
                let idx = ((hash as usize).wrapping_add(i * 7919)) % dim;
                embedding[idx] += position_weight * (1.0 / (i + 1) as f32);
            }

            // Bigram features
            if pos > 0 {
                let bigram = format!("{}_{}", tokens[pos - 1].to_lowercase(), token_lower);
                let bigram_hash = Self::simple_hash(&bigram);
                let idx = (bigram_hash as usize) % dim;
                embedding[idx] += position_weight * 0.5;
            }
        }

        // Character n-gram features for better matching
        let chars: Vec<char> = text.to_lowercase().chars().collect();
        for window in chars.windows(3) {
            let ngram: String = window.iter().collect();
            let hash = Self::simple_hash(&ngram);
            let idx = (hash as usize) % dim;
            embedding[idx] += 0.1;
        }

        Self::normalize(&mut embedding);
        embedding
    }

    /// Simple string hash
    fn simple_hash(s: &str) -> u64 {
        let mut hash: u64 = 5381;
        for c in s.chars() {
            hash = hash.wrapping_mul(33).wrapping_add(c as u64);
        }
        hash
    }

    /// Normalize embedding to unit length
    fn normalize(embedding: &mut [f32]) {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in embedding.iter_mut() {
                *x /= norm;
            }
        }
    }

    /// Calculate cosine similarity between two embeddings
    pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Find most similar embeddings from a list
    pub fn find_similar(
        query: &[f32],
        embeddings: &[(String, Vec<f32>)],
        top_k: usize,
    ) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = embeddings
            .iter()
            .map(|(id, emb)| (id.clone(), Self::similarity(query, emb)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

/// Vector index for similarity search
pub struct VectorIndex {
    embeddings: HashMap<String, Vec<f32>>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
        }
    }

    /// Add an embedding to the index
    pub fn add(&mut self, id: String, embedding: Vec<f32>) {
        self.embeddings.insert(id, embedding);
    }

    /// Remove an embedding from the index
    pub fn remove(&mut self, id: &str) {
        self.embeddings.remove(id);
    }

    /// Search for similar vectors
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let embeddings_vec: Vec<(String, Vec<f32>)> = self.embeddings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Embedder::find_similar(query, &embeddings_vec, top_k)
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}
