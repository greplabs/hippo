//! Tests for the result reranker (Phase 2 of elite retrieval).
//!
//! These are fully OFFLINE and DETERMINISTIC: the reranker is a pure function
//! over in-memory `MemorySearchResult`s. No storage, no embedder, no network.

use std::path::PathBuf;

use chrono::{Duration, Utc};
use hippo_core::models::{Highlight, Memory, MemoryId, MemoryKind, MemorySearchResult, Source};
use hippo_core::search::{rerank_stage, HeuristicReranker, Reranker};

// ------------------------------------------------------------------ helpers

fn mem(name: &str) -> Memory {
    Memory::new(
        PathBuf::from(format!("/docs/{name}")),
        Source::Local {
            root_path: PathBuf::from("/docs"),
        },
        MemoryKind::Document {
            format: hippo_core::models::DocumentFormat::PlainText,
            page_count: None,
        },
    )
}

/// Build a candidate with an explicit fused score and optional title.
fn candidate(filename: &str, fused_score: f32, title: Option<&str>) -> MemorySearchResult {
    let mut m = mem(filename);
    m.metadata.title = title.map(|t| t.to_string());
    MemorySearchResult {
        memory: m,
        score: fused_score,
        highlights: vec![Highlight {
            field: "semantic".into(),
            snippet: "x".into(),
        }],
    }
}

fn ids(results: &[MemorySearchResult]) -> Vec<MemoryId> {
    results.iter().map(|r| r.memory.id).collect()
}

// -------------------------------------------------------------------- tests

/// A candidate whose title EXACTLY matches the query must outrank a candidate
/// with a strictly higher fused (RRF) score but no title match.
#[test]
fn exact_title_match_beats_higher_rrf() {
    let reranker = HeuristicReranker::new();

    // `high` has the larger fused score but no lexical match.
    // `exact` has a lower fused score but its title IS the query.
    let high = candidate("unrelated.txt", 0.95, Some("something else entirely"));
    let high_id = high.memory.id;
    let exact = candidate("report.txt", 0.40, Some("quarterly revenue"));
    let exact_id = exact.memory.id;

    let out = reranker.rerank("quarterly revenue", vec![high, exact]);

    assert_eq!(
        out[0].memory.id, exact_id,
        "exact title match should be reranked first"
    );
    assert_eq!(out[1].memory.id, high_id);
}

/// A contiguous phrase match must beat the same terms appearing scattered.
#[test]
fn phrase_match_beats_scattered_terms() {
    let reranker = HeuristicReranker::new();

    // Same fused score; both contain the words "machine" and "learning".
    let phrase = candidate("a.txt", 0.5, Some("machine learning basics"));
    let phrase_id = phrase.memory.id;
    let scattered = candidate("b.txt", 0.5, Some("learning to use a sewing machine"));
    let scattered_id = scattered.memory.id;

    let out = reranker.rerank("machine learning", vec![scattered, phrase]);

    assert_eq!(
        out[0].memory.id, phrase_id,
        "contiguous phrase match should rank above scattered terms"
    );
    assert_eq!(out[1].memory.id, scattered_id);
}

/// With all stronger signals equal, `is_favorite` breaks the tie.
#[test]
fn favorite_breaks_tie() {
    let reranker = HeuristicReranker::new();

    // Identical fused score, identical (absent) lexical match.
    let mut fav = candidate("a.txt", 0.5, None);
    fav.memory.is_favorite = true;
    let fav_id = fav.memory.id;
    let plain = candidate("b.txt", 0.5, None);
    let plain_id = plain.memory.id;

    // Put the non-favorite first so a win requires a real reorder.
    let out = reranker.rerank("anything", vec![plain, fav]);

    assert_eq!(out[0].memory.id, fav_id, "favorite should win the tie");
    assert_eq!(out[1].memory.id, plain_id);
}

