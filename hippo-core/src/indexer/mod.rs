//! File indexing and metadata extraction
//! 
//! The indexer walks through sources, extracts metadata, and queues
//! files for embedding generation.

use crate::{
    error::{HippoError, Result},
    models::*,
    storage::Storage,
    embeddings::Embedder,
    HippoConfig,
};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs::File;
use tokio::sync::mpsc;
use walkdir::WalkDir;
use tracing::{info, warn, debug, instrument};
use rayon::prelude::*;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub mod extractors;
pub mod code_parser;

/// The main indexer that orchestrates file discovery and processing
#[allow(dead_code)]
pub struct Indexer {
    storage: Arc<Storage>,
    embedder: Arc<Embedder>,
    config: IndexerConfig,
    task_tx: mpsc::Sender<IndexTask>,
}

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub parallelism: usize,
    pub batch_size: usize,
    pub supported_extensions: Vec<String>,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            parallelism: 4,
            batch_size: 100,
            supported_extensions: vec![
                // Images
                "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "heic", "heif", "raw", "cr2", "nef",
                // Videos
                "mp4", "mov", "avi", "mkv", "webm", "m4v",
                // Audio
                "mp3", "wav", "flac", "m4a", "ogg", "aac",
                // Documents
                "pdf", "doc", "docx", "txt", "md", "rtf", "odt",
                // Spreadsheets
                "xls", "xlsx", "csv", "ods",
                // Presentations
                "ppt", "pptx", "odp",
                // Code
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp",
                "rb", "php", "swift", "kt", "scala", "sh", "bash", "zsh", "sql", "html", "css",
                "json", "yaml", "yml", "toml", "xml",
                // Archives
                "zip", "tar", "gz", "7z", "rar",
            ].into_iter().map(String::from).collect(),
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
        
        let indexer_config = IndexerConfig {
            parallelism: config.indexing_parallelism,
            ..Default::default()
        };
        
        // Spawn background worker
        let storage_clone = storage.clone();
        let embedder_clone = embedder.clone();
        let config_clone = indexer_config.clone();
        
        tokio::spawn(async move {
            Self::background_worker(task_rx, storage_clone, embedder_clone, config_clone).await;
        });
        
        Ok(Self {
            storage,
            embedder,
            config: indexer_config,
            task_tx,
        })
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
                self.task_tx.send(IndexTask::IndexPath(root_path.clone(), source)).await
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
                self.task_tx.send(IndexTask::IndexPath(root_path.clone(), source.clone())).await
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
            return Err(HippoError::Indexing(format!("File does not exist: {:?}", path)));
        }

        let ext = path.extension()
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

        // Embed it
        if let Err(e) = self.embedder.embed_memory(&memory).await {
            debug!("Failed to embed memory {}: {}", memory.id, e);
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
    ) {
        println!("[Indexer] Background worker started");
        info!("Indexer background worker started");
        
        while let Some(task) = rx.recv().await {
            println!("[Indexer] Received task: {:?}", task);
            match task {
                IndexTask::IndexPath(path, source) => {
                    println!("[Indexer] Starting to index: {:?}", path);
                    if let Err(e) = Self::index_path(&path, &source, &storage, &embedder, &config).await {
                        println!("[Indexer] Failed to index {:?}: {}", path, e);
                        warn!("Failed to index path {:?}: {}", path, e);
                    } else {
                        println!("[Indexer] Finished indexing: {:?}", path);
                    }
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
    
    /// Index all files in a path
    #[instrument(skip(storage, embedder, config))]
    async fn index_path(
        path: &Path,
        source: &Source,
        storage: &Storage,
        embedder: &Embedder,
        config: &IndexerConfig,
    ) -> Result<()> {
        println!("[Indexer] Starting index of path: {:?}", path);
        info!("Starting index of path: {:?}", path);
        
        if !path.exists() {
            println!("[Indexer] Path does not exist: {:?}", path);
            return Err(HippoError::Indexing(format!("Path does not exist: {:?}", path)));
        }
        
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
        
        // Process in batches
        for (batch_idx, batch) in files.chunks(config.batch_size).enumerate() {
            println!("[Indexer] Processing batch {} ({} files)", batch_idx + 1, batch.len());
            let memories: Vec<Memory> = batch
                .par_iter()
                .filter_map(|file_path| {
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
            
            // Store memories
            for memory in &memories {
                if let Err(e) = storage.upsert_memory(memory).await {
                    println!("[Indexer] Failed to store memory: {}", e);
                    warn!("Failed to store memory: {}", e);
                }
            }
            
            // Generate embeddings (skip for now - just log)
            for memory in &memories {
                if let Err(e) = embedder.embed_memory(memory).await {
                    debug!("Failed to embed memory {}: {}", memory.id, e);
                }
            }
            
            println!("[Indexer] Stored batch of {} files", memories.len());
            info!("Processed batch of {} files", memories.len());
        }
        
        println!("[Indexer] Completed indexing path: {:?}", path);
        info!("Completed indexing path: {:?}", path);
        Ok(())
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
        memory.created_at = file_meta.created()
            .map(chrono::DateTime::from)
            .unwrap_or_else(|_| chrono::Utc::now());
        memory.modified_at = file_meta.modified()
            .map(chrono::DateTime::from)
            .unwrap_or_else(|_| chrono::Utc::now());
        
        // Detect MIME type
        memory.metadata.mime_type = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string());

        // Compute content hash for duplicate detection
        // Skip very large files (> 500MB) to avoid slowdowns
        if memory.metadata.file_size < 500 * 1024 * 1024 {
            if let Ok(hash) = crate::duplicates::compute_file_hash(path) {
                memory.metadata.hash = Some(hash);
            }
        }

        // Add system tags based on file type
        memory.tags.push(Tag::system(format!("type:{}", memory.kind_name())));
        
        // Add folder-based tags
        if let Some(parent) = path.parent() {
            if let Some(folder_name) = parent.file_name() {
                memory.tags.push(Tag::system(format!("folder:{}", folder_name.to_string_lossy())));
            }
        }
        
        Ok(memory)
    }
    
    /// Detect the kind of file based on extension and content
    fn detect_kind(path: &Path) -> Result<MemoryKind> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        let kind = match ext.as_str() {
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "heic" | "heif" | "raw" | "cr2" | "nef" => {
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
            "pdf" => MemoryKind::Document { format: DocumentFormat::Pdf, page_count: None },
            "doc" | "docx" => MemoryKind::Document { format: DocumentFormat::Word, page_count: None },
            "txt" => MemoryKind::Document { format: DocumentFormat::PlainText, page_count: None },
            "md" => MemoryKind::Document { format: DocumentFormat::Markdown, page_count: None },
            "html" | "htm" => MemoryKind::Document { format: DocumentFormat::Html, page_count: None },
            "rtf" => MemoryKind::Document { format: DocumentFormat::Rtf, page_count: None },
            
            // Spreadsheets
            "xls" | "xlsx" | "csv" | "ods" => {
                MemoryKind::Spreadsheet { sheet_count: 1 }
            }
            
            // Presentations
            "ppt" | "pptx" | "odp" => {
                MemoryKind::Presentation { slide_count: 0 }
            }
            
            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | 
            "h" | "hpp" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | 
            "zsh" | "sql" | "css" | "json" | "yaml" | "yml" | "toml" | "xml" => {
                let language = Self::detect_language(&ext);
                let lines = std::fs::read_to_string(path)
                    .map(|s| s.lines().count() as u32)
                    .unwrap_or(0);
                MemoryKind::Code { language, lines }
            }
            
            // Archives
            "zip" | "tar" | "gz" | "7z" | "rar" => {
                MemoryKind::Archive { item_count: 0 }
            }
            
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
        }.to_string()
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
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
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
