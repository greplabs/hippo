//! File watching and real-time updates
//!
//! This module provides file system monitoring capabilities to detect
//! changes to indexed files and automatically update the index.

use crate::{error::Result, indexer::Indexer, models::Source, storage::Storage, HippoError};
use notify::{
    event::ModifyKind, Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// File system event types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[allow(missing_docs)]
pub enum WatchEvent {
    /// A new file was created
    Created {
        /// Path to the created file
        path: PathBuf,
        /// Source containing the file
        source: Source,
    },
    /// An existing file was modified
    Modified {
        /// Path to the modified file
        path: PathBuf,
        /// Source containing the file
        source: Source,
    },
    /// A file was deleted
    Deleted {
        /// Path to the deleted file
        path: PathBuf,
    },
    /// A file was renamed
    Renamed {
        /// Original path before rename
        from: PathBuf,
        /// New path after rename
        to: PathBuf,
        /// Source containing the file
        source: Source,
    },
    /// Watcher started
    WatcherStarted,
    /// Watcher stopped
    WatcherStopped,
    /// Watcher error
    Error {
        /// Error message
        message: String,
    },
}

/// Statistics about watched files
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WatchStats {
    /// Total number of paths being watched
    pub total_watched_paths: usize,
    /// Total events processed since start
    pub events_processed: u64,
    /// Number of files created
    pub files_created: u64,
    /// Number of files modified
    pub files_modified: u64,
    /// Number of files deleted
    pub files_deleted: u64,
    /// Number of files renamed
    pub files_renamed: u64,
    /// Whether the watcher is actively monitoring
    pub is_watching: bool,
    /// Whether the watcher is temporarily paused
    pub is_paused: bool,
}

/// Debounced event tracker to prevent rapid-fire updates
struct DebouncedEvents {
    /// Pending events with their last update time
    pending: HashMap<PathBuf, (WatchEvent, Instant)>,
    /// Debounce duration
    debounce_duration: Duration,
}

impl DebouncedEvents {
    fn new(debounce_duration: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            debounce_duration,
        }
    }

    /// Add an event to the debounced queue
    #[allow(dead_code)]
    fn add(&mut self, event: WatchEvent) {
        let path = match &event {
            WatchEvent::Created { path, .. } => path.clone(),
            WatchEvent::Modified { path, .. } => path.clone(),
            WatchEvent::Deleted { path } => path.clone(),
            WatchEvent::Renamed { to, .. } => to.clone(),
            _ => return,
        };

        self.pending.insert(path, (event, Instant::now()));
    }

    /// Get events that are ready to be processed (past debounce window)
    fn get_ready_events(&mut self) -> Vec<WatchEvent> {
        let now = Instant::now();
        let mut ready = Vec::new();
        let mut to_remove = Vec::new();

        for (path, (event, timestamp)) in &self.pending {
            if now.duration_since(*timestamp) >= self.debounce_duration {
                ready.push(event.clone());
                to_remove.push(path.clone());
            }
        }

        for path in to_remove {
            self.pending.remove(&path);
        }

        ready
    }

    /// Check if there are pending events
    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

/// Internal watcher state
struct WatcherState {
    stats: RwLock<WatchStats>,
    watched_paths: RwLock<HashMap<PathBuf, Source>>,
    is_paused: RwLock<bool>,
    debounced_events: RwLock<DebouncedEvents>,
    /// Flag to signal shutdown to background tasks
    shutdown: std::sync::atomic::AtomicBool,
}

impl WatcherState {
    fn new(debounce_ms: u64) -> Self {
        Self {
            stats: RwLock::new(WatchStats::default()),
            watched_paths: RwLock::new(HashMap::new()),
            is_paused: RwLock::new(false),
            debounced_events: RwLock::new(DebouncedEvents::new(Duration::from_millis(debounce_ms))),
            shutdown: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Signal shutdown to all background tasks
    fn signal_shutdown(&self) {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if shutdown was signaled
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(std::sync::atomic::Ordering::SeqCst)
    }

    async fn increment_stat(&self, stat_type: &str) {
        let mut stats = self.stats.write().await;
        stats.events_processed += 1;
        match stat_type {
            "created" => stats.files_created += 1,
            "modified" => stats.files_modified += 1,
            "deleted" => stats.files_deleted += 1,
            "renamed" => stats.files_renamed += 1,
            _ => {}
        }
    }
}

/// File watcher for real-time index updates
pub struct FileWatcher {
    state: Arc<WatcherState>,
    event_tx: tokio::sync::broadcast::Sender<WatchEvent>,
    storage: Arc<Storage>,
    indexer: Option<Arc<Indexer>>,
    watcher: Option<RecommendedWatcher>,
}

impl FileWatcher {
    /// Create a new file watcher
    ///
    /// # Arguments
    /// * `storage` - Storage backend for updating the index
    /// * `debounce_ms` - Milliseconds to wait before processing duplicate events (default: 500)
    pub fn new(storage: Arc<Storage>, debounce_ms: Option<u64>) -> Result<Self> {
        let debounce = debounce_ms.unwrap_or(500);
        let state = Arc::new(WatcherState::new(debounce));
        let (event_tx, _) = tokio::sync::broadcast::channel(1000);

        Ok(Self {
            state,
            event_tx,
            storage,
            indexer: None,
            watcher: None,
        })
    }

    /// Set the indexer for re-indexing files on changes
    pub fn set_indexer(&mut self, indexer: Arc<Indexer>) {
        self.indexer = Some(indexer);
    }

    /// Subscribe to watch events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }

    /// Start watching a path
    pub async fn watch(&mut self, path: &Path, source: Source) -> Result<()> {
        if !path.exists() {
            return Err(HippoError::Watcher(format!(
                "Path does not exist: {:?}",
                path
            )));
        }

        // Initialize the notify watcher if not already done
        if self.watcher.is_none() {
            self.init_notify_watcher()?;
        }

        // Add path to the actual notify watcher
        if let Some(watcher) = &mut self.watcher {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| HippoError::Watcher(format!("Failed to watch path: {}", e)))?;
        }

        let mut watched_paths = self.state.watched_paths.write().await;
        watched_paths.insert(path.to_path_buf(), source.clone());

        let mut stats = self.state.stats.write().await;
        stats.total_watched_paths = watched_paths.len();
        stats.is_watching = true;

        info!("Started watching: {:?}", path);
        Ok(())
    }

    /// Initialize the notify watcher with event handling
    fn init_notify_watcher(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<Event>();

        let config = Config::default()
            .with_poll_interval(Duration::from_secs(2))
            .with_compare_contents(false);

        let watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            config,
        )
        .map_err(|e| HippoError::Watcher(format!("Failed to create watcher: {}", e)))?;

        self.watcher = Some(watcher);

        // Store rx for background processing - we'll spawn a task to handle events
        // Convert to async processing
        let state = self.state.clone();
        let storage = self.storage.clone();
        let indexer = self.indexer.clone();

        // Spawn background task to process notify events
        tokio::spawn(async move {
            Self::process_notify_events(rx, state, storage, indexer).await;
        });

        info!("Notify watcher initialized");
        Ok(())
    }

    /// Background task to process events from notify
    async fn process_notify_events(
        rx: std::sync::mpsc::Receiver<Event>,
        state: Arc<WatcherState>,
        storage: Arc<Storage>,
        indexer: Option<Arc<Indexer>>,
    ) {
        loop {
            // Check for shutdown signal
            if state.is_shutdown() {
                info!("Watcher shutdown signaled, stopping event processing");
                break;
            }

            // Use recv_timeout to avoid blocking forever
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    // Check for shutdown again (could have been signaled while waiting)
                    if state.is_shutdown() {
                        break;
                    }

                    // Check if paused
                    if *state.is_paused.read().await {
                        continue;
                    }

                    // Process the event
                    if let Err(e) =
                        Self::handle_notify_event(event, &state, &storage, &indexer).await
                    {
                        warn!("Failed to handle notify event: {}", e);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Normal timeout, continue (also allows checking shutdown flag)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    info!("Notify watcher channel disconnected, stopping event processing");
                    break;
                }
            }
        }
        debug!("Watcher event processing task exited");
    }

    /// Handle a single notify event
    async fn handle_notify_event(
        event: Event,
        state: &Arc<WatcherState>,
        storage: &Arc<Storage>,
        indexer: &Option<Arc<Indexer>>,
    ) -> Result<()> {
        let watched_paths = state.watched_paths.read().await;

        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    if !path.is_file() {
                        continue;
                    }

                    // Find which source this file belongs to
                    if let Some(source) = Self::find_source_for_path_static(&path, &watched_paths) {
                        info!("File created: {:?}", path);
                        state.increment_stat("created").await;

                        // Re-index the file
                        if let Some(idx) = indexer {
                            if let Err(e) = idx.index_single_file(&path, &source).await {
                                warn!("Failed to index created file {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
                for path in event.paths {
                    if !path.is_file() {
                        continue;
                    }

                    if let Some(source) = Self::find_source_for_path_static(&path, &watched_paths) {
                        info!("File modified: {:?}", path);
                        state.increment_stat("modified").await;

                        // Delete old entry and re-index
                        let _ = storage.remove_memory_by_path(&path).await;

                        if let Some(idx) = indexer {
                            if let Err(e) = idx.index_single_file(&path, &source).await {
                                warn!("Failed to re-index modified file {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    info!("File deleted: {:?}", path);
                    state.increment_stat("deleted").await;

                    // Remove from index
                    if let Err(e) = storage.remove_memory_by_path(&path).await {
                        debug!("Failed to delete memory for {:?}: {}", path, e);
                    }
                }
            }
            _ => {
                debug!("Ignoring event kind: {:?}", event.kind);
            }
        }

        Ok(())
    }

    /// Static version of find_source_for_path for use in async contexts
    fn find_source_for_path_static(
        path: &Path,
        watched_paths: &HashMap<PathBuf, Source>,
    ) -> Option<Source> {
        for (watched_path, source) in watched_paths {
            if path.starts_with(watched_path) {
                return Some(source.clone());
            }
        }
        None
    }

    /// Stop watching a specific path
    pub async fn unwatch(&mut self, path: &Path) -> Result<()> {
        // Unwatch from notify
        if let Some(watcher) = &mut self.watcher {
            let _ = watcher.unwatch(path);
        }

        let mut watched_paths = self.state.watched_paths.write().await;
        watched_paths.remove(path);

        let mut stats = self.state.stats.write().await;
        stats.total_watched_paths = watched_paths.len();
        if watched_paths.is_empty() {
            stats.is_watching = false;
        }

        info!("Stopped watching: {:?}", path);
        Ok(())
    }

    /// Stop watching all paths
    pub async fn stop_all(&mut self) -> Result<()> {
        // Unwatch all paths from notify
        if let Some(watcher) = &mut self.watcher {
            let watched_paths = self.state.watched_paths.read().await;
            for path in watched_paths.keys() {
                let _ = watcher.unwatch(path);
            }
        }

        let mut watched_paths = self.state.watched_paths.write().await;
        watched_paths.clear();

        let mut stats = self.state.stats.write().await;
        stats.total_watched_paths = 0;
        stats.is_watching = false;

        info!("Stopped all watching");

        // Emit stopped event
        let _ = self.event_tx.send(WatchEvent::WatcherStopped);

        Ok(())
    }

    /// Pause watching (events are ignored but watchers remain active)
    pub async fn pause(&self) -> Result<()> {
        *self.state.is_paused.write().await = true;
        let mut stats = self.state.stats.write().await;
        stats.is_paused = true;
        info!("Watcher paused");
        Ok(())
    }

    /// Resume watching
    pub async fn resume(&self) -> Result<()> {
        *self.state.is_paused.write().await = false;
        let mut stats = self.state.stats.write().await;
        stats.is_paused = false;
        info!("Watcher resumed");
        Ok(())
    }

    /// Check if watcher is paused
    pub async fn is_paused(&self) -> bool {
        *self.state.is_paused.read().await
    }

    /// Get current watch statistics
    pub async fn stats(&self) -> WatchStats {
        self.state.stats.read().await.clone()
    }

    /// Get list of watched paths
    pub async fn watched_paths(&self) -> Vec<(PathBuf, Source)> {
        self.state
            .watched_paths
            .read()
            .await
            .iter()
            .map(|(p, s)| (p.clone(), s.clone()))
            .collect()
    }

    /// Process a file system event
    #[allow(dead_code)]
    async fn process_event(&self, event: Event) -> Result<()> {
        // Skip if paused
        if self.is_paused().await {
            return Ok(());
        }

        let watched_paths = self.state.watched_paths.read().await;

        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    if !path.is_file() {
                        continue;
                    }

                    // Find which source this file belongs to
                    if let Some(source) = self.find_source_for_path(&path, &watched_paths) {
                        let watch_event = WatchEvent::Created {
                            path: path.clone(),
                            source: source.clone(),
                        };

                        // Add to debounced queue
                        self.state.debounced_events.write().await.add(watch_event);
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    if !path.is_file() {
                        continue;
                    }

                    if let Some(source) = self.find_source_for_path(&path, &watched_paths) {
                        let watch_event = WatchEvent::Modified {
                            path: path.clone(),
                            source: source.clone(),
                        };

                        self.state.debounced_events.write().await.add(watch_event);
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    let watch_event = WatchEvent::Deleted { path: path.clone() };
                    self.state.debounced_events.write().await.add(watch_event);
                }
            }
            _ => {
                debug!("Ignoring event: {:?}", event.kind);
            }
        }

        Ok(())
    }

    /// Find which source a path belongs to
    #[allow(dead_code)]
    fn find_source_for_path(
        &self,
        path: &Path,
        watched_paths: &HashMap<PathBuf, Source>,
    ) -> Option<Source> {
        for (watched_path, source) in watched_paths {
            if path.starts_with(watched_path) {
                return Some(source.clone());
            }
        }
        None
    }

    /// Process debounced events and emit them
    pub async fn flush_events(&self) -> Result<()> {
        let ready_events = self.state.debounced_events.write().await.get_ready_events();

        for event in ready_events {
            // Update stats
            match &event {
                WatchEvent::Created { .. } => self.state.increment_stat("created").await,
                WatchEvent::Modified { .. } => self.state.increment_stat("modified").await,
                WatchEvent::Deleted { .. } => self.state.increment_stat("deleted").await,
                WatchEvent::Renamed { .. } => self.state.increment_stat("renamed").await,
                _ => {}
            }

            // Emit event
            if let Err(e) = self.event_tx.send(event.clone()) {
                error!("Failed to send watch event: {}", e);
            }

            // Handle the event (update index)
            if let Err(e) = self.handle_event(&event).await {
                warn!("Failed to handle watch event: {}", e);
            }
        }

        Ok(())
    }

    /// Handle a watch event by updating the index
    async fn handle_event(&self, event: &WatchEvent) -> Result<()> {
        match event {
            WatchEvent::Created { path, source: _ } | WatchEvent::Modified { path, source: _ } => {
                // Delete existing entry if it exists (for modified files)
                if let Err(e) = self.storage.remove_memory_by_path(path).await {
                    debug!("Failed to delete old memory for {:?}: {}", path, e);
                }

                info!("Detected file change: {:?}", path);
            }
            WatchEvent::Deleted { path } => {
                // Remove from index
                if let Err(e) = self.storage.remove_memory_by_path(path).await {
                    warn!("Failed to delete memory for {:?}: {}", path, e);
                } else {
                    info!("Removed deleted file from index: {:?}", path);
                }
            }
            WatchEvent::Renamed {
                from,
                to,
                source: _,
            } => {
                // Delete old path
                if let Err(e) = self.storage.remove_memory_by_path(from).await {
                    debug!("Failed to delete old memory for {:?}: {}", from, e);
                }

                info!("Detected file rename: {:?} -> {:?}", from, to);
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if there are pending debounced events
    pub async fn has_pending_events(&self) -> bool {
        self.state.debounced_events.read().await.has_pending()
    }

    /// Watch all sources from the storage
    pub async fn watch_all_sources(&mut self) -> Result<()> {
        // Get all sources from storage
        let sources = self.storage.list_sources().await?;

        for source_config in sources {
            if let Source::Local { root_path } = &source_config.source {
                self.watch(root_path, source_config.source.clone()).await?;
            }
        }

        // Emit started event
        let _ = self.event_tx.send(WatchEvent::WatcherStarted);

        Ok(())
    }

    /// Get the number of active watchers
    pub async fn active_count(&self) -> usize {
        self.state.watched_paths.read().await.len()
    }

    /// Unwatch all sources
    pub async fn unwatch_all(&mut self) -> Result<()> {
        self.stop_all().await
    }

    /// Shutdown the watcher and all background tasks
    /// This signals the background event processing task to exit
    pub fn shutdown(&self) {
        info!("Shutting down file watcher");
        self.state.signal_shutdown();
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        // Signal shutdown to background tasks
        self.state.signal_shutdown();
        debug!("FileWatcher dropped, shutdown signaled");
    }
}

/// Start a background task to flush debounced events periodically
/// Returns a handle to the spawned task
pub async fn start_flush_task(watcher: Arc<RwLock<FileWatcher>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            let watcher_lock = watcher.read().await;

            // Check for shutdown signal
            if watcher_lock.state.is_shutdown() {
                info!("Flush task shutting down");
                break;
            }

            if let Err(e) = watcher_lock.flush_events().await {
                error!("Failed to flush watch events: {}", e);
            }
        }
        debug!("Flush task exited");
    })
}
