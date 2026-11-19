//! Tests for indexer re-index detection, watcher debounce/skip logic, and
//! scheduler sync-trigger decisions.
//!
//! All time-dependent watcher/scheduler assertions use injected timestamps so
//! there is no real `sleep` anywhere in this file.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Utc;
use hippo_core::indexer::{Indexer, ReindexStatus};
use hippo_core::scheduler::should_sync;
use hippo_core::storage::Storage;
use hippo_core::watcher::{should_skip_path, DebouncedEvents, WatchEvent};
use hippo_core::{HippoConfig, Memory, MemoryKind, Source};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn make_storage() -> (Storage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = HippoConfig {
        data_dir: temp_dir.path().to_path_buf(),
        qdrant_url: "http://localhost:9999".to_string(),
        ..Default::default()
    };
    let storage = Storage::new(&config).await.unwrap();
    (storage, temp_dir)
}

fn local_source(root: &Path) -> Source {
    Source::Local {
        root_path: root.to_path_buf(),
    }
}

// ---------------------------------------------------------------------------
// needs_reindex
// ---------------------------------------------------------------------------

#[tokio::test]
async fn needs_reindex_new_when_no_db_row() {
    let (storage, tmp) = make_storage().await;
    let file = tmp.path().join("fresh.txt");
    std::fs::write(&file, b"hello").unwrap();

    let status = Indexer::needs_reindex(&storage, &file).await;
    assert!(
        matches!(status, ReindexStatus::New),
        "expected New, got {:?}",
        status
    );
}

#[tokio::test]
async fn needs_reindex_unchanged_when_mtime_before_indexed_at() {
    let (storage, tmp) = make_storage().await;
    let file = tmp.path().join("stable.txt");
    std::fs::write(&file, b"hello").unwrap();

    // indexed_at is in the future relative to the file's (now) mtime.
    let mut memory = Memory::new(file.clone(), local_source(tmp.path()), MemoryKind::Database);
    memory.indexed_at = Utc::now() + chrono::Duration::hours(1);
    storage.upsert_memory(&memory).await.unwrap();

    let status = Indexer::needs_reindex(&storage, &file).await;
    assert!(
        matches!(status, ReindexStatus::Unchanged),
        "expected Unchanged, got {:?}",
        status
    );
}

#[tokio::test]
async fn needs_reindex_modified_when_mtime_after_indexed_at() {
    let (storage, tmp) = make_storage().await;
    let file = tmp.path().join("edited.txt");
    std::fs::write(&file, b"hello").unwrap();

    // indexed_at is in the past; the file's current mtime is newer.
    let mut memory = Memory::new(file.clone(), local_source(tmp.path()), MemoryKind::Database);
    memory.indexed_at = Utc::now() - chrono::Duration::hours(1);
    storage.upsert_memory(&memory).await.unwrap();

    let status = Indexer::needs_reindex(&storage, &file).await;
    assert!(
        matches!(status, ReindexStatus::Modified),
        "expected Modified, got {:?}",
        status
    );
}

#[tokio::test]
async fn needs_reindex_error_when_file_missing() {
    let (storage, tmp) = make_storage().await;
    let missing = tmp.path().join("does_not_exist.txt");

    let status = Indexer::needs_reindex(&storage, &missing).await;
    assert!(
        matches!(status, ReindexStatus::Error(_)),
        "expected Error, got {:?}",
        status
    );
}

// ---------------------------------------------------------------------------
// Watcher: should_skip_path
// ---------------------------------------------------------------------------

#[test]
fn skip_path_matches_vcs_and_build_dirs() {
    assert!(should_skip_path(Path::new("/home/u/proj/.git/config")));
    assert!(should_skip_path(Path::new(
        "/home/u/proj/node_modules/pkg/index.js"
    )));
    assert!(should_skip_path(Path::new("/home/u/proj/target/debug/app")));
    assert!(should_skip_path(Path::new("/home/u/proj/.venv/lib/x.py")));
    assert!(should_skip_path(Path::new(
        "/home/u/proj/__pycache__/m.pyc"
    )));
}

#[test]
fn skip_path_allows_normal_source_files() {
    assert!(!should_skip_path(Path::new("/home/u/proj/src/main.rs")));
    assert!(!should_skip_path(Path::new("/home/u/proj/docs/readme.md")));
}

