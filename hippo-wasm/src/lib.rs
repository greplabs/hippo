//! WebAssembly bindings for Hippo search engine
//!
//! This crate provides client-side search capabilities for the Hippo file organizer,
//! allowing fuzzy matching, semantic scoring, and local search to run directly in the browser.

#![allow(missing_docs)]

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Set up panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

/// Version information for the WASM module
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Calculate fuzzy match score using Levenshtein distance
/// Returns a value between 0.0 and 1.0, where 1.0 means exact match
#[wasm_bindgen]
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

/// Calculate cosine similarity between two embedding vectors
/// Returns a value between -1.0 and 1.0, where 1.0 means identical direction
#[wasm_bindgen]
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

/// Simplified Memory struct for WASM (only fields needed for search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMemory {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub file_size: u64,
    pub modified_at: String, // ISO 8601 string
    pub kind: String,        // "image", "video", "document", etc.
}

/// Search result with score and highlights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSearchResult {
    pub memory: WasmMemory,
    pub score: f32,
    pub highlights: Vec<WasmHighlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmHighlight {
    pub field: String,
    pub snippet: String,
}

/// Search memories using fuzzy matching (client-side filtering)
///
/// Takes a JSON array of memories and a query string, returns JSON array of results
#[wasm_bindgen]
pub fn search_local(memories_json: &str, query: &str) -> Result<String, JsValue> {
    // Parse the memories from JSON
    let memories: Vec<WasmMemory> = serde_json::from_str(memories_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse memories: {}", e)))?;

    // Perform the search
    let results = search_memories(&memories, query);

    // Serialize results back to JSON
    serde_json::to_string(&results)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

/// Internal search implementation
fn search_memories(memories: &[WasmMemory], query: &str) -> Vec<WasmSearchResult> {
    if query.is_empty() {
        // Return all memories with score 1.0 if no query
        return memories
            .iter()
            .map(|m| WasmSearchResult {
                memory: m.clone(),
                score: 1.0,
                highlights: vec![],
            })
            .collect();
    }

    let query_lower = query.to_lowercase();
    let mut results: Vec<WasmSearchResult> = Vec::new();

    for memory in memories {
        let mut score = 0.0f32;
        let mut highlights = Vec::new();

        // Title match
        if let Some(ref title) = memory.title {
            let title_score = fuzzy_match(&query_lower, &title.to_lowercase());
            if title_score > 0.3 {
                score += title_score * 10.0;
                if title.to_lowercase().contains(&query_lower) {
                    score += 5.0;
                    highlights.push(WasmHighlight {
                        field: "title".to_string(),
                        snippet: title.clone(),
                    });
                }
            }
        }

        // Filename match
        let filename = memory.path.split('/').last().unwrap_or(&memory.path);
        let filename_score = fuzzy_match(&query_lower, &filename.to_lowercase());
        if filename_score > 0.3 {
            score += filename_score * 8.0;
            if filename.to_lowercase().contains(&query_lower) {
                score += 4.0;
                highlights.push(WasmHighlight {
                    field: "filename".to_string(),
                    snippet: filename.to_string(),
                });
            }
        }

        // Tag match
        for tag in &memory.tags {
            let tag_score = fuzzy_match(&query_lower, &tag.to_lowercase());
            if tag_score > 0.5 {
                score += tag_score * 7.0;
                highlights.push(WasmHighlight {
                    field: "tag".to_string(),
                    snippet: tag.clone(),
                });
            }
        }

        // Path match
        let path_score = fuzzy_match(&query_lower, &memory.path.to_lowercase());
        if path_score > 0.3 {
            score += path_score * 3.0;
        }

        // Only include results with some relevance
        if score > 0.0 {
            results.push(WasmSearchResult {
                memory: memory.clone(),
                score,
                highlights,
            });
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Filter memories by type (client-side filtering)
#[wasm_bindgen]
pub fn filter_by_type(memories_json: &str, kind: &str) -> Result<String, JsValue> {
    let memories: Vec<WasmMemory> = serde_json::from_str(memories_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse memories: {}", e)))?;

    let filtered: Vec<WasmMemory> = memories
        .into_iter()
        .filter(|m| m.kind.to_lowercase() == kind.to_lowercase())
        .collect();

    serde_json::to_string(&filtered)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

/// Sort memories by field (client-side sorting)
#[wasm_bindgen]
pub fn sort_memories(memories_json: &str, field: &str, ascending: bool) -> Result<String, JsValue> {
    let mut memories: Vec<WasmMemory> = serde_json::from_str(memories_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse memories: {}", e)))?;

    match field {
        "name" => {
            memories.sort_by(|a, b| {
                let a_name = a.path.split('/').last().unwrap_or(&a.path);
                let b_name = b.path.split('/').last().unwrap_or(&b.path);
                if ascending {
                    a_name.cmp(b_name)
                } else {
                    b_name.cmp(a_name)
                }
            });
        }
        "size" => {
            memories.sort_by(|a, b| {
                if ascending {
                    a.file_size.cmp(&b.file_size)
                } else {
                    b.file_size.cmp(&a.file_size)
                }
            });
        }
        "date" => {
            memories.sort_by(|a, b| {
                if ascending {
                    a.modified_at.cmp(&b.modified_at)
                } else {
                    b.modified_at.cmp(&a.modified_at)
                }
            });
        }
        _ => {
            return Err(JsValue::from_str(&format!("Unknown sort field: {}", field)));
        }
    }

    serde_json::to_string(&memories)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

/// Get statistics about memories (client-side aggregation)
#[wasm_bindgen]
pub fn get_stats(memories_json: &str) -> Result<String, JsValue> {
    let memories: Vec<WasmMemory> = serde_json::from_str(memories_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse memories: {}", e)))?;

    let mut stats = HashMap::new();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut total_size = 0u64;

    for memory in &memories {
        *by_kind.entry(memory.kind.clone()).or_insert(0) += 1;
        total_size += memory.file_size;
    }

    stats.insert("total_count", memories.len());
    stats.insert("total_size", total_size as usize);

    let result = serde_json::json!({
        "total_count": memories.len(),
        "total_size": total_size,
        "by_kind": by_kind,
    });

    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize stats: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        assert_eq!(fuzzy_match("hello", "hello"), 1.0);
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert_eq!(fuzzy_match("Hello", "HELLO"), 1.0);
    }

    #[test]
    fn test_fuzzy_match_contains() {
        let score = fuzzy_match("hello", "hello world");
        assert!(score > 0.8);
    }

    #[test]
    fn test_fuzzy_match_similar() {
        let score = fuzzy_match("hello", "helo");
        assert!(score > 0.5);
    }

    #[test]
    fn test_fuzzy_match_different() {
        let score = fuzzy_match("hello", "world");
        assert!(score < 0.3);
    }

    #[test]
    fn test_semantic_score_identical() {
        let vec = vec![1.0, 2.0, 3.0];
        assert_eq!(semantic_score(&vec, &vec), 1.0);
    }

    #[test]
    fn test_semantic_score_opposite() {
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![-1.0, 0.0];
        assert!(semantic_score(&vec1, &vec2) < 0.0);
    }

    #[test]
    fn test_semantic_score_orthogonal() {
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![0.0, 1.0];
        assert_eq!(semantic_score(&vec1, &vec2), 0.0);
    }

    #[test]
    fn test_semantic_score_different_lengths() {
        let vec1 = vec![1.0, 2.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        assert_eq!(semantic_score(&vec1, &vec2), 0.0);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_search_memories() {
        let memories = vec![
            WasmMemory {
                id: "1".to_string(),
                path: "/photos/vacation.jpg".to_string(),
                title: Some("Beach Vacation".to_string()),
                tags: vec!["beach".to_string(), "summer".to_string()],
                file_size: 1024,
                modified_at: "2025-01-15T10:00:00Z".to_string(),
                kind: "image".to_string(),
            },
            WasmMemory {
                id: "2".to_string(),
                path: "/docs/report.pdf".to_string(),
                title: Some("Annual Report".to_string()),
                tags: vec!["work".to_string()],
                file_size: 2048,
                modified_at: "2025-01-14T10:00:00Z".to_string(),
                kind: "document".to_string(),
            },
        ];

        let results = search_memories(&memories, "beach");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.id, "1");
        assert!(results[0].score > 0.0);
    }
}
