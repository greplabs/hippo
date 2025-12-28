//! File indexing and metadata extraction
//!
//! The indexer walks through sources, extracts metadata, and queues
//! files for embedding generation.

use crate::{
    embeddings::Embedder,
    error::{HippoError, Result},
    models::*,
    ollama::OllamaClient,
    storage::Storage,
    HippoConfig,
};

use rayon::prelude::*;
use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tokio::task::JoinHandle;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::sync::{mpsc, watch, RwLock};
use tracing::{debug, info, instrument, warn};
use walkdir::WalkDir;

pub mod code_parser;
pub mod extractors;

/// Progress stage during indexing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum IndexingStage {
    Scanning,
    Embedding,
    Tagging,
    Complete,
}

/// Indexing progress information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexingProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: Option<String>,
    pub stage: IndexingStage,
    pub is_paused: bool,
    pub files_per_second: f64,
    pub estimated_seconds_remaining: Option<u64>,
}

impl Default for IndexingProgress {
    fn default() -> Self {
        Self {
            total: 0,
            processed: 0,
            current_file: None,
            stage: IndexingStage::Scanning,
            is_paused: false,
            files_per_second: 0.0,
            estimated_seconds_remaining: None,
        }
    }
}

impl IndexingProgress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.processed as f64 / self.total as f64) * 100.0
        }
    }
}

/// File with priority for indexing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PrioritizedFile {
    path: PathBuf,
    source: Source,
    priority: u8, // Higher is more urgent (0-255)
}

/// Shared indexing state
struct IndexingState {
    progress: RwLock<IndexingProgress>,
    is_paused: AtomicBool,
    /// Flag to signal shutdown to background tasks
    shutdown: AtomicBool,
    priority_queue: RwLock<VecDeque<PrioritizedFile>>,
    start_time: RwLock<Option<std::time::Instant>>,
    files_processed_count: AtomicUsize,
}

impl IndexingState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            progress: RwLock::new(IndexingProgress::new()),
            is_paused: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            priority_queue: RwLock::new(VecDeque::new()),
            start_time: RwLock::new(None),
            files_processed_count: AtomicUsize::new(0),
        })
    }

    /// Signal shutdown to background tasks
    fn signal_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown was signaled
    #[allow(dead_code)] // Reserved for future use in background worker
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    async fn update_progress(&self, update_fn: impl FnOnce(&mut IndexingProgress)) {
        let mut progress = self.progress.write().await;
        update_fn(&mut progress);

        // Update ETA
        if progress.processed > 0 {
            let start_time = self.start_time.read().await;
            if let Some(start) = *start_time {
                let elapsed = start.elapsed().as_secs_f64();
                progress.files_per_second = progress.processed as f64 / elapsed;

                let remaining = progress.total.saturating_sub(progress.processed);
                if progress.files_per_second > 0.0 {
                    progress.estimated_seconds_remaining =
                        Some((remaining as f64 / progress.files_per_second) as u64);
                }
            }
        }
    }

    async fn reset_progress(&self) {
        let mut progress = self.progress.write().await;
        *progress = IndexingProgress::new();
        self.files_processed_count.store(0, Ordering::SeqCst);
        *self.start_time.write().await = None;
    }
}

/// The main indexer that orchestrates file discovery and processing
pub struct Indexer {
    storage: Arc<Storage>,
    #[allow(dead_code)] // Reserved for embedding generation during indexing
    embedder: Arc<Embedder>,
    #[allow(dead_code)] // Configuration is passed at construction, stored for future use
    config: IndexerConfig,
    task_tx: mpsc::Sender<IndexTask>,
    state: Arc<IndexingState>,
    /// Watch channel for progress updates (only keeps latest value, no buffer accumulation)
    progress_tx: watch::Sender<IndexingProgress>,
    /// Handle to the background worker task for proper cleanup
    worker_handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub parallelism: usize,
    pub batch_size: usize,
    pub supported_extensions: Vec<String>,
    pub auto_tag_enabled: bool,
    /// When true, only re-index files that have been modified since last indexing
    pub smart_reindex: bool,
    /// Directory patterns to skip (e.g., ".git", "node_modules")
    pub skip_patterns: Vec<String>,
    /// When true, generate AI embeddings via Ollama (slower but enables semantic search)
    /// When false, use fast hash-based embeddings (instant, good for basic search)
    pub generate_ai_embeddings: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            // Higher parallelism for I/O-bound operations
            parallelism: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
                .min(16),
            // Larger batch size for better throughput (memory is cheap, latency is expensive)
            batch_size: 200,
            supported_extensions: vec![
                // Images
                "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "heic", "heif", "raw", "cr2",
                "nef", // Videos
                "mp4", "mov", "avi", "mkv", "webm", "m4v", // Audio
                "mp3", "wav", "flac", "m4a", "ogg", "aac", // Documents
                "pdf", "doc", "docx", "txt", "md", "rtf", "odt", // Spreadsheets
                "xls", "xlsx", "csv", "ods", // Presentations
                "ppt", "pptx", "odp", // Code
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp", "rb",
                "php", "swift", "kt", "scala", "sh", "bash", "zsh", "sql", "html", "css", "json",
                "yaml", "yml", "toml", "xml", // Archives
                "zip", "tar", "gz", "7z", "rar",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            auto_tag_enabled: false,
            // Smart re-indexing enabled by default for faster syncs
            smart_reindex: true,
            // Skip common non-user directories for faster indexing
            skip_patterns: vec![
                ".git",
                "node_modules",
                ".venv",
                "__pycache__",
                ".cache",
                ".npm",
                "target",  // Rust build directory
                "build",
                "dist",
                ".DS_Store",
                "Thumbs.db",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            // Fast mode by default - use hash embeddings for instant indexing
            // Set to true to enable AI-powered semantic search (requires Ollama)
            generate_ai_embeddings: false,
        }
    }
}