// ---------------------------------------------------------------------------
// Watcher: debounce coalescing with injected timestamps (no real sleep)
// ---------------------------------------------------------------------------

#[test]
fn debounce_coalesces_duplicate_events_for_same_path() {
    let mut queue = DebouncedEvents::new(Duration::from_millis(500));
    let t0 = Instant::now();
    let src = local_source(Path::new("/tmp/watched"));
    let path = PathBuf::from("/tmp/watched/a.txt");

    // Three rapid-fire events for the same path within the window.
    queue.add_at(
        WatchEvent::Created {
            path: path.clone(),
            source: src.clone(),
        },
        t0,
    );
    queue.add_at(
        WatchEvent::Modified {
            path: path.clone(),
            source: src.clone(),
        },
        t0 + Duration::from_millis(100),
    );
    queue.add_at(
        WatchEvent::Modified {
            path: path.clone(),
            source: src.clone(),
        },
        t0 + Duration::from_millis(200),
    );

    // Coalesced into a single pending entry keyed by path.
    assert_eq!(queue.pending_count(), 1);
}

#[test]
fn debounce_releases_only_after_window_elapses() {
    let mut queue = DebouncedEvents::new(Duration::from_millis(500));
    let t0 = Instant::now();
    let src = local_source(Path::new("/tmp/watched"));
    let path = PathBuf::from("/tmp/watched/b.txt");

    // Last update timestamp becomes t0 + 100ms.
    queue.add_at(
        WatchEvent::Created {
            path: path.clone(),
            source: src.clone(),
        },
        t0,
    );
    queue.add_at(
        WatchEvent::Modified {
            path: path.clone(),
            source: src.clone(),
        },
        t0 + Duration::from_millis(100),
    );

    // Before the window elapses (100 + 500 = 600): nothing ready, still pending.
    let early = queue.get_ready_events_at(t0 + Duration::from_millis(400));
    assert!(early.is_empty());
    assert_eq!(queue.pending_count(), 1);

    // After the window elapses: exactly one event released, queue drained.
    let ready = queue.get_ready_events_at(t0 + Duration::from_millis(600));
    assert_eq!(ready.len(), 1);
    assert!(
        matches!(ready[0], WatchEvent::Modified { .. }),
        "coalesced event should be the most recent (Modified)"
    );
    assert_eq!(queue.pending_count(), 0);
}

#[test]
fn debounce_distinct_paths_are_independent() {
    let mut queue = DebouncedEvents::new(Duration::from_millis(500));
    let t0 = Instant::now();
    let src = local_source(Path::new("/tmp/watched"));

    queue.add_at(
        WatchEvent::Created {
            path: PathBuf::from("/tmp/watched/x.txt"),
            source: src.clone(),
        },
        t0,
    );
    queue.add_at(
        WatchEvent::Created {
            path: PathBuf::from("/tmp/watched/y.txt"),
            source: src.clone(),
        },
        t0,
    );

    assert_eq!(queue.pending_count(), 2);
    let ready = queue.get_ready_events_at(t0 + Duration::from_millis(500));
    assert_eq!(ready.len(), 2);
}

// ---------------------------------------------------------------------------
// Scheduler: should_sync
// ---------------------------------------------------------------------------

#[test]
fn should_sync_triggers_when_never_synced() {
    let now = Utc::now();
    assert!(should_sync(None, now, 3600));
}

#[test]
fn should_sync_boundary_exactly_at_interval() {
    let now = Utc::now();
    let last = now - chrono::Duration::seconds(3600);
    assert!(
        should_sync(Some(last), now, 3600),
        "elapsed == interval must trigger"
    );

    let just_under = now - chrono::Duration::seconds(3599);
    assert!(
        !should_sync(Some(just_under), now, 3600),
        "just under must not trigger"
    );
}

#[test]
fn should_sync_per_source_override_vs_default() {
    let now = Utc::now();
    let last = now - chrono::Duration::seconds(600); // 10 minutes ago

    // Default 1h interval: not yet due.
    assert!(!should_sync(Some(last), now, 3600));
    // Per-source 5min override: due.
    assert!(should_sync(Some(last), now, 300));
}

#[test]
fn should_sync_ignores_future_last_sync() {
    let now = Utc::now();
    // Clock skew: last_sync in the future should not overflow into a trigger.
    let future = now + chrono::Duration::seconds(120);
    assert!(!should_sync(Some(future), now, 3600));
}
