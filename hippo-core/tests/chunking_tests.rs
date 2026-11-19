//! Tests for document chunking + chunk-level retrieval with parent roll-up.
//!
//! These are deterministic and OFFLINE: the chunker is a pure function, and the
//! storage/roll-up tests use the hash embedder (`local_embeddings = false`,
//! no API key → `EmbeddingProvider::Local`) against a temp-dir SQLite database
//! with Qdrant pointed at a closed port, exactly like `storage_correctness_tests`.

use std::path::PathBuf;

use hippo_core::embeddings::Embedder;
use hippo_core::models::{Chunk, ChunkRollup, Memory, MemoryKind, Source};
use hippo_core::storage::Storage;
use hippo_core::HippoConfig;
use tempfile::TempDir;
use uuid::Uuid;

// ------------------------------------------------------------------ helpers

/// A config whose embedder is the deterministic offline hash backend and whose
/// Qdrant points at a closed port (SQLite-only path).
fn offline_config(data_dir: PathBuf) -> HippoConfig {
    HippoConfig {
        data_dir,
        local_embeddings: false, // no Ollama probe → hash fallback
        ai_api_key: None,        // → EmbeddingProvider::Local
        qdrant_url: "http://127.0.0.1:59321".to_string(),
        ..HippoConfig::default()
    }
}

async fn make_storage() -> (Storage, TempDir) {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let storage = Storage::new(&offline_config(tmp.path().to_path_buf()))
        .await
        .expect("init storage");
    (storage, tmp)
}

async fn make_embedder(tmp: &TempDir) -> Embedder {
    Embedder::new(&offline_config(tmp.path().to_path_buf()))
        .await
        .expect("init embedder")
}

fn doc_memory(path: &str) -> Memory {
    Memory::new(
        PathBuf::from(path),
        Source::Local {
            root_path: PathBuf::from("/tmp"),
        },
        MemoryKind::Document {
            format: hippo_core::models::DocumentFormat::PlainText,
            page_count: None,
        },
    )
}

fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

// ------------------------------------------------------------------ chunker

#[test]
fn long_text_produces_overlapping_sentence_aligned_chunks() {
    // 8 five-word sentences = 40 words total.
    let text = "Alpha beta gamma delta epsilon. \
                Zeta eta theta iota kappa. \
                Lambda mu nu xi omicron. \
                Pi rho sigma tau upsilon. \
                Phi chi psi omega aleph. \
                Bet gimel dalet he waw. \
                Zayin het tet yod kaf. \
                Lamed mem nun samek ayin.";

    let chunk_tokens = 10;
    let overlap = 3;
    let chunks = Embedder::chunk_document(text, chunk_tokens, overlap);

    assert!(
        chunks.len() >= 3,
        "a 40-word text with chunk size 10 should split into several chunks, got {}",
        chunks.len()
    );

    for (i, c) in chunks.iter().enumerate() {
        // Offsets round-trip exactly.
        assert_eq!(
            &text[c.char_start..c.char_end],
            c.text,
            "chunk {i} offsets must slice back to the chunk text"
        );

        // Never split a word: a chunk begins at the start of a word and ends at
        // the end of a word (boundary is start-of-text / end-of-text or a space).
        assert!(
            c.char_start == 0 || text.as_bytes()[c.char_start - 1].is_ascii_whitespace(),
            "chunk {i} must start on a word boundary"
        );
        assert!(
            c.char_end == text.len() || text.as_bytes()[c.char_end].is_ascii_whitespace(),
            "chunk {i} must end on a word boundary"
        );

        // Size cap holds for every chunk; non-last chunks are at least half target.
        let wc = word_count(&c.text);
        assert!(wc <= chunk_tokens, "chunk {i} ({wc} words) exceeds the cap");
        if i + 1 < chunks.len() {
            assert!(
                wc >= chunk_tokens / 2,
                "non-final chunk {i} ({wc} words) is unexpectedly small"
            );
        }
    }

    // Consecutive chunks overlap: the next chunk starts before the previous one
    // ended (byte ranges intersect → shared words).
    for w in chunks.windows(2) {
        assert!(
            w[1].char_start < w[0].char_end,
            "consecutive chunks must overlap: {} !< {}",
            w[1].char_start,
            w[0].char_end
        );
    }
}

#[test]
fn short_text_yields_exactly_one_chunk() {
    let text = "Just a short sentence with a handful of words.";
    let chunks = Embedder::chunk_document(text, 512, 76);
    assert_eq!(chunks.len(), 1, "short text must be a single chunk");
    assert_eq!(chunks[0].char_start, 0);
    assert_eq!(chunks[0].char_end, text.len());
    assert_eq!(chunks[0].text, text);
}

