//! Embedding generation for semantic search
//!
//! Supports multiple embedding backends:
//! - Ollama local models (nomic-embed-text, mxbai-embed-large)
//! - API-based embeddings (OpenAI)
//! - Simple hash-based fallback for offline use

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::ollama::{OllamaClient, OllamaConfig};
use crate::HippoConfig;
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Embedding dimension for different model types.
///
/// IMPORTANT: `TEXT_EMBEDDING_DIM` MUST match the runtime output dim of the
/// default local embedding model (Ollama `nomic-embed-text` → 768, see
/// `crate::ollama::NOMIC_EMBED_DIM`). The hash fallback also emits this dim so
/// that fallback vectors live in the same space as Ollama vectors. A regression
/// here silently breaks vector storage (dim mismatch on Qdrant upsert).
pub const TEXT_EMBEDDING_DIM: usize = 768; // nomic-embed-text (default local model)
pub const IMAGE_EMBEDDING_DIM: usize = 512; // CLIP compatible
pub const CODE_EMBEDDING_DIM: usize = 768; // CodeBERT compatible / nomic
/// OpenAI text-embedding-3-small / ada-002 dimension (only used when the OpenAI
/// backend is the active provider).
pub const OPENAI_EMBEDDING_DIM: usize = 1536;

/// Default target chunk size in approximate tokens (whitespace words).
pub const DEFAULT_CHUNK_TOKENS: usize = 512;
/// Default overlap between consecutive chunks (~15% of `DEFAULT_CHUNK_TOKENS`).
pub const DEFAULT_CHUNK_OVERLAP_TOKENS: usize = 76;

/// A slice of a document produced by [`Embedder::chunk_document`].
///
/// `char_start`/`char_end` are BYTE offsets into the original text, so
/// `&text[char_start..char_end] == text` of the chunk (for ASCII these equal
/// character offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChunk {
    pub text: String,
    pub char_start: usize,
    pub char_end: usize,
}

/// Cache configuration
const EMBEDDING_CACHE_SIZE: usize = 5000;
const EMBEDDING_CACHE_TTL_SECS: u64 = 3600; // 1 hour TTL
/// Maximum memory for embedding cache (50MB)
const EMBEDDING_CACHE_MAX_BYTES: usize = 50 * 1024 * 1024;

/// Cached embedding with timestamp and size tracking
#[derive(Clone)]
struct CachedEmbedding {
    embedding: Vec<f32>,
    created_at: Instant,
    /// Size in bytes (embedding.len() * 4 for f32)
    size_bytes: usize,
}

impl CachedEmbedding {
    fn new(embedding: Vec<f32>) -> Self {
        let size_bytes = embedding.len() * std::mem::size_of::<f32>();
        Self {
            embedding,
            created_at: Instant::now(),
            size_bytes,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(EMBEDDING_CACHE_TTL_SECS)
    }
}

/// Thread-safe embedding cache with LRU eviction and memory limits
/// Uses VecDeque for O(1) front removal during eviction
struct EmbeddingCache {
    entries: HashMap<String, CachedEmbedding>,
    access_order: VecDeque<String>, // For LRU tracking - O(1) pop_front
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    hits: u64,
    misses: u64,
}

impl EmbeddingCache {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries.min(1000)), // Don't pre-allocate too much
            access_order: VecDeque::with_capacity(max_entries.min(1000)),
            max_entries,
            max_bytes: EMBEDDING_CACHE_MAX_BYTES,
            current_bytes: 0,
            hits: 0,
            misses: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<Vec<f32>> {
        if let Some(cached) = self.entries.get(key) {
            if !cached.is_expired() {
                self.hits += 1;
                // Move to end for LRU - O(n) but only on cache hit
                if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                    self.access_order.remove(pos);
                    self.access_order.push_back(key.to_string());
                }
                return Some(cached.embedding.clone());
            } else {
                // Remove expired entry and update byte count
                if let Some(removed) = self.entries.remove(key) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.size_bytes);
                }
                self.access_order.retain(|k| k != key);
            }
        }
        self.misses += 1;
        None
    }

    fn insert(&mut self, key: String, embedding: Vec<f32>) {
        let new_size = embedding.len() * std::mem::size_of::<f32>();

        // Evict if at entry capacity or memory limit - O(1) with VecDeque::pop_front
        while (self.entries.len() >= self.max_entries
            || self.current_bytes + new_size > self.max_bytes)
            && !self.access_order.is_empty()
        {
            if let Some(oldest) = self.access_order.pop_front() {
                if let Some(removed) = self.entries.remove(&oldest) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.size_bytes);
                }
            }
        }

        let cached = CachedEmbedding::new(embedding);
        self.current_bytes += cached.size_bytes;
        self.entries.insert(key.clone(), cached);
        self.access_order.push_back(key);
    }

    fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.entries.len())
    }

    /// Get memory usage in bytes
    #[allow(dead_code)] // Reserved for future memory monitoring
    fn memory_bytes(&self) -> usize {
        self.current_bytes
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.current_bytes = 0;
    }
}

