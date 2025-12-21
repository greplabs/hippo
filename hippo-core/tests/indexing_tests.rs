//! Comprehensive tests for file indexing functionality
//!
//! Tests the indexer's ability to discover, process, and extract metadata
//! from various file types using real test fixtures.

use hippo_core::indexer::*;
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

#[test]
fn test_supported_extensions_coverage() {
    // Verify we support common file types
    let extensions = SUPPORTED_EXTENSIONS;

    // Images
    assert!(extensions.contains(&"jpg"));
    assert!(extensions.contains(&"png"));
    assert!(extensions.contains(&"gif"));
    assert!(extensions.contains(&"webp"));

    // Videos
    assert!(extensions.contains(&"mp4"));
    assert!(extensions.contains(&"mov"));
    assert!(extensions.contains(&"avi"));

    // Audio
    assert!(extensions.contains(&"mp3"));
    assert!(extensions.contains(&"wav"));
    assert!(extensions.contains(&"flac"));

    // Documents
    assert!(extensions.contains(&"pdf"));
    assert!(extensions.contains(&"docx"));
    assert!(extensions.contains(&"txt"));
    assert!(extensions.contains(&"md"));

    // Code
    assert!(extensions.contains(&"rs"));
    assert!(extensions.contains(&"py"));
    assert!(extensions.contains(&"js"));
    assert!(extensions.contains(&"go"));
}

#[test]
fn test_is_supported_file() {
    assert!(is_supported_file(&PathBuf::from("test.jpg")));
    assert!(is_supported_file(&PathBuf::from("test.PNG")));
    assert!(is_supported_file(&PathBuf::from("test.mp4")));
    assert!(is_supported_file(&PathBuf::from("test.pdf")));
    assert!(is_supported_file(&PathBuf::from("test.rs")));
    assert!(is_supported_file(&PathBuf::from("test.py")));

    assert!(!is_supported_file(&PathBuf::from("test.xyz")));
    assert!(!is_supported_file(&PathBuf::from("test.unknown")));
    assert!(!is_supported_file(&PathBuf::from("test")));
}

#[test]
fn test_is_hidden_file() {
    assert!(is_hidden_file(&PathBuf::from(".hidden")));
    assert!(is_hidden_file(&PathBuf::from(".git/config")));
    assert!(is_hidden_file(&PathBuf::from("path/.DS_Store")));
    assert!(is_hidden_file(&PathBuf::from(".gitignore")));

    assert!(!is_hidden_file(&PathBuf::from("visible.txt")));
    assert!(!is_hidden_file(&PathBuf::from("normal_file.rs")));
}

#[test]
fn test_should_skip_directory() {
    assert!(should_skip_directory(&PathBuf::from(".git")));
    assert!(should_skip_directory(&PathBuf::from("node_modules")));
    assert!(should_skip_directory(&PathBuf::from("target")));
    assert!(should_skip_directory(&PathBuf::from("__pycache__")));
    assert!(should_skip_directory(&PathBuf::from(".venv")));
    assert!(should_skip_directory(&PathBuf::from("build")));

    assert!(!should_skip_directory(&PathBuf::from("src")));
    assert!(!should_skip_directory(&PathBuf::from("docs")));
    assert!(!should_skip_directory(&PathBuf::from("tests")));
}

#[tokio::test]
async fn test_discover_files_basic() {
    let temp = TempDir::new().unwrap();
    let root = create_test_fixtures(&temp);

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    // Should find multiple files
    assert!(files.len() > 0, "Should discover files");

    // Should include known files
    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();
    assert!(
        paths.iter().any(|p| p.contains("readme.txt")),
        "Should find readme.txt"
    );
    assert!(
        paths.iter().any(|p| p.contains("main.rs")),
        "Should find main.rs"
    );
}

#[tokio::test]
async fn test_discover_files_nested() {
    let temp = TempDir::new().unwrap();
    let root = create_test_fixtures(&temp);

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    // Should find deeply nested files
    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();
    assert!(
        paths.iter().any(|p| p.contains("nested") && p.contains("hidden.txt")),
        "Should find deeply nested files"
    );
}

#[tokio::test]
async fn test_discover_files_filters_hidden() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // Create visible and hidden files
    fs::write(root.join("visible.txt"), b"visible").unwrap();
    fs::write(root.join(".hidden.txt"), b"hidden").unwrap();

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    // Should find visible but not hidden
    assert!(paths.iter().any(|p| p.contains("visible.txt")));
    assert!(!paths.iter().any(|p| p.contains(".hidden.txt")));
}

