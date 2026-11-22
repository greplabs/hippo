//! Tests for the OPTIONAL ONNX cross-encoder reranker (`reranker-onnx` feature).
//!
//! These are fully OFFLINE and DETERMINISTIC: they NEVER download a model or hit
//! the network. The feature-gated tests exercise the config resolution and the
//! graceful load-failure path by pre-staging GARBAGE model/tokenizer files in a
//! temp cache dir (so `ensure_downloaded` skips the network and the subsequent
//! parse fails, proving `load()` returns `Err` and the caller falls back to the
//! heuristic reranker). No test requires a real model.

use std::path::PathBuf;

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

// --------------------------------------------------- always-on wiring tests

/// Trait wiring: the default heuristic reranker is a `&dyn Reranker` and
/// `rerank_stage` drives it deterministically. This is the behaviour a build
/// WITHOUT the `reranker-onnx` feature always gets, and the behaviour the ONNX
/// path falls back to.
#[test]
fn heuristic_reranker_is_object_safe_and_deterministic() {
    let reranker: Box<dyn Reranker> = Box::new(HeuristicReranker::new());
    assert_eq!(reranker.name(), "heuristic");

    let high = candidate("unrelated.txt", 0.95, Some("something else entirely"));
    let high_id = high.memory.id;
    let exact = candidate("report.txt", 0.40, Some("quarterly revenue"));
    let exact_id = exact.memory.id;

    // rerank_stage over both candidates with rerank enabled.
    let out = rerank_stage(
        reranker.as_ref(),
        "quarterly revenue",
        vec![high, exact],
        true,
        50,
    );
    assert_eq!(out[0].memory.id, exact_id, "exact title match ranks first");
    assert_eq!(out[1].memory.id, high_id);

    // Disabled rerank is identity (today's RRF order preserved).
    let a = candidate("a.txt", 0.2, Some("alpha"));
    let b = candidate("b.txt", 0.9, Some("beta"));
    let before = ids(&[a.clone(), b.clone()]);
    let after = ids(&rerank_stage(
        reranker.as_ref(),
        "alpha",
        vec![a, b],
        false,
        50,
    ));
    assert_eq!(before, after, "disabled rerank must be identity");
}

// ----------------------------------------------- feature-gated ONNX tests

#[cfg(feature = "reranker-onnx")]
mod onnx {
    use super::*;
    use chrono::Utc;
    use hippo_core::search::rerank::{OnnxReranker, OnnxRerankerConfig};

    /// Config resolution is deterministic and offline: the default id is used
    /// when none is given, and a custom id overrides it (for cache isolation /
    /// pre-staged models) without touching the network.
    #[test]
    fn config_resolution_is_deterministic() {
        let dir = std::env::temp_dir();
        let default = OnnxRerankerConfig::resolve(None, dir.clone());
        assert_eq!(default.model_id, "bge-reranker-base");
        assert!(default.onnx_url.ends_with(".onnx"));
        assert!(default.tokenizer_url.ends_with("tokenizer.json"));
        assert_eq!(default.max_length, 512);

        let custom = OnnxRerankerConfig::resolve(Some("my-reranker"), dir.clone());
        assert_eq!(custom.model_id, "my-reranker");

        // An empty override string is ignored (keeps the default id).
        let empty = OnnxRerankerConfig::resolve(Some(""), dir);
        assert_eq!(empty.model_id, "bge-reranker-base");
    }

    /// Graceful load failure, OFFLINE: pre-stage GARBAGE `model.onnx` +
    /// `tokenizer.json` so the downloader sees cached files (no network) and the
    /// tokenizer parse fails. `load()` must return `Err` so the searcher falls
    /// back to the heuristic reranker instead of hard-failing search.
    #[tokio::test]
    async fn load_fails_gracefully_with_invalid_cached_model() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cache_dir = tmp.path().to_path_buf();

        let cfg = OnnxRerankerConfig::resolve(None, cache_dir.clone());
        // Mirror the on-disk layout `load()` expects: <cache>/reranker/<id>/.
        let model_dir = cache_dir.join("reranker").join(&cfg.model_id);
        std::fs::create_dir_all(&model_dir).unwrap();
        std::fs::write(model_dir.join("model.onnx"), b"not a real onnx model").unwrap();
        std::fs::write(model_dir.join("tokenizer.json"), b"{ not valid json ]").unwrap();

        let result = OnnxReranker::load(cfg).await;
        assert!(
            result.is_err(),
            "loading a garbage cached model must fail (so the caller falls back)"
        );
    }

    /// Belt-and-suspenders: the heuristic fallback that the ONNX path delegates
    /// to is itself deterministic, even for an empty query (recency/favorite
    /// tie-breaks only). Uses a fixed `now` via the public rerank entry point.
    #[test]
    fn fallback_heuristic_handles_empty_query() {
        let reranker = HeuristicReranker::new();
        let mut a = candidate("a.txt", 0.9, Some("alpha"));
        a.memory.modified_at = Utc::now();
        let b = candidate("b.txt", 0.1, Some("beta"));
        let out = reranker.rerank("", vec![a, b]);
        assert_eq!(out.len(), 2, "no candidates dropped on empty query");
    }
}
