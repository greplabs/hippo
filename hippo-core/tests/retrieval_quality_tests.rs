//! Retrieval / embedding quality regression tests.
//!
//! These guard the fixes that restored semantic search:
//! - the text embedding dimension contract (P0),
//! - L2-normalization so dot product == cosine (P0),
//! - Reciprocal Rank Fusion for hybrid search (P1).
//!
//! They are deliberately infra-free (no Ollama / Qdrant / network) so they run
//! in CI: the hash fallback path is exercised for normalization, and RRF is a
//! pure function.

use hippo_core::embeddings::{Embedder, OPENAI_EMBEDDING_DIM, TEXT_EMBEDDING_DIM};
use hippo_core::ollama::NOMIC_EMBED_DIM;
use hippo_core::search::reciprocal_rank_fusion;
use hippo_core::HippoConfig;
use uuid::Uuid;

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let n = l2_norm(&v);
    if n > 0.0 {
        for x in &mut v {
            *x /= n;
        }
    }
    v
}

/// P0: the active text embedding dim MUST equal the default local model's
/// output dim (nomic-embed-text → 768). If these drift apart again, Qdrant's
/// `hippo_text` collection is created at the wrong size and every text vector
/// is silently dropped on upsert.
#[test]
fn text_embedding_dim_matches_default_local_model() {
    assert_eq!(
        TEXT_EMBEDDING_DIM, NOMIC_EMBED_DIM,
        "TEXT_EMBEDDING_DIM ({}) must match nomic-embed-text dim ({})",
        TEXT_EMBEDDING_DIM, NOMIC_EMBED_DIM
    );
    assert_eq!(OPENAI_EMBEDDING_DIM, 1536);
}

/// P0: `Embedder::similarity` is the canonical cosine. For L2-normalized inputs
/// the dot product equals the cosine similarity.
#[test]
fn canonical_similarity_is_cosine_for_normalized_vectors() {
    // Identical (normalized) vectors -> 1.0
    let a = normalize(vec![3.0, 4.0, 0.0]);
    let s = Embedder::similarity(&a, &a);
    assert!((s - 1.0).abs() < 1e-5, "identical => 1.0, got {}", s);

    // Orthogonal (normalized) vectors -> ~0.0
    let x = normalize(vec![1.0, 0.0, 0.0]);
    let y = normalize(vec![0.0, 1.0, 0.0]);
    assert!(
        Embedder::similarity(&x, &y).abs() < 1e-5,
        "orthogonal => 0.0"
    );

    // Dimension mismatch -> 0.0 (never panics)
    assert_eq!(Embedder::similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
}

/// P0: every generated embedding is L2-normalized exactly once. Exercised via
/// the deterministic hash fallback (Local provider: no Ollama, no API key, no
/// network).
#[tokio::test]
async fn generated_embeddings_are_l2_normalized() {
    let config = HippoConfig {
        local_embeddings: false, // force no Ollama
        ai_api_key: None,        // force no OpenAI -> Local hash provider
        ..Default::default()
    };
    let embedder = Embedder::new(&config).await.expect("embedder");

    let v = embedder
        .embed_query("a quick brown fox jumps over the lazy dog")
        .await
        .expect("embed query");

    assert_eq!(v.len(), TEXT_EMBEDDING_DIM, "fallback must use active dim");
    let norm = l2_norm(&v);
    assert!(
        (norm - 1.0).abs() < 1e-3,
        "query embedding must be unit length, got {}",
        norm
    );
    assert_eq!(embedder.text_output_dim(), TEXT_EMBEDDING_DIM);
}

/// P1: a document that appears in BOTH ranked lists must outrank documents that
/// appear in only one list — even when the single-list docs sit at rank 0 of
/// their arm. This is the whole point of fusion and replaces the old
/// tautological weighted-multiply behavior.
#[test]
fn rrf_doc_in_both_lists_outranks_single_list_docs() {
    let both = Uuid::new_v4();
    let semantic_only = Uuid::new_v4();
    let keyword_only = Uuid::new_v4();

    // Single-list docs are placed FIRST (rank 0) in their arm; the shared doc
    // is one position lower in each arm, yet still wins via accumulation.
    let semantic = vec![semantic_only, both];
    let keyword = vec![keyword_only, both];

    let scores = reciprocal_rank_fusion(&[(1.0, semantic), (1.0, keyword)], 60.0);

    let s_both = scores[&both];
    let s_sem = scores[&semantic_only];
    let s_kw = scores[&keyword_only];

    assert!(
        s_both > s_sem,
        "doc in both arms ({}) must outrank semantic-only ({})",
        s_both,
        s_sem
    );
    assert!(
        s_both > s_kw,
        "doc in both arms ({}) must outrank keyword-only ({})",
        s_both,
        s_kw
    );
}

/// P1: RRF arm weights actually matter — a doc at rank 0 of the heavier arm
/// beats a doc at rank 0 of the lighter arm.
#[test]
fn rrf_arm_weights_apply() {
    let heavy = Uuid::new_v4();
    let light = Uuid::new_v4();

    let scores = reciprocal_rank_fusion(&[(0.7, vec![heavy]), (0.3, vec![light])], 60.0);

    assert!(
        scores[&heavy] > scores[&light],
        "heavier-weighted arm's top doc must score higher"
    );
}

/// RRF is scale-free: an empty arm contributes nothing and never panics.
#[test]
fn rrf_handles_empty_arms() {
    let only = Uuid::new_v4();
    let scores = reciprocal_rank_fusion(&[(1.0, vec![only]), (1.0, vec![])], 60.0);
    assert_eq!(scores.len(), 1);
    assert!(scores[&only] > 0.0);
}