/// With all stronger signals equal, recency breaks the tie.
#[test]
fn recency_breaks_tie() {
    let reranker = HeuristicReranker::new();

    let mut recent = candidate("a.txt", 0.5, None);
    recent.memory.modified_at = Utc::now() - Duration::days(1); // within a week
    let recent_id = recent.memory.id;

    let mut old = candidate("b.txt", 0.5, None);
    old.memory.modified_at = Utc::now() - Duration::days(400); // old
    let old_id = old.memory.id;

    let out = reranker.rerank("anything", vec![old, recent]);

    assert_eq!(
        out[0].memory.id, recent_id,
        "recent file should win the tie"
    );
    assert_eq!(out[1].memory.id, old_id);
}

/// `rerank_enabled = false` is the identity: order is preserved exactly.
#[test]
fn disabled_is_identity() {
    let reranker = HeuristicReranker::new();

    // An order that the heuristic WOULD change if it ran.
    let strong = candidate("report.txt", 0.10, Some("exact query"));
    let weak = candidate("unrelated.txt", 0.90, Some("nope"));
    let input = vec![weak, strong];
    let expected = ids(&input);

    let out = rerank_stage(
        &reranker,
        "exact query",
        input,
        /* enabled */ false,
        50,
    );

    assert_eq!(ids(&out), expected, "disabled rerank must not reorder");
}

/// Only the top `rerank_candidates` are reranked; the tail keeps its RRF order,
/// even if a tail candidate would otherwise rank first.
#[test]
fn respects_candidate_cutoff() {
    let reranker = HeuristicReranker::new();

    // 4 candidates in fused order. The LAST one is an exact title match that
    // would jump to #1 if it were inside the rerank window.
    let c0 = candidate("c0.txt", 0.9, Some("alpha"));
    let c1 = candidate("c1.txt", 0.8, Some("beta"));
    let c2 = candidate("c2.txt", 0.7, Some("gamma"));
    let c3 = candidate("c3.txt", 0.6, Some("exact query")); // would-be winner
    let (c0id, c1id, c2id, c3id) = (c0.memory.id, c1.memory.id, c2.memory.id, c3.memory.id);

    // Cutoff of 2 → only c0, c1 are reranked; c2, c3 stay put.
    let out = rerank_stage(&reranker, "exact query", vec![c0, c1, c2, c3], true, 2);

    let out_ids = ids(&out);
    assert_eq!(out.len(), 4);
    // Tail preserved in original positions regardless of lexical match.
    assert_eq!(out_ids[2], c2id, "tail position 2 preserved");
    assert_eq!(out_ids[3], c3id, "exact match in tail is NOT promoted");
    // Head still contains exactly the first two candidates (some order).
    assert!(out_ids[..2].contains(&c0id) && out_ids[..2].contains(&c1id));
}

/// The trait is object-safe and dispatches dynamically through `&dyn Reranker`.
#[test]
fn trait_is_object_safe() {
    let reranker: Box<dyn Reranker> = Box::new(HeuristicReranker::new());
    assert_eq!(reranker.name(), "heuristic");

    let a = candidate("a.txt", 0.5, Some("hello world"));
    let a_id = a.memory.id;
    let b = candidate("b.txt", 0.9, Some("nothing relevant"));

    // Dispatch through the trait object (also exercises rerank_stage(&dyn ...)).
    let out = rerank_stage(reranker.as_ref(), "hello world", vec![b, a], true, 50);
    assert_eq!(out[0].memory.id, a_id);
}

/// Single-candidate and empty inputs are returned unchanged (no panic).
#[test]
fn trivial_inputs_unchanged() {
    let reranker = HeuristicReranker::new();

    let empty: Vec<MemorySearchResult> = reranker.rerank("q", vec![]);
    assert!(empty.is_empty());

    let one = vec![candidate("a.txt", 0.5, Some("solo"))];
    let one_id = one[0].memory.id;
    let out = reranker.rerank("q", one);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].memory.id, one_id);
}
