//! Result reranking.
//!
//! This is the second-stage precision pass of the "elite retrieval" pipeline.
//! It runs AFTER Reciprocal Rank Fusion (RRF) merges the semantic and keyword
//! arms, and BEFORE the final truncation to the caller's `limit`:
//!
//! ```text
//! semantic (Qdrant, chunk roll-up) ⨁ FTS5 BM25
//!     → reciprocal_rank_fusion            (first-stage recall / coarse order)
//!     → rerank_stage (this module)        (second-stage precision re-order)
//!     → truncate(limit)                   (final results)
//! ```
//!
//! First-stage fusion maximises *recall* cheaply: RRF is rank-based and
//! scale-free, so it is good at getting the right documents into the candidate
//! set, but its ordering ignores strong precision signals (an exact title hit
//! is worth more than being ranked 3rd in both arms). The reranker re-scores a
//! bounded window of top candidates using those signals and reorders them.
//!
//! Two implementations share the [`Reranker`] trait seam:
//!
//! * [`HeuristicReranker`] — the DEFAULT. Zero new dependencies, fully offline
//!   and deterministic. It re-scores using signals already attached to each
//!   candidate (fused score, lexical title/filename/tag matches, phrase vs
//!   scattered-term matches, favorite, recency). This is a real precision win
//!   over raw RRF order with no extra infrastructure.
//! * `OnnxReranker` — an OPTIONAL, feature-gated scaffold (see the bottom of
//!   this file) for a future cross-encoder model. It is NOT wired to a model and
//!   adds NO dependency to the default build.

use crate::models::MemorySearchResult;
use chrono::Utc;
use std::cmp::Ordering;

/// Reorders a set of already-retrieved candidates to improve precision.
///
/// Object-safe by design: no generic methods, no `Self`-by-value returns, so a
/// `&dyn Reranker` / `Arc<dyn Reranker>` can be stored on the searcher and a
/// different backend (heuristic today, a cross-encoder later) dropped in without
/// touching the pipeline. Implementations must be cheap enough to run on every
/// hybrid query over the top-N candidates.
pub trait Reranker: Send + Sync {
    /// Re-order `candidates` for `query`, best-first. Implementations may also
    /// overwrite each result's `score` with the new rerank score; the caller
    /// treats the returned order (not the score magnitude) as authoritative.
    fn rerank(&self, query: &str, candidates: Vec<MemorySearchResult>) -> Vec<MemorySearchResult>;

    /// Short name for logging / telemetry.
    fn name(&self) -> &'static str {
        "reranker"
    }
}

/// Transparent, documented weights for [`HeuristicReranker`].
///
/// The final per-candidate score is a simple weighted sum (see
/// [`HeuristicReranker::heuristic_score`]):
///
/// ```text
/// score = base   * (fused_score / max_fused_score)   // normalised recall signal, ~[0,1]
///       + exact      if query == title/filename/stem   // strongest lexical signal
///       + phrase     if full query is a substring hit   // contiguous match (implied by exact)
///       + coverage * (matched_terms / total_terms)      // partial / scattered-term match
///       + tag        if a tag equals a query term/query  // curated label match
///       + favorite   if the file is starred              // user-intent tie-break
///       + recency    if modified within 7d / 30d         // freshness tie-break
/// ```
///
/// Rationale for the magnitudes:
/// * `base = 1.0` keeps the fused recall order as the backbone, but
/// * `exact = 3.0` and `phrase = 1.5` are large enough to pull a strongly
///   lexically-matching candidate above one that merely ranked higher in RRF —
///   the whole point of a precision rerank.
/// * `coverage = 1.0` rewards matching more query terms, but a *contiguous*
///   phrase match (exact + phrase) always beats the same terms scattered.
/// * `favorite` / `recency` are deliberately small: they only decide ties
///   between otherwise-equivalent candidates.
#[derive(Debug, Clone, Copy)]
pub struct RerankWeights {
    pub base: f32,
    pub exact: f32,
    pub phrase: f32,
    pub coverage: f32,
    pub tag: f32,
    pub favorite: f32,
    pub recent_week: f32,
    pub recent_month: f32,
}

impl Default for RerankWeights {
    fn default() -> Self {
        Self {
            base: 1.0,
            exact: 3.0,
            phrase: 1.5,
            coverage: 1.0,
            tag: 0.8,
            favorite: 0.3,
            recent_week: 0.2,
            recent_month: 0.1,
        }
    }
}

/// The default, zero-dependency reranker.
///
/// Re-scores candidates using signals already present on each
/// [`MemorySearchResult`] — no model, no network, fully deterministic.
#[derive(Debug, Clone, Default)]
pub struct HeuristicReranker {
    weights: RerankWeights,
}

impl HeuristicReranker {
    /// Reranker with the documented default [`RerankWeights`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Reranker with custom weights.
    pub fn with_weights(weights: RerankWeights) -> Self {
        Self { weights }
    }