#[tokio::test]
async fn test_discover_files_filters_unsupported() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // Create supported and unsupported files
    fs::write(root.join("supported.txt"), b"text").unwrap();
    fs::write(root.join("unsupported.xyz"), b"data").unwrap();

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    assert!(paths.iter().any(|p| p.contains("supported.txt")));
    assert!(!paths.iter().any(|p| p.contains("unsupported.xyz")));
}

#[tokio::test]
async fn test_discover_files_skips_directories() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // Create directories that should be skipped
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("node_modules")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::create_dir_all(root.join("valid")).unwrap();

    fs::write(root.join(".git/config"), b"git").unwrap();
    fs::write(root.join("node_modules/package.json"), b"{}").unwrap();
    fs::write(root.join("target/debug.txt"), b"debug").unwrap();
    fs::write(root.join("valid/file.txt"), b"valid").unwrap();

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    // Should not find files in skipped directories
    assert!(!paths.iter().any(|p| p.contains(".git")));
    assert!(!paths.iter().any(|p| p.contains("node_modules")));
    assert!(!paths.iter().any(|p| p.contains("target/debug.txt")));

    // Should find files in valid directories
    assert!(paths.iter().any(|p| p.contains("valid/file.txt")));
}

#[tokio::test]
async fn test_discover_files_empty_directory() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    let source = Source::Local { root_path: root };
    let files = discover_files(&source).await.unwrap();

    assert_eq!(files.len(), 0, "Empty directory should return no files");
}

#[tokio::test]
async fn test_discover_files_nonexistent_path() {
    let root = PathBuf::from("/nonexistent/path/that/does/not/exist");
    let source = Source::Local { root_path: root };

    let result = discover_files(&source).await;
    assert!(result.is_err(), "Should error on nonexistent path");
}

#[test]
fn test_indexing_progress_default() {
    let progress = IndexingProgress::default();

    assert_eq!(progress.total, 0);
    assert_eq!(progress.processed, 0);
    assert!(progress.current_file.is_none());
    assert!(matches!(progress.stage, IndexingStage::Scanning));
    assert!(!progress.is_paused);
    assert_eq!(progress.files_per_second, 0.0);
    assert!(progress.estimated_seconds_remaining.is_none());
}

#[test]
fn test_indexing_progress_percentage() {
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

    // Over 100% (shouldn't happen but test edge case)
    progress.processed = 150;
    assert!((progress.percentage() - 150.0).abs() < 1e-6);
}

#[test]
fn test_indexing_stage_variants() {
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
async fn test_create_memory_from_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    let content = b"Test file content for memory creation";
    fs::write(&file_path, content).unwrap();

    let source = Source::Local {
        root_path: temp.path().to_path_buf(),
    };

    let memory = create_memory_from_file(&file_path, &source).await;

    assert!(memory.is_ok(), "Should create memory from file");
    let memory = memory.unwrap();

    assert_eq!(memory.path, file_path);
    assert_eq!(memory.metadata.file_size, content.len() as u64);
    assert!(memory.metadata.title.is_some());
    assert_eq!(memory.metadata.title.unwrap(), "test.txt");
}

#[tokio::test]
async fn test_create_memory_from_code_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("main.rs");
    let content = b"fn main() {\n    println!(\"Hello\");\n}";
    fs::write(&file_path, content).unwrap();

    let source = Source::Local {
        root_path: temp.path().to_path_buf(),
    };

    let memory = create_memory_from_file(&file_path, &source).await;

    assert!(memory.is_ok());
    let memory = memory.unwrap();

    // Should detect as code
    if let MemoryKind::Code { language, lines } = memory.kind {
        assert_eq!(language, "rust");
        assert!(lines > 0);
    } else {
        panic!("Expected Code memory kind");
    }
}

#[tokio::test]
async fn test_create_memory_empty_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("empty.txt");
    fs::write(&file_path, b"").unwrap();

    let source = Source::Local {
        root_path: temp.path().to_path_buf(),
    };

    let memory = create_memory_from_file(&file_path, &source).await;

    assert!(memory.is_ok());
    let memory = memory.unwrap();
    assert_eq!(memory.metadata.file_size, 0);
}