/// Main embedder that handles all embedding generation
pub struct Embedder {
    config: EmbedderConfig,
    client: Client,
    ollama: Option<OllamaClient>,
    /// Query embedding cache for faster repeated searches
    cache: Arc<RwLock<EmbeddingCache>>,
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
    Ollama,
    OpenAI,
}

impl Embedder {
    pub async fn new(config: &HippoConfig) -> Result<Self> {
        // Try to create Ollama client
        let ollama = if config.local_embeddings {
            let ollama_client = OllamaClient::new();
            if ollama_client.is_available().await {
                info!("Ollama is available for local embeddings");
                Some(ollama_client)
            } else {
                info!("Ollama not available, using fallback embeddings");
                None
            }
        } else {
            None
        };

        let provider = if ollama.is_some() {
            EmbeddingProvider::Ollama
        } else if config.ai_api_key.is_some() {
            EmbeddingProvider::OpenAI
        } else {
            EmbeddingProvider::Local
        };

        Ok(Self {
            config: EmbedderConfig {
                use_local: config.local_embeddings,
                model_path: Some(config.data_dir.join("models")),
                api_key: config.ai_api_key.clone(),
                api_provider: provider,
            },
            client: Client::new(),
            ollama,
            cache: Arc::new(RwLock::new(EmbeddingCache::new(EMBEDDING_CACHE_SIZE))),
        })
    }