    /// Weighted heuristic score for a single candidate. `max_base` is the
    /// largest fused score in the current candidate window (for normalisation),
    /// `now` is captured once per rerank call for deterministic recency.
    fn heuristic_score(
        &self,
        query_lower: &str,
        terms: &[&str],
        c: &MemorySearchResult,
        max_base: f32,
        now: chrono::DateTime<Utc>,
    ) -> f32 {
        let w = &self.weights;

        // Normalised recall backbone (fused RRF score scaled to ~[0,1]).
        let mut score = w.base * (c.score / max_base);

        // Favorite + recency are always available as mild tie-breaks, even for
        // an empty query.
        if c.memory.is_favorite {
            score += w.favorite;
        }
        let age_days = (now - c.memory.modified_at).num_days();
        if age_days <= 7 {
            score += w.recent_week;
        } else if age_days <= 30 {
            score += w.recent_month;
        }

        if query_lower.is_empty() {
            return score;
        }

        let title = c
            .memory
            .metadata
            .title
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let filename = c
            .memory
            .path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = c
            .memory
            .path
            .file_stem()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let preview = c
            .memory
            .metadata
            .text_preview
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let tags: Vec<String> = c
            .memory
            .tags
            .iter()
            .map(|t| t.name.to_lowercase())
            .collect();

        // Exact match: the query IS the title, filename, or filename stem.
        if title == query_lower || filename == query_lower || stem == query_lower {
            score += w.exact;
        }

        // Phrase match: the full query appears as a contiguous substring. Exact
        // matches also satisfy this (monotonic: exact is strictly best), and a
        // contiguous phrase always outscores the same terms scattered.
        if title.contains(query_lower)
            || filename.contains(query_lower)
            || preview.contains(query_lower)
        {
            score += w.phrase;
        }

        // Term coverage: fraction of query terms found anywhere in the candidate.
        if !terms.is_empty() {
            let haystack = format!("{} {} {} {}", title, filename, preview, tags.join(" "));
            let hits = terms.iter().filter(|t| haystack.contains(**t)).count();
            score += w.coverage * (hits as f32 / terms.len() as f32);
        }

        // Tag match: a curated tag equals the whole query or one of its terms.
        let tag_match = tags
            .iter()
            .any(|tag| tag == query_lower || terms.iter().any(|t| tag == t));
        if tag_match {
            score += w.tag;
        }

        score
    }
}

impl Reranker for HeuristicReranker {
    fn rerank(&self, query: &str, candidates: Vec<MemorySearchResult>) -> Vec<MemorySearchResult> {
        if candidates.len() <= 1 {
            return candidates;
        }

        let query_lower = query.trim().to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();

        // Normalise the fused score against the best in this window. Guard a
        // zero/negative max so division is always well-defined.
        let max_base = candidates
            .iter()
            .map(|c| c.score)
            .fold(0.0_f32, f32::max)
            .max(f32::EPSILON);

        let now = Utc::now();

        let mut scored: Vec<(f32, MemorySearchResult)> = candidates
            .into_iter()
            .map(|c| {
                let s = self.heuristic_score(&query_lower, &terms, &c, max_base, now);
                (s, c)
            })
            .collect();

        // Stable sort by descending rerank score: candidates that tie keep their
        // incoming (fused) relative order, so equal scores never reshuffle.
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

        scored
            .into_iter()
            .map(|(s, mut c)| {
                c.score = s;
                c
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "heuristic"
    }
}

/// Apply the (gated) rerank stage to a fused, best-first result list.
///
/// This is the single entry point the search pipeline calls, kept as a free
/// function so it is exhaustively testable offline:
///
/// * `enabled == false` → identity (returns `results` untouched, i.e. today's
///   RRF order).
/// * only the top `rerank_candidates` (by incoming fused order) are reranked;
///   the tail keeps its RRF order and is appended unchanged. The combined list
///   is NOT re-sorted, so the head (reranked) always precedes the tail.
pub fn rerank_stage(
    reranker: &dyn Reranker,
    query: &str,
    mut results: Vec<MemorySearchResult>,
    enabled: bool,
    rerank_candidates: usize,
) -> Vec<MemorySearchResult> {
    if !enabled || rerank_candidates == 0 || results.len() <= 1 {
        return results;
    }

    let cutoff = rerank_candidates.min(results.len());
    let tail = results.split_off(cutoff);
    let mut head = reranker.rerank(query, results);
    head.extend(tail);
    head
}

// ---------------------------------------------------------------------------
// OPTIONAL ONNX cross-encoder scaffold (NOT in the default build).
//
// TODO(reranker-onnx): wire a real cross-encoder reranker (e.g. BAAI/bge-reranker
// or mixedbread-ai/mxbai-rerank) behind the `reranker-onnx` cargo feature. The
// intended shape:
//   * optional deps (declared in Cargo.toml ONLY under this feature):
//       ort        — ONNX Runtime bindings for inference
//       tokenizers — HF tokenizer for the (query, passage) pair input
//   * load a quantised .onnx model + tokenizer.json at startup (lazily, with a
//     documented download/cache step — NO model is bundled or downloaded today),
//   * for each candidate, tokenise the (query, candidate-text) pair, run the
//     cross-encoder to get a relevance logit, sort by it, and (optionally) blend
//     with the fused score the same way `HeuristicReranker` blends `base`.
//
// The feature is intentionally EMPTY in Cargo.toml for now (adds no deps); this
// stub compiles only when the feature is enabled and is an identity reranker so
// the seam is exercised without pulling in any model or runtime.
// ---------------------------------------------------------------------------

/// Scaffold for a future ONNX cross-encoder reranker. See the module TODO.
/// Currently an identity reranker; not wired to any model.
#[cfg(feature = "reranker-onnx")]
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct OnnxReranker {
    // TODO(reranker-onnx): session: ort::Session, tokenizer: tokenizers::Tokenizer, ...
}

#[cfg(feature = "reranker-onnx")]
impl Reranker for OnnxReranker {
    fn rerank(&self, _query: &str, candidates: Vec<MemorySearchResult>) -> Vec<MemorySearchResult> {
        // TODO(reranker-onnx): real cross-encoder scoring. Scaffold = identity.
        candidates
    }

    fn name(&self) -> &'static str {
        "onnx-cross-encoder (scaffold)"
    }
}
