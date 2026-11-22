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
// OPTIONAL ONNX cross-encoder reranker (NOT in the default build).
//
// Gated behind the `reranker-onnx` cargo feature, which pulls in the OPTIONAL
// `ort` (ONNX Runtime) + `tokenizers` deps. The default build references none
// of this, so it compiles no `ort-sys` (which panics on Windows CI).
//
// Unlike `HeuristicReranker`, this is a true cross-encoder: it tokenises each
// (query, candidate-text) PAIR and runs a transformer that jointly attends over
// both sides, emitting a single relevance logit per pair. That is strictly more
// powerful than the bag-of-signals heuristic, at the cost of a model download +
// per-candidate inference.
//
// NOTE ON RUNTIME VALIDATION: the model/tokenizer download and ONNX inference
// cannot be exercised in CI (no model is bundled or fetched there). This code is
// COMPILE-verified under `--features reranker-onnx`; the live download + inference
// path runs only on a developer/user machine. Every failure mode (download, file
// I/O, model load, tokenisation, inference) degrades gracefully to the offline
// `HeuristicReranker` and never hard-fails search.
// ---------------------------------------------------------------------------

#[cfg(feature = "reranker-onnx")]
pub use self::onnx::{OnnxReranker, OnnxRerankerConfig};

#[cfg(feature = "reranker-onnx")]
mod onnx {
    use super::{HeuristicReranker, MemorySearchResult, Reranker};
    use crate::error::{HippoError, Result};
    use std::borrow::Cow;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    use ort::session::builder::GraphOptimizationLevel;
    use ort::session::{Session, SessionInputValue};
    use ort::value::Tensor;
    use tokenizers::Tokenizer;

    /// Max characters of candidate text fed to the tokenizer before tokenisation.
    /// Keeps tokenisation cheap; the token stream is additionally capped to
    /// `max_length` below.
    const MAX_PASSAGE_CHARS: usize = 2_000;

    /// Which cross-encoder to load, and where to fetch it from.
    ///
    /// Defaults to `BAAI/bge-reranker-base` (XLM-RoBERTa based: needs only
    /// `input_ids` + `attention_mask`, no `token_type_ids`), served from the
    /// Xenova ONNX mirror which hosts `model.onnx` + `tokenizer.json`. Both URLs
    /// are overridable so a smaller model (e.g. `mxbai-rerank-xsmall`) or a
    /// locally-staged model can be used without code changes.
    ///
    /// The URLs are NOT validated in CI; they are resolved only on a live first
    /// use. If either download fails the reranker falls back to the heuristic.
    #[derive(Debug, Clone)]
    pub struct OnnxRerankerConfig {
        /// Stable identifier used for the on-disk cache directory and logging.
        pub model_id: String,
        /// Full URL to the ONNX model file.
        pub onnx_url: String,
        /// Full URL to the HuggingFace `tokenizer.json`.
        pub tokenizer_url: String,
        /// Directory under which `<model_id>/` is cached. Usually the app cache dir.
        pub cache_dir: PathBuf,
        /// Maximum token sequence length fed to the model (pair truncated to fit).
        pub max_length: usize,
    }

    impl OnnxRerankerConfig {
        /// Default `bge-reranker-base` config, caching under `cache_dir`.
        pub fn bge_reranker_base(cache_dir: PathBuf) -> Self {
            Self {
                model_id: "bge-reranker-base".to_string(),
                onnx_url:
                    "https://huggingface.co/Xenova/bge-reranker-base/resolve/main/onnx/model.onnx"
                        .to_string(),
                tokenizer_url:
                    "https://huggingface.co/Xenova/bge-reranker-base/resolve/main/tokenizer.json"
                        .to_string(),
                cache_dir,
                max_length: 512,
            }
        }

        /// Resolve a config from a model id / optional override, caching under `cache_dir`.
        ///
        /// Currently only the default `bge-reranker-base` id is recognised; any
        /// other id reuses the default URLs but caches under that id so a caller
        /// can pre-stage a `model.onnx` + `tokenizer.json` there to override it.
        pub fn resolve(model_id: Option<&str>, cache_dir: PathBuf) -> Self {
            let mut cfg = Self::bge_reranker_base(cache_dir);
            if let Some(id) = model_id {
                if !id.is_empty() && id != cfg.model_id {
                    cfg.model_id = id.to_string();
                }
            }
            cfg
        }

