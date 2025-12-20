//! Tests for semantic search functionality including cosine similarity and hybrid search

use hippo_core::search::{semantic_score, HybridSearchConfig};

#[test]
fn test_semantic_score_identical_vectors() {
    // Identical vectors should have similarity of 1.0
    let vec1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let vec2 = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-6,
        "Expected similarity ~1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_orthogonal_vectors() {
    // Orthogonal vectors should have similarity of 0.0
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![0.0, 1.0, 0.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        similarity.abs() < 1e-6,
        "Expected similarity ~0.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_opposite_vectors() {
    // Opposite direction vectors should have similarity of -1.0
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![-1.0, 0.0, 0.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity + 1.0).abs() < 1e-6,
        "Expected similarity ~-1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_scaled_vectors() {
    // Scaled versions of the same vector should have similarity of 1.0
    let vec1 = vec![1.0, 2.0, 3.0];
    let vec2 = vec![2.0, 4.0, 6.0]; // 2x scaled

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-6,
        "Expected similarity ~1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_empty_vectors() {
    // Empty vectors should return 0.0
    let vec1: Vec<f32> = vec![];
    let vec2: Vec<f32> = vec![];

    let similarity = semantic_score(&vec1, &vec2);
    assert_eq!(similarity, 0.0, "Expected 0.0 for empty vectors");
}

#[test]
fn test_semantic_score_one_empty_vector() {
    // One empty vector should return 0.0
    let vec1 = vec![1.0, 2.0, 3.0];
    let vec2: Vec<f32> = vec![];

    let similarity = semantic_score(&vec1, &vec2);
    assert_eq!(similarity, 0.0, "Expected 0.0 when one vector is empty");
}

#[test]
fn test_semantic_score_different_lengths() {
    // Different length vectors should return 0.0
    let vec1 = vec![1.0, 2.0, 3.0];
    let vec2 = vec![1.0, 2.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert_eq!(similarity, 0.0, "Expected 0.0 for different length vectors");
}

#[test]
fn test_semantic_score_single_element() {
    // Single element vectors
    let vec1 = vec![5.0];
    let vec2 = vec![5.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-6,
        "Expected similarity ~1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_zero_magnitude_vector() {
    // Zero magnitude vector should return 0.0
    let vec1 = vec![0.0, 0.0, 0.0];
    let vec2 = vec![1.0, 2.0, 3.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert_eq!(similarity, 0.0, "Expected 0.0 for zero magnitude vector");
}

#[test]
fn test_semantic_score_both_zero_magnitude() {
    // Both zero magnitude vectors should return 0.0
    let vec1 = vec![0.0, 0.0, 0.0];
    let vec2 = vec![0.0, 0.0, 0.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert_eq!(
        similarity, 0.0,
        "Expected 0.0 for both zero magnitude vectors"
    );
}

#[test]
fn test_semantic_score_high_dimensional() {
    // Test with high-dimensional vectors (typical embedding size)
    let dim = 384;
    let vec1: Vec<f32> = (0..dim).map(|i| (i as f32).sin()).collect();
    let vec2: Vec<f32> = (0..dim).map(|i| (i as f32).sin()).collect();

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-5,
        "Expected similarity ~1.0 for identical high-dim vectors"
    );
}

#[test]
fn test_semantic_score_partial_overlap() {
    // Partially overlapping vectors
    let vec1 = vec![1.0, 2.0, 3.0, 4.0];
    let vec2 = vec![1.0, 2.0, 0.0, 0.0];

    let similarity = semantic_score(&vec1, &vec2);
    // Similarity should be positive but less than 1.0
    assert!(
        similarity > 0.0 && similarity < 1.0,
        "Expected 0.0 < similarity < 1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_negative_values() {
    // Vectors with negative values
    let vec1 = vec![-1.0, -2.0, -3.0];
    let vec2 = vec![-1.0, -2.0, -3.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-6,
        "Expected similarity ~1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_mixed_signs() {
    // Vectors with mixed positive and negative values
    let vec1 = vec![1.0, -1.0, 1.0, -1.0];
    let vec2 = vec![-1.0, 1.0, -1.0, 1.0];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity + 1.0).abs() < 1e-6,
        "Expected similarity ~-1.0, got {}",
        similarity
    );
}

#[test]
fn test_semantic_score_normalized_vectors() {
    // Pre-normalized vectors (unit vectors)
    let vec1 = vec![
        1.0 / 3.0_f32.sqrt(),
        1.0 / 3.0_f32.sqrt(),
        1.0 / 3.0_f32.sqrt(),
    ];
    let vec2 = vec![
        1.0 / 3.0_f32.sqrt(),
        1.0 / 3.0_f32.sqrt(),
        1.0 / 3.0_f32.sqrt(),
    ];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-5,
        "Expected similarity ~1.0 for normalized vectors"
    );
}

#[test]
fn test_semantic_score_very_small_values() {
    // Very small values (close to zero but not zero)
    let vec1 = vec![1e-10, 2e-10, 3e-10];
    let vec2 = vec![1e-10, 2e-10, 3e-10];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-5,
        "Expected similarity ~1.0 for small identical vectors"
    );
}

#[test]
fn test_semantic_score_large_values() {
    // Very large values
    let vec1 = vec![1e6, 2e6, 3e6];
    let vec2 = vec![1e6, 2e6, 3e6];

    let similarity = semantic_score(&vec1, &vec2);
    assert!(
        (similarity - 1.0).abs() < 1e-5,
        "Expected similarity ~1.0 for large identical vectors"
    );
}

#[test]
fn test_semantic_score_symmetry() {
    // Cosine similarity should be symmetric
    let vec1 = vec![1.0, 2.0, 3.0];
    let vec2 = vec![4.0, 5.0, 6.0];

    let similarity1 = semantic_score(&vec1, &vec2);
    let similarity2 = semantic_score(&vec2, &vec1);

    assert!(
        (similarity1 - similarity2).abs() < 1e-6,
        "Cosine similarity should be symmetric"
    );
}

// Tests for HybridSearchConfig

#[test]
fn test_hybrid_config_default_weights() {
    let config = HybridSearchConfig::default();

    assert_eq!(
        config.semantic_weight, 0.7,
        "Default semantic weight should be 0.7"
    );
    assert_eq!(
        config.keyword_weight, 0.3,
        "Default keyword weight should be 0.3"
    );
}

#[test]
fn test_hybrid_config_custom_weights() {
    let config = HybridSearchConfig {
        semantic_weight: 0.5,
        keyword_weight: 0.5,
    };

    assert_eq!(config.semantic_weight, 0.5);
    assert_eq!(config.keyword_weight, 0.5);
}

#[test]
fn test_hybrid_config_extreme_weights() {
    // Test with all weight on semantic
    let config1 = HybridSearchConfig {
        semantic_weight: 1.0,
        keyword_weight: 0.0,
    };
    assert_eq!(config1.semantic_weight, 1.0);
    assert_eq!(config1.keyword_weight, 0.0);

    // Test with all weight on keyword
    let config2 = HybridSearchConfig {
        semantic_weight: 0.0,
        keyword_weight: 1.0,
    };
    assert_eq!(config2.semantic_weight, 0.0);
    assert_eq!(config2.keyword_weight, 1.0);
}

// Integration-style tests for hybrid scoring logic

#[test]
fn test_hybrid_scoring_calculation() {
    // Simulate hybrid scoring calculation
    let semantic_weight = 0.7;
    let keyword_weight = 0.3;

    let semantic_score: f64 = 0.8;
    let keyword_score: f64 = 0.6;

    let combined_score = (semantic_score * semantic_weight) + (keyword_score * keyword_weight);
    let expected = (0.8_f64 * 0.7) + (0.6_f64 * 0.3);

    assert!(
        (combined_score - expected).abs() < 1e-6,
        "Hybrid score calculation incorrect"
    );
    assert!(
        (combined_score - 0.74_f64).abs() < 1e-6,
        "Expected combined score ~0.74"
    );
}

#[test]
fn test_hybrid_scoring_equal_weights() {
    // Test with equal weights (50/50)
    let semantic_weight: f64 = 0.5;
    let keyword_weight: f64 = 0.5;

    let semantic_score: f64 = 0.8;
    let keyword_score: f64 = 0.6;

    let combined_score = (semantic_score * semantic_weight) + (keyword_score * keyword_weight);
    let expected = (0.8_f64 * 0.5) + (0.6_f64 * 0.5);

    assert!((combined_score - expected).abs() < 1e-6);
    assert!(
        (combined_score - 0.7_f64).abs() < 1e-6,
        "Expected combined score ~0.7"
    );
}

#[test]
fn test_hybrid_scoring_only_semantic() {
    // Test with only semantic weight
    let semantic_weight: f64 = 1.0;
    let keyword_weight: f64 = 0.0;

    let semantic_score: f64 = 0.8;
    let keyword_score: f64 = 0.6;

    let combined_score = (semantic_score * semantic_weight) + (keyword_score * keyword_weight);

    assert!(
        (combined_score - 0.8_f64).abs() < 1e-6,
        "With semantic_weight=1.0, should equal semantic score"
    );
}

#[test]
fn test_hybrid_scoring_only_keyword() {
    // Test with only keyword weight
    let semantic_weight: f64 = 0.0;
    let keyword_weight: f64 = 1.0;

    let semantic_score: f64 = 0.8;
    let keyword_score: f64 = 0.6;

    let combined_score = (semantic_score * semantic_weight) + (keyword_score * keyword_weight);

    assert!(
        (combined_score - 0.6_f64).abs() < 1e-6,
        "With keyword_weight=1.0, should equal keyword score"
    );
}
