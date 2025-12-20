//! Tests for advanced filter capabilities

use chrono::{Duration, Utc};
use hippo_core::search::{FilterBuilder, MatchMode};
use hippo_core::{DocumentFormat, Memory, MemoryKind, MemoryMetadata, Source, Tag};
use std::path::PathBuf;
use uuid::Uuid;

// Helper function to create test memories
fn create_test_memory(
    file_name: &str,
    size: u64,
    extension: Option<&str>,
    modified_days_ago: i64,
) -> Memory {
    let now = Utc::now();
    let modified = now - Duration::days(modified_days_ago);

    let path = if let Some(ext) = extension {
        PathBuf::from(format!("/test/{}.{}", file_name, ext))
    } else {
        PathBuf::from(format!("/test/{}", file_name))
    };

    Memory {
        id: Uuid::new_v4(),
        path,
        source: Source::Local {
            root_path: PathBuf::from("/test"),
        },
        kind: MemoryKind::Document {
            format: DocumentFormat::Pdf,
            page_count: None,
        },
        metadata: MemoryMetadata {
            title: Some(file_name.to_string()),
            file_size: size,
            ..Default::default()
        },
        tags: vec![],
        embedding_id: None,
        connections: vec![],
        is_favorite: false,
        created_at: modified,
        modified_at: modified,
        indexed_at: now,
    }
}

fn create_memory_with_hash(file_name: &str, size: u64, hash: &str) -> Memory {
    let mut memory = create_test_memory(file_name, size, Some("txt"), 0);
    memory.metadata.hash = Some(hash.to_string());
    memory
}

fn create_memory_with_content(file_name: &str, title: &str, description: &str) -> Memory {
    let mut memory = create_test_memory(file_name, 1000, Some("txt"), 0);
    memory.metadata.title = Some(title.to_string());
    memory.metadata.description = Some(description.to_string());
    memory
}

