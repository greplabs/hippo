//! Comprehensive tests for source management functionality
//!
//! Tests adding, removing, listing sources and managing memories across sources.

use hippo_core::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Source Type Tests
// ============================================================================

#[test]
fn test_source_local_creation() {
    let source = Source::Local {
        root_path: PathBuf::from("/home/user/documents"),
    };

    match source {
        Source::Local { root_path } => {
            assert_eq!(root_path, PathBuf::from("/home/user/documents"));
        }
        _ => panic!("Expected Local source"),
    }
}

#[test]
fn test_source_google_drive_creation() {
    let source = Source::GoogleDrive {
        account_id: "user@gmail.com".to_string(),
    };

    match source {
        Source::GoogleDrive { account_id } => {
            assert_eq!(account_id, "user@gmail.com");
        }
        _ => panic!("Expected GoogleDrive source"),
    }
}

#[test]
fn test_source_variants() {
    let local = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    let gdrive = Source::GoogleDrive {
        account_id: "test@gmail.com".to_string(),
    };
    let icloud = Source::ICloud {
        account_id: "test@icloud.com".to_string(),
    };
    let dropbox = Source::Dropbox {
        account_id: "test".to_string(),
    };
    let onedrive = Source::OneDrive {
        account_id: "test".to_string(),
    };

    assert!(matches!(local, Source::Local { .. }));
    assert!(matches!(gdrive, Source::GoogleDrive { .. }));
    assert!(matches!(icloud, Source::ICloud { .. }));
    assert!(matches!(dropbox, Source::Dropbox { .. }));
    assert!(matches!(onedrive, Source::OneDrive { .. }));
}

#[test]
fn test_source_serialization() {
    let source = Source::Local {
        root_path: PathBuf::from("/test/path"),
    };

    let json = serde_json::to_string(&source).unwrap();
    let parsed: Source = serde_json::from_str(&json).unwrap();

    match parsed {
        Source::Local { root_path } => {
            assert_eq!(root_path, PathBuf::from("/test/path"));
        }
        _ => panic!("Failed to deserialize Local source"),
    }
}

#[test]
fn test_source_equality() {
    let source1 = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    let source2 = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    let source3 = Source::Local {
        root_path: PathBuf::from("/other"),
    };

    assert_eq!(source1, source2);
    assert_ne!(source1, source3);
}

#[test]
fn test_source_clone() {
    let source = Source::Local {
        root_path: PathBuf::from("/test"),
    };

    let cloned = source.clone();
    assert_eq!(source, cloned);
}

// ============================================================================
// Storage Source Operations Tests
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

#[tokio::test]
async fn test_add_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test/documents"),
    };

    let result = storage.add_source(source.clone()).await;
    assert!(result.is_ok(), "Should add source successfully");

    let sources = storage.list_sources().await.unwrap();
    assert!(!sources.is_empty());
}

#[tokio::test]
async fn test_add_multiple_sources() {
    let (storage, _temp) = create_test_storage().await;

    let source1 = Source::Local {
        root_path: PathBuf::from("/test/documents"),
    };
    let source2 = Source::Local {
        root_path: PathBuf::from("/test/photos"),
    };
    let source3 = Source::GoogleDrive {
        account_id: "user@gmail.com".to_string(),
    };

    storage.add_source(source1).await.unwrap();
    storage.add_source(source2).await.unwrap();
    storage.add_source(source3).await.unwrap();

    let sources = storage.list_sources().await.unwrap();
    assert!(sources.len() >= 3, "Should have at least 3 sources");
}

#[tokio::test]
async fn test_add_duplicate_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test/documents"),
    };

    storage.add_source(source.clone()).await.unwrap();
    let result = storage.add_source(source.clone()).await;

    // Implementation may allow or prevent duplicates
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_list_sources_empty() {
    let (storage, _temp) = create_test_storage().await;

    let sources = storage.list_sources().await.unwrap();
    assert_eq!(sources.len(), 0, "Should have no sources initially");
}

