//! Advanced filtering capabilities for memory search

use crate::models::*;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Advanced filter for complex search queries
#[derive(Debug, Clone, Default)]
pub struct AdvancedFilter {
    /// File size range in bytes (min, max)
    pub file_size_range: Option<(u64, u64)>,
    /// Date range for file creation/modification
    pub date_range: Option<DateRange>,
    /// Extension filter (whitelist or blacklist)
    pub extension_filter: Option<ExtensionFilter>,
    /// Content substring matching (case-insensitive)
    pub content_contains: Option<String>,
    /// Metadata field matching
    pub metadata_matches: Vec<MetadataMatch>,
    /// Filter for duplicate content hashes
    pub duplicates_only: bool,
    /// Minimum content hash matches
    pub min_duplicate_count: Option<usize>,
}

/// Extension filter mode
#[derive(Debug, Clone)]
pub enum ExtensionFilter {
    /// Only include these extensions
    Whitelist(HashSet<String>),
    /// Exclude these extensions
    Blacklist(HashSet<String>),
}

/// Metadata field matching
#[derive(Debug, Clone)]
pub struct MetadataMatch {
    pub field: String,
    pub value: String,
    pub match_mode: MatchMode,
}

/// Match mode for metadata filtering
#[derive(Debug, Clone)]
pub enum MatchMode {
    Exact,
    Contains,
    StartsWith,
    EndsWith,
    Regex(String),
}

/// Builder for constructing advanced filters with a fluent API
#[derive(Debug, Clone, Default)]
pub struct FilterBuilder {
    filter: AdvancedFilter,
}

impl FilterBuilder {
    /// Create a new filter builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum file size in bytes
    pub fn min_size(mut self, size: u64) -> Self {
        let max = self.filter.file_size_range.map(|(_, m)| m).unwrap_or(u64::MAX);
        self.filter.file_size_range = Some((size, max));
        self
    }

    /// Set maximum file size in bytes
    pub fn max_size(mut self, size: u64) -> Self {
        let min = self.filter.file_size_range.map(|(m, _)| m).unwrap_or(0);
        self.filter.file_size_range = Some((min, size));
        self
    }

    /// Set both minimum and maximum file size
    pub fn size_range(mut self, min: u64, max: u64) -> Self {
        self.filter.file_size_range = Some((min, max));
        self
    }

    /// Set date range for filtering
    pub fn date_range(mut self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self {
        self.filter.date_range = Some(DateRange { start, end });
        self
    }

    /// Set start date
    pub fn after(mut self, date: DateTime<Utc>) -> Self {
        let end = self.filter.date_range.as_ref().and_then(|r| r.end);
        self.filter.date_range = Some(DateRange {
            start: Some(date),
            end,
        });
        self
    }

    /// Set end date
    pub fn before(mut self, date: DateTime<Utc>) -> Self {
        let start = self.filter.date_range.as_ref().and_then(|r| r.start);
        self.filter.date_range = Some(DateRange {
            start,
            end: Some(date),
        });
        self
    }

    /// Include only these file extensions
    pub fn extensions(mut self, extensions: Vec<String>) -> Self {
        let normalized: HashSet<String> = extensions
            .into_iter()
            .map(|e| e.trim_start_matches('.').to_lowercase())
            .collect();
        self.filter.extension_filter = Some(ExtensionFilter::Whitelist(normalized));
        self
    }

    /// Exclude these file extensions
    pub fn exclude_extensions(mut self, extensions: Vec<String>) -> Self {
        let normalized: HashSet<String> = extensions
            .into_iter()
            .map(|e| e.trim_start_matches('.').to_lowercase())
            .collect();
        self.filter.extension_filter = Some(ExtensionFilter::Blacklist(normalized));
        self
    }

    /// Filter by content substring (case-insensitive)
    pub fn content_contains(mut self, text: String) -> Self {
        self.filter.content_contains = Some(text);
        self
    }

    /// Add a metadata field match
    pub fn metadata_match(mut self, field: String, value: String, mode: MatchMode) -> Self {
        self.filter.metadata_matches.push(MetadataMatch {
            field,
            value,
            match_mode: mode,
        });
        self
    }

    /// Filter for duplicates only (same content hash)
    pub fn duplicates_only(mut self) -> Self {
        self.filter.duplicates_only = true;
        self
    }

    /// Set minimum number of duplicates
    pub fn min_duplicate_count(mut self, count: usize) -> Self {
        self.filter.min_duplicate_count = Some(count);
        self.filter.duplicates_only = true;
        self
    }

    /// Build the final filter
    pub fn build(self) -> AdvancedFilter {
        self.filter
    }
}

impl AdvancedFilter {
    /// Apply this filter to a list of memories
    pub fn apply_filters(&self, memories: Vec<Memory>) -> Vec<Memory> {
        let mut filtered = memories;

        // Apply file size filter
        if let Some((min, max)) = self.file_size_range {
            filtered.retain(|m| {
                let size = m.metadata.file_size;
                size >= min && size <= max
            });
        }

        // Apply date range filter
        if let Some(ref date_range) = self.date_range {
            filtered.retain(|m| {
                let mut in_range = true;
                if let Some(start) = date_range.start {
                    in_range = in_range && m.modified_at >= start;
                }
                if let Some(end) = date_range.end {
                    in_range = in_range && m.modified_at <= end;
                }
                in_range
            });
        }

        // Apply extension filter
        if let Some(ref ext_filter) = self.extension_filter {
            filtered.retain(|m| {
                let extension = m
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase());

                match (ext_filter, extension) {
                    (ExtensionFilter::Whitelist(whitelist), Some(ext)) => whitelist.contains(&ext),
                    (ExtensionFilter::Whitelist(_), None) => false,
                    (ExtensionFilter::Blacklist(blacklist), Some(ext)) => !blacklist.contains(&ext),
                    (ExtensionFilter::Blacklist(_), None) => true,
                }
            });
        }

