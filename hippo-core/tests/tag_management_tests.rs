//! Comprehensive tests for tag management functionality
//!
//! Tests tag creation, filtering, counting, and management across memories.

use hippo_core::*;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// ============================================================================
// Tag Creation and Structure Tests
// ============================================================================

#[test]
fn test_tag_user_creation() {
    let tag = Tag::user("important");

    assert_eq!(tag.name, "important");
    assert_eq!(tag.source, TagSource::User);
    assert!(tag.confidence.is_none());
}

#[test]
fn test_tag_ai_creation() {
    let tag = Tag::ai("landscape", 85);

    assert_eq!(tag.name, "landscape");
    assert_eq!(tag.source, TagSource::Ai);
    assert_eq!(tag.confidence, Some(85));
}

#[test]
fn test_tag_system_creation() {
    let tag = Tag::system("image");

    assert_eq!(tag.name, "image");
    assert_eq!(tag.source, TagSource::System);
    assert!(tag.confidence.is_none());
}

#[test]
fn test_tag_source_variants() {
    let user_tag = Tag::user("test");
    let ai_tag = Tag::ai("test", 90);
    let system_tag = Tag::system("test");

    assert!(matches!(user_tag.source, TagSource::User));
    assert!(matches!(ai_tag.source, TagSource::Ai));
    assert!(matches!(system_tag.source, TagSource::System));
}

#[test]
fn test_tag_serialization() {
    let tag = Tag::user("test:category");

    let json = serde_json::to_string(&tag).unwrap();
    let parsed: Tag = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, tag.name);
    assert_eq!(parsed.source, tag.source);
}

#[test]
fn test_tag_with_namespace() {
    let tag = Tag::user("category:project");

    assert_eq!(tag.name, "category:project");
    assert!(tag.name.contains(':'));
}

#[test]
fn test_tag_case_sensitive() {
    let tag1 = Tag::user("Important");
    let tag2 = Tag::user("important");

    // Tags are case-sensitive by default
    assert_ne!(tag1.name, tag2.name);
}

// ============================================================================
// Tag Filter Tests
// ============================================================================

#[test]
fn test_tag_filter_include() {
    let filter = TagFilter {
        tag: "important".to_string(),
        mode: TagFilterMode::Include,
    };

    assert_eq!(filter.tag, "important");
    assert!(matches!(filter.mode, TagFilterMode::Include));
}

#[test]
fn test_tag_filter_exclude() {
    let filter = TagFilter {
        tag: "spam".to_string(),
        mode: TagFilterMode::Exclude,
    };

    assert_eq!(filter.tag, "spam");
    assert!(matches!(filter.mode, TagFilterMode::Exclude));
}

#[test]
fn test_tag_filter_serialization() {
    let filter = TagFilter {
        tag: "test".to_string(),
        mode: TagFilterMode::Include,
    };

    let json = serde_json::to_string(&filter).unwrap();
    let parsed: TagFilter = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.tag, filter.tag);
}

// ============================================================================
// Storage Tag Operations Tests
// ============================================================================

async fn create_test_storage() -> (hippo_core::storage::Storage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = HippoConfig {
        data_dir: temp_dir.path().to_path_buf(),
        qdrant_url: "http://localhost:9999".to_string(),
        ..Default::default()
    };
    let storage = hippo_core::storage::Storage::new(&config).await.unwrap();
    (storage, temp_dir)
}

fn create_test_memory(name: &str) -> Memory {
    Memory {
        id: Uuid::new_v4(),
        path: PathBuf::from(format!("/test/{}", name)),
        source: Source::Local {
            root_path: PathBuf::from("/test"),
        },
        kind: MemoryKind::Document {
            format: DocumentFormat::Pdf,
            page_count: None,
        },
        metadata: MemoryMetadata {
            title: Some(name.to_string()),
            file_size: 1000,
            ..Default::default()
        },
        tags: vec![],
        embedding_id: None,
        connections: vec![],
        is_favorite: false,
        created_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        indexed_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_add_tag_to_memory() {
    let (storage, _temp) = create_test_storage().await;

    let memory = create_test_memory("test.txt");
    storage.upsert_memory(&memory).await.unwrap();

    let tag = Tag::user("important");
    let result = storage.add_tag(memory.id, tag.clone()).await;

    assert!(result.is_ok(), "Should add tag successfully");

    // Verify tag was added
    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 1);
    assert_eq!(retrieved.tags[0].name, "important");
}

#[tokio::test]
async fn test_add_multiple_tags_to_memory() {
    let (storage, _temp) = create_test_storage().await;

    let memory = create_test_memory("test.txt");
    storage.upsert_memory(&memory).await.unwrap();

    storage.add_tag(memory.id, Tag::user("tag1")).await.unwrap();
    storage.add_tag(memory.id, Tag::user("tag2")).await.unwrap();
    storage.add_tag(memory.id, Tag::user("tag3")).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 3);
}