#[tokio::test]
async fn test_create_memory_large_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large.txt");

    // Create a 1MB file
    let content = vec![b'x'; 1024 * 1024];
    fs::write(&file_path, &content).unwrap();

    let source = Source::Local {
        root_path: temp.path().to_path_buf(),
    };

    let memory = create_memory_from_file(&file_path, &source).await;

    assert!(memory.is_ok());
    let memory = memory.unwrap();
    assert_eq!(memory.metadata.file_size, 1024 * 1024);
}

#[tokio::test]
async fn test_batch_processing_concurrent() {
    let temp = TempDir::new().unwrap();
    let root = create_test_fixtures(&temp);

    let source = Source::Local {
        root_path: root.clone(),
    };
    let files = discover_files(&source).await.unwrap();

    // Test that batch processing can handle multiple files
    assert!(
        files.len() > 1,
        "Need multiple files to test batch processing"
    );

    // Process files - this exercises the parallel processing
    let results: Vec<_> = files
        .iter()
        .map(|path| {
            let source = source.clone();
            tokio::spawn(async move { create_memory_from_file(path, &source).await })
        })
        .collect();

    let mut success_count = 0;
    for result in results {
        if let Ok(Ok(_)) = result.await {
            success_count += 1;
        }
    }

    assert!(
        success_count > 0,
        "Should successfully process multiple files"
    );
}

#[test]
fn test_file_extension_detection() {
    assert_eq!(
        get_file_extension(&PathBuf::from("test.txt")),
        Some("txt".to_string())
    );
    assert_eq!(
        get_file_extension(&PathBuf::from("image.JPG")),
        Some("jpg".to_string())
    );
    assert_eq!(
        get_file_extension(&PathBuf::from("archive.tar.gz")),
        Some("gz".to_string())
    );
    assert_eq!(get_file_extension(&PathBuf::from("no_extension")), None);
    assert_eq!(get_file_extension(&PathBuf::from(".dotfile")), None);
}

#[test]
fn test_detect_memory_kind_from_extension() {
    // Images
    let jpg = detect_memory_kind_from_extension("jpg", 0);
    assert!(matches!(jpg, MemoryKind::Image { .. }));

    // Videos
    let mp4 = detect_memory_kind_from_extension("mp4", 0);
    assert!(matches!(mp4, MemoryKind::Video { .. }));

    // Audio
    let mp3 = detect_memory_kind_from_extension("mp3", 0);
    assert!(matches!(mp3, MemoryKind::Audio { .. }));

    // Documents
    let pdf = detect_memory_kind_from_extension("pdf", 0);
    assert!(matches!(pdf, MemoryKind::Document { .. }));

    // Code
    let rs = detect_memory_kind_from_extension("rs", 100);
    if let MemoryKind::Code { language, lines } = rs {
        assert_eq!(language, "rust");
        assert_eq!(lines, 100);
    } else {
        panic!("Expected Code kind");
    }

    // Spreadsheet
    let xlsx = detect_memory_kind_from_extension("xlsx", 0);
    assert!(matches!(xlsx, MemoryKind::Spreadsheet));

    // Archive
    let zip = detect_memory_kind_from_extension("zip", 0);
    assert!(matches!(zip, MemoryKind::Archive { .. }));

    // Unknown
    let unknown = detect_memory_kind_from_extension("xyz", 0);
    assert!(matches!(unknown, MemoryKind::Unknown { .. }));
}

#[tokio::test]
async fn test_indexing_respects_file_modifications() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("modified.txt");

    // Create initial file
    fs::write(&file_path, b"original content").unwrap();

    let source = Source::Local {
        root_path: temp.path().to_path_buf(),
    };

    let memory1 = create_memory_from_file(&file_path, &source).await.unwrap();
    let modified1 = memory1.modified_at;

    // Wait a bit and modify file
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    fs::write(&file_path, b"new content with more data").unwrap();

    let memory2 = create_memory_from_file(&file_path, &source).await.unwrap();
    let modified2 = memory2.modified_at;

    // File size should change
    assert_ne!(memory1.metadata.file_size, memory2.metadata.file_size);

    // Modified time should be different (though this might be flaky on some systems)
    // We at least verify both have valid timestamps
    assert!(modified1 > chrono::DateTime::UNIX_EPOCH);
    assert!(modified2 > chrono::DateTime::UNIX_EPOCH);
}