        // Apply content substring filter
        if let Some(ref text) = self.content_contains {
            let text_lower = text.to_lowercase();
            filtered.retain(|m| {
                // Check in title
                if let Some(ref title) = m.metadata.title {
                    if title.to_lowercase().contains(&text_lower) {
                        return true;
                    }
                }
                // Check in description
                if let Some(ref desc) = m.metadata.description {
                    if desc.to_lowercase().contains(&text_lower) {
                        return true;
                    }
                }
                // Check in text preview
                if let Some(ref preview) = m.metadata.text_preview {
                    if preview.to_lowercase().contains(&text_lower) {
                        return true;
                    }
                }
                // Check in tags
                for tag in &m.tags {
                    if tag.name.to_lowercase().contains(&text_lower) {
                        return true;
                    }
                }
                false
            });
        }

        // Apply metadata matches
        for metadata_match in &self.metadata_matches {
            let field = &metadata_match.field;
            let value = &metadata_match.value;
            let mode = &metadata_match.match_mode;

            filtered.retain(|m| {
                let field_value = match field.as_str() {
                    "title" => m.metadata.title.as_ref(),
                    "description" => m.metadata.description.as_ref(),
                    "mime_type" => m.metadata.mime_type.as_ref(),
                    _ => {
                        // Check custom fields
                        m.metadata
                            .custom
                            .get(field)
                            .and_then(|v| v.as_str())
                            .as_ref()
                    }
                };

                if let Some(field_val) = field_value {
                    match mode {
                        MatchMode::Exact => field_val == value,
                        MatchMode::Contains => {
                            field_val.to_lowercase().contains(&value.to_lowercase())
                        }
                        MatchMode::StartsWith => {
                            field_val.to_lowercase().starts_with(&value.to_lowercase())
                        }
                        MatchMode::EndsWith => {
                            field_val.to_lowercase().ends_with(&value.to_lowercase())
                        }
                        MatchMode::Regex(pattern) => {
                            if let Ok(re) = Regex::new(pattern) {
                                re.is_match(field_val)
                            } else {
                                false
                            }
                        }
                    }
                } else {
                    false
                }
            });
        }

        // Apply duplicate filter
        if self.duplicates_only {
            let mut hash_groups: HashMap<String, Vec<Memory>> = HashMap::new();

            // Group by content hash
            for memory in filtered {
                if let Some(ref hash) = memory.metadata.hash {
                    hash_groups.entry(hash.clone()).or_default().push(memory);
                }
            }

            // Filter to only keep groups with duplicates
            filtered = hash_groups
                .into_iter()
                .filter(|(_, group)| {
                    let count = group.len();
                    if let Some(min_count) = self.min_duplicate_count {
                        count >= min_count
                    } else {
                        count > 1
                    }
                })
                .flat_map(|(_, group)| group)
                .collect();
        }

        filtered
    }
}
