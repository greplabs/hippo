//! Search engine combining SQL search, fuzzy matching, and semantic search

mod advanced_filter;

pub use advanced_filter::{
    AdvancedFilter, ExtensionFilter, FilterBuilder, MatchMode, MetadataMatch,
};

use crate::embeddings::Embedder;
use crate::error::Result;
use crate::models::*;
use crate::storage::Storage;
use crate::HippoConfig;
use chrono::{Duration, Utc};
use regex::Regex;
use std::sync::Arc;
use tracing::warn;

/// Configuration for hybrid search scoring weights
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for semantic similarity score (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Weight for keyword matching score (0.0 to 1.0)
    pub keyword_weight: f32,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

/// Calculate cosine similarity between two embedding vectors
/// Returns a value between -1.0 and 1.0, where 1.0 means identical direction
pub fn semantic_score(query_embedding: &[f32], doc_embedding: &[f32]) -> f32 {
    // Handle edge cases
    if query_embedding.is_empty() || doc_embedding.is_empty() {
        return 0.0;
    }

    if query_embedding.len() != doc_embedding.len() {
        return 0.0;
    }

    // Calculate dot product and magnitudes
    let mut dot_product = 0.0;
    let mut query_magnitude = 0.0;
    let mut doc_magnitude = 0.0;

    for i in 0..query_embedding.len() {
        dot_product += query_embedding[i] * doc_embedding[i];
        query_magnitude += query_embedding[i] * query_embedding[i];
        doc_magnitude += doc_embedding[i] * doc_embedding[i];
    }

    // Handle zero magnitude vectors
    if query_magnitude == 0.0 || doc_magnitude == 0.0 {
        return 0.0;
    }

    // Calculate cosine similarity
    let similarity = dot_product / (query_magnitude.sqrt() * doc_magnitude.sqrt());

    // Clamp to [-1.0, 1.0] to handle floating point errors
    similarity.max(-1.0).min(1.0)
}

/// Calculate fuzzy match score using Levenshtein distance
/// Returns a value between 0.0 and 1.0, where 1.0 means exact match
pub fn fuzzy_match(query: &str, text: &str) -> f32 {
    if query.is_empty() && text.is_empty() {
        return 1.0;
    }
    if query.is_empty() || text.is_empty() {
        return 0.0;
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    // Exact match
    if query_lower == text_lower {
        return 1.0;
    }

    // Contains match (high score)
    if text_lower.contains(&query_lower) {
        let ratio = query_lower.len() as f32 / text_lower.len() as f32;
        return 0.8 + (0.2 * ratio);
    }

    // Calculate Levenshtein distance
    let distance = levenshtein_distance(&query_lower, &text_lower);
    let max_len = query_lower.len().max(text_lower.len());

    // Convert distance to similarity score (0.0 to 1.0)
    let similarity = 1.0 - (distance as f32 / max_len as f32);
    similarity.max(0.0)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    // Quick optimizations
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Create distance matrix
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first column
    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }

    // Initialize first row
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    // Fill in the rest of the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };

            matrix[i][j] = (matrix[i - 1][j] + 1)           // deletion
                .min(matrix[i][j - 1] + 1)                   // insertion
                .min(matrix[i - 1][j - 1] + cost);           // substitution
        }
    }

    matrix[len1][len2]
}