#[test]
fn test_filter_builder_min_size() {
    let filter = FilterBuilder::new().min_size(1000).build();

    let memories = vec![
        create_test_memory("small", 500, Some("txt"), 0),
        create_test_memory("medium", 1000, Some("txt"), 0),
        create_test_memory("large", 2000, Some("txt"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|m| m.metadata.file_size >= 1000));
}

#[test]
fn test_filter_builder_max_size() {
    let filter = FilterBuilder::new().max_size(1500).build();

    let memories = vec![
        create_test_memory("small", 500, Some("txt"), 0),
        create_test_memory("medium", 1000, Some("txt"), 0),
        create_test_memory("large", 2000, Some("txt"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|m| m.metadata.file_size <= 1500));
}

#[test]
fn test_filter_builder_size_range() {
    let filter = FilterBuilder::new().size_range(800, 1500).build();

    let memories = vec![
        create_test_memory("small", 500, Some("txt"), 0),
        create_test_memory("medium", 1000, Some("txt"), 0),
        create_test_memory("large", 2000, Some("txt"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.file_size, 1000);
}

#[test]
fn test_filter_builder_date_range_after() {
    let now = Utc::now();
    let threshold = now - Duration::days(5);
    let filter = FilterBuilder::new().after(threshold).build();

    let memories = vec![
        create_test_memory("recent", 1000, Some("txt"), 2),
        create_test_memory("old", 1000, Some("txt"), 10),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "recent");
}

#[test]
fn test_filter_builder_date_range_before() {
    let now = Utc::now();
    let threshold = now - Duration::days(5);
    let filter = FilterBuilder::new().before(threshold).build();

    let memories = vec![
        create_test_memory("recent", 1000, Some("txt"), 2),
        create_test_memory("old", 1000, Some("txt"), 10),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "old");
}

#[test]
fn test_filter_builder_date_range_between() {
    let now = Utc::now();
    let start = now - Duration::days(15);
    let end = now - Duration::days(5);
    let filter = FilterBuilder::new()
        .date_range(Some(start), Some(end))
        .build();

    let memories = vec![
        create_test_memory("very_old", 1000, Some("txt"), 20),
        create_test_memory("old", 1000, Some("txt"), 10),
        create_test_memory("recent", 1000, Some("txt"), 2),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "old");
}

#[test]
fn test_extension_whitelist() {
    let filter = FilterBuilder::new()
        .extensions(vec!["jpg".to_string(), "png".to_string()])
        .build();

    let memories = vec![
        create_test_memory("image1", 1000, Some("jpg"), 0),
        create_test_memory("image2", 1000, Some("png"), 0),
        create_test_memory("doc", 1000, Some("pdf"), 0),
        create_test_memory("code", 1000, Some("rs"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|m| {
        let ext = m.path.extension().and_then(|e| e.to_str()).unwrap();
        ext == "jpg" || ext == "png"
    }));
}

#[test]
fn test_extension_blacklist() {
    let filter = FilterBuilder::new()
        .exclude_extensions(vec!["tmp".to_string(), "log".to_string()])
        .build();

    let memories = vec![
        create_test_memory("data", 1000, Some("txt"), 0),
        create_test_memory("temp", 1000, Some("tmp"), 0),
        create_test_memory("debug", 1000, Some("log"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "data");
}

#[test]
fn test_extension_normalization() {
    // Test that extensions are normalized (case-insensitive, leading dot removed)
    let filter = FilterBuilder::new()
        .extensions(vec![".JPG".to_string(), "PNG".to_string()])
        .build();

    let memories = vec![
        create_test_memory("image1", 1000, Some("jpg"), 0),
        create_test_memory("image2", 1000, Some("PNG"), 0),
        create_test_memory("doc", 1000, Some("pdf"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_content_contains_in_title() {
    let filter = FilterBuilder::new()
        .content_contains("vacation".to_string())
        .build();

    let memories = vec![
        create_memory_with_content("file1", "vacation photos", ""),
        create_memory_with_content("file2", "work documents", ""),
        create_memory_with_content("file3", "summer Vacation", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_content_contains_in_description() {
    let filter = FilterBuilder::new()
        .content_contains("important".to_string())
        .build();

    let memories = vec![
        create_memory_with_content("file1", "random", "very important document"),
        create_memory_with_content("file2", "test", "just a test file"),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "random");
}

#[test]
fn test_content_contains_case_insensitive() {
    let filter = FilterBuilder::new()
        .content_contains("IMPORTANT".to_string())
        .build();

    let memories = vec![
        create_memory_with_content("file1", "Important stuff", ""),
        create_memory_with_content("file2", "random", "not important"),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_metadata_match_exact() {
    let filter = FilterBuilder::new()
        .metadata_match("title".to_string(), "test".to_string(), MatchMode::Exact)
        .build();

    let memories = vec![
        create_memory_with_content("file1", "test", ""),
        create_memory_with_content("file2", "testing", ""),
        create_memory_with_content("file3", "Test", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_metadata_match_contains() {
    let filter = FilterBuilder::new()
        .metadata_match("title".to_string(), "test".to_string(), MatchMode::Contains)
        .build();

    let memories = vec![
        create_memory_with_content("file1", "my test file", ""),
        create_memory_with_content("file2", "testing phase", ""),
        create_memory_with_content("file3", "production", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_metadata_match_starts_with() {
    let filter = FilterBuilder::new()
        .metadata_match(
            "title".to_string(),
            "test".to_string(),
            MatchMode::StartsWith,
        )
        .build();

    let memories = vec![
        create_memory_with_content("file1", "test file", ""),
        create_memory_with_content("file2", "my test", ""),
        create_memory_with_content("file3", "Testing phase", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_metadata_match_ends_with() {
    let filter = FilterBuilder::new()
        .metadata_match("title".to_string(), "file".to_string(), MatchMode::EndsWith)
        .build();

    let memories = vec![
        create_memory_with_content("file1", "test file", ""),
        create_memory_with_content("file2", "file data", ""),
        create_memory_with_content("file3", "data File", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_metadata_match_regex() {
    let filter = FilterBuilder::new()
        .metadata_match(
            "title".to_string(),
            "test".to_string(),
            MatchMode::Regex(r"^test\d+$".to_string()),
        )
        .build();

    let memories = vec![
        create_memory_with_content("file1", "test123", ""),
        create_memory_with_content("file2", "test", ""),
        create_memory_with_content("file3", "test456", ""),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_duplicates_only() {
    let filter = FilterBuilder::new().duplicates_only().build();

    let memories = vec![
        create_memory_with_hash("file1", 1000, "hash_a"),
        create_memory_with_hash("file2", 1000, "hash_a"),
        create_memory_with_hash("file3", 2000, "hash_b"),
        create_memory_with_hash("file4", 2000, "hash_b"),
        create_memory_with_hash("file5", 3000, "hash_c"),
    ];

    let filtered = filter.apply_filters(memories);
    // Should have 4 files (2 with hash_a, 2 with hash_b)
    assert_eq!(filtered.len(), 4);

    // Verify file5 (unique hash) is not included
    assert!(!filtered
        .iter()
        .any(|m| m.metadata.title.as_ref().unwrap() == "file5"));
}

#[test]
fn test_min_duplicate_count() {
    let filter = FilterBuilder::new().min_duplicate_count(3).build();

    let memories = vec![
        create_memory_with_hash("file1", 1000, "hash_a"),
        create_memory_with_hash("file2", 1000, "hash_a"),
        create_memory_with_hash("file3", 1000, "hash_a"),
        create_memory_with_hash("file4", 2000, "hash_b"),
        create_memory_with_hash("file5", 2000, "hash_b"),
    ];

    let filtered = filter.apply_filters(memories);
    // Only hash_a group has 3 or more files
    assert_eq!(filtered.len(), 3);
    assert!(filtered.iter().all(|m| m.metadata.file_size == 1000));
}

#[test]
fn test_combined_filters() {
    let now = Utc::now();
    let start_date = now - Duration::days(10);

    let filter = FilterBuilder::new()
        .min_size(500)
        .max_size(1500)
        .after(start_date)
        .extensions(vec!["txt".to_string(), "md".to_string()])
        .content_contains("important".to_string())
        .build();

    let mut memories = vec![
        create_test_memory("small_old", 300, Some("txt"), 15),
        create_test_memory("good", 1000, Some("txt"), 5),
        create_test_memory("large", 2000, Some("txt"), 5),
        create_test_memory("wrong_ext", 1000, Some("pdf"), 5),
        create_test_memory("no_content", 1000, Some("txt"), 5),
    ];

    // Set content for relevant files
    memories[1].metadata.title = Some("important file".to_string());

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].metadata.title.as_ref().unwrap(),
        "important file"
    );
}

#[test]
fn test_filter_builder_fluent_api() {
    // Test that the builder pattern works correctly with method chaining
    let filter = FilterBuilder::new()
        .min_size(100)
        .max_size(1000)
        .extensions(vec!["rs".to_string()])
        .content_contains("test".to_string())
        .build();

    assert!(filter.file_size_range.is_some());
    assert!(filter.extension_filter.is_some());
    assert!(filter.content_contains.is_some());
}

#[test]
fn test_empty_filter() {
    let filter = FilterBuilder::new().build();

    let memories = vec![
        create_test_memory("file1", 100, Some("txt"), 0),
        create_test_memory("file2", 200, Some("pdf"), 5),
        create_test_memory("file3", 300, Some("rs"), 10),
    ];

    let count = memories.len();
    let filtered = filter.apply_filters(memories);

    // Empty filter should not filter anything
    assert_eq!(filtered.len(), count);
}

#[test]
fn test_no_matching_results() {
    let filter = FilterBuilder::new()
        .extensions(vec!["nonexistent".to_string()])
        .build();

    let memories = vec![
        create_test_memory("file1", 100, Some("txt"), 0),
        create_test_memory("file2", 200, Some("pdf"), 0),
    ];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_multiple_metadata_matches() {
    let filter = FilterBuilder::new()
        .metadata_match("title".to_string(), "test".to_string(), MatchMode::Contains)
        .metadata_match(
            "description".to_string(),
            "important".to_string(),
            MatchMode::Contains,
        )
        .build();

    let memories = vec![
        create_memory_with_content("file1", "test file", "important document"),
        create_memory_with_content("file2", "test file", "regular document"),
        create_memory_with_content("file3", "random", "important data"),
    ];

    let filtered = filter.apply_filters(memories);
    // Only file1 matches both conditions
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].metadata.title.as_ref().unwrap(), "test file");
    assert_eq!(
        filtered[0].metadata.description.as_ref().unwrap(),
        "important document"
    );
}

#[test]
fn test_filter_with_tags() {
    let filter = FilterBuilder::new()
        .content_contains("rust".to_string())
        .build();

    let mut memories = vec![
        create_test_memory("file1", 1000, Some("txt"), 0),
        create_test_memory("file2", 1000, Some("txt"), 0),
    ];

    memories[0].tags = vec![Tag::user("rust programming")];
    memories[1].tags = vec![Tag::user("python code")];

    let filtered = filter.apply_filters(memories);
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].tags.iter().any(|t| t.name.contains("rust")));
}