    /// Create embedder with Ollama
    pub fn with_ollama(url: Option<String>) -> Self {
        let ollama_config = OllamaConfig {
            base_url: url.unwrap_or_else(|| crate::ollama::DEFAULT_OLLAMA_URL.to_string()),
            ..Default::default()
        };
        Self {
            config: EmbedderConfig {
                use_local: true,
                model_path: None,
                api_key: None,
                api_provider: EmbeddingProvider::Ollama,
            },
            client: Client::new(),
            ollama: Some(OllamaClient::with_config(ollama_config)),
            cache: Arc::new(RwLock::new(EmbeddingCache::new(EMBEDDING_CACHE_SIZE))),
        }
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
            ollama: None,
            cache: Arc::new(RwLock::new(EmbeddingCache::new(EMBEDDING_CACHE_SIZE))),
        }
    }

    /// Check if Ollama is available
    pub async fn ollama_available(&self) -> bool {
        if let Some(ollama) = &self.ollama {
            ollama.is_available().await
        } else {
            false
        }
    }

    /// Get the current embedding provider
    pub fn provider(&self) -> &EmbeddingProvider {
        &self.config.api_provider
    }

    /// Runtime output dimension of the active TEXT embedding backend.
    ///
    /// Qdrant collections should be created/validated against THIS value rather
    /// than a hardcoded constant, so that the stored collection dim always
    /// matches what the embedder actually emits.
    pub fn text_output_dim(&self) -> usize {
        match self.config.api_provider {
            EmbeddingProvider::OpenAI => OPENAI_EMBEDDING_DIM,
            // Ollama `nomic-embed-text` and the hash fallback both emit TEXT_EMBEDDING_DIM.
            EmbeddingProvider::Ollama | EmbeddingProvider::Local => TEXT_EMBEDDING_DIM,
        }
    }

    /// Stable identifier of the active embedding model (provider + model + dim).
    ///
    /// Used in cache keys so that switching models/providers never returns a
    /// stale vector from a previous model.
    pub fn model_id(&self) -> String {
        match self.config.api_provider {
            EmbeddingProvider::Ollama => self
                .ollama
                .as_ref()
                .map(|o| o.embedding_model().to_string())
                .unwrap_or_else(|| "ollama".to_string()),
            EmbeddingProvider::OpenAI => "openai:text-embedding-3-small".to_string(),
            EmbeddingProvider::Local => "hash".to_string(),
        }
    }

    /// L2-normalize a freshly generated embedding exactly once.
    ///
    /// Call this at the single point where a vector is produced by a backend
    /// (Ollama / OpenAI). After normalization, dot product == cosine similarity,
    /// which is what `Embedder::similarity` and Qdrant's cosine distance assume.
    fn normalized(mut embedding: Vec<f32>) -> Vec<f32> {
        Self::normalize(&mut embedding);
        embedding
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
    ///
    /// TODO(image-embeddings): This is a degenerate placeholder, NOT a real
    /// image embedding. It writes raw downsampled RGB pixel values into the
    /// vector; the 16x16 downsample only fills ~170 of the 512 dims (the
    /// `i * 3 + 2 < len` guard stops early) and the dimension-info block at
    /// `dim_idx = 256 * 3 = 768` is dead code (768 > IMAGE_EMBEDDING_DIM=512).
    /// These vectors are not semantically comparable across images. Replace with
    /// real CLIP ViT-B/32 (512-dim) embeddings via ONNX Runtime. Left as-is
    /// (behavior unchanged) because a proper fix needs model/ONNX infra that is
    /// unavailable in this environment. See report "Deferred" section.
    async fn embed_image(&self, path: &Path) -> Result<Vec<f32>> {
        debug!("Generating image embedding for: {:?}", path);

        match image::open(path) {
            Ok(img) => {
                // Create a simple embedding from image statistics
                let rgb = img.to_rgb8();
                let (width, height) = rgb.dimensions();

                // Calculate basic statistics per channel
                let mut embedding = vec![0.0f32; IMAGE_EMBEDDING_DIM];

                // Resize to 16x16 for feature extraction
                let small =
                    image::imageops::resize(&rgb, 16, 16, image::imageops::FilterType::Triangle);

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
        let truncated: String = code.chars().take(4000).collect();
        // Prepend language context so the single text model sees the language.
        let content = format!("{} source code\n{}", language, truncated);

        // Share the SAME backend/space as the batch path (Ollama/OpenAI, hash
        // only as last resort) so code and text vectors are comparable.
        self.embed_text_content(&content).await
    }

    /// Embed a document file
    async fn embed_document(&self, path: &Path) -> Result<Vec<f32>> {
        debug!("Generating document embedding for: {:?}", path);

        // Try to read text content
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t.chars().take(4000).collect::<String>(),
            Err(_) => {
                // For binary documents, use filename and metadata
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            }
        };

        self.embed_text_content(&text).await
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

        self.embed_text_content(&text).await
    }

    /// Embed arbitrary text content for storage (document path).
    ///
    /// Prefers the configured semantic backend (Ollama / OpenAI) and only falls
    /// back to the deterministic hash embedding as a genuine last resort. This
    /// is the SAME backend the batch path uses, so single-item and batch
    /// embeddings always land in one comparable vector space. The result is
    /// L2-normalized exactly once.
    async fn embed_text_content(&self, text: &str) -> Result<Vec<f32>> {
        // Prefer Ollama (document task prefix applied inside embed_documents).
        if let Some(ollama) = &self.ollama {
            if matches!(self.config.api_provider, EmbeddingProvider::Ollama) {
                match ollama
                    .embed_documents(std::slice::from_ref(&text.to_string()))
                    .await
                {
                    Ok(mut embs) if !embs.is_empty() => {
                        return Ok(Self::normalized(embs.remove(0)));
                    }
                    Ok(_) => warn!("Ollama returned no document embedding, falling back to hash"),
                    Err(e) => warn!(
                        "Ollama document embedding failed, falling back to hash: {}",
                        e
                    ),
                }
            }
        }

        // OpenAI API if configured.
        if let Some(api_key) = &self.config.api_key {
            if matches!(self.config.api_provider, EmbeddingProvider::OpenAI) {
                return Ok(Self::normalized(
                    self.embed_with_openai(text, api_key).await?,
                ));
            }
        }

        // Last resort: deterministic hash embedding (already normalized).
        Ok(self.hash_embed(text, self.text_output_dim()))
    }

    /// Split a long document into sentence/paragraph-aware chunks with overlap.
    ///
    /// Tokens are approximated by whitespace word count (no tokenizer dependency).
    /// Chunks target `chunk_tokens` words, carry `overlap_tokens` words over from
    /// the previous chunk, and are never cut mid-word — every boundary falls on a
    /// word boundary, and chunk ends prefer the nearest sentence/paragraph
    /// boundary when one lands reasonably close to the size cap. Each returned
    /// [`TextChunk`] carries BYTE offsets such that
    /// `&text[char_start..char_end] == chunk.text` (for ASCII these equal
    /// character offsets).
    ///
    /// A short text (<= `chunk_tokens` words) yields exactly one chunk covering
    /// the whole text; whitespace-only / empty text yields no chunks.
    pub fn chunk_document(
        text: &str,
        chunk_tokens: usize,
        overlap_tokens: usize,
    ) -> Vec<TextChunk> {
        let chunk_tokens = chunk_tokens.max(1);
        // Overlap must be strictly smaller than the chunk size to guarantee the
        // window advances every iteration (otherwise the loop could not progress).
        let overlap_tokens = overlap_tokens.min(chunk_tokens.saturating_sub(1));

        // Byte span (start, end) of every whitespace-delimited word. Using byte
        // offsets makes `&text[start..end]` slice exactly, so chunks round-trip.
        let mut words: Vec<(usize, usize)> = Vec::new();
        let mut word_start: Option<usize> = None;
        for (i, ch) in text.char_indices() {
            if ch.is_whitespace() {
                if let Some(s) = word_start.take() {
                    words.push((s, i));
                }
            } else if word_start.is_none() {
                word_start = Some(i);
            }
        }
        if let Some(s) = word_start {
            words.push((s, text.len()));
        }

        let n = words.len();
        if n == 0 {
            return Vec::new();
        }

        // `sentence_end[i]` marks word `i` as the last word of a sentence or
        // paragraph: it ends with terminal punctuation (ignoring trailing
        // quotes/brackets), the gap to the next word contains a newline, or it is
        // the final word of the text.
        let bytes = text.as_bytes();
        let mut sentence_end = vec![false; n];
        for i in 0..n {
            let (start, end) = words[i];
            let word_str = &text[start..end];
            let trimmed = word_str.trim_end_matches(['"', '\'', ')', ']', '}', '»', '”', '’']);
            let ends_punct =
                trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?');
            let gap_has_newline = if i + 1 < n {
                bytes[end..words[i + 1].0].contains(&b'\n')
            } else {
                true
            };
            sentence_end[i] = ends_punct || gap_has_newline;
        }

        // Don't let a sentence-boundary trim produce a chunk smaller than half the
        // target; below that we keep the hard word cap instead.
        let min_chunk = (chunk_tokens / 2).max(1);

        let mut chunks = Vec::new();
        let mut start = 0usize;
        loop {
            let hard_end = (start + chunk_tokens).min(n);
            let mut end = hard_end;

            if hard_end < n {
                // Prefer the last sentence boundary in [start + min_chunk, hard_end).
                let lower = start + min_chunk;
                let mut w = hard_end;
                while w > lower {
                    w -= 1;
                    if sentence_end[w] {
                        end = w + 1; // `end` is exclusive
                        break;
                    }
                }
            }

            let char_start = words[start].0;
            let char_end = words[end - 1].1;
            chunks.push(TextChunk {
                text: text[char_start..char_end].to_string(),
                char_start,
                char_end,
            });

            if end >= n {
                break;
            }

            // Start the next chunk `overlap_tokens` words before this one ended,
            // but always advance by at least one word.
            let mut next_start = end.saturating_sub(overlap_tokens);
            if next_start <= start {
                next_start = start + 1;
            }
            start = next_start;
        }

        chunks
    }

    /// Embed a search query (with caching for repeated queries)
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // Check cache first. Key includes model id + dim so that switching
        // models/providers never returns a stale vector from a prior model.
        let cache_key = format!(
            "query:{}:{}:{}",
            self.model_id(),
            self.text_output_dim(),
            query
        );
        {
            let mut cache = self.cache.write();
            if let Some(cached) = cache.get(&cache_key) {
                debug!("Embedding cache hit for query: {}", query);
                return Ok(cached);
            }
        }

        // Generate embedding
        let embedding = self.embed_query_uncached(query).await?;

        // Cache the result
        {
            let mut cache = self.cache.write();
            cache.insert(cache_key, embedding.clone());
        }

        Ok(embedding)
    }

    /// Embed a search query without caching (internal use)
    async fn embed_query_uncached(&self, query: &str) -> Result<Vec<f32>> {
        // Use Ollama if available (query task prefix applied inside embed_query).
        if let Some(ollama) = &self.ollama {
            if matches!(self.config.api_provider, EmbeddingProvider::Ollama) {
                match ollama.embed_query(query).await {
                    Ok(embedding) => return Ok(Self::normalized(embedding)),
                    Err(e) => {
                        warn!("Ollama embedding failed, falling back to hash: {}", e);
                    }
                }
            }
        }

        // Use OpenAI API if available
        if let Some(api_key) = &self.config.api_key {
            if matches!(self.config.api_provider, EmbeddingProvider::OpenAI) {
                return Ok(Self::normalized(
                    self.embed_with_openai(query, api_key).await?,
                ));
            }
        }

        // Fallback to hash-based embedding (already normalized).
        Ok(self.hash_embed(query, self.text_output_dim()))
    }

    /// Get embedding cache statistics (hits, misses, size)
    pub fn cache_stats(&self) -> (u64, u64, usize) {
        self.cache.read().stats()
    }

    /// Clear the embedding cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
        debug!("Embedding cache cleared");
    }

    /// Embed multiple texts (document path) using the configured backend.
    /// Every returned vector is L2-normalized exactly once.
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if let Some(ollama) = &self.ollama {
            if matches!(self.config.api_provider, EmbeddingProvider::Ollama) {
                match ollama.embed_documents(texts).await {
                    Ok(embs) => return Ok(embs.into_iter().map(Self::normalized).collect()),
                    Err(e) => warn!("Ollama batch embedding failed, falling back to hash: {}", e),
                }
            }
        }
        // Fallback to individual hash embeddings (already normalized).
        Ok(texts
            .iter()
            .map(|t| self.hash_embed(t, self.text_output_dim()))
            .collect())
    }

    /// Batch embed multiple memories efficiently
    /// Groups memories by type and uses batch API for text-based content
    pub async fn embed_memories_batch(
        &self,
        memories: &[Memory],
    ) -> Result<Vec<(MemoryId, Vec<f32>, &'static str)>> {
        let mut results: Vec<(MemoryId, Vec<f32>, &'static str)> =
            Vec::with_capacity(memories.len());

        // Separate memories by type for optimized processing
        let mut text_memories: Vec<(usize, &Memory)> = Vec::new();
        let mut image_memories: Vec<(usize, &Memory)> = Vec::new();
        let mut code_memories: Vec<(usize, &Memory)> = Vec::new();

        for (idx, memory) in memories.iter().enumerate() {
            match &memory.kind {
                MemoryKind::Image { .. } => image_memories.push((idx, memory)),
                MemoryKind::Code { .. } => code_memories.push((idx, memory)),
                _ => text_memories.push((idx, memory)),
            }
        }

        // Process images individually (local computation, fast)
        for (_idx, memory) in &image_memories {
            match self.embed_image(&memory.path).await {
                Ok(embedding) => {
                    results.push((memory.id, embedding, "image_embedding"));
                }
                Err(e) => {
                    warn!("Failed to embed image {}: {}", memory.id, e);
                }
            }
        }

        // Batch process code files if we have Ollama, otherwise use hash
        if !code_memories.is_empty() {
            let code_texts: Vec<String> = code_memories
                .iter()
                .map(|(_, m)| {
                    std::fs::read_to_string(&m.path)
                        .unwrap_or_else(|_| m.path.display().to_string())
                        .chars()
                        .take(4000) // Limit for efficiency
                        .collect()
                })
                .collect();

            if let Some(ollama) = &self.ollama {
                // Batch embed code with Ollama (document task prefix applied).
                match ollama.embed_documents(&code_texts).await {
                    Ok(embeddings) => {
                        for (embedding, (_, memory)) in
                            embeddings.into_iter().zip(code_memories.iter())
                        {
                            results.push((
                                memory.id,
                                Self::normalized(embedding),
                                "code_embedding",
                            ));
                        }
                    }
                    Err(e) => {
                        warn!("Ollama batch code embedding failed, using fallback: {}", e);
                        for (text, (_, memory)) in code_texts.iter().zip(code_memories.iter()) {
                            results.push((
                                memory.id,
                                self.hash_embed(text, CODE_EMBEDDING_DIM),
                                "code_embedding",
                            ));
                        }
                    }
                }
            } else {
                // Fallback to hash embeddings
                for (text, (_, memory)) in code_texts.iter().zip(code_memories.iter()) {
                    results.push((
                        memory.id,
                        self.hash_embed(text, CODE_EMBEDDING_DIM),
                        "code_embedding",
                    ));
                }
            }
        }

        // Batch process text/document memories
        if !text_memories.is_empty() {
            let texts: Vec<String> = text_memories
                .iter()
                .map(|(_, m)| {
                    // Get text content for embedding
                    let tags: Vec<String> = m.tags.iter().map(|t| t.name.clone()).collect();
                    let content = std::fs::read_to_string(&m.path)
                        .ok()
                        .map(|s| s.chars().take(4000).collect::<String>())
                        .unwrap_or_default();

                    format!("{} {} {}", m.path.display(), tags.join(" "), content)
                })
                .collect();

            if let Some(ollama) = &self.ollama {
                // Batch embed with Ollama (document task prefix applied).
                match ollama.embed_documents(&texts).await {
                    Ok(embeddings) => {
                        for (embedding, (_, memory)) in
                            embeddings.into_iter().zip(text_memories.iter())
                        {
                            results.push((
                                memory.id,
                                Self::normalized(embedding),
                                "text_embedding",
                            ));
                        }
                    }
                    Err(e) => {
                        warn!("Ollama batch text embedding failed, using fallback: {}", e);
                        for (text, (_, memory)) in texts.iter().zip(text_memories.iter()) {
                            results.push((
                                memory.id,
                                self.hash_embed(text, TEXT_EMBEDDING_DIM),
                                "text_embedding",
                            ));
                        }
                    }
                }
            } else {
                // Fallback to hash embeddings
                for (text, (_, memory)) in texts.iter().zip(text_memories.iter()) {
                    results.push((
                        memory.id,
                        self.hash_embed(text, TEXT_EMBEDDING_DIM),
                        "text_embedding",
                    ));
                }
            }
        }

        debug!(
            "Batch embedded {} memories ({} images, {} code, {} text)",
            results.len(),
            image_memories.len(),
            code_memories.len(),
            text_memories.len()
        );

        Ok(results)
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
            // text-embedding-ada-002 is deprecated; text-embedding-3-small is
            // the current small model (also 1536-dim, cheaper, higher quality).
            model: "text-embedding-3-small".to_string(),
            input: truncated,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(
                "OpenAI embedding failed with status {}, falling back to local",
                status
            );
            // Keep the fallback in the OpenAI space dimension so it matches the
            // collection created for the OpenAI provider.
            return Ok(self.hash_embed(text, OPENAI_EMBEDDING_DIM));
        }

        let embed_response: EmbedResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse embedding response: {}", e)))?;

        embed_response
            .data
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

    /// Canonical cosine similarity between two embeddings.
    ///
    /// All embeddings produced by this module are L2-normalized exactly once at
    /// generation time, so for such vectors the dot product IS the cosine
    /// similarity. Returns 0.0 on dimension mismatch.
    ///
    /// NOTE: `search::semantic_score`, `storage::cosine_similarity` and
    /// `duplicates::cosine_similarity` are near-duplicates of this. Collapsing
    /// them into this one is a cross-module follow-up (those files are owned by
    /// other agents / depend on true-magnitude cosine in existing tests).
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
        let embeddings_vec: Vec<(String, Vec<f32>)> = self
            .embeddings
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