#[test]
fn empty_text_yields_no_chunks() {
    assert!(Embedder::chunk_document("", 512, 76).is_empty());
    assert!(Embedder::chunk_document("   \n\t  ", 512, 76).is_empty());
}

#[test]
fn long_unpunctuated_run_still_chunks_on_word_boundaries() {
    // No sentence punctuation at all (e.g. a code-ish blob): must still split by
    // the hard word cap, never mid-word.
    let text = (0..60)
        .map(|i| format!("tok{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let chunks = Embedder::chunk_document(&text, 10, 2);
    assert!(chunks.len() >= 5, "60 words / 10 should be many chunks");
    for c in &chunks {
        assert_eq!(&text[c.char_start..c.char_end], c.text);
        assert!(word_count(&c.text) <= 10);
    }
}

// ------------------------------------------------------------------ storage

fn chunks_for(parent: Uuid, texts: &[&str]) -> Vec<Chunk> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let id = Uuid::new_v4().to_string();
            Chunk {
                chunk_id: id.clone(),
                parent_memory_id: parent,
                ordinal: i,
                text: (*t).to_string(),
                char_start: 0,
                char_end: t.len(),
                embedding_id: Some(id),
            }
        })
        .collect()
}

#[tokio::test]
async fn upsert_and_get_chunks_round_trip() {
    let (storage, _tmp) = make_storage().await;
    let mem = doc_memory("/tmp/a.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");

    let chunks = chunks_for(mem.id, &["first passage", "second passage", "third passage"]);
    storage
        .upsert_chunks(mem.id, &chunks)
        .await
        .expect("upsert chunks");

    let fetched = storage.get_chunks(mem.id).await.expect("get chunks");
    assert_eq!(fetched.len(), 3);
    assert_eq!(fetched, chunks, "chunks must round-trip in ordinal order");
}

