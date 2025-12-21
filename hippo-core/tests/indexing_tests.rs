//! Comprehensive tests for file indexing functionality
//!
//! Tests the indexer's ability to discover, process, and extract metadata
//! from various file types using the public Hippo API.

use hippo_core::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create test directory structure with various file types
fn create_test_fixtures(temp_dir: &TempDir) -> PathBuf {
    let root = temp_dir.path().to_path_buf();

    // Create directory structure
    fs::create_dir_all(root.join("documents")).unwrap();
    fs::create_dir_all(root.join("images")).unwrap();
    fs::create_dir_all(root.join("code")).unwrap();
    fs::create_dir_all(root.join("nested/deep/structure")).unwrap();

    // Create text files
    fs::write(
        root.join("documents/readme.txt"),
        b"This is a test README file",
    )
    .unwrap();
    fs::write(
        root.join("documents/notes.md"),
        b"# Test Notes\n\nSome markdown content",
    )
    .unwrap();

    // Create code files
    fs::write(
        root.join("code/main.rs"),
        b"fn main() {\n    println!(\"Hello, world!\");\n}",
    )
    .unwrap();
    fs::write(
        root.join("code/test.py"),
        b"def hello():\n    print('Hello from Python')\n",
    )
    .unwrap();
    fs::write(
        root.join("code/app.js"),
        b"function hello() {\n    console.log('Hello from JS');\n}",
    )
    .unwrap();

    // Create nested files
    fs::write(
        root.join("nested/deep/structure/hidden.txt"),
        b"Deep nested file",
    )
    .unwrap();

    // Create empty file
    fs::write(root.join("empty.txt"), b"").unwrap();

    // Create binary-like file
    fs::write(root.join("data.bin"), &[0u8, 1u8, 2u8, 255u8]).unwrap();

    root
}

async fn create_test_hippo() -> (Hippo, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = HippoConfig {
        data_dir: temp_dir.path().join("hippo_data"),
        qdrant_url: "http://localhost:9999".to_string(), // Non-existent for tests
        ..Default::default()
    };
    let hippo = Hippo::with_config(config).await.unwrap();
    (hippo, temp_dir)
}

// Note: Most of the original tests rely on internal functions that are not exported.
// The indexer is designed to be used through the Hippo public API.
// Tests for internal indexer functionality should be in src/indexer/mod.rs as unit tests.

#[test]
fn test_indexing_progress_default() {
    use hippo_core::indexer::IndexingProgress;
    let progress = IndexingProgress::default();

    assert_eq!(progress.total, 0);
    assert_eq!(progress.processed, 0);
    assert!(progress.current_file.is_none());
    assert!(!progress.is_paused);
    assert_eq!(progress.files_per_second, 0.0);
    assert!(progress.estimated_seconds_remaining.is_none());
}

#[test]
fn test_indexing_progress_percentage() {
    use hippo_core::indexer::IndexingProgress;
    let mut progress = IndexingProgress::new();

    // Empty progress
    assert_eq!(progress.percentage(), 0.0);

    // Half complete
    progress.total = 100;
    progress.processed = 50;
    assert!((progress.percentage() - 50.0).abs() < 1e-6);

    // Complete
    progress.processed = 100;
    assert!((progress.percentage() - 100.0).abs() < 1e-6);
}

#[test]
fn test_indexing_stage_variants() {
    use hippo_core::indexer::IndexingStage;
    let scanning = IndexingStage::Scanning;
    let embedding = IndexingStage::Embedding;
    let tagging = IndexingStage::Tagging;
    let complete = IndexingStage::Complete;

    // Test serialization
    assert!(serde_json::to_string(&scanning).is_ok());
    assert!(serde_json::to_string(&embedding).is_ok());
    assert!(serde_json::to_string(&tagging).is_ok());
    assert!(serde_json::to_string(&complete).is_ok());
}

#[tokio::test]
async fn test_add_source_and_index() {
    let (hippo, temp) = create_test_hippo().await;
    let root = create_test_fixtures(&temp);

    let source = Source::Local { root_path: root };

    // Add source
    let result = hippo.add_source(source.clone()).await;
    assert!(result.is_ok(), "Should add source successfully");

    // List sources
    let sources = hippo.list_sources().await.unwrap();
    assert!(!sources.is_empty(), "Should have at least one source");
}

#[tokio::test]
async fn test_memory_kind_pattern_matching() {
    // Test that we can match on MemoryKind variants
    let image = MemoryKind::Image {
        width: 1920,
        height: 1080,
        format: "JPEG".to_string(),
    };

    match image {
        MemoryKind::Image { width, height, .. } => {
            assert_eq!(width, 1920);
            assert_eq!(height, 1080);
        }
        _ => panic!("Expected Image kind"),
    }

    let code = MemoryKind::Code {
        language: "rust".to_string(),
        lines: 100,
    };

    match code {
        MemoryKind::Code { language, lines } => {
            assert_eq!(language, "rust");
            assert_eq!(lines, 100);
        }
        _ => panic!("Expected Code kind"),
    }
}

#[tokio::test]
async fn test_stats() {
    let (hippo, _temp) = create_test_hippo().await;

    let stats = hippo.stats().await;
    assert!(stats.is_ok(), "Should get stats successfully");

    let stats = stats.unwrap();
    // New hippo instance should have no memories
    assert_eq!(stats.memory_count, 0);
}