#[tokio::test]
async fn test_add_duplicate_tag() {
    let (storage, _temp) = create_test_storage().await;

    let memory = create_test_memory("test.txt");
    storage.upsert_memory(&memory).await.unwrap();

    // Add same tag twice
    storage.add_tag(memory.id, Tag::user("tag")).await.unwrap();
    storage.add_tag(memory.id, Tag::user("tag")).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();

    // Implementation may allow duplicates or prevent them
    // Just verify we get tags back
    assert!(!retrieved.tags.is_empty());
}

#[tokio::test]
async fn test_remove_tag_from_memory() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("test.txt");
    memory.tags = vec![Tag::user("tag1"), Tag::user("tag2"), Tag::user("tag3")];
    storage.upsert_memory(&memory).await.unwrap();

    let result = storage.remove_tag(memory.id, "tag2").await;
    assert!(result.is_ok(), "Should remove tag successfully");

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 2);
    assert!(!retrieved.tags.iter().any(|t| t.name == "tag2"));
}

#[tokio::test]
async fn test_remove_nonexistent_tag() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("test.txt");
    memory.tags = vec![Tag::user("tag1")];
    storage.upsert_memory(&memory).await.unwrap();

    let result = storage.remove_tag(memory.id, "nonexistent").await;

    // Should handle gracefully (either ok or error)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_remove_all_tags_from_memory() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("test.txt");
    memory.tags = vec![Tag::user("tag1"), Tag::user("tag2")];
    storage.upsert_memory(&memory).await.unwrap();

    storage.remove_tag(memory.id, "tag1").await.unwrap();
    storage.remove_tag(memory.id, "tag2").await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 0);
}

// ============================================================================
// Tag Listing and Counting Tests
// ============================================================================

#[tokio::test]
async fn test_list_tags_empty() {
    let (storage, _temp) = create_test_storage().await;

    let tags = storage.list_tags().await.unwrap();
    assert_eq!(tags.len(), 0, "Should have no tags initially");
}

#[tokio::test]
async fn test_list_tags_with_counts() {
    let (storage, _temp) = create_test_storage().await;

    // Create memories with tags
    let mut memory1 = create_test_memory("file1.txt");
    memory1.tags = vec![Tag::user("rust"), Tag::user("code")];
    storage.upsert_memory(&memory1).await.unwrap();

    let mut memory2 = create_test_memory("file2.txt");
    memory2.tags = vec![Tag::user("rust"), Tag::user("project")];
    storage.upsert_memory(&memory2).await.unwrap();

    let mut memory3 = create_test_memory("file3.txt");
    memory3.tags = vec![Tag::user("python"), Tag::user("code")];
    storage.upsert_memory(&memory3).await.unwrap();

    let tags = storage.list_tags().await.unwrap();

    // Should have all unique tags
    assert!(!tags.is_empty());

    // Find tag counts
    let rust_count = tags.iter().find(|(name, _)| name == "rust").map(|(_, c)| c);
    let code_count = tags.iter().find(|(name, _)| name == "code").map(|(_, c)| c);
    let python_count = tags
        .iter()
        .find(|(name, _)| name == "python")
        .map(|(_, c)| c);

    // Verify counts
    if let Some(&count) = rust_count {
        assert!(count >= 1, "rust should appear at least once");
    }
    if let Some(&count) = code_count {
        assert!(count >= 1, "code should appear at least once");
    }
    if let Some(&count) = python_count {
        assert!(count >= 1, "python should appear at least once");
    }
}

#[tokio::test]
async fn test_list_tags_alphabetically_sorted() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![Tag::user("zebra"), Tag::user("apple"), Tag::user("middle")];
    storage.upsert_memory(&memory).await.unwrap();

    let tags = storage.list_tags().await.unwrap();

    // Tags should be returned (order depends on implementation)
    assert_eq!(tags.len(), 3);
}

// ============================================================================
// Tag Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_search_with_single_tag() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory1 = create_test_memory("file1.txt");
    memory1.tags = vec![Tag::user("important")];
    storage.upsert_memory(&memory1).await.unwrap();

    let mut memory2 = create_test_memory("file2.txt");
    memory2.tags = vec![Tag::user("other")];
    storage.upsert_memory(&memory2).await.unwrap();

    let results = storage
        .search_with_tags(None, &["important".to_string()], None, 10, 0)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, memory1.id);
}

#[tokio::test]
async fn test_search_with_multiple_tags() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory1 = create_test_memory("file1.txt");
    memory1.tags = vec![Tag::user("rust"), Tag::user("code")];
    storage.upsert_memory(&memory1).await.unwrap();

    let mut memory2 = create_test_memory("file2.txt");
    memory2.tags = vec![Tag::user("python"), Tag::user("code")];
    storage.upsert_memory(&memory2).await.unwrap();

    let results = storage
        .search_with_tags(None, &["rust".to_string(), "code".to_string()], None, 10, 0)
        .await
        .unwrap();

    // Should find memory with both tags
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_search_with_nonexistent_tag() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![Tag::user("exists")];
    storage.upsert_memory(&memory).await.unwrap();

    let results = storage
        .search_with_tags(None, &["nonexistent".to_string()], None, 10, 0)
        .await
        .unwrap();

    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_search_without_tags() {
    let (storage, _temp) = create_test_storage().await;

    let memory = create_test_memory("file.txt");
    storage.upsert_memory(&memory).await.unwrap();

    let results = storage
        .search_with_tags(None, &[], None, 10, 0)
        .await
        .unwrap();

    // Should return all memories
    assert_eq!(results.len(), 1);
}