/// Find best fuzzy match in a text, returning the score and matched substring
pub fn fuzzy_find_best_match(query: &str, text: &str) -> (f32, Option<String>) {
    if query.is_empty() || text.is_empty() {
        return (0.0, None);
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    // Check for exact match first
    if text_lower.contains(&query_lower) {
        return (1.0, Some(query.to_string()));
    }

    // Split text into words and find best matching word
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut best_score = 0.0f32;
    let mut best_match = None;

    for word in words {
        let score = fuzzy_match(&query_lower, &word.to_lowercase());
        if score > best_score {
            best_score = score;
            best_match = Some(word.to_string());
        }
    }

    (best_score, best_match)
}

pub struct Searcher {
    storage: Arc<Storage>,
    embedder: Arc<Embedder>,
    hybrid_config: HybridSearchConfig,
}

impl Searcher {
    pub async fn new(
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        _config: &HippoConfig,
    ) -> Result<Self> {
        Ok(Self {
            storage,
            embedder,
            hybrid_config: HybridSearchConfig::default(),
        })
    }

    /// Create a new Searcher with custom hybrid search configuration
    pub async fn with_hybrid_config(
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        _config: &HippoConfig,
        hybrid_config: HybridSearchConfig,
    ) -> Result<Self> {
        Ok(Self {
            storage,
            embedder,
            hybrid_config,
        })
    }

    /// Perform a search with the given query - uses SQL for performance
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let limit = if query.limit > 0 { query.limit } else { 100 };
        let offset = query.offset;

        // Extract tag names for filtering
        let tag_filters: Vec<String> = query
            .tags
            .iter()
            .filter(|t| matches!(t.mode, TagFilterMode::Include))
            .map(|t| t.tag.clone())
            .collect();

        // Extract kind filter
        let kind_filter = if !query.kinds.is_empty() {
            Some(Self::kind_name(&query.kinds[0]))
        } else {
            None
        };

        // Use SQL-based search
        let memories = self
            .storage
            .search_with_tags(
                query.text.as_deref(),
                &tag_filters,
                kind_filter,
                limit,
                offset,
            )
            .await?;

        // Get total count
        let total_count = self
            .storage
            .count_search_results(query.text.as_deref(), &tag_filters, kind_filter)
            .await?;

        // Apply exclusion filters and date range in memory (fast for small result set)
        let exclude_tags: Vec<String> = query
            .tags
            .iter()
            .filter(|t| matches!(t.mode, TagFilterMode::Exclude))
            .map(|t| t.tag.to_lowercase())
            .collect();

        let mut results: Vec<MemorySearchResult> = memories
            .into_iter()
            .filter(|memory| {
                // Exclude tag filter
                if !exclude_tags.is_empty() {
                    for tag in &memory.tags {
                        if exclude_tags.contains(&tag.name.to_lowercase()) {
                            return false;
                        }
                    }
                }

                // Date range filter
                if let Some(ref date_range) = query.date_range {
                    if let Some(start) = date_range.start {
                        if memory.modified_at < start {
                            return false;
                        }
                    }
                    if let Some(end) = date_range.end {
                        if memory.modified_at > end {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|memory| {
                let mut score = 1.0;
                let mut highlights = Vec::new();

                // Calculate relevance score
                if let Some(ref text) = query.text {
                    let text_lower = text.to_lowercase();

                    // Title match
                    if let Some(ref title) = memory.metadata.title {
                        if title.to_lowercase().contains(&text_lower) {
                            score += 10.0;
                            if title.to_lowercase().starts_with(&text_lower) {
                                score += 5.0;
                            }
                            highlights.push(Highlight {
                                field: "title".to_string(),
                                snippet: title.clone(),
                            });
                        }
                    }

                    // Filename match
                    let filename = memory
                        .path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if filename.to_lowercase().contains(&text_lower) {
                        score += 8.0;
                        if filename.to_lowercase().starts_with(&text_lower) {
                            score += 4.0;
                        }
                        highlights.push(Highlight {
                            field: "filename".to_string(),
                            snippet: filename,
                        });
                    }

                    // Tag match
                    for tag in &memory.tags {
                        if tag.name.to_lowercase().contains(&text_lower) {
                            score += 7.0;
                            highlights.push(Highlight {
                                field: "tag".to_string(),
                                snippet: tag.name.clone(),
                            });
                        }
                    }
                }

                // Recency boost
                let age_days = (chrono::Utc::now() - memory.modified_at).num_days();
                if age_days < 7 {
                    score *= 1.1;
                } else if age_days < 30 {
                    score *= 1.05;
                }

                MemorySearchResult {
                    memory,
                    score,
                    highlights,
                }
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Get tag suggestions
        let suggested_tags = if let Some(ref text) = query.text {
            self.suggest_tags(text).await?
        } else {
            vec![]
        };

        Ok(SearchResults {
            memories: results,
            total_count,
            suggested_tags,
            clusters: vec![],
        })
    }

    /// Perform semantic search using embeddings (Qdrant or SQLite fallback)
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        // Generate query embedding
        let query_embedding = match self.embedder.embed_query(query).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!(
                    "Failed to generate query embedding: {}, falling back to text search",
                    e
                );
                return self
                    .search(SearchQuery {
                        text: Some(query.to_string()),
                        limit,
                        ..Default::default()
                    })
                    .await;
            }
        };

        // Use Qdrant-backed search (with SQLite fallback)
        let scored = self
            .storage
            .search_vectors(query_embedding, None, limit)
            .await?;

        if scored.is_empty() {
            // No results, fall back to text search
            return self
                .search(SearchQuery {
                    text: Some(query.to_string()),
                    limit,
                    ..Default::default()
                })
                .await;
        }

        // Fetch full memories for top results
        let mut results = Vec::new();
        for (id, similarity) in scored {
            if let Some(memory) = self.storage.get_memory(id).await? {
                results.push(MemorySearchResult {
                    memory,
                    score: similarity * 100.0, // Scale to percentage-like score
                    highlights: vec![Highlight {
                        field: "semantic".to_string(),
                        snippet: format!("{:.1}% similar", similarity * 100.0),
                    }],
                });
            }
        }

        let total = results.len();
        Ok(SearchResults {
            memories: results,
            total_count: total,
            suggested_tags: vec![],
            clusters: vec![],
        })
    }

    /// Hybrid search combining semantic and keyword scoring with configurable weights
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        // Use configured weights
        let semantic_weight = self.hybrid_config.semantic_weight;
        let keyword_weight = self.hybrid_config.keyword_weight;

        // Get semantic results
        let semantic_results = self.semantic_search(query, limit * 2).await?;

        // Get keyword results
        let keyword_results = self
            .search(SearchQuery {
                text: Some(query.to_string()),
                limit: limit * 2,
                ..Default::default()
            })
            .await?;

        // Combine and score
        let mut combined: std::collections::HashMap<MemoryId, (Memory, f32, Vec<Highlight>)> =
            std::collections::HashMap::new();

        // Add semantic results with weighted score
        for result in semantic_results.memories {
            combined.insert(
                result.memory.id,
                (
                    result.memory,
                    result.score * semantic_weight,
                    result.highlights,
                ),
            );
        }

        // Add/update keyword results with weighted score
        for result in keyword_results.memories {
            combined
                .entry(result.memory.id)
                .and_modify(|(_, score, highlights)| {
                    *score += result.score * keyword_weight;
                    highlights.extend(result.highlights.clone());
                })
                .or_insert((
                    result.memory,
                    result.score * keyword_weight,
                    result.highlights,
                ));
        }

        // Convert back to results and sort
        let mut results: Vec<MemorySearchResult> = combined
            .into_iter()
            .map(|(_, (memory, score, highlights))| MemorySearchResult {
                memory,
                score,
                highlights,
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        let total = results.len();
        Ok(SearchResults {
            memories: results,
            total_count: total,
            suggested_tags: vec![],
            clusters: vec![],
        })
    }

    /// Fuzzy search for typo-tolerant matching
    pub async fn fuzzy_search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        // First try exact search
        let exact_results = self
            .search(SearchQuery {
                text: Some(query.to_string()),
                limit,
                ..Default::default()
            })
            .await?;

        if !exact_results.memories.is_empty() {
            return Ok(exact_results);
        }

        // No exact matches - try fuzzy matching
        // Generate variations of the query (simple typo correction)
        let variations = self.generate_fuzzy_variations(query);

        let mut all_results = Vec::new();
        for variation in variations {
            let results = self
                .search(SearchQuery {
                    text: Some(variation),
                    limit: limit / 2,
                    ..Default::default()
                })
                .await?;
            all_results.extend(results.memories);
        }

        // Deduplicate by ID
        all_results.sort_by(|a, b| a.memory.id.cmp(&b.memory.id));
        all_results.dedup_by(|a, b| a.memory.id == b.memory.id);

        // Re-sort by score
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        let total = all_results.len();
        Ok(SearchResults {
            memories: all_results,
            total_count: total,
            suggested_tags: vec![],
            clusters: vec![],
        })
    }

    /// Generate fuzzy variations of a query for typo tolerance
    fn generate_fuzzy_variations(&self, query: &str) -> Vec<String> {
        let mut variations = Vec::new();
        let chars: Vec<char> = query.chars().collect();

        // Skip characters (handle missing letters)
        for i in 0..chars.len() {
            let variation: String = chars
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, c)| *c)
                .collect();
            if variation.len() >= 2 {
                variations.push(variation);
            }
        }

        // Swap adjacent characters (handle transpositions)
        for i in 0..chars.len().saturating_sub(1) {
            let mut swapped = chars.clone();
            swapped.swap(i, i + 1);
            variations.push(swapped.into_iter().collect());
        }

        // Common character substitutions
        let substitutions = [
            ('a', 'e'),
            ('e', 'a'),
            ('i', 'y'),
            ('y', 'i'),
            ('o', 'u'),
            ('u', 'o'),
            ('c', 'k'),
            ('k', 'c'),
        ];

        for (from, to) in substitutions {
            if query.contains(from) {
                variations.push(query.replace(from, &to.to_string()));
            }
        }

        variations
    }

    /// Get a string name for a MemoryKind variant
    fn kind_name(kind: &MemoryKind) -> &'static str {
        match kind {
            MemoryKind::Image { .. } => "image",
            MemoryKind::Video { .. } => "video",
            MemoryKind::Audio { .. } => "audio",
            MemoryKind::Document { .. } => "document",
            MemoryKind::Spreadsheet { .. } => "spreadsheet",
            MemoryKind::Presentation { .. } => "presentation",
            MemoryKind::Code { .. } => "code",
            MemoryKind::Archive { .. } => "archive",
            MemoryKind::Database => "database",
            MemoryKind::Folder => "folder",
            MemoryKind::Unknown => "unknown",
        }
    }

    /// Suggest tags based on search text with fuzzy matching
    pub async fn suggest_tags(&self, text: &str) -> Result<Vec<String>> {
        let all_tags = self.storage.list_tags().await?;
        let text_lower = text.to_lowercase();

        // Score and sort tags with fuzzy matching
        let mut scored_tags: Vec<(String, u64, f32)> = all_tags
            .into_iter()
            .filter_map(|(name, count)| {
                let name_lower = name.to_lowercase();

                // Calculate base score based on match type
                let base_score = if name_lower == text_lower {
                    100.0 // Exact match
                } else if name_lower.starts_with(&text_lower) {
                    80.0 // Prefix match
                } else if name_lower.contains(&text_lower) {
                    50.0 // Contains match
                } else if name_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| word.starts_with(&text_lower))
                {
                    40.0 // Word boundary match
                } else {
                    // Try fuzzy match
                    let similarity = self.string_similarity(&text_lower, &name_lower);
                    if similarity > 0.6 {
                        similarity * 30.0 // Fuzzy match
                    } else {
                        return None;
                    }
                };

                // Boost by usage count
                let score = base_score + (count as f32).ln().max(0.0);

                Some((name, count, score))
            })
            .collect();

        // Sort by score descending
        scored_tags.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_tags
            .into_iter()
            .take(10)
            .map(|(name, _, _)| name)
            .collect())
    }

    /// Calculate string similarity (Jaro-Winkler-like)
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1 == s2 {
            return 1.0;
        }

        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // Simple character overlap ratio
        let chars1: std::collections::HashSet<char> = s1.chars().collect();
        let chars2: std::collections::HashSet<char> = s2.chars().collect();
        let intersection = chars1.intersection(&chars2).count();
        let union = chars1.union(&chars2).count();

        if union == 0 {
            return 0.0;
        }

        let jaccard = intersection as f32 / union as f32;

        // Prefix bonus
        let common_prefix = s1
            .chars()
            .zip(s2.chars())
            .take_while(|(a, b)| a == b)
            .count();
        let prefix_bonus = (common_prefix.min(4) as f32) * 0.1;

        (jaccard + prefix_bonus).min(1.0)
    }

    /// Parse natural language query into structured search filters
    pub fn parse_natural_query(&self, query: &str) -> Result<ParsedQuery> {
        let query_lower = query.to_lowercase();
        let mut keywords = query.to_string();
        let mut file_types = Vec::new();
        let mut date_range = None;
        let mut interpretations = Vec::new();

        // Extract file types
        let type_patterns = [
            (
                r"\b(image|images|photo|photos|picture|pictures|pic|pics)\b",
                "image",
            ),
            (r"\b(video|videos|movie|movies|clip|clips)\b", "video"),
            (r"\b(audio|music|song|songs|sound|sounds)\b", "audio"),
            (
                r"\b(document|documents|doc|docs|pdf|pdfs|text|texts)\b",
                "document",
            ),
            (r"\b(code|source|script|scripts|program|programs)\b", "code"),
        ];

        for (pattern, kind_name) in type_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(&query_lower) {
                    let kind = match kind_name {
                        "image" => MemoryKind::Image {
                            width: 0,
                            height: 0,
                            format: String::new(),
                        },
                        "video" => MemoryKind::Video {
                            duration_ms: 0,
                            format: String::new(),
                        },
                        "audio" => MemoryKind::Audio {
                            duration_ms: 0,
                            format: String::new(),
                        },
                        "document" => MemoryKind::Document {
                            format: DocumentFormat::Pdf,
                            page_count: None,
                        },
                        "code" => MemoryKind::Code {
                            language: String::new(),
                            lines: 0,
                        },
                        _ => continue,
                    };
                    file_types.push(kind);
                    keywords = re.replace_all(&keywords, "").to_string();
                    interpretations.push(format!("file type: {}", kind_name));
                }
            }
        }

        // Extract date ranges
        let now = Utc::now();
        let date_patterns = [
            (r"\b(today|tonight)\b", 0, "today"),
            (r"\b(yesterday)\b", 1, "yesterday"),
            (r"\blast week\b", 7, "last week"),
            (r"\blast month\b", 30, "last month"),
            (r"\blast year\b", 365, "last year"),
            (r"\bthis week\b", 7, "this week"),
            (r"\bthis month\b", 30, "this month"),
            (r"\bthis year\b", 365, "this year"),
        ];

        for (pattern, days, desc) in date_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(&query_lower) {
                    let start = now - Duration::days(days);
                    date_range = Some(DateRange {
                        start: Some(start),
                        end: Some(now),
                    });
                    keywords = re.replace_all(&keywords, "").to_string();
                    interpretations.push(format!("date range: {}", desc));
                    break;
                }
            }
        }

        // Clean up keywords - remove extra whitespace
        keywords = keywords
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        Ok(ParsedQuery {
            original: query.to_string(),
            keywords: if keywords.is_empty() {
                None
            } else {
                Some(keywords)
            },
            file_types,
            date_range,
            interpretation: if interpretations.is_empty() {
                None
            } else {
                Some(interpretations.join(", "))
            },
        })
    }
}