        fn model_dir(&self) -> PathBuf {
            self.cache_dir.join("reranker").join(&self.model_id)
        }
    }

    /// Real ONNX cross-encoder reranker.
    ///
    /// Holds a loaded `ort::Session` (behind a `Mutex`, because `Session::run`
    /// needs `&mut self`) and the HF tokenizer. `rerank` scores each candidate
    /// by the model's relevance logit for the `(query, candidate)` pair. ANY
    /// runtime error falls back to the embedded [`HeuristicReranker`].
    pub struct OnnxReranker {
        session: Mutex<Session>,
        tokenizer: Tokenizer,
        /// Whether the loaded model declares a `token_type_ids` input (BERT/DeBERTa
        /// family). XLM-RoBERTa rerankers (bge-*) do not.
        needs_token_type_ids: bool,
        max_length: usize,
        model_id: String,
        /// Offline fallback used when inference fails at runtime.
        fallback: HeuristicReranker,
        /// One-shot guard so a runtime failure logs a single warning, not one per query.
        warned: AtomicBool,
    }

    impl OnnxReranker {
        /// Load (downloading + caching on first use) the model named by `config`.
        ///
        /// Mirrors the Qdrant manager's auto-download: files are fetched to
        /// `<cache_dir>/reranker/<model_id>/` once and reused thereafter. Async
        /// because it performs network I/O; call it from the (async) searcher
        /// setup. On any failure it returns `Err`, and the caller is expected to
        /// fall back to the heuristic reranker.
        pub async fn load(config: OnnxRerankerConfig) -> Result<Self> {
            let dir = config.model_dir();
            std::fs::create_dir_all(&dir)?;

            let onnx_path = dir.join("model.onnx");
            let tokenizer_path = dir.join("tokenizer.json");

            ensure_downloaded(&onnx_path, &config.onnx_url, "ONNX model").await?;
            ensure_downloaded(&tokenizer_path, &config.tokenizer_url, "tokenizer").await?;

            let tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| HippoError::Other(format!("Failed to load tokenizer: {e}")))?;

            let session = Session::builder()
                .map_err(onnx_err)?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(onnx_err)?
                .with_intra_threads(1)
                .map_err(onnx_err)?
                .commit_from_file(&onnx_path)
                .map_err(onnx_err)?;

            let needs_token_type_ids = session.inputs.iter().any(|i| i.name == "token_type_ids");

            tracing::info!(
                model = %config.model_id,
                token_type_ids = needs_token_type_ids,
                "Loaded ONNX cross-encoder reranker"
            );

            Ok(Self {
                session: Mutex::new(session),
                tokenizer,
                needs_token_type_ids,
                max_length: config.max_length.max(8),
                model_id: config.model_id,
                fallback: HeuristicReranker::new(),
                warned: AtomicBool::new(false),
            })
        }

        /// Score every candidate with the cross-encoder. Returns `None` on the
        /// first error so the caller can fall back wholesale (keeps ordering
        /// deterministic instead of mixing scored and unscored candidates).
        fn score_all(&self, query: &str, candidates: &[MemorySearchResult]) -> Option<Vec<f32>> {
            let mut session = self.session.lock().ok()?;
            let mut scores = Vec::with_capacity(candidates.len());
            for c in candidates {
                let text = candidate_text(c);
                let logit = self.score_pair(&mut session, query, &text)?;
                scores.push(logit);
            }
            Some(scores)
        }