/// Result of checking if a file needs re-indexing
#[derive(Debug)]
pub enum ReindexStatus {
    /// File is new, needs full indexing
    New,
    /// File has been modified since last index
    Modified,
    /// File hasn't changed, skip re-indexing
    Unchanged,
    /// Error checking status
    Error(String),
}

impl Indexer {
    /// Check if a file needs re-indexing based on modification time
    pub async fn needs_reindex(storage: &Storage, path: &Path) -> ReindexStatus {
        // Get file's current modification time
        let file_mtime = match std::fs::metadata(path) {
            Ok(meta) => match meta.modified() {
                Ok(mtime) => mtime,
                Err(e) => return ReindexStatus::Error(format!("Failed to get mtime: {}", e)),
            },
            Err(e) => return ReindexStatus::Error(format!("Failed to get metadata: {}", e)),
        };

        // Check if file exists in database
        match storage.get_memory_by_path(path).await {
            Ok(Some(memory)) => {
                // Compare with indexed_at time
                let indexed_at = memory.indexed_at;
                let file_mtime_chrono = chrono::DateTime::<chrono::Utc>::from(file_mtime);

                if file_mtime_chrono > indexed_at {
                    ReindexStatus::Modified
                } else {
                    ReindexStatus::Unchanged
                }
            }
            Ok(None) => ReindexStatus::New,
            Err(e) => ReindexStatus::Error(format!("Database error: {}", e)),
        }
    }
}

#[derive(Debug)]
enum IndexTask {
    IndexPath(PathBuf, Source),
    #[allow(dead_code)] // Planned for selective re-indexing feature
    Reindex(MemoryId),
    #[allow(dead_code)] // Planned for graceful shutdown
    Shutdown,
}

impl Indexer {
    pub fn new(
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        config: &HippoConfig,
    ) -> Result<Self> {
        let (task_tx, task_rx) = mpsc::channel(1000);
        // Use watch channel - only keeps latest value, no buffer accumulation
        let (progress_tx, _progress_rx) = watch::channel(IndexingProgress::default());

        let indexer_config = IndexerConfig {
            parallelism: config.indexing_parallelism,
            auto_tag_enabled: config.auto_tag_enabled,
            ..Default::default()
        };

        let state = IndexingState::new();

        // Spawn background worker and store handle for cleanup
        let storage_clone = storage.clone();
        let embedder_clone = embedder.clone();
        let config_clone = indexer_config.clone();
        let state_clone = state.clone();
        let progress_tx_clone = progress_tx.clone();

        let worker_handle = tokio::spawn(async move {
            Self::background_worker(
                task_rx,
                storage_clone,
                embedder_clone,
                config_clone,
                state_clone,
                progress_tx_clone,
            )
            .await;
        });

        Ok(Self {
            storage,
            embedder,
            config: indexer_config,
            task_tx,
            state,
            progress_tx,
            worker_handle: Some(worker_handle),
        })
    }

    /// Gracefully shutdown the indexer
    pub async fn shutdown(&self) {
        info!("Shutting down indexer...");
        self.state.signal_shutdown();
        // Send shutdown task to break the worker loop
        let _ = self.task_tx.send(IndexTask::Shutdown).await;
    }

    /// Get current indexing progress
    pub async fn get_progress(&self) -> IndexingProgress {
        let progress = self.state.progress.read().await;
        progress.clone()
    }

    /// Subscribe to indexing progress updates
    /// Returns a watch::Receiver that always has the latest progress value
    pub fn subscribe_progress(&self) -> watch::Receiver<IndexingProgress> {
        self.progress_tx.subscribe()
    }

    /// Pause indexing
    pub async fn pause(&self) -> Result<()> {
        self.state.is_paused.store(true, Ordering::SeqCst);
        self.state
            .update_progress(|p| {
                p.is_paused = true;
            })
            .await;
        info!("Indexing paused");
        Ok(())
    }

    /// Resume indexing
    pub async fn resume(&self) -> Result<()> {
        self.state.is_paused.store(false, Ordering::SeqCst);
        self.state
            .update_progress(|p| {
                p.is_paused = false;
            })
            .await;
        info!("Indexing resumed");
        Ok(())
    }

    /// Check if indexing is paused
    pub fn is_paused(&self) -> bool {
        self.state.is_paused.load(Ordering::SeqCst)
    }

    /// Set priority for a specific file path
    pub async fn set_priority(&self, path: &Path, source: Source) -> Result<()> {
        let mut queue = self.state.priority_queue.write().await;
        queue.push_front(PrioritizedFile {
            path: path.to_path_buf(),
            source,
            priority: 255, // Highest priority
        });
        info!("Added high-priority file to queue: {:?}", path);
        Ok(())
    }