/// Parsed natural language query
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParsedQuery {
    /// Original query text
    pub original: String,
    /// Extracted keywords (after removing file types and dates)
    pub keywords: Option<String>,
    /// Extracted file types
    pub file_types: Vec<MemoryKind>,
    /// Extracted date range
    pub date_range: Option<DateRange>,
    /// Human-readable interpretation
    pub interpretation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_similarity_identical() {
        let searcher = create_mock_searcher();
        let similarity = searcher.string_similarity("hello", "hello");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_string_similarity_different() {
        let searcher = create_mock_searcher();
        let similarity = searcher.string_similarity("abc", "xyz");
        assert!(similarity < 0.5);
    }

    #[test]
    fn test_string_similarity_similar() {
        let searcher = create_mock_searcher();
        let similarity = searcher.string_similarity("hello", "helo");
        assert!(similarity > 0.5);
    }

    #[test]
    fn test_string_similarity_empty() {
        let searcher = create_mock_searcher();
        let similarity = searcher.string_similarity("", "test");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_generate_fuzzy_variations() {
        let searcher = create_mock_searcher();
        let variations = searcher.generate_fuzzy_variations("hello");
        assert!(!variations.is_empty());
        // Should include character skips
        assert!(variations.iter().any(|v| v.len() < 5));
    }

    #[test]
    fn test_kind_name() {
        assert_eq!(
            Searcher::kind_name(&MemoryKind::Image {
                width: 0,
                height: 0,
                format: String::new()
            }),
            "image"
        );
        assert_eq!(
            Searcher::kind_name(&MemoryKind::Video {
                duration_ms: 0,
                format: String::new()
            }),
            "video"
        );
        assert_eq!(
            Searcher::kind_name(&MemoryKind::Audio {
                duration_ms: 0,
                format: String::new()
            }),
            "audio"
        );
        assert_eq!(
            Searcher::kind_name(&MemoryKind::Code {
                language: String::new(),
                lines: 0
            }),
            "code"
        );
        assert_eq!(
            Searcher::kind_name(&MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: None
            }),
            "document"
        );
        assert_eq!(Searcher::kind_name(&MemoryKind::Unknown), "unknown");
    }

    #[tokio::test]
    async fn test_parse_natural_query_audio() {
        let searcher = create_mock_searcher();
        let parsed = searcher.parse_natural_query("find music files").unwrap();
        assert!(!parsed.file_types.is_empty());
        assert!(matches!(parsed.file_types[0], MemoryKind::Audio { .. }));
    }

    #[tokio::test]
    async fn test_parse_natural_query_document() {
        let searcher = create_mock_searcher();
        let parsed = searcher.parse_natural_query("show me documents").unwrap();
        assert!(!parsed.file_types.is_empty());
        assert!(matches!(parsed.file_types[0], MemoryKind::Document { .. }));
    }

    #[tokio::test]
    async fn test_parse_natural_query_this_year() {
        let searcher = create_mock_searcher();
        let parsed = searcher
            .parse_natural_query("files from this year")
            .unwrap();
        assert!(parsed.date_range.is_some());
    }

    #[tokio::test]
    async fn test_parse_natural_query_this_month() {
        let searcher = create_mock_searcher();
        let parsed = searcher.parse_natural_query("this month photos").unwrap();
        assert!(parsed.date_range.is_some());
    }

    // Helper function to create a mock searcher for tests
    fn create_mock_searcher() -> Searcher {
        use std::sync::Arc;
        use tempfile::TempDir;

        // This is a simplified mock - in real tests you'd use proper async setup
        let temp_dir = TempDir::new().unwrap();
        let config = crate::HippoConfig {
            data_dir: temp_dir.path().to_path_buf(),
            qdrant_url: "http://localhost:9999".to_string(),
            ..Default::default()
        };

        // Create minimal components for testing
        // Note: This will fail in some tests that need actual storage
        // but works for pure function tests like string_similarity
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let storage = Arc::new(crate::storage::Storage::new(&config).await.unwrap());
            let embedder = Arc::new(crate::embeddings::Embedder::new(&config).await.unwrap());
            Searcher::new(storage, embedder, &config).await.unwrap()
        })
    }
}