        /// Run one `(query, passage)` pair through the model, returning its logit.
        fn score_pair(&self, session: &mut Session, query: &str, passage: &str) -> Option<f32> {
            let encoding = self.tokenizer.encode((query, passage), true).ok()?;

            let mut ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
            let mut mask: Vec<i64> = encoding
                .get_attention_mask()
                .iter()
                .map(|&x| x as i64)
                .collect();
            let mut type_ids: Vec<i64> =
                encoding.get_type_ids().iter().map(|&x| x as i64).collect();

            // Hard cap at max_length. (Simple tail truncation; a production path
            // may prefer sequence-aware truncation, but this keeps inference safe
            // for over-long passages.)
            ids.truncate(self.max_length);
            mask.truncate(self.max_length);
            type_ids.truncate(self.max_length);

            let seq = ids.len() as i64;
            let shape = vec![1_i64, seq];

            let mut feeds: Vec<(Cow<str>, SessionInputValue)> = Vec::with_capacity(3);
            feeds.push((
                Cow::from("input_ids"),
                SessionInputValue::from(Tensor::from_array((shape.clone(), ids)).ok()?),
            ));
            feeds.push((
                Cow::from("attention_mask"),
                SessionInputValue::from(Tensor::from_array((shape.clone(), mask)).ok()?),
            ));
            if self.needs_token_type_ids {
                feeds.push((
                    Cow::from("token_type_ids"),
                    SessionInputValue::from(Tensor::from_array((shape, type_ids)).ok()?),
                ));
            }

            let outputs = session.run(feeds).ok()?;
            let (_shape, data) = outputs[0].try_extract_tensor::<f32>().ok()?;
            // Cross-encoder rerankers emit a single relevance logit (shape [1,1]);
            // for a [1,N] head the first element is the relevance class.
            data.first().copied()
        }

        /// Log a single warning the first time we fall back at runtime.
        fn warn_once(&self, reason: &str) {
            if !self.warned.swap(true, Ordering::Relaxed) {
                tracing::warn!(
                    model = %self.model_id,
                    "ONNX reranker unavailable ({reason}); falling back to heuristic reranker for this session"
                );
            }
        }
    }

    impl Reranker for OnnxReranker {
        fn rerank(
            &self,
            query: &str,
            candidates: Vec<MemorySearchResult>,
        ) -> Vec<MemorySearchResult> {
            if candidates.len() <= 1 {
                return candidates;
            }

            let Some(scores) = self.score_all(query, &candidates) else {
                self.warn_once("inference failed");
                return self.fallback.rerank(query, candidates);
            };

            let mut scored: Vec<(f32, MemorySearchResult)> =
                scores.into_iter().zip(candidates).collect();
            // Stable sort, descending by logit: ties keep incoming fused order.
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored
                .into_iter()
                .map(|(s, mut c)| {
                    c.score = s;
                    c
                })
                .collect()
        }

        fn name(&self) -> &'static str {
            "onnx-cross-encoder"
        }
    }

    /// Text side of the `(query, text)` pair for a candidate: prefer title +
    /// preview, fall back to the filename. Bounded to `MAX_PASSAGE_CHARS`.
    fn candidate_text(c: &MemorySearchResult) -> String {
        let title = c.memory.metadata.title.as_deref().unwrap_or("");
        let preview = c.memory.metadata.text_preview.as_deref().unwrap_or("");
        let filename = c
            .memory
            .path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();

        let mut text = String::new();
        if !title.is_empty() {
            text.push_str(title);
        }
        if !preview.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(preview);
        }
        if text.is_empty() {
            text = filename;
        }

        if text.len() > MAX_PASSAGE_CHARS {
            // Truncate on a char boundary.
            let mut end = MAX_PASSAGE_CHARS;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        text
    }

    /// Download `url` to `dest` if it is not already cached. Mirrors the Qdrant
    /// manager's download step (reqwest GET → bytes → write).
    async fn ensure_downloaded(dest: &Path, url: &str, what: &str) -> Result<()> {
        if dest.exists() {
            tracing::debug!(path = %dest.display(), "Reranker {what} already cached");
            return Ok(());
        }
        tracing::info!(url = %url, "Downloading reranker {what}");

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to download {what}: {e}")))?;
        if !response.status().is_success() {
            return Err(HippoError::Other(format!(
                "Failed to download {what}: HTTP {}",
                response.status()
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to download {what}: {e}")))?;

        // Write atomically via a temp file next to the destination.
        let tmp = dest.with_extension("partial");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, dest)?;
        Ok(())
    }

    fn onnx_err(e: ort::Error) -> HippoError {
        HippoError::Other(format!("ONNX runtime error: {e}"))
    }
}