    /// Get a reference to the embedder
    pub fn embedder(&self) -> &Arc<Embedder> {
        &self.embedder
    }

    /// Queue a source for indexing
    #[instrument(skip(self))]
    pub async fn queue_source(&self, source: Source) -> Result<()> {
        match &source {
            Source::Local { root_path } => {
                println!("[Indexer] Queuing local path: {:?}", root_path);
                info!("Queuing local path for indexing: {:?}", root_path);
                self.task_tx
                    .send(IndexTask::IndexPath(root_path.clone(), source))
                    .await
                    .map_err(|e| HippoError::Indexing(e.to_string()))?;
                println!("[Indexer] Task queued successfully");
            }
            _ => {
                println!("[Indexer] Cloud source not implemented: {:?}", source);
                warn!("Cloud source indexing not yet implemented: {:?}", source);
            }
        }
        Ok(())
    }

    /// Sync a specific source
    pub async fn sync_source(&self, source: &Source) -> Result<()> {
        match source {
            Source::Local { root_path } => {
                self.task_tx
                    .send(IndexTask::IndexPath(root_path.clone(), source.clone()))
                    .await
                    .map_err(|e| HippoError::Indexing(e.to_string()))?;
            }
            _ => {
                warn!("Cloud sync not yet implemented");
            }
        }
        Ok(())
    }

