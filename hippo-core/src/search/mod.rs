//! Search engine combining vector similarity and metadata filtering

use crate::error::Result;
use crate::models::*;
use crate::storage::Storage;
use crate::embeddings::Embedder;
use crate::HippoConfig;
use std::sync::Arc;

pub struct Searcher {
    storage: Arc<Storage>,
    #[allow(dead_code)]
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

    /// Perform a search with the given query
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        // Get all memories from storage
        let all_memories = self.storage.get_all_memories().await?;

        let mut results: Vec<MemorySearchResult> = all_memories
            .into_iter()
            .filter_map(|memory| {
                let mut score = 0.0;
                let mut highlights: Vec<Highlight> = Vec::new();

                // Text filter with scoring
                if let Some(ref text) = query.text {
                    let text_lower = text.to_lowercase();
                    let search_terms: Vec<&str> = text_lower.split_whitespace().collect();

                    if search_terms.is_empty() {
                        score = 1.0;
                    } else {
                        let mut term_matches = 0;

                        for term in &search_terms {
                            let mut term_matched = false;

                            // Title matching (highest weight)
                            if let Some(ref title) = memory.metadata.title {
                                let title_lower = title.to_lowercase();
                                if title_lower.contains(term) {
                                    score += 10.0;
                                    term_matched = true;
                                    // Exact match bonus
                                    if title_lower == *term || title_lower.starts_with(&format!("{} ", term)) {
                                        score += 5.0;
                                    }
                                    highlights.push(Highlight {
                                        field: "title".to_string(),
                                        snippet: title.clone(),
                                    });
                                }
                            }

                            // Filename matching (high weight)
                            let filename = memory.path.file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let filename_lower = filename.to_lowercase();
                            if filename_lower.contains(term) {
                                score += 8.0;
                                term_matched = true;
                                // Exact filename match bonus
                                if filename_lower.starts_with(term) {
                                    score += 4.0;
                                }
                                highlights.push(Highlight {
                                    field: "filename".to_string(),
                                    snippet: filename.clone(),
                                });
                            }

                            // Path matching (medium weight)
                            let path_str = memory.path.to_string_lossy().to_lowercase();
                            if path_str.contains(term) && !filename_lower.contains(term) {
                                score += 3.0;
                                term_matched = true;
                            }

                            // Tag matching (high weight)
                            for tag in &memory.tags {
                                if tag.name.to_lowercase().contains(term) {
                                    score += 7.0;
                                    term_matched = true;
                                    highlights.push(Highlight {
                                        field: "tag".to_string(),
                                        snippet: tag.name.clone(),
                                    });
                                    // Exact tag match bonus
                                    if tag.name.to_lowercase() == *term {
                                        score += 3.0;
                                    }
                                }
                            }

                            // Extension matching
                            if let Some(ext) = memory.path.extension() {
                                if ext.to_string_lossy().to_lowercase().contains(term) {
                                    score += 2.0;
                                    term_matched = true;
                                }
                            }

                            if term_matched {
                                term_matches += 1;
                            }
                        }

                        // Only include if at least one term matched
                        if term_matches == 0 {
                            return None;
                        }

                        // Bonus for matching all terms
                        if term_matches == search_terms.len() {
                            score *= 1.5;
                        }
                    }
                } else {
                    // No search text - give base score
                    score = 1.0;
                }

                // Tag filters
                for tag_filter in &query.tags {
                    let has_tag = memory.tags.iter().any(|t| t.name.to_lowercase() == tag_filter.tag.to_lowercase());
                    match tag_filter.mode {
                        TagFilterMode::Include => {
                            if !has_tag { return None; }
                            score += 5.0; // Boost for matching required tags
                        }
                        TagFilterMode::Exclude => if has_tag { return None; }
                    }
                }

                // Kind filter - compare by discriminant name
                if !query.kinds.is_empty() {
                    let memory_kind_name = Self::kind_name(&memory.kind);
                    let matches_kind = query.kinds.iter().any(|k| Self::kind_name(k) == memory_kind_name);
                    if !matches_kind {
                        return None;
                    }
                }

                // Date range filter
                if let Some(ref date_range) = query.date_range {
                    if let Some(start) = date_range.start {
                        if memory.modified_at < start {
                            return None;
                        }
                    }
                    if let Some(end) = date_range.end {
                        if memory.modified_at > end {
                            return None;
                        }
                    }
                }

                // Recency boost (slight boost for recently modified files)
                let age_days = (chrono::Utc::now() - memory.modified_at).num_days();
                if age_days < 7 {
                    score *= 1.1;
                } else if age_days < 30 {
                    score *= 1.05;
                }

                Some(MemorySearchResult {
                    memory,
                    score,
                    highlights,
                })
            })
            .collect();

        // Sort by score first, then by modified date
        results.sort_by(|a, b| {
            // Primary sort by score (descending)
            let score_cmp = b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal);
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }
            // Secondary sort by modified date (newest first)
            b.memory.modified_at.cmp(&a.memory.modified_at)
        });

        // Limit results - increased limit for better UX
        let total_count = results.len();
        let limit = if query.limit > 0 { query.limit } else { 500 };
        results.truncate(limit);

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

        // Score and sort tags
        let mut scored_tags: Vec<(String, u64, f32)> = all_tags
            .into_iter()
            .filter_map(|(name, count)| {
                let name_lower = name.to_lowercase();

                // Calculate base score based on match type
                let base_score = if name_lower == text_lower {
                    // Exact match
                    100.0
                } else if name_lower.starts_with(&text_lower) {
                    // Prefix match
                    80.0
                } else if name_lower.contains(&text_lower) {
                    // Contains match
                    50.0
                } else if name_lower.split(|c: char| !c.is_alphanumeric())
                    .any(|word| word.starts_with(&text_lower)) {
                    // Word boundary match
                    40.0
                } else {
                    return None;
                };

                // Boost by usage count
                let score = base_score + (count as f32).ln().max(0.0);

                Some((name, count, score))
            })
            .collect();

        // Sort by score descending
        scored_tags.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_tags.into_iter().take(10).map(|(name, _, _)| name).collect())
    }
}
