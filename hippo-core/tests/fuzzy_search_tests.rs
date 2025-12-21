//! Tests for fuzzy search functionality including Levenshtein distance matching

use hippo_core::search::{fuzzy_find_best_match, fuzzy_match};

#[test]
fn test_fuzzy_match_exact_match() {
    // Exact match should return 1.0
    let score = fuzzy_match("hello", "hello");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for exact match, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_case_insensitive() {
    // Case-insensitive exact match should return 1.0
    let score = fuzzy_match("Hello", "hello");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for case-insensitive match, got {}",
        score
    );

    let score2 = fuzzy_match("HELLO", "hello");
    assert!(
        (score2 - 1.0).abs() < 1e-6,
        "Expected 1.0 for uppercase match, got {}",
        score2
    );
}

#[test]
fn test_fuzzy_match_contains() {
    // Contains should return high score (0.8+)
    let score = fuzzy_match("test", "this is a test file");
    assert!(
        score >= 0.8,
        "Expected >= 0.8 for contains match, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_typo_single_char() {
    // Single character typo should return high score
    let score = fuzzy_match("hello", "helo");
    assert!(score > 0.7, "Expected > 0.7 for single typo, got {}", score);

    let score2 = fuzzy_match("test", "tset");
    assert!(
        score2 >= 0.5,
        "Expected > 0.5 for transposition, got {}",
        score2
    );
}

#[test]
fn test_fuzzy_match_typo_two_chars() {
    // Two character typo should return moderate score
    let score = fuzzy_match("hello", "hllo");
    assert!(
        score > 0.5 && score < 1.0,
        "Expected moderate score for two-char diff, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_completely_different() {
    // Completely different strings should return low score
    let score = fuzzy_match("hello", "world");
    assert!(
        score < 0.5,
        "Expected < 0.5 for different strings, got {}",
        score
    );

    let score2 = fuzzy_match("abc", "xyz");
    assert!(
        score2 < 0.3,
        "Expected < 0.3 for completely different, got {}",
        score2
    );
}

#[test]
fn test_fuzzy_match_empty_strings() {
    // Both empty should return 1.0
    let score = fuzzy_match("", "");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for both empty, got {}",
        score
    );

    // One empty should return 0.0
    let score2 = fuzzy_match("", "hello");
    assert_eq!(score2, 0.0, "Expected 0.0 when query is empty");

    let score3 = fuzzy_match("hello", "");
    assert_eq!(score3, 0.0, "Expected 0.0 when text is empty");
}

#[test]
fn test_fuzzy_match_prefix_match() {
    // Prefix match should have decent score
    let score = fuzzy_match("test", "testing");
    assert!(
        score > 0.5,
        "Expected > 0.5 for prefix match, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_suffix_match() {
    // Suffix match should have decent score
    let score = fuzzy_match("ing", "testing");
    assert!(
        score >= 0.8,
        "Expected >= 0.8 for suffix contained, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_unicode() {
    // Unicode strings should work
    let score = fuzzy_match("café", "café");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for unicode exact match, got {}",
        score
    );

    let score2 = fuzzy_match("日本語", "日本語");
    assert!(
        (score2 - 1.0).abs() < 1e-6,
        "Expected 1.0 for Japanese match, got {}",
        score2
    );
}

#[test]
fn test_fuzzy_match_numbers() {
    // Numbers should work
    let score = fuzzy_match("123", "123");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for number match, got {}",
        score
    );

    let score2 = fuzzy_match("123", "1234");
    assert!(
        score2 > 0.7,
        "Expected > 0.7 for close number, got {}",
        score2
    );
}

#[test]
fn test_fuzzy_match_special_chars() {
    // Special characters should work
    let score = fuzzy_match("test@example.com", "test@example.com");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for email match, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_whitespace() {
    // Whitespace handling
    let score = fuzzy_match("hello world", "hello world");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for exact with space, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_long_strings() {
    // Longer strings with minor differences
    let s1 = "the quick brown fox jumps over the lazy dog";
    let s2 = "the quick brown fox jumps over the lazy cat";

    let score = fuzzy_match(s1, s2);
    assert!(
        score > 0.9,
        "Expected > 0.9 for minor difference in long string, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_single_char() {
    // Single character strings
    let score = fuzzy_match("a", "a");
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Expected 1.0 for single char match"
    );

    let score2 = fuzzy_match("a", "b");
    assert_eq!(score2, 0.0, "Expected 0.0 for different single chars");
}

#[test]
fn test_fuzzy_match_symmetry() {
    // Levenshtein distance should be symmetric
    let score1 = fuzzy_match("hello", "helo");
    let score2 = fuzzy_match("helo", "hello");
    // Note: scores might differ slightly due to length normalization
    let diff = (score1 - score2).abs();
    assert!(diff < 0.2, "Expected symmetric scores, diff was {}", diff);
}

#[test]
fn test_fuzzy_match_insertion() {
    // Insertion edit
    let score = fuzzy_match("helo", "hello");
    assert!(
        score > 0.7,
        "Expected > 0.7 for one insertion, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_deletion() {
    // Deletion edit
    let score = fuzzy_match("hello", "helo");
    assert!(
        score > 0.7,
        "Expected > 0.7 for one deletion, got {}",
        score
    );
}

#[test]
fn test_fuzzy_match_substitution() {
    // Substitution edit
    let score = fuzzy_match("hello", "hallo");
    assert!(
        score > 0.7,
        "Expected > 0.7 for one substitution, got {}",
        score
    );
}

// Tests for fuzzy_find_best_match

#[test]
fn test_fuzzy_find_best_match_exact() {
    let (score, matched) = fuzzy_find_best_match("test", "this is a test");
    assert_eq!(score, 1.0, "Expected 1.0 for exact word match");
    assert_eq!(matched, Some("test".to_string()));
}

#[test]
fn test_fuzzy_find_best_match_typo() {
    let (score, matched) = fuzzy_find_best_match("tset", "test file");
    assert!(
        score >= 0.5,
        "Expected >= 0.5 for typo match, got {}",
        score
    );
    assert!(matched.is_some());
}

#[test]
fn test_fuzzy_find_best_match_no_match() {
    let (score, _) = fuzzy_find_best_match("xyz", "abc def");
    assert!(
        score < 0.5,
        "Expected low score for no match, got {}",
        score
    );
}

#[test]
fn test_fuzzy_find_best_match_empty() {
    let (score, matched) = fuzzy_find_best_match("", "test");
    assert_eq!(score, 0.0);
    assert_eq!(matched, None);

    let (score2, matched2) = fuzzy_find_best_match("test", "");
    assert_eq!(score2, 0.0);
    assert_eq!(matched2, None);
}

#[test]
fn test_fuzzy_find_best_match_best_word() {
    // Should find the best matching word
    let (score, matched) = fuzzy_find_best_match("hello", "hi hello world");
    assert_eq!(score, 1.0);
    // The matched word could be "hello" from the contains check
    assert!(matched.is_some());
}

#[test]
fn test_fuzzy_match_score_ordering() {
    // More similar strings should have higher scores
    let exact = fuzzy_match("hello", "hello");
    let one_off = fuzzy_match("hello", "hallo");
    let two_off = fuzzy_match("hello", "hxllo");
    let different = fuzzy_match("hello", "world");

    assert!(exact > one_off, "Exact should score higher than one-off");
    assert!(one_off >= two_off, "One-off should score >= two-off");
    assert!(
        two_off > different,
        "Two-off should score higher than different"
    );
}

#[test]
fn test_fuzzy_match_common_typos() {
    // Common typing mistakes - should have decent similarity
    // The Levenshtein-based fuzzy match returns lower scores for longer strings
    let score1 = fuzzy_match("receive", "recieve");
    assert!(
        score1 > 0.5,
        "Expected receive/recieve > 0.5, got {}",
        score1
    );

    let score2 = fuzzy_match("separate", "seperate");
    assert!(
        score2 > 0.5,
        "Expected separate/seperate > 0.5, got {}",
        score2
    );

    let score3 = fuzzy_match("definitely", "definately");
    assert!(
        score3 > 0.5,
        "Expected definitely/definately > 0.5, got {}",
        score3
    );
}

#[test]
fn test_fuzzy_match_path_components() {
    // Matching path-like strings
    let score = fuzzy_match("mod.rs", "mod.rs");
    assert!((score - 1.0).abs() < 1e-6);

    let score2 = fuzzy_match("search", "searcher");
    assert!(
        score2 > 0.7,
        "Expected > 0.7 for prefix match, got {}",
        score2
    );
}