#[tokio::test]
async fn delete_chunks_removes_them() {
    let (storage, _tmp) = make_storage().await;
    let mem = doc_memory("/tmp/b.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");
    storage
        .upsert_chunks(mem.id, &chunks_for(mem.id, &["x", "y"]))
        .await
        .expect("upsert chunks");

    assert_eq!(storage.get_chunks(mem.id).await.unwrap().len(), 2);
    storage.delete_chunks(mem.id).await.expect("delete chunks");
    assert!(storage.get_chunks(mem.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn reindex_replaces_chunks_without_orphans_or_dupes() {
    let (storage, _tmp) = make_storage().await;
    let mem = doc_memory("/tmp/c.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");

    storage
        .upsert_chunks(mem.id, &chunks_for(mem.id, &["one", "two", "three", "four"]))
        .await
        .expect("first index");
    assert_eq!(storage.get_chunks(mem.id).await.unwrap().len(), 4);

    // Re-index with FEWER chunks: the old rows must be fully replaced.
    let new_chunks = chunks_for(mem.id, &["alpha", "beta"]);
    storage
        .upsert_chunks(mem.id, &new_chunks)
        .await
        .expect("reindex");

    let fetched = storage.get_chunks(mem.id).await.unwrap();
    assert_eq!(fetched.len(), 2, "re-index must replace, not append");
    assert_eq!(fetched, new_chunks);
}

#[tokio::test]
async fn deleting_memory_tears_down_its_chunks() {
    let (storage, _tmp) = make_storage().await;
    let mem = doc_memory("/tmp/d.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");
    storage
        .upsert_chunks(mem.id, &chunks_for(mem.id, &["p", "q"]))
        .await
        .expect("upsert chunks");

    storage.delete_memory(mem.id).await.expect("delete memory");
    assert!(
        storage.get_chunks(mem.id).await.unwrap().is_empty(),
        "deleting a memory must remove its chunk rows"
    );
}

// ------------------------------------------------------------------ roll-up

#[tokio::test]
async fn rollup_scores_parent_by_best_chunk_and_dedupes() {
    let (storage, tmp) = make_storage().await;
    let embedder = make_embedder(&tmp).await;

    let mem = doc_memory("/tmp/rollup.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");

    let texts = [
        "the quick brown fox jumps",
        "lorem ipsum dolor sit amet",
        "completely unrelated gardening tips",
    ];
    let chunks = chunks_for(mem.id, &texts);
    storage
        .upsert_chunks(mem.id, &chunks)
        .await
        .expect("upsert chunks");

    // Embed and store each chunk's vector (hash backend → deterministic).
    let owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
    let embeddings = embedder.embed_batch(&owned).await.expect("embed chunks");
    for (chunk, emb) in chunks.iter().zip(embeddings.iter()) {
        storage
            .store_chunk_embedding_with_qdrant(chunk, emb)
            .await
            .expect("store chunk embedding");
    }

    // Query is identical to chunk #1's text → that chunk is the best match
    // (cosine ~1.0 with the hash embedder), so the parent is scored by it.
    let query = embedder.embed_query(texts[1]).await.expect("embed query");
    let hits = storage
        .semantic_rollup(&query, 10, ChunkRollup::Max)
        .await
        .expect("rollup");

    assert_eq!(hits.len(), 1, "the parent must appear exactly once");
    let hit = &hits[0];
    assert_eq!(hit.memory_id, mem.id);
    assert!(
        hit.score > 0.99,
        "max roll-up must take the best (identical) chunk, score was {}",
        hit.score
    );
    let best = hit.best_chunk.as_ref().expect("best chunk present");
    assert_eq!(best.ordinal, 1, "best chunk must be the matching passage");
}

#[tokio::test]
async fn memory_without_chunks_retrievable_via_whole_doc_fallback() {
    let (storage, tmp) = make_storage().await;
    let embedder = make_embedder(&tmp).await;

    // A memory with ONLY a whole-document embedding (no chunks).
    let mem = doc_memory("/tmp/wholedoc.txt");
    storage.upsert_memory(&mem).await.expect("upsert memory");

    let doc_text = "annual financial report for the fiscal year";
    let doc_emb = embedder
        .embed_batch(&[doc_text.to_string()])
        .await
        .expect("embed doc")
        .remove(0);
    storage
        .store_embedding_with_qdrant(mem.id, &doc_emb, "text_embedding", &mem.kind)
        .await
        .expect("store whole-doc embedding");

    let query = embedder.embed_query(doc_text).await.expect("embed query");
    let hits = storage
        .semantic_rollup(&query, 10, ChunkRollup::Max)
        .await
        .expect("rollup");

    let hit = hits
        .iter()
        .find(|h| h.memory_id == mem.id)
        .expect("chunk-less memory must still be retrievable via whole-doc embedding");
    assert!(hit.score > 0.99, "whole-doc self-match score was {}", hit.score);
    assert!(
        hit.best_chunk.is_none(),
        "a chunk-less memory has no best chunk"
    );
}

#[tokio::test]
async fn rollup_mixes_chunked_and_chunkless_memories() {
    let (storage, tmp) = make_storage().await;
    let embedder = make_embedder(&tmp).await;

    // Memory A: chunked, one chunk is about "mountain hiking trails".
    let a = doc_memory("/tmp/a_chunked.txt");
    storage.upsert_memory(&a).await.unwrap();
    let a_texts = ["intro boilerplate text", "mountain hiking trails guide"];
    let a_chunks = chunks_for(a.id, &a_texts);
    storage.upsert_chunks(a.id, &a_chunks).await.unwrap();
    let a_owned: Vec<String> = a_texts.iter().map(|s| s.to_string()).collect();
    let a_embs = embedder.embed_batch(&a_owned).await.unwrap();
    for (c, e) in a_chunks.iter().zip(a_embs.iter()) {
        storage
            .store_chunk_embedding_with_qdrant(c, e)
            .await
            .unwrap();
    }

    // Memory B: chunk-less, whole-doc about something unrelated.
    let b = doc_memory("/tmp/b_wholedoc.txt");
    storage.upsert_memory(&b).await.unwrap();
    let b_emb = embedder
        .embed_batch(&["database indexing internals".to_string()])
        .await
        .unwrap()
        .remove(0);
    storage
        .store_embedding_with_qdrant(b.id, &b_emb, "text_embedding", &b.kind)
        .await
        .unwrap();

    // Query matches A's second chunk.
    let query = embedder.embed_query(a_texts[1]).await.unwrap();
    let hits = storage
        .semantic_rollup(&query, 10, ChunkRollup::Max)
        .await
        .unwrap();

    // Both memories are present exactly once; A ranks first (near-perfect match).
    assert_eq!(hits.iter().filter(|h| h.memory_id == a.id).count(), 1);
    assert_eq!(hits.iter().filter(|h| h.memory_id == b.id).count(), 1);
    assert_eq!(hits[0].memory_id, a.id, "the chunk match should rank first");
    assert!(hits[0].score > 0.99);
}