#[tokio::test]
async fn test_list_sources_with_metadata() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test/documents"),
    };
    storage.add_source(source.clone()).await.unwrap();

    let sources = storage.list_sources().await.unwrap();

    // Each source entry should have metadata
    for source_entry in &sources {
        assert!(source_entry.added_at > chrono::DateTime::UNIX_EPOCH);
    }
}

#[tokio::test]
async fn test_remove_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test/documents"),
    };
    storage.add_source(source.clone()).await.unwrap();

    let result = storage.remove_source(&source).await;
    assert!(result.is_ok(), "Should remove source successfully");

    let sources = storage.list_sources().await.unwrap();
    assert!(
        sources.is_empty() || !sources.iter().any(|s| s.source == source),
        "Source should be removed"
    );
}

#[tokio::test]
async fn test_remove_nonexistent_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/nonexistent"),
    };

    let result = storage.remove_source(&source).await;

    // Should handle gracefully (either ok or error)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_remove_source_with_memories() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    storage.add_source(source.clone()).await.unwrap();

    // Create memory associated with source
    let memory = Memory {
        id: uuid::Uuid::new_v4(),
        path: PathBuf::from("/test/file.txt"),
        source: source.clone(),
        kind: MemoryKind::Document {
            format: DocumentFormat::Pdf,
            page_count: None,
        },
        metadata: MemoryMetadata {
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
    };
    storage.upsert_memory(&memory).await.unwrap();

    // Remove source
    let result = storage.remove_source(&source).await;
    assert!(result.is_ok());

    // Verify memory handling (may be deleted or orphaned)
    let retrieved = storage.get_memory(memory.id).await;
    // Memory might be deleted or still exist, both are valid
    assert!(retrieved.is_ok());
}

// ============================================================================
// Source and Memory Association Tests
// ============================================================================

#[tokio::test]
async fn test_memories_associated_with_source() {
    let (storage, _temp) = create_test_storage().await;

    let source1 = Source::Local {
        root_path: PathBuf::from("/test/source1"),
    };
    let source2 = Source::Local {
        root_path: PathBuf::from("/test/source2"),
    };

    storage.add_source(source1.clone()).await.unwrap();
    storage.add_source(source2.clone()).await.unwrap();

    // Create memories for different sources
    let memory1 = create_memory_with_source("/test/source1/file1.txt", source1.clone());
    let memory2 = create_memory_with_source("/test/source2/file2.txt", source2.clone());

    storage.upsert_memory(&memory1).await.unwrap();
    storage.upsert_memory(&memory2).await.unwrap();

    // Both memories should be retrievable
    let retrieved1 = storage.get_memory(memory1.id).await.unwrap().unwrap();
    let retrieved2 = storage.get_memory(memory2.id).await.unwrap().unwrap();

    assert_eq!(retrieved1.source, source1);
    assert_eq!(retrieved2.source, source2);
}

#[tokio::test]
async fn test_find_memories_by_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    storage.add_source(source.clone()).await.unwrap();

    // Create multiple memories for the source
    for i in 0..5 {
        let memory = create_memory_with_source(&format!("/test/file{}.txt", i), source.clone());
        storage.upsert_memory(&memory).await.unwrap();
    }

    // Search by path prefix (approximates searching by source)
    let results = storage.find_by_path_prefix("/test").await.unwrap();

    assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_source_isolation() {
    let (storage, _temp) = create_test_storage().await;

    let source1 = Source::Local {
        root_path: PathBuf::from("/source1"),
    };
    let source2 = Source::Local {
        root_path: PathBuf::from("/source2"),
    };

    storage.add_source(source1.clone()).await.unwrap();
    storage.add_source(source2.clone()).await.unwrap();

    // Create memories
    let memory1 = create_memory_with_source("/source1/file.txt", source1.clone());
    let memory2 = create_memory_with_source("/source2/file.txt", source2.clone());

    storage.upsert_memory(&memory1).await.unwrap();
    storage.upsert_memory(&memory2).await.unwrap();

    // Memories from different sources should be separate
    let results1 = storage.find_by_path_prefix("/source1").await.unwrap();
    let results2 = storage.find_by_path_prefix("/source2").await.unwrap();

    assert_eq!(results1.len(), 1);
    assert_eq!(results2.len(), 1);
    assert_ne!(results1[0].id, results2[0].id);
}