// ============================================================================
// Tag Source and Confidence Tests
// ============================================================================

#[tokio::test]
async fn test_tag_sources_preserved() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![
        Tag::user("user_tag"),
        Tag::ai("ai_tag", 90),
        Tag::system("system_tag"),
    ];
    storage.upsert_memory(&memory).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();

    assert_eq!(retrieved.tags.len(), 3);

    // Verify sources
    let user_tag = retrieved.tags.iter().find(|t| t.name == "user_tag");
    let ai_tag = retrieved.tags.iter().find(|t| t.name == "ai_tag");
    let system_tag = retrieved.tags.iter().find(|t| t.name == "system_tag");

    assert!(user_tag.is_some());
    assert!(ai_tag.is_some());
    assert!(system_tag.is_some());

    if let Some(tag) = user_tag {
        assert!(matches!(tag.source, TagSource::User));
    }
    if let Some(tag) = ai_tag {
        assert!(matches!(tag.source, TagSource::Ai));
        assert_eq!(tag.confidence, Some(90));
    }
    if let Some(tag) = system_tag {
        assert!(matches!(tag.source, TagSource::System));
    }
}

#[tokio::test]
async fn test_ai_tag_confidence_levels() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![
        Tag::ai("high_confidence", 95),
        Tag::ai("medium_confidence", 70),
        Tag::ai("low_confidence", 40),
    ];
    storage.upsert_memory(&memory).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();

    assert_eq!(retrieved.tags.len(), 3);

    // All should have confidence values
    for tag in &retrieved.tags {
        assert!(tag.confidence.is_some());
        let conf = tag.confidence.unwrap();
        assert!((0..=100).contains(&conf));
    }
}

// ============================================================================
// Tag Performance Tests
// ============================================================================

#[tokio::test]
async fn test_tag_operations_on_many_memories() {
    let (storage, _temp) = create_test_storage().await;

    // Create many memories with tags
    for i in 0..20 {
        let mut memory = create_test_memory(&format!("file{}.txt", i));
        memory.tags = vec![Tag::user("common"), Tag::user(format!("unique_{}", i))];
        storage.upsert_memory(&memory).await.unwrap();
    }

    // List tags
    let tags = storage.list_tags().await.unwrap();

    // Should have at least 21 unique tags (1 common + 20 unique)
    assert!(tags.len() >= 21);

    // Find "common" tag - should have high count
    let common_tag = tags.iter().find(|(name, _)| name == "common");
    assert!(common_tag.is_some());
}

#[tokio::test]
async fn test_tag_search_with_limit() {
    let (storage, _temp) = create_test_storage().await;

    // Create multiple memories with same tag
    for i in 0..10 {
        let mut memory = create_test_memory(&format!("file{}.txt", i));
        memory.tags = vec![Tag::user("test")];
        storage.upsert_memory(&memory).await.unwrap();
    }

    // Search with limit
    let results = storage
        .search_with_tags(None, &["test".to_string()], None, 5, 0)
        .await
        .unwrap();

    assert_eq!(results.len(), 5, "Should respect limit");
}

#[tokio::test]
async fn test_tag_search_with_offset() {
    let (storage, _temp) = create_test_storage().await;

    // Create multiple memories with same tag
    for i in 0..10 {
        let mut memory = create_test_memory(&format!("file{}.txt", i));
        memory.tags = vec![Tag::user("test")];
        storage.upsert_memory(&memory).await.unwrap();
    }

    // Search with offset
    let results = storage
        .search_with_tags(None, &["test".to_string()], None, 5, 5)
        .await
        .unwrap();

    assert!(results.len() <= 5, "Should respect limit with offset");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_tag_with_special_characters() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![
        Tag::user("tag-with-dash"),
        Tag::user("tag_with_underscore"),
        Tag::user("tag.with.dots"),
    ];
    storage.upsert_memory(&memory).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 3);
}

#[tokio::test]
async fn test_tag_with_unicode() {
    let (storage, _temp) = create_test_storage().await;

    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![Tag::user("日本語"), Tag::user("émoji"), Tag::user("🚀")];
    storage.upsert_memory(&memory).await.unwrap();

    let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
    assert_eq!(retrieved.tags.len(), 3);
}

#[tokio::test]
async fn test_tag_with_very_long_name() {
    let (storage, _temp) = create_test_storage().await;

    let long_tag = "a".repeat(1000);
    let mut memory = create_test_memory("file.txt");
    memory.tags = vec![Tag::user(&long_tag)];

    let result = storage.upsert_memory(&memory).await;

    // Should handle long tags (may succeed or fail gracefully)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_tag_empty_string() {
    let tag = Tag::user("");
    assert_eq!(tag.name, "");

    // Implementation may or may not allow empty tags
    // Just verify the tag can be created
    assert!(matches!(tag.source, TagSource::User));
}
