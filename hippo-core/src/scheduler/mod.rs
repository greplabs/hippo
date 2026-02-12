//! Scheduled auto-indexing for Hippo sources
//!
//! Periodically checks source sync intervals and triggers re-indexing
//! when the configured interval has elapsed since the last sync.

use crate::error::Result;
use crate::models::Source;
use crate::storage::Storage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Scheduler state and statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchedulerStats {
    pub running: bool,
    pub check_interval_secs: u64,
    pub total_checks: u64,
    pub total_syncs_triggered: u64,
    pub last_check: Option<String>,
    pub next_check: Option<String>,
}

/// Configuration for the scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// How often to check if sources need re-indexing (default: 300s = 5 min)
    pub check_interval_secs: u64,
    /// Default sync interval for sources without explicit config (default: 3600s = 1 hour)
    pub default_sync_interval_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 300,
            default_sync_interval_secs: 3600,
        }
    }
}

/// Background scheduler that auto-indexes sources on configured intervals
pub struct Scheduler {
    config: SchedulerConfig,
    storage: Arc<Storage>,
    running: Arc<AtomicBool>,
    stats: Arc<RwLock<SchedulerInternalStats>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

struct SchedulerInternalStats {
    total_checks: u64,
    total_syncs_triggered: u64,
    last_check: Option<chrono::DateTime<chrono::Utc>>,
}

impl Scheduler {
    pub fn new(storage: Arc<Storage>, config: SchedulerConfig) -> Self {
        Self {
            config,
            storage,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(SchedulerInternalStats {
                total_checks: 0,
                total_syncs_triggered: 0,
                last_check: None,
            })),
            task_handle: None,
        }
    }

    /// Start the background scheduler loop
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            info!("Scheduler already running");
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        info!(
            "Starting scheduler: check every {}s, default sync interval {}s",
            self.config.check_interval_secs, self.config.default_sync_interval_secs
        );

        let running = self.running.clone();
        let storage = self.storage.clone();
        let stats = self.stats.clone();
        let check_interval = self.config.check_interval_secs;
        let default_sync_interval = self.config.default_sync_interval_secs;

        let handle = tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                // Sleep for the check interval
                tokio::time::sleep(tokio::time::Duration::from_secs(check_interval)).await;

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // Check sources that need re-indexing
                let now = chrono::Utc::now();
                debug!("Scheduler: checking sources for re-indexing");

                match storage.list_sources().await {
                    Ok(sources) => {
                        let mut syncs_triggered = 0u64;

                        for source_config in &sources {
                            if !source_config.enabled {
                                continue;
                            }

                            let sync_interval = if source_config.sync_interval_secs > 0 {
                                source_config.sync_interval_secs
                            } else {
                                default_sync_interval
                            };

                            let needs_sync = match &source_config.last_sync {
                                Some(last) => {
                                    let elapsed = (now - *last).num_seconds() as u64;
                                    elapsed >= sync_interval
                                }
                                None => true, // Never synced
                            };

                            if needs_sync {
                                let path = match &source_config.source {
                                    Source::Local { root_path } => {
                                        root_path.to_string_lossy().to_string()
                                    }
                                    _ => continue, // Skip non-local sources
                                };

                                info!(
                                    "Scheduler: triggering re-index for {}",
                                    path
                                );

                                // Update last_sync timestamp
                                if let Err(e) = storage
                                    .update_source_last_sync(&source_config.source)
                                    .await
                                {
                                    warn!("Scheduler: failed to update last_sync: {}", e);
                                }

                                syncs_triggered += 1;
                            }
                        }

                        // Update stats
                        let mut s = stats.write().await;
                        s.total_checks += 1;
                        s.total_syncs_triggered += syncs_triggered;
                        s.last_check = Some(now);

                        if syncs_triggered > 0 {
                            info!(
                                "Scheduler: triggered {} re-index(es) this cycle",
                                syncs_triggered
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Scheduler: failed to list sources: {}", e);
                    }
                }
            }

            info!("Scheduler stopped");
        });

        self.task_handle = Some(handle);
        Ok(())
    }

    /// Stop the scheduler
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        info!("Scheduler stopped");
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get scheduler statistics
    pub async fn stats(&self) -> SchedulerStats {
        let s = self.stats.read().await;
        let next_check = s.last_check.map(|last| {
            (last + chrono::Duration::seconds(self.config.check_interval_secs as i64))
                .to_rfc3339()
        });

        SchedulerStats {
            running: self.running.load(Ordering::SeqCst),
            check_interval_secs: self.config.check_interval_secs,
            total_checks: s.total_checks,
            total_syncs_triggered: s.total_syncs_triggered,
            last_check: s.last_check.map(|d| d.to_rfc3339()),
            next_check,
        }
    }

    /// Update the check interval
    pub fn set_check_interval(&mut self, secs: u64) {
        self.config.check_interval_secs = secs;
        info!("Scheduler check interval updated to {}s", secs);
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.check_interval_secs, 300);
        assert_eq!(config.default_sync_interval_secs, 3600);
    }

    #[test]
    fn test_scheduler_stats_default() {
        let stats = SchedulerStats {
            running: false,
            check_interval_secs: 300,
            total_checks: 0,
            total_syncs_triggered: 0,
            last_check: None,
            next_check: None,
        };
        assert!(!stats.running);
        assert_eq!(stats.total_checks, 0);
    }
}
