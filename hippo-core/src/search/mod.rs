//! Search engine combining SQL search, fuzzy matching, and semantic search

use crate::embeddings::Embedder;
use crate::error::Result;
use crate::models::*;
use crate::storage::Storage;
use crate::HippoConfig;
use std::sync::Arc;
use tracing::warn;

pub struct Searcher {
    storage: Arc<Storage>,
    embedder: Arc<Embedder>,
}

impl Searcher {
    pub async fn new(
        storage: Arc<Storage>,
        embedder: Arc<Embedder>,
        _config: &HippoConfig,
    ) -> Result<Self> {
        Ok(Self { storage, embedder })
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

    /// Perform semantic search using embeddings
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

        // Get all stored embeddings
        let stored_embeddings = self.storage.get_all_embeddings().await?;

        if stored_embeddings.is_empty() {
            // No embeddings yet, fall back to text search
            return self
                .search(SearchQuery {
                    text: Some(query.to_string()),
                    limit,
                    ..Default::default()
                })
                .await;
        }

        // Calculate similarities
        let mut scored: Vec<(MemoryId, f32)> = stored_embeddings
            .iter()
            .map(|(id, emb)| (*id, Embedder::similarity(&query_embedding, emb)))
            .collect();

        // Sort by similarity (highest first)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

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
}
