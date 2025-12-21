//! Tests for hippo-wasm
//!
//! Run with: wasm-pack test --node

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import functions from lib
use hippo_wasm::{filter_by_type, fuzzy_match, search_local, semantic_score, sort_memories};

#[wasm_bindgen_test]
fn test_fuzzy_match_exact() {
    assert_eq!(fuzzy_match("hello", "hello"), 1.0);
}

#[wasm_bindgen_test]
fn test_fuzzy_match_case_insensitive() {
    assert_eq!(fuzzy_match("Hello", "HELLO"), 1.0);
}

#[wasm_bindgen_test]
fn test_fuzzy_match_contains() {
    let score = fuzzy_match("hello", "hello world");
    assert!(score > 0.8, "Expected score > 0.8, got {}", score);
}

#[wasm_bindgen_test]
fn test_fuzzy_match_partial() {
    let score = fuzzy_match("hello", "helo");
    assert!(score > 0.5, "Expected score > 0.5, got {}", score);
}

#[wasm_bindgen_test]
fn test_semantic_score_identical() {
    let vec = vec![1.0, 2.0, 3.0];
    let score = semantic_score(&vec, &vec);
    assert_eq!(score, 1.0);
}

#[wasm_bindgen_test]
fn test_semantic_score_orthogonal() {
    let vec1 = vec![1.0, 0.0];
    let vec2 = vec![0.0, 1.0];
    let score = semantic_score(&vec1, &vec2);
    assert_eq!(score, 0.0);
}

#[wasm_bindgen_test]
fn test_semantic_score_similar() {
    let vec1 = vec![1.0, 2.0, 3.0];
    let vec2 = vec![1.1, 2.1, 3.1];
    let score = semantic_score(&vec1, &vec2);
    assert!(score > 0.99, "Expected high similarity, got {}", score);
}

#[wasm_bindgen_test]
fn test_search_local() {
    let memories_json = r#"[
        {
            "id": "1",
            "path": "/photos/beach.jpg",
            "title": "Beach Vacation",
            "tags": ["beach", "summer"],
            "file_size": 1024000,
            "modified_at": "2025-01-15T10:00:00Z",
            "kind": "image"
        },
        {
            "id": "2",
            "path": "/docs/report.pdf",
            "title": "Annual Report",
            "tags": ["work"],
            "file_size": 512000,
            "modified_at": "2025-01-14T09:00:00Z",
            "kind": "document"
        }
    ]"#;

    let results = search_local(memories_json, "beach").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&results).unwrap();

    assert!(parsed.is_array());
    let results_array = parsed.as_array().unwrap();
    assert_eq!(results_array.len(), 1);
    assert_eq!(results_array[0]["memory"]["id"], "1");
}

#[wasm_bindgen_test]
fn test_filter_by_type() {
    let memories_json = r#"[
        {
            "id": "1",
            "path": "/photos/beach.jpg",
            "title": "Beach",
            "tags": [],
            "file_size": 1024,
            "modified_at": "2025-01-15T10:00:00Z",
            "kind": "image"
        },
        {
            "id": "2",
            "path": "/video.mp4",
            "title": "Video",
            "tags": [],
            "file_size": 2048,
            "modified_at": "2025-01-14T09:00:00Z",
            "kind": "video"
        }
    ]"#;

    let filtered = filter_by_type(memories_json, "image").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&filtered).unwrap();

    let filtered_array = parsed.as_array().unwrap();
    assert_eq!(filtered_array.len(), 1);
    assert_eq!(filtered_array[0]["kind"], "image");
}

#[wasm_bindgen_test]
fn test_sort_memories_by_name() {
    let memories_json = r#"[
        {
            "id": "1",
            "path": "/zebra.jpg",
            "title": "Zebra",
            "tags": [],
            "file_size": 1024,
            "modified_at": "2025-01-15T10:00:00Z",
            "kind": "image"
        },
        {
            "id": "2",
            "path": "/apple.jpg",
            "title": "Apple",
            "tags": [],
            "file_size": 2048,
            "modified_at": "2025-01-14T09:00:00Z",
            "kind": "image"
        }
    ]"#;

    let sorted = sort_memories(memories_json, "name", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&sorted).unwrap();

    let sorted_array = parsed.as_array().unwrap();
    assert_eq!(sorted_array[0]["id"], "2"); // apple comes first
    assert_eq!(sorted_array[1]["id"], "1"); // zebra comes second
}

#[wasm_bindgen_test]
fn test_sort_memories_by_size() {
    let memories_json = r#"[
        {
            "id": "1",
            "path": "/large.jpg",
            "title": "Large",
            "tags": [],
            "file_size": 2048,
            "modified_at": "2025-01-15T10:00:00Z",
            "kind": "image"
        },
        {
            "id": "2",
            "path": "/small.jpg",
            "title": "Small",
            "tags": [],
            "file_size": 1024,
            "modified_at": "2025-01-14T09:00:00Z",
            "kind": "image"
        }
    ]"#;

    let sorted = sort_memories(memories_json, "size", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&sorted).unwrap();

    let sorted_array = parsed.as_array().unwrap();
    assert_eq!(sorted_array[0]["id"], "2"); // small first (ascending)
    assert_eq!(sorted_array[1]["id"], "1"); // large second
}