    /// Index a single file (used by file watcher)
    pub async fn index_single_file(&self, path: &Path, source: &Source) -> Result<()> {
        // Check if file exists and has supported extension
        if !path.exists() || !path.is_file() {
            return Err(HippoError::Indexing(format!(
                "File does not exist: {:?}",
                path
            )));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !self.config.supported_extensions.contains(&ext) {
            return Ok(()); // Silently skip unsupported files
        }

        // Process the file
        let memory = Self::process_file(path, source)?;

        // Store it
        self.storage.upsert_memory(&memory).await?;

        // Generate and store embedding
        match self.embedder.embed_memory(&memory).await {
            Ok(embedding) => {
                let model_name = match &memory.kind {
                    MemoryKind::Image { .. } => "image_embedding",
                    MemoryKind::Code { .. } => "code_embedding",
                    _ => "text_embedding",
                };
                if let Err(e) = self
                    .storage
                    .store_embedding_with_qdrant(memory.id, &embedding, model_name, &memory.kind)
                    .await
                {
                    debug!("Failed to store embedding for {}: {}", memory.id, e);
                }
            }
            Err(e) => {
                debug!("Failed to embed memory {}: {}", memory.id, e);
            }
        }

        info!("Indexed single file: {:?}", path);
        Ok(())
    }

    /// Background worker that processes index tasks
    async fn background_worker(
        mut rx: mpsc::Receiver<IndexTask>,
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        config: IndexerConfig,
        state: Arc<IndexingState>,
        progress_tx: watch::Sender<IndexingProgress>,
    ) {
        println!("[Indexer] Background worker started");
        info!("Indexer background worker started");

        while let Some(task) = rx.recv().await {
            println!("[Indexer] Received task: {:?}", task);
            match task {
                IndexTask::IndexPath(path, source) => {
                    println!("[Indexer] Starting to index: {:?}", path);
                    // Reset progress for new indexing task
                    state.reset_progress().await;
                    *state.start_time.write().await = Some(std::time::Instant::now());

                    if let Err(e) = Self::index_path_with_progress(
                        &path,
                        &source,
                        &storage,
                        &embedder,
                        &config,
                        &state,
                        &progress_tx,
                    )
                    .await
                    {
                        println!("[Indexer] Failed to index {:?}: {}", path, e);
                        warn!("Failed to index path {:?}: {}", path, e);
                    } else {
                        println!("[Indexer] Finished indexing: {:?}", path);
                    }

                    // Mark as complete
                    state
                        .update_progress(|p| {
                            p.stage = IndexingStage::Complete;
                        })
                        .await;
                    let _ = progress_tx.send(state.progress.read().await.clone());
                }
                IndexTask::Reindex(id) => {
                    debug!("Reindexing memory: {}", id);
                }
                IndexTask::Shutdown => {
                    println!("[Indexer] Shutting down");
                    info!("Indexer shutting down");
                    break;
                }
            }
        }
        println!("[Indexer] Background worker stopped");
    }

    /// Index all files in a path with progress tracking
    async fn index_path_with_progress(
        path: &Path,
        source: &Source,
        storage: &Storage,
        embedder: &Embedder,
        config: &IndexerConfig,
        state: &Arc<IndexingState>,
        progress_tx: &watch::Sender<IndexingProgress>,
    ) -> Result<()> {
        println!("[Indexer] Starting index of path: {:?}", path);
        info!("Starting index of path: {:?}", path);

        if !path.exists() {
            println!("[Indexer] Path does not exist: {:?}", path);
            return Err(HippoError::Indexing(format!(
                "Path does not exist: {:?}",
                path
            )));
        }

        // STAGE 1: Scanning
        state
            .update_progress(|p| {
                p.stage = IndexingStage::Scanning;
                p.current_file = Some("Scanning directory...".to_string());
            })
            .await;
        let _ = progress_tx.send(state.progress.read().await.clone());

        // Collect all supported files, skipping excluded directories
        let skip_patterns = config.skip_patterns.clone();
        let all_files: Vec<PathBuf> = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                // Skip directories matching skip patterns
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        // Skip hidden directories and excluded patterns
                        if name.starts_with('.') || skip_patterns.contains(&name.to_string()) {
                            return false;
                        }
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| config.supported_extensions.contains(&ext.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Smart re-indexing: filter to only files that need re-indexing
        let (files, skipped_count) = if config.smart_reindex {
            let mut needs_index = Vec::with_capacity(all_files.len());
            let mut skipped = 0usize;

            for file_path in all_files {
                match Self::needs_reindex(storage, &file_path).await {
                    ReindexStatus::New | ReindexStatus::Modified => {
                        needs_index.push(file_path);
                    }
                    ReindexStatus::Unchanged => {
                        skipped += 1;
                    }
                    ReindexStatus::Error(e) => {
                        debug!("Error checking reindex status for {:?}: {}", file_path, e);
                        // On error, include the file to be safe
                        needs_index.push(file_path);
                    }
                }
            }

            (needs_index, skipped)
        } else {
            (all_files, 0)
        };

        if skipped_count > 0 {
            println!(
                "[Indexer] Smart reindex: {} unchanged files skipped, {} files to process",
                skipped_count,
                files.len()
            );
            info!(
                "Smart reindex: {} unchanged files skipped, {} files to process",
                skipped_count,
                files.len()
            );
        }

        println!("[Indexer] Found {} files to index", files.len());
        info!("Found {} files to index", files.len());

        // Set total count
        state
            .update_progress(|p| {
                p.total = files.len();
                p.stage = IndexingStage::Embedding;
            })
            .await;
        let _ = progress_tx.send(state.progress.read().await.clone());

        // STAGE 2: Process in batches with pause support
        for (batch_idx, batch) in files.chunks(config.batch_size).enumerate() {
            // Check for pause
            while state.is_paused.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            println!(
                "[Indexer] Processing batch {} ({} files)",
                batch_idx + 1,
                batch.len()
            );

            let memories: Vec<Memory> = batch
                .par_iter()
                .filter_map(|file_path| {
                    // Update current file (note: may be overwritten by parallel processing)
                    let _file_name = file_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    match Self::process_file(file_path, source) {
                        Ok(memory) => Some(memory),
                        Err(e) => {
                            debug!("Failed to process file {:?}: {}", file_path, e);
                            None
                        }
                    }
                })
                .collect();

            println!("[Indexer] Processed {} memories from batch", memories.len());

            // Store memories and track progress
            for memory in &memories {
                state
                    .update_progress(|p| {
                        p.current_file = Some(
                            memory
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        );
                        p.stage = IndexingStage::Embedding;
                    })
                    .await;

                if let Err(e) = storage.upsert_memory(memory).await {
                    println!("[Indexer] Failed to store memory: {}", e);
                    warn!("Failed to store memory: {}", e);
                }

                // Check for pause
                while state.is_paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            // Generate and store embeddings in batch (much faster)
            // Skip if generate_ai_embeddings is disabled for fast indexing
            if config.generate_ai_embeddings {
                state
                    .update_progress(|p| {
                        p.current_file =
                            Some(format!("Embedding batch of {} files...", memories.len()));
                    })
                    .await;

            // Use batch embedding for efficiency
            match embedder.embed_memories_batch(&memories).await {
                Ok(embeddings) => {
                    for (memory_id, embedding, model_name) in embeddings {
                        // Check for pause
                        while state.is_paused.load(Ordering::SeqCst) {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }

                        // Find the memory to get its kind
                        if let Some(memory) = memories.iter().find(|m| m.id == memory_id) {
                            if let Err(e) = storage
                                .store_embedding_with_qdrant(
                                    memory_id,
                                    &embedding,
                                    model_name,
                                    &memory.kind,
                                )
                                .await
                            {
                                debug!("Failed to store embedding for {}: {}", memory_id, e);
                            }

                            // Update progress counter
                            state.files_processed_count.fetch_add(1, Ordering::SeqCst);
                            state
                                .update_progress(|p| {
                                    p.processed += 1;
                                    p.current_file = Some(
                                        memory
                                            .path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                    );
                                })
                                .await;
                            let _ = progress_tx.send(state.progress.read().await.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!("Batch embedding failed: {}, falling back to individual", e);
                    // Fallback to individual embedding if batch fails
                    for memory in &memories {
                        match embedder.embed_memory(memory).await {
                            Ok(embedding) => {
                                let model_name = match &memory.kind {
                                    MemoryKind::Image { .. } => "image_embedding",
                                    MemoryKind::Code { .. } => "code_embedding",
                                    _ => "text_embedding",
                                };
                                if let Err(e) = storage
                                    .store_embedding_with_qdrant(
                                        memory.id,
                                        &embedding,
                                        model_name,
                                        &memory.kind,
                                    )
                                    .await
                                {
                                    debug!("Failed to store embedding for {}: {}", memory.id, e);
                                }
                            }
                            Err(e) => {
                                debug!("Failed to embed memory {}: {}", memory.id, e);
                            }
                        }

                        state.files_processed_count.fetch_add(1, Ordering::SeqCst);
                        state
                            .update_progress(|p| {
                                p.processed += 1;
                            })
                            .await;
                        let _ = progress_tx.send(state.progress.read().await.clone());
                    }
                }
            }
            } else {
                // Fast mode: skip AI embeddings, just update progress
                for memory in &memories {
                    state.files_processed_count.fetch_add(1, Ordering::SeqCst);
                    state
                        .update_progress(|p| {
                            p.processed += 1;
                            p.current_file = Some(
                                memory
                                    .path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            );
                        })
                        .await;
                    let _ = progress_tx.send(state.progress.read().await.clone());
                }
            }

            // STAGE 3: Auto-tagging (if enabled)
            if config.auto_tag_enabled && !memories.is_empty() {
                state
                    .update_progress(|p| {
                        p.stage = IndexingStage::Tagging;
                    })
                    .await;
                let _ = progress_tx.send(state.progress.read().await.clone());

                Self::auto_tag_batch(&memories, storage).await;

                // Check for pause
                while state.is_paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            println!("[Indexer] Stored batch of {} files", memories.len());
            info!("Processed batch of {} files", memories.len());
        }

        println!("[Indexer] Completed indexing path: {:?}", path);
        info!("Completed indexing path: {:?}", path);
        Ok(())
    }

    /// Index all files in a path (backward compatibility wrapper)
    #[allow(dead_code)]
    #[instrument(skip(storage, embedder, config))]
    async fn index_path(
        path: &Path,
        source: &Source,
        storage: &Storage,
        embedder: &Embedder,
        config: &IndexerConfig,
    ) -> Result<()> {
        // Create minimal state for compatibility
        let state = IndexingState::new();
        let (progress_tx, _) = watch::channel(IndexingProgress::default());

        Self::index_path_with_progress(
            path,
            source,
            storage,
            embedder,
            config,
            &state,
            &progress_tx,
        )
        .await
    }

    /// Auto-tag a batch of memories using Ollama with content-aware analysis
    ///
    /// This uses actual file content for smart tagging:
    /// - Images: Vision model (llava) for scene/object detection + EXIF metadata
    /// - Audio: Artist, album, genre from ID3/metadata tags
    /// - Video: Scene description + technical metadata
    /// - Code/Documents: Content analysis for topics and keywords
    async fn auto_tag_batch(memories: &[Memory], storage: &Storage) {
        let ollama_client = OllamaClient::new();

        // Check if Ollama is available
        if !ollama_client.is_available().await {
            debug!("Ollama not available, skipping auto-tagging");
            return;
        }

        println!(
            "[Indexer] Auto-tagging {} files with content-aware AI",
            memories.len()
        );

        for memory in memories {
            let filename = memory
                .path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut all_tags: Vec<String> = Vec::new();

            match &memory.kind {
                MemoryKind::Image { .. } => {
                    // 1. Try vision model for image content analysis
                    if let Ok(caption) = ollama_client.caption_image(&memory.path).await {
                        // Extract tags from the image caption
                        let prompt = format!(
                            "Based on this image description, suggest 5-8 single-word tags for categorization. Description: '{}'. Return only comma-separated lowercase tags, no explanations.",
                            caption.chars().take(500).collect::<String>()
                        );
                        if let Ok(response) = ollama_client.generate(&prompt, None).await {
                            let caption_tags: Vec<String> = response
                                .split(',')
                                .map(|s| s.trim().to_lowercase().replace(' ', "-"))
                                .filter(|s| !s.is_empty() && s.len() <= 30 && s.len() > 1)
                                .take(8)
                                .collect();
                            all_tags.extend(caption_tags);
                        }
                    }

                    // 2. Add tags from EXIF metadata
                    if let Some(ref exif) = memory.metadata.exif {
                        // Camera/equipment tags
                        if let Some(ref make) = exif.camera_make {
                            let clean_make = make.trim().trim_matches('"').to_lowercase();
                            if !clean_make.is_empty() {
                                all_tags.push(format!(
                                    "camera:{}",
                                    clean_make.split_whitespace().next().unwrap_or(&clean_make)
                                ));
                            }
                        }
                        // Add photography-related tags based on settings
                        if exif.aperture.is_some() || exif.iso.is_some() {
                            all_tags.push("photography".to_string());
                        }
                    }

                    // 3. Add location tags from GPS
                    if let Some(ref loc) = memory.metadata.location {
                        if let Some(ref city) = loc.city {
                            all_tags.push(format!("location:{}", city.to_lowercase()));
                        }
                        if let Some(ref country) = loc.country {
                            all_tags.push(format!("country:{}", country.to_lowercase()));
                        }
                        // Generic geo tag if we have coordinates
                        if loc.latitude != 0.0 && loc.longitude != 0.0 {
                            all_tags.push("geotagged".to_string());
                        }
                    }
                }

                MemoryKind::Audio { .. } => {
                    // Extract tags from audio metadata (ID3, Vorbis, etc.)
                    if let Some(ref audio) = memory.metadata.audio_metadata {
                        if let Some(ref artist) = audio.artist {
                            let clean_artist = artist.trim().to_lowercase();
                            if !clean_artist.is_empty() && clean_artist.len() <= 30 {
                                all_tags.push(format!("artist:{}", clean_artist.replace(' ', "-")));
                            }
                        }
                        if let Some(ref album) = audio.album {
                            let clean_album = album.trim().to_lowercase();
                            if !clean_album.is_empty() && clean_album.len() <= 30 {
                                all_tags.push(format!("album:{}", clean_album.replace(' ', "-")));
                            }
                        }
                        if let Some(ref genre) = audio.genre {
                            let clean_genre = genre.trim().to_lowercase();
                            if !clean_genre.is_empty() {
                                all_tags.push(format!("genre:{}", clean_genre.replace(' ', "-")));
                            }
                        }
                        if let Some(year) = audio.year {
                            if year > 1900 && year < 2100 {
                                all_tags.push(format!("year:{}", year));
                            }
                        }
                    }
                    all_tags.push("music".to_string());
                }

                MemoryKind::Video { duration_ms, .. } => {
                    // Add duration-based tags
                    let duration_secs = *duration_ms / 1000;
                    if duration_secs < 60 {
                        all_tags.push("short-video".to_string());
                    } else if duration_secs < 600 {
                        all_tags.push("clip".to_string());
                    } else {
                        all_tags.push("long-video".to_string());
                    }

                    // Extract tags from video metadata
                    if let Some(ref video) = memory.metadata.video_metadata {
                        if let (Some(w), Some(h)) = (video.width, video.height) {
                            if w >= 3840 || h >= 2160 {
                                all_tags.push("4k".to_string());
                            } else if w >= 1920 || h >= 1080 {
                                all_tags.push("hd".to_string());
                            }
                        }
                    }

                    // Use AI for video filename analysis (can't easily analyze video content)
                    let prompt = format!(
                        "Suggest 3-5 tags for a video file named '{}'. Return only comma-separated lowercase tags.",
                        filename
                    );
                    if let Ok(response) = ollama_client.generate(&prompt, None).await {
                        let video_tags: Vec<String> = response
                            .split(',')
                            .map(|s| s.trim().to_lowercase().replace(' ', "-"))
                            .filter(|s| !s.is_empty() && s.len() <= 30 && s.len() > 1)
                            .take(5)
                            .collect();
                        all_tags.extend(video_tags);
                    }
                }

                MemoryKind::Code { language, lines } => {
                    // Add language tag
                    all_tags.push(format!("lang:{}", language.to_lowercase()));

                    // Size-based tags
                    if *lines < 100 {
                        all_tags.push("small-file".to_string());
                    } else if *lines > 1000 {
                        all_tags.push("large-file".to_string());
                    }

                    // Analyze code content for imports/frameworks
                    if let Some(ref code_info) = memory.metadata.code_info {
                        // Detect frameworks from imports
                        for import in &code_info.imports {
                            let import_lower = import.to_lowercase();
                            if import_lower.contains("react") {
                                all_tags.push("react".to_string());
                            } else if import_lower.contains("tokio") {
                                all_tags.push("async".to_string());
                            } else if import_lower.contains("django")
                                || import_lower.contains("flask")
                            {
                                all_tags.push("web".to_string());
                            } else if import_lower.contains("tensorflow")
                                || import_lower.contains("torch")
                            {
                                all_tags.push("ml".to_string());
                            }
                        }

                        // Check for test files
                        if code_info
                            .functions
                            .iter()
                            .any(|f| f.name.starts_with("test_") || f.name.contains("test"))
                        {
                            all_tags.push("tests".to_string());
                        }
                    }
                }

                MemoryKind::Document { format, .. } => {
                    // Add format tag
                    all_tags.push(format!("doc:{:?}", format).to_lowercase());

                    // Analyze document content if available
                    if let Some(ref preview) = memory.metadata.text_preview {
                        if preview.len() > 50 {
                            let prompt = format!(
                                "Suggest 3-5 topic tags for this document excerpt: '{}'. Return only comma-separated lowercase tags.",
                                preview.chars().take(500).collect::<String>()
                            );
                            if let Ok(response) = ollama_client.generate(&prompt, None).await {
                                let doc_tags: Vec<String> = response
                                    .split(',')
                                    .map(|s| s.trim().to_lowercase().replace(' ', "-"))
                                    .filter(|s| !s.is_empty() && s.len() <= 30 && s.len() > 1)
                                    .take(5)
                                    .collect();
                                all_tags.extend(doc_tags);
                            }
                        }
                    }
                }

                _ => {
                    // Fallback: use filename-based tagging for other types
                    let kind_name = match &memory.kind {
                        MemoryKind::Spreadsheet { .. } => "spreadsheet",
                        MemoryKind::Presentation { .. } => "presentation",
                        MemoryKind::Archive { .. } => "archive",
                        _ => "file",
                    };

                    let prompt = format!(
                        "Suggest 3-5 tags for this file: {}, type: {}. Return only comma-separated lowercase tags.",
                        filename, kind_name
                    );
                    if let Ok(response) = ollama_client.generate(&prompt, None).await {
                        let fallback_tags: Vec<String> = response
                            .split(',')
                            .map(|s| s.trim().to_lowercase().replace(' ', "-"))
                            .filter(|s| !s.is_empty() && s.len() <= 30 && s.len() > 1)
                            .take(5)
                            .collect();
                        all_tags.extend(fallback_tags);
                    }
                }
            }

            // Deduplicate and add tags
            all_tags.sort();
            all_tags.dedup();

            // Add all discovered tags
            for tag_name in all_tags.into_iter().take(10) {
                let confidence = if tag_name.contains(':') { 90 } else { 75 }; // Metadata tags are more confident
                let tag = Tag::ai(tag_name.clone(), confidence);
                if let Err(e) = storage.add_tag(memory.id, tag).await {
                    debug!("Failed to add auto-tag '{}': {}", tag_name, e);
                }
            }

            debug!("Content-aware auto-tagged file: {}", filename);
        }

        println!("[Indexer] Content-aware auto-tagging completed");
    }

    /// Process a single file and extract metadata
    fn process_file(path: &Path, source: &Source) -> Result<Memory> {
        let kind = Self::detect_kind(path)?;
        let mut memory = Memory::new(path.to_path_buf(), source.clone(), kind);

        // Extract metadata based on file type
        memory.metadata = extractors::extract_metadata(path, &memory.kind)?;

        // Extract file stats
        let file_meta = std::fs::metadata(path)?;
        memory.metadata.file_size = file_meta.len();
        memory.created_at = file_meta
            .created()
            .map(chrono::DateTime::from)
            .unwrap_or_else(|_| chrono::Utc::now());
        memory.modified_at = file_meta
            .modified()
            .map(chrono::DateTime::from)
            .unwrap_or_else(|_| chrono::Utc::now());

        // Detect MIME type
        memory.metadata.mime_type = mime_guess::from_path(path).first().map(|m| m.to_string());

        // Compute content hash for duplicate detection
        // Skip very large files (> 500MB) to avoid slowdowns
        if memory.metadata.file_size < 500 * 1024 * 1024 {
            if let Ok(hash) = crate::duplicates::compute_file_hash(path) {
                memory.metadata.hash = Some(hash);
            }
        }

        // Add system tags based on file type
        memory
            .tags
            .push(Tag::system(format!("type:{}", memory.kind_name())));

        // Add folder-based tags
        if let Some(parent) = path.parent() {
            if let Some(folder_name) = parent.file_name() {
                memory.tags.push(Tag::system(format!(
                    "folder:{}",
                    folder_name.to_string_lossy()
                )));
            }
        }

        Ok(memory)
    }

    /// Detect the kind of file based on extension and content
    fn detect_kind(path: &Path) -> Result<MemoryKind> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let kind = match ext.as_str() {
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "heic" | "heif" | "raw"
            | "cr2" | "nef" => {
                // Try to get dimensions
                if let Ok(dims) = image::image_dimensions(path) {
                    MemoryKind::Image {
                        width: dims.0,
                        height: dims.1,
                        format: ext.clone(),
                    }
                } else {
                    MemoryKind::Image {
                        width: 0,
                        height: 0,
                        format: ext.clone(),
                    }
                }
            }

            // Videos
            "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" => {
                let duration_ms = Self::extract_video_duration(path).unwrap_or(0);
                MemoryKind::Video {
                    duration_ms,
                    format: ext.clone(),
                }
            }

            // Audio
            "mp3" | "wav" | "flac" | "m4a" | "ogg" | "aac" => {
                let duration_ms = Self::extract_audio_duration(path).unwrap_or(0);
                MemoryKind::Audio {
                    duration_ms,
                    format: ext.clone(),
                }
            }

            // Documents
            "pdf" => MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: None,
            },
            "doc" | "docx" => MemoryKind::Document {
                format: DocumentFormat::Word,
                page_count: None,
            },
            "txt" => MemoryKind::Document {
                format: DocumentFormat::PlainText,
                page_count: None,
            },
            "md" => MemoryKind::Document {
                format: DocumentFormat::Markdown,
                page_count: None,
            },
            "html" | "htm" => MemoryKind::Document {
                format: DocumentFormat::Html,
                page_count: None,
            },
            "rtf" => MemoryKind::Document {
                format: DocumentFormat::Rtf,
                page_count: None,
            },

            // Spreadsheets
            "xls" | "xlsx" | "csv" | "ods" => MemoryKind::Spreadsheet { sheet_count: 1 },

            // Presentations
            "ppt" | "pptx" | "odp" => MemoryKind::Presentation { slide_count: 0 },

            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh" | "sql"
            | "css" | "json" | "yaml" | "yml" | "toml" | "xml" => {
                let language = Self::detect_language(&ext);
                let lines = std::fs::read_to_string(path)
                    .map(|s| s.lines().count() as u32)
                    .unwrap_or(0);
                MemoryKind::Code { language, lines }
            }

            // Archives
            "zip" | "tar" | "gz" | "7z" | "rar" => MemoryKind::Archive { item_count: 0 },

            _ => MemoryKind::Unknown,
        };

        Ok(kind)
    }

    fn detect_language(ext: &str) -> String {
        match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "javascript-react",
            "tsx" => "typescript-react",
            "go" => "go",
            "java" => "java",
            "c" => "c",
            "cpp" | "cc" | "cxx" => "cpp",
            "h" | "hpp" => "c-header",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" => "kotlin",
            "scala" => "scala",
            "sh" | "bash" | "zsh" => "shell",
            "sql" => "sql",
            "html" | "htm" => "html",
            "css" => "css",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "xml" => "xml",
            _ => "unknown",
        }
        .to_string()
    }

    /// Extract audio duration in milliseconds using symphonia
    fn extract_audio_duration(path: &Path) -> Option<u64> {
        let file = File::open(path).ok()?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .ok()?;

        let format = probed.format;

        // Get the default track
        let track = format.default_track()?;

        // Calculate duration from codec params
        let time_base = track.codec_params.time_base?;
        let n_frames = track.codec_params.n_frames?;

        // Convert to milliseconds
        let duration_secs = time_base.calc_time(n_frames);
        let duration_ms = (duration_secs.seconds as f64 + duration_secs.frac) * 1000.0;

        Some(duration_ms as u64)
    }

    /// Extract video duration using ffprobe (requires ffmpeg to be installed)
    fn extract_video_duration(path: &Path) -> Option<u64> {
        use std::process::Command;

        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let duration_str = String::from_utf8_lossy(&output.stdout);
        let duration_secs: f64 = duration_str.trim().parse().ok()?;

        Some((duration_secs * 1000.0) as u64)
    }
}

// Extension trait to get a display name for MemoryKind
trait MemoryKindExt {
    fn kind_name(&self) -> &'static str;
}

impl MemoryKindExt for Memory {
    fn kind_name(&self) -> &'static str {
        match &self.kind {
            MemoryKind::Image { .. } => "image",
            MemoryKind::Video { .. } => "video",
            MemoryKind::Audio { .. } => "audio",
            MemoryKind::Document { .. } => "document",
            MemoryKind::Spreadsheet { .. } => "spreadsheet",
            MemoryKind::Presentation { .. } => "presentation",
            MemoryKind::Code { .. } => "code",
            MemoryKind::Archive { .. } => "archive",
            MemoryKind::Database => "database",
            MemoryKind::Folder => "folder",
            MemoryKind::Unknown => "unknown",
        }
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        // Signal shutdown to background worker
        self.state.signal_shutdown();

        // Abort the worker task if it's still running
        if let Some(handle) = self.worker_handle.take() {
            handle.abort();
            debug!("Indexer worker task aborted");
        }

        debug!("Indexer dropped, resources cleaned up");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_config_default() {
        let config = IndexerConfig::default();
        assert!(config.smart_reindex, "Smart reindex should be enabled by default");
        assert!(!config.auto_tag_enabled, "Auto-tagging should be disabled by default");
        assert!(config.parallelism >= 1, "Parallelism should be at least 1");
        assert!(config.batch_size > 0, "Batch size should be positive");
        assert!(
            config.supported_extensions.contains(&"jpg".to_string()),
            "Should support jpg extension"
        );
        assert!(
            config.supported_extensions.contains(&"rs".to_string()),
            "Should support rs extension"
        );
        assert!(
            config.supported_extensions.contains(&"pdf".to_string()),
            "Should support pdf extension"
        );
    }

    #[test]
    fn test_indexing_progress_percentage() {
        let mut progress = IndexingProgress::new();
        assert_eq!(progress.percentage(), 0.0);

        progress.total = 100;
        progress.processed = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);

        progress.processed = 100;
        assert!((progress.percentage() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_reindex_status_variants() {
        // Test that all variants exist and can be matched
        let new = ReindexStatus::New;
        let modified = ReindexStatus::Modified;
        let unchanged = ReindexStatus::Unchanged;
        let error = ReindexStatus::Error("test error".to_string());

        assert!(matches!(new, ReindexStatus::New));
        assert!(matches!(modified, ReindexStatus::Modified));
        assert!(matches!(unchanged, ReindexStatus::Unchanged));
        assert!(matches!(error, ReindexStatus::Error(_)));
    }

    #[test]
    fn test_supported_extensions_comprehensive() {
        let config = IndexerConfig::default();

        // Images
        for ext in &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "heic"] {
            assert!(
                config.supported_extensions.contains(&ext.to_string()),
                "Should support image extension: {}",
                ext
            );
        }

        // Videos
        for ext in &["mp4", "mov", "avi", "mkv", "webm"] {
            assert!(
                config.supported_extensions.contains(&ext.to_string()),
                "Should support video extension: {}",
                ext
            );
        }

        // Audio
        for ext in &["mp3", "wav", "flac", "m4a", "ogg", "aac"] {
            assert!(
                config.supported_extensions.contains(&ext.to_string()),
                "Should support audio extension: {}",
                ext
            );
        }

        // Code
        for ext in &["rs", "py", "js", "ts", "go", "java", "c", "cpp"] {
            assert!(
                config.supported_extensions.contains(&ext.to_string()),
                "Should support code extension: {}",
                ext
            );
        }

        // Documents
        for ext in &["pdf", "doc", "docx", "txt", "md"] {
            assert!(
                config.supported_extensions.contains(&ext.to_string()),
                "Should support document extension: {}",
                ext
            );
        }
    }

    #[test]
    fn test_indexing_stage_serialization() {
        let stages = vec![
            IndexingStage::Scanning,
            IndexingStage::Embedding,
            IndexingStage::Tagging,
            IndexingStage::Complete,
        ];

        for stage in stages {
            let json = serde_json::to_string(&stage).unwrap();
            let parsed: IndexingStage = serde_json::from_str(&json).unwrap();
            // Verify roundtrip works
            let json2 = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, json2);
        }
    }
}