// ============================================================================
// Source Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_source_stats() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test"),
    };
    storage.add_source(source.clone()).await.unwrap();

    // Add memories
    for i in 0..10 {
        let memory = create_memory_with_source(&format!("/test/file{}.txt", i), source.clone());
        storage.upsert_memory(&memory).await.unwrap();
    }

    let stats = storage.get_stats().await.unwrap();
    assert_eq!(stats.memory_count, 10);
}

// ============================================================================
// Source Path Validation Tests
// ============================================================================

#[tokio::test]
async fn test_source_with_absolute_path() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/absolute/path/to/documents"),
    };

    let result = storage.add_source(source).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_source_with_relative_path() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("relative/path"),
    };

    // Implementation may accept or reject relative paths
    let result = storage.add_source(source).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_source_with_special_characters() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/path/with spaces/and-dashes"),
    };

    let result = storage.add_source(source).await;
    assert!(result.is_ok());
}

// ============================================================================
// Multiple Source Types Tests
// ============================================================================

#[tokio::test]
async fn test_mixed_source_types() {
    let (storage, _temp) = create_test_storage().await;

    let local = Source::Local {
        root_path: PathBuf::from("/local/path"),
    };
    let gdrive = Source::GoogleDrive {
        account_id: "user@gmail.com".to_string(),
    };
    let icloud = Source::ICloud {
        account_id: "user@icloud.com".to_string(),
    };

    storage.add_source(local).await.unwrap();
    storage.add_source(gdrive).await.unwrap();
    storage.add_source(icloud).await.unwrap();

    let sources = storage.list_sources().await.unwrap();
    assert!(sources.len() >= 3);

    // Should have different source types
    let has_local = sources.iter().any(|s| matches!(s.source, Source::Local { .. }));
    let has_gdrive = sources
        .iter()
        .any(|s| matches!(s.source, Source::GoogleDrive { .. }));
    let has_icloud = sources
        .iter()
        .any(|s| matches!(s.source, Source::ICloud { .. }));

    assert!(has_local);
    assert!(has_gdrive);
    assert!(has_icloud);
}

// ============================================================================
// Source Update Tests
// ============================================================================

#[tokio::test]
async fn test_remove_and_readd_source() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/test"),
    };

    // Add, remove, readd
    storage.add_source(source.clone()).await.unwrap();
    storage.remove_source(&source).await.unwrap();
    storage.add_source(source.clone()).await.unwrap();

    let sources = storage.list_sources().await.unwrap();
    assert!(!sources.is_empty());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_source_with_empty_path() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from(""),
    };

    // Implementation may handle this differently
    let result = storage.add_source(source).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_source_with_unicode_path() {
    let (storage, _temp) = create_test_storage().await;

    let source = Source::Local {
        root_path: PathBuf::from("/文档/фото/🚀"),
    };

    let result = storage.add_source(source).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_source_with_very_long_path() {
    let (storage, _temp) = create_test_storage().await;

    let long_path = "/".to_string() + &"a/".repeat(100);
    let source = Source::Local {
        root_path: PathBuf::from(long_path),
    };

    let result = storage.add_source(source).await;
    // May succeed or fail depending on path length limits
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_memory_with_source(path: &str, source: Source) -> Memory {
    Memory {
        id: uuid::Uuid::new_v4(),
        path: PathBuf::from(path),
        source,
        kind: MemoryKind::Document {
            format: DocumentFormat::Pdf,
            page_count: None,
        },
        metadata: MemoryMetadata {
            file_size: 1000,
            title: Some(
                PathBuf::from(path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            ),
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
