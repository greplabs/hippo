//! Regression tests for storage-layer correctness bugs fixed in
//! `fix/storage-correctness`:
//!   * P0 self-deadlock in `search_fts` when FTS matches zero rows
//!   * P0 tag-count inflation (counts must be derived from truth)
//!   * P1 embedding durability (SQLite is always the source of truth)
//!   * P1 `get_all_memories` no longer caps at 5000 rows
//!   * P2 LIKE-wildcard escaping in user queries
//!
//! These are integration tests: they build a real `Storage` against a
//! temp-dir SQLite database with Qdrant pointed at a closed port (so it is
//! always "unavailable").

use std::path::PathBuf;
use std::time::Duration;

use hippo_core::models::{Memory, MemoryKind, Source, Tag};
use hippo_core::storage::Storage;
use hippo_core::HippoConfig;
use tempfile::TempDir;

/// Build a `Storage` backed by a fresh temp dir. Qdrant points at a closed
/// local port so `is_available()` is false (SQLite-only path). The returned
/// `TempDir` must be kept alive for the lifetime of the `Storage`.
async fn make_storage() -> (Storage, TempDir) {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let config = HippoConfig {
        data_dir: tmp.path().to_path_buf(),
        // A port nothing is listening on -> Qdrant health check fails fast.
        qdrant_url: "http://127.0.0.1:59321".to_string(),
        ..HippoConfig::default()
    };
    let storage = Storage::new(&config).await.expect("init storage");
    (storage, tmp)
}

fn memory_with(path: &str, tags: &[&str]) -> Memory {
    let mut m = Memory::new(
        PathBuf::from(path),
        Source::Local {
            root_path: PathBuf::from("/tmp"),
        },
        MemoryKind::Unknown,
    );
    m.tags = tags.iter().map(|t| Tag::user(*t)).collect();
    m
}

async fn tag_count(storage: &Storage, name: &str) -> Option<u64> {
    storage
        .list_tags()
        .await
        .expect("list_tags")
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, c)| c)
}

/// P0: a zero-result FTS search must return `Ok(empty)` promptly. Before the
/// fix the `get_db()` guard stayed alive across the `search_like` fallback,
/// which re-locked the same non-reentrant mutex on the same thread and
/// deadlocked forever. The timeout makes a regression show up as a failure
/// instead of a hung test.
#[tokio::test]
async fn zero_result_search_does_not_deadlock() {
    let (storage, _tmp) = make_storage().await;

    // Seed one memory that will NOT match, so FTS genuinely returns zero rows
    // and the LIKE fallback is exercised.
    storage
        .upsert_memory(&memory_with("/tmp/apple.txt", &[]))
        .await
        .expect("upsert");

    let result = tokio::time::timeout(
        Duration::from_secs(5),
        storage.search_fts("zzzznomatch99999", None, 10, 0),
    )
    .await;

    let memories = result
        .expect("search_fts deadlocked / timed out (regression!)")
        .expect("search_fts returned an error");
    assert!(memories.is_empty(), "expected no matches");
}

/// P0: tag counts must reflect reality and never inflate, regardless of how
/// many times a memory is re-upserted or how tags are added/removed/deleted.
#[tokio::test]
async fn tag_counts_never_inflate() {
    let (storage, _tmp) = make_storage().await;

    let mem = memory_with("/tmp/doc.txt", &["foo"]);
    let id = mem.id;

    // Upsert the SAME memory three times (simulates reindexing).
    for _ in 0..3 {
        storage.upsert_memory(&mem).await.expect("upsert");
    }
    assert_eq!(
        tag_count(&storage, "foo").await,
        Some(1),
        "re-upserting the same memory must not inflate the tag count"
    );

    // Add a second tag, then remove it: count must track reality.
    storage
        .add_tag(id, Tag::user("bar"))
        .await
        .expect("add_tag");
    assert_eq!(tag_count(&storage, "foo").await, Some(1));
    assert_eq!(tag_count(&storage, "bar").await, Some(1));

    storage.remove_tag(id, "bar").await.expect("remove_tag");
    assert_eq!(tag_count(&storage, "foo").await, Some(1));
    assert_eq!(
        tag_count(&storage, "bar").await,
        None,
        "a tag no longer on any memory must drop out of the counts"
    );

    // A second memory with the same tag -> foo == 2.
    let mem2 = memory_with("/tmp/doc2.txt", &["foo"]);
    let id2 = mem2.id;
    storage.upsert_memory(&mem2).await.expect("upsert");
    assert_eq!(tag_count(&storage, "foo").await, Some(2));

    // Deleting a memory must decrement derived counts.
    storage.delete_memory(id2).await.expect("delete_memory");
    assert_eq!(
        tag_count(&storage, "foo").await,
        Some(1),
        "deleting a memory must reduce the derived tag count"
    );
}

