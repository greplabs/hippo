//! File system watcher for detecting changes
//!
//! Watches indexed sources for file changes (create, modify, delete)
//! and triggers re-indexing automatically.

use crate::{
    error::{HippoError, Result},
    models::Source,
    storage::Storage,
    indexer::Indexer,
};

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, debug, error};

/// File system watcher that monitors sources for changes
#[allow(dead_code)]
pub struct FileWatcher {
    /// Active watchers by source path
    watchers: Arc<RwLock<HashMap<PathBuf, WatcherHandle>>>,
    /// Storage for looking up sources
    storage: Arc<Storage>,
    /// Indexer for processing changes
    indexer: Arc<Indexer>,
    /// Channel for watch events
    event_tx: mpsc::Sender<WatchEvent>,
    /// Debounce duration for events
    debounce_ms: u64,
}

/// Handle to a running watcher
#[allow(dead_code)]
struct WatcherHandle {
    watcher: RecommendedWatcher,
    source: Source,
}

/// Events from the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was created
    Created(PathBuf),
    /// File was modified
    Modified(PathBuf),
    /// File was deleted
    Deleted(PathBuf),
    /// File was renamed (old, new)
    Renamed(PathBuf, PathBuf),
    /// Error occurred
    Error(String),
}

/// Statistics about watching activity
#[derive(Debug, Clone, Default)]
pub struct WatchStats {
    pub sources_watched: usize,
    pub events_processed: u64,
    pub files_created: u64,
    pub files_modified: u64,
    pub files_deleted: u64,
    pub errors: u64,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(storage: Arc<Storage>, indexer: Arc<Indexer>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let watcher = Self {
            watchers: Arc::new(RwLock::new(HashMap::new())),
            storage: storage.clone(),
            indexer: indexer.clone(),
            event_tx,
            debounce_ms: 500,
        };

        // Spawn event processor
        let indexer_clone = indexer.clone();
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            Self::process_events(event_rx, indexer_clone, storage_clone).await;
        });

        Ok(watcher)
    }

    /// Start watching a source
    pub async fn watch(&self, source: &Source) -> Result<()> {
        match source {
            Source::Local { root_path } => {
                self.watch_path(root_path, source.clone()).await
            }
            _ => {
                warn!("File watching not supported for cloud sources");
                Ok(())
            }
        }
    }

    /// Start watching a local path
    async fn watch_path(&self, path: &Path, source: Source) -> Result<()> {
        let path = path.to_path_buf();

        // Check if already watching
        {
            let watchers = self.watchers.read().await;
            if watchers.contains_key(&path) {
                info!("Already watching: {:?}", path);
                return Ok(());
            }
        }

        // Create the watcher
        let event_tx = self.event_tx.clone();
        let path_clone = path.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                let tx = event_tx.clone();
                let path = path_clone.clone();

                match res {
                    Ok(event) => {
                        if let Some(watch_event) = Self::convert_event(&event, &path) {
                            // Use blocking send since we're in a sync callback
                            let _ = tx.blocking_send(watch_event);
                        }
                    }
                    Err(e) => {
                        let _ = tx.blocking_send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(false),
        ).map_err(|e| HippoError::Other(format!("Failed to create watcher: {}", e)))?;

        // Start watching
        watcher.watch(&path, RecursiveMode::Recursive)
            .map_err(|e| HippoError::Other(format!("Failed to watch path: {}", e)))?;

        info!("Started watching: {:?}", path);

        // Store the watcher
        let handle = WatcherHandle { watcher, source };
        self.watchers.write().await.insert(path.clone(), handle);

        Ok(())
    }

    /// Stop watching a source
    pub async fn unwatch(&self, source: &Source) -> Result<()> {
        match source {
            Source::Local { root_path } => {
                let mut watchers = self.watchers.write().await;
                if watchers.remove(root_path).is_some() {
                    info!("Stopped watching: {:?}", root_path);
                }
                Ok(())
            }
            _ => Ok(())
        }
    }

    /// Stop all watchers
    pub async fn unwatch_all(&self) -> Result<()> {
        let mut watchers = self.watchers.write().await;
        let count = watchers.len();
        watchers.clear();
        info!("Stopped {} watchers", count);
        Ok(())
    }

    /// Watch all configured sources
    pub async fn watch_all_sources(&self) -> Result<()> {
        let sources = self.storage.list_sources().await?;

        for source_config in sources {
            if let Err(e) = self.watch(&source_config.source).await {
                warn!("Failed to watch source {:?}: {}", source_config.source, e);
            }
        }

        Ok(())
    }

    /// Get the number of active watchers
    pub async fn active_count(&self) -> usize {
        self.watchers.read().await.len()
    }

    /// Get list of watched paths
    pub async fn watched_paths(&self) -> Vec<PathBuf> {
        self.watchers.read().await.keys().cloned().collect()
    }

    /// Convert notify event to our watch event
    fn convert_event(event: &Event, _base_path: &Path) -> Option<WatchEvent> {
        // Get the affected paths
        let paths: Vec<PathBuf> = event.paths.clone();

        if paths.is_empty() {
            return None;
        }

        match &event.kind {
            EventKind::Create(CreateKind::File) => {
                Some(WatchEvent::Created(paths[0].clone()))
            }
            EventKind::Create(CreateKind::Any) => {
                // Check if it's a file
                if paths[0].is_file() {
                    Some(WatchEvent::Created(paths[0].clone()))
                } else {
                    None
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) |
            EventKind::Modify(ModifyKind::Any) => {
                if paths[0].is_file() {
                    Some(WatchEvent::Modified(paths[0].clone()))
                } else {
                    None
                }
            }
            EventKind::Remove(RemoveKind::File) |
            EventKind::Remove(RemoveKind::Any) => {
                Some(WatchEvent::Deleted(paths[0].clone()))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                if paths.len() >= 2 {
                    Some(WatchEvent::Renamed(paths[0].clone(), paths[1].clone()))
                } else {
                    None
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                Some(WatchEvent::Deleted(paths[0].clone()))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                Some(WatchEvent::Created(paths[0].clone()))
            }
            _ => None
        }
    }

    /// Process watch events
    async fn process_events(
        mut rx: mpsc::Receiver<WatchEvent>,
        indexer: Arc<Indexer>,
        storage: Arc<Storage>,
    ) {
        use std::collections::HashSet;
        use tokio::time::{interval, Duration};

        // Debounce buffer
        let mut pending_creates: HashSet<PathBuf> = HashSet::new();
        let mut pending_modifies: HashSet<PathBuf> = HashSet::new();
        let mut pending_deletes: HashSet<PathBuf> = HashSet::new();

        let mut debounce_timer = interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    match event {
                        WatchEvent::Created(path) => {
                            debug!("File created: {:?}", path);
                            pending_deletes.remove(&path);
                            pending_creates.insert(path);
                        }
                        WatchEvent::Modified(path) => {
                            debug!("File modified: {:?}", path);
                            if !pending_creates.contains(&path) {
                                pending_modifies.insert(path);
                            }
                        }
                        WatchEvent::Deleted(path) => {
                            debug!("File deleted: {:?}", path);
                            pending_creates.remove(&path);
                            pending_modifies.remove(&path);
                            pending_deletes.insert(path);
                        }
                        WatchEvent::Renamed(old, new) => {
                            debug!("File renamed: {:?} -> {:?}", old, new);
                            pending_deletes.insert(old);
                            pending_creates.insert(new);
                        }
                        WatchEvent::Error(e) => {
                            error!("Watch error: {}", e);
                        }
                    }
                }
                _ = debounce_timer.tick() => {
                    // Process pending events

                    // Handle deletions
                    for path in pending_deletes.drain() {
                        if let Err(e) = storage.remove_memory_by_path(&path).await {
                            debug!("Failed to remove deleted file from index: {}", e);
                        } else {
                            info!("Removed from index: {:?}", path);
                        }
                    }

                    // Handle creates and modifies (both need re-indexing)
                    let mut to_index: HashSet<PathBuf> = HashSet::new();
                    to_index.extend(pending_creates.drain());
                    to_index.extend(pending_modifies.drain());

                    for path in to_index {
                        // Find which source this belongs to
                        if let Ok(sources) = storage.list_sources().await {
                            for source_config in sources {
                                if let Source::Local { root_path } = &source_config.source {
                                    if path.starts_with(root_path) {
                                        // Queue for re-indexing (index single file)
                                        if let Err(e) = indexer.index_single_file(&path, &source_config.source).await {
                                            debug!("Failed to index file {:?}: {}", path, e);
                                        } else {
                                            info!("Indexed: {:?}", path);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
