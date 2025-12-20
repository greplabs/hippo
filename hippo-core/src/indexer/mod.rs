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
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::collections::VecDeque;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::sync::{mpsc, RwLock};
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

impl IndexingProgress {
    pub fn new() -> Self {
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
struct PrioritizedFile {
    path: PathBuf,
    source: Source,
    priority: u8, // Higher is more urgent (0-255)
}

/// Shared indexing state
struct IndexingState {
    progress: RwLock<IndexingProgress>,
    is_paused: AtomicBool,
    priority_queue: RwLock<VecDeque<PrioritizedFile>>,
    start_time: RwLock<Option<std::time::Instant>>,
    files_processed_count: AtomicUsize,
}

impl IndexingState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            progress: RwLock::new(IndexingProgress::new()),
            is_paused: AtomicBool::new(false),
            priority_queue: RwLock::new(VecDeque::new()),
            start_time: RwLock::new(None),
            files_processed_count: AtomicUsize::new(0),
        })
    }

    async fn update_progress(&self, update_fn: impl FnOnce(&mut IndexingProgress)) {
        let mut progress = self.progress.write().await;
        update_fn(&mut *progress);

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
#[allow(dead_code)]
pub struct Indexer {
    storage: Arc<Storage>,
    embedder: Arc<Embedder>,
    config: IndexerConfig,
    task_tx: mpsc::Sender<IndexTask>,
    state: Arc<IndexingState>,
    progress_tx: tokio::sync::broadcast::Sender<IndexingProgress>,
}

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub parallelism: usize,
    pub batch_size: usize,
    pub supported_extensions: Vec<String>,
    pub auto_tag_enabled: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            parallelism: 4,
            batch_size: 100,
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
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum IndexTask {
    IndexPath(PathBuf, Source),
    Reindex(MemoryId),
    Shutdown,
}

impl Indexer {
    pub fn new(
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        config: &HippoConfig,
    ) -> Result<Self> {
        let (task_tx, task_rx) = mpsc::channel(1000);
        let (progress_tx, _) = tokio::sync::broadcast::channel(100);

        let indexer_config = IndexerConfig {
            parallelism: config.indexing_parallelism,
            auto_tag_enabled: config.auto_tag_enabled,
            ..Default::default()
        };

        let state = IndexingState::new();

        // Spawn background worker
        let storage_clone = storage.clone();
        let embedder_clone = embedder.clone();
        let config_clone = indexer_config.clone();
        let state_clone = state.clone();
        let progress_tx_clone = progress_tx.clone();

        tokio::spawn(async move {
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
        })
    }

    /// Get current indexing progress
    pub async fn get_progress(&self) -> IndexingProgress {
        let progress = self.state.progress.read().await;
        progress.clone()
    }

    /// Subscribe to indexing progress updates
    pub fn subscribe_progress(&self) -> tokio::sync::broadcast::Receiver<IndexingProgress> {
        self.progress_tx.subscribe()
    }

    /// Pause indexing
    pub async fn pause(&self) -> Result<()> {
        self.state.is_paused.store(true, Ordering::SeqCst);
        self.state.update_progress(|p| {
            p.is_paused = true;
        }).await;
        info!("Indexing paused");
        Ok(())
    }

    /// Resume indexing
    pub async fn resume(&self) -> Result<()> {
        self.state.is_paused.store(false, Ordering::SeqCst);
        self.state.update_progress(|p| {
            p.is_paused = false;
        }).await;
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
        progress_tx: tokio::sync::broadcast::Sender<IndexingProgress>,
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
                    state.update_progress(|p| {
                        p.stage = IndexingStage::Complete;
                    }).await;
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
        progress_tx: &tokio::sync::broadcast::Sender<IndexingProgress>,
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
        state.update_progress(|p| {
            p.stage = IndexingStage::Scanning;
            p.current_file = Some("Scanning directory...".to_string());
        }).await;
        let _ = progress_tx.send(state.progress.read().await.clone());

        // Collect all files
        let files: Vec<PathBuf> = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
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

        println!("[Indexer] Found {} files to index", files.len());
        info!("Found {} files to index", files.len());

        // Set total count
        state.update_progress(|p| {
            p.total = files.len();
            p.stage = IndexingStage::Embedding;
        }).await;
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
                    let file_name = file_path
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
                state.update_progress(|p| {
                    p.current_file = Some(
                        memory
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    );
                    p.stage = IndexingStage::Embedding;
                }).await;

                if let Err(e) = storage.upsert_memory(memory).await {
                    println!("[Indexer] Failed to store memory: {}", e);
                    warn!("Failed to store memory: {}", e);
                }

                // Check for pause
                while state.is_paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            // Generate and store embeddings
            for memory in &memories {
                state.update_progress(|p| {
                    p.current_file = Some(
                        memory
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    );
                }).await;

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

                // Update progress counter
                state.files_processed_count.fetch_add(1, Ordering::SeqCst);
                state.update_progress(|p| {
                    p.processed += 1;
                }).await;
                let _ = progress_tx.send(state.progress.read().await.clone());

                // Check for pause
                while state.is_paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            // STAGE 3: Auto-tagging (if enabled)
            if config.auto_tag_enabled && !memories.is_empty() {
                state.update_progress(|p| {
                    p.stage = IndexingStage::Tagging;
                }).await;
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
        let (progress_tx, _) = tokio::sync::broadcast::channel(1);

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

    /// Auto-tag a batch of memories using Ollama
    async fn auto_tag_batch(memories: &[Memory], storage: &Storage) {
        let ollama_client = OllamaClient::new();

        // Check if Ollama is available
        if !ollama_client.is_available().await {
            debug!("Ollama not available, skipping auto-tagging");
            return;
        }

        println!("[Indexer] Auto-tagging {} files with Ollama", memories.len());

        for memory in memories {
            // Create a prompt based on file information
            let filename = memory
                .path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            let kind_name = match &memory.kind {
                MemoryKind::Image { .. } => "image",
                MemoryKind::Video { .. } => "video",
                MemoryKind::Audio { .. } => "audio",
                MemoryKind::Document { .. } => "document",
                MemoryKind::Code { language, .. } => language.as_str(),
                MemoryKind::Spreadsheet { .. } => "spreadsheet",
                MemoryKind::Presentation { .. } => "presentation",
                MemoryKind::Archive { .. } => "archive",
                _ => "file",
            };

            let prompt = format!(
                "Suggest 3-5 short tags for this file: {}, type: {}. Return only comma-separated tags, no explanations.",
                filename, kind_name
            );

            // Generate tags using Ollama
            match ollama_client.generate(&prompt, None).await {
                Ok(response) => {
                    // Parse comma-separated tags
                    let tags: Vec<String> = response
                        .split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty() && s.len() <= 30)
                        .take(5)
                        .map(String::from)
                        .collect();

                    // Add AI tags to the memory
                    for tag_name in tags {
                        let tag = Tag::ai(tag_name, 80);
                        if let Err(e) = storage.add_tag(memory.id, tag).await {
                            debug!("Failed to add auto-tag: {}", e);
                        }
                    }

                    debug!("Auto-tagged file: {}", filename);
                }
                Err(e) => {
                    debug!("Failed to auto-tag {}: {}", filename, e);
                }
            }
        }

        println!("[Indexer] Auto-tagging completed");
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