/// `list_tags_with_colors` keeps the stored color while deriving the count.
#[tokio::test]
async fn tag_colors_preserved_with_derived_counts() {
    let (storage, _tmp) = make_storage().await;

    storage
        .upsert_memory(&memory_with("/tmp/a.txt", &["foo"]))
        .await
        .expect("upsert");
    storage
        .set_tag_color("foo", Some("#ff0000"))
        .await
        .expect("set color");

    let rows = storage
        .list_tags_with_colors()
        .await
        .expect("list_tags_with_colors");
    let foo = rows
        .into_iter()
        .find(|(n, _, _)| n == "foo")
        .expect("foo present");
    assert_eq!(foo.1, 1);
    assert_eq!(foo.2.as_deref(), Some("#ff0000"));
}

/// P1: SQLite is the durable source of truth for embeddings. With Qdrant
/// unavailable, `store_embedding_with_qdrant` must still persist to SQLite so
/// the vector is retrievable.
#[tokio::test]
async fn embedding_is_always_persisted_to_sqlite() {
    let (storage, _tmp) = make_storage().await;

    let mem = memory_with("/tmp/vec.txt", &[]);
    let id = mem.id;
    storage.upsert_memory(&mem).await.expect("upsert");

    let vector = vec![0.1f32, 0.2, 0.3, 0.4];
    storage
        .store_embedding_with_qdrant(id, &vector, "test-model", &MemoryKind::Unknown)
        .await
        .expect("store embedding");

    let fetched = storage
        .get_embedding(id)
        .await
        .expect("get_embedding")
        .expect("embedding should be present in SQLite");
    assert_eq!(fetched, vector);
}

/// P1: `get_all_memories` must not silently cap at 5000 rows. We can't cheaply
/// insert 5000+ rows in a unit test, but we assert the full set round-trips and
/// that the paginated variant windows correctly.
#[tokio::test]
async fn get_all_memories_returns_everything_and_paginates() {
    let (storage, _tmp) = make_storage().await;

    for i in 0..25 {
        storage
            .upsert_memory(&memory_with(&format!("/tmp/f{i}.txt"), &[]))
            .await
            .expect("upsert");
    }

    let all = storage.get_all_memories().await.expect("get_all_memories");
    assert_eq!(all.len(), 25);

    let page = storage
        .get_all_memories_paginated(10, 0)
        .await
        .expect("paginated");
    assert_eq!(page.len(), 10);

    let last = storage
        .get_all_memories_paginated(10, 20)
        .await
        .expect("paginated");
    assert_eq!(last.len(), 5);
}

/// P2: LIKE wildcards in user queries are escaped, so a literal `%` only
/// matches a filename that actually contains `%`.
#[tokio::test]
async fn like_wildcards_in_query_are_escaped() {
    let (storage, _tmp) = make_storage().await;

    storage
        .upsert_memory(&memory_with("/tmp/50percent.txt", &[]))
        .await
        .expect("upsert");
    storage
        .upsert_memory(&memory_with("/tmp/50%off.txt", &[]))
        .await
        .expect("upsert");

    // Searching for the literal "50%" must match only the file containing '%',
    // not everything starting with "50".
    let results = storage
        .search_like("50%", None, 50, 0)
        .await
        .expect("search_like");
    assert_eq!(results.len(), 1, "'%' must be matched literally");
    assert!(results[0].path.to_string_lossy().contains("50%off"));
}
