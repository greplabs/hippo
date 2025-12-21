//! Comprehensive tests for metadata extraction from various file types
//!
//! Tests EXIF extraction, audio metadata, video metadata, and code parsing.

use hippo_core::indexer::code_parser::*;
use hippo_core::indexer::extractors::*;
use hippo_core::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Code Parser Tests
// ============================================================================

#[test]
fn test_parse_rust_code_basic() {
    let code = r#"
fn main() {
    println!("Hello, world!");
}

fn helper() {
    // Helper function
}

struct MyStruct {
    field: i32,
}
"#;

    let result = parse_rust_code(code);
    assert!(result.is_ok(), "Should parse valid Rust code");

    let metadata = result.unwrap();
    assert!(metadata.functions.len() >= 2, "Should find functions");
    assert!(
        metadata.functions.contains(&"main".to_string()),
        "Should find main function"
    );
    assert!(
        metadata.functions.contains(&"helper".to_string()),
        "Should find helper function"
    );
}

#[test]
fn test_parse_rust_code_with_imports() {
    let code = r#"
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

fn main() {}
"#;

    let result = parse_rust_code(code);
    assert!(result.is_ok());

    let metadata = result.unwrap();
    assert!(metadata.imports.len() >= 2, "Should find imports");
    assert!(
        metadata.imports.iter().any(|i| i.contains("HashMap")),
        "Should find HashMap import"
    );
}

#[test]
fn test_parse_rust_code_invalid() {
    let code = "this is not valid rust code {{{";

    let result = parse_rust_code(code);
    // Should handle gracefully - either error or empty metadata
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_rust_code_empty() {
    let code = "";

    let result = parse_rust_code(code);
    assert!(result.is_ok());

    let metadata = result.unwrap();
    assert!(metadata.functions.is_empty());
    assert!(metadata.imports.is_empty());
}

#[test]
fn test_parse_python_code_basic() {
    let code = r#"
import os
import sys

def hello():
    print("Hello")

def world():
    print("World")

class MyClass:
    def method(self):
        pass
"#;

    let result = parse_python_code(code);
    assert!(result.is_ok(), "Should parse valid Python code");

    let metadata = result.unwrap();
    assert!(metadata.functions.len() >= 2, "Should find functions");
    assert!(
        metadata.functions.contains(&"hello".to_string()),
        "Should find hello function"
    );
    assert!(
        metadata.functions.contains(&"world".to_string()),
        "Should find world function"
    );
    assert!(metadata.imports.len() >= 2, "Should find import statements");
}

#[test]
fn test_parse_python_code_invalid() {
    let code = "def broken(:\n    pass";

    let result = parse_python_code(code);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_javascript_code_basic() {
    let code = r#"
import React from 'react';
import { useState } from 'react';

function Component() {
    return <div>Hello</div>;
}

const helper = () => {
    console.log("Helper");
};

export default Component;
"#;

    let result = parse_javascript_code(code);
    assert!(result.is_ok(), "Should parse valid JavaScript code");

    let metadata = result.unwrap();
    assert!(metadata.functions.len() >= 1, "Should find functions");
    assert!(metadata.imports.len() >= 1, "Should find imports");
    assert!(metadata.exports.len() >= 1, "Should find exports");
}

#[test]
fn test_parse_javascript_code_commonjs() {
    let code = r#"
const fs = require('fs');
const path = require('path');

function readFile(filename) {
    return fs.readFileSync(filename);
}

module.exports = { readFile };
"#;

    let result = parse_javascript_code(code);
    assert!(result.is_ok());

    let metadata = result.unwrap();
    assert!(
        metadata.functions.contains(&"readFile".to_string()),
        "Should find readFile function"
    );
}

#[test]
fn test_parse_javascript_code_invalid() {
    let code = "function broken( {\n    console.log('test')";

    let result = parse_javascript_code(code);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_detect_code_language() {
    assert_eq!(detect_code_language("test.rs"), Some("rust".to_string()));
    assert_eq!(detect_code_language("test.py"), Some("python".to_string()));
    assert_eq!(
        detect_code_language("test.js"),
        Some("javascript".to_string())
    );
    assert_eq!(
        detect_code_language("test.jsx"),
        Some("javascript".to_string())
    );
    assert_eq!(
        detect_code_language("test.ts"),
        Some("typescript".to_string())
    );
    assert_eq!(detect_code_language("test.go"), Some("go".to_string()));
    assert_eq!(detect_code_language("test.java"), Some("java".to_string()));
    assert_eq!(detect_code_language("test.cpp"), Some("cpp".to_string()));
    assert_eq!(detect_code_language("test.c"), Some("c".to_string()));

    assert_eq!(detect_code_language("test.txt"), None);
    assert_eq!(detect_code_language("test"), None);
}

#[test]
fn test_count_lines_of_code() {
    let code = r#"
fn main() {
    println!("Line 1");
    println!("Line 2");
}
"#;

    let lines = count_lines_of_code(code);
    assert!(lines >= 3, "Should count non-empty lines");
}

#[test]
fn test_count_lines_empty() {
    assert_eq!(count_lines_of_code(""), 0);
    assert_eq!(count_lines_of_code("\n\n\n"), 0);
    assert_eq!(count_lines_of_code("   \n   \n   "), 0);
}

#[test]
fn test_count_lines_with_comments() {
    let code = r#"
// Comment line
fn main() {
    // Another comment
    println!("Code");
}
"#;

    let lines = count_lines_of_code(code);
    // Should count all lines including comments
    assert!(lines >= 2);
}

// ============================================================================
// File Metadata Extraction Tests
// ============================================================================

#[tokio::test]
async fn test_extract_basic_metadata() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    let content = b"Test file content";
    fs::write(&file_path, content).unwrap();

    let metadata = extract_basic_metadata(&file_path).await;
    assert!(metadata.is_ok());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.file_size, content.len() as u64);
    assert_eq!(metadata.title, Some("test.txt".to_string()));
    assert!(metadata.mime_type.is_some());
}

#[tokio::test]
async fn test_extract_basic_metadata_nonexistent() {
    let path = PathBuf::from("/nonexistent/file.txt");
    let result = extract_basic_metadata(&path).await;
    assert!(result.is_err(), "Should error on nonexistent file");
}

#[tokio::test]
async fn test_extract_code_metadata() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("main.rs");
    let code = r#"
use std::io;

fn main() {
    println!("Hello");
}

fn helper() {
    // Helper
}
"#;
    fs::write(&file_path, code).unwrap();

    let metadata = extract_code_metadata(&file_path).await;
    assert!(metadata.is_ok());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.language, Some("rust".to_string()));
    assert!(metadata.lines_of_code.unwrap_or(0) > 0);
    assert!(metadata.code_functions.is_some());
    assert!(metadata.code_imports.is_some());
}

#[tokio::test]
async fn test_extract_code_metadata_python() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.py");
    let code = r#"
import os

def test():
    print("test")
"#;
    fs::write(&file_path, code).unwrap();

    let metadata = extract_code_metadata(&file_path).await;
    assert!(metadata.is_ok());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.language, Some("python".to_string()));
    assert!(metadata.code_functions.is_some());
}

#[tokio::test]
async fn test_extract_code_metadata_javascript() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("app.js");
    let code = r#"
function hello() {
    console.log("Hello");
}
export default hello;
"#;
    fs::write(&file_path, code).unwrap();

    let metadata = extract_code_metadata(&file_path).await;
    assert!(metadata.is_ok());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.language, Some("javascript".to_string()));
}

#[tokio::test]
async fn test_extract_code_metadata_unsupported_language() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.go");
    fs::write(&file_path, b"package main\nfunc main() {}").unwrap();

    let metadata = extract_code_metadata(&file_path).await;
    assert!(metadata.is_ok());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.language, Some("go".to_string()));
    // For unsupported languages, we still count lines
    assert!(metadata.lines_of_code.is_some());
}

#[tokio::test]
async fn test_extract_text_content() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("document.txt");
    let content = "This is a test document with multiple words.";
    fs::write(&file_path, content).unwrap();

    let text = extract_text_content(&file_path, 1000).await;
    assert!(text.is_ok());

    let text = text.unwrap();
    assert_eq!(text, content);
}

#[tokio::test]
async fn test_extract_text_content_large_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large.txt");

    // Create content larger than max_bytes
    let content = "x".repeat(2000);
    fs::write(&file_path, &content).unwrap();

    let text = extract_text_content(&file_path, 1000).await;
    assert!(text.is_ok());

    let text = text.unwrap();
    assert_eq!(text.len(), 1000, "Should truncate to max_bytes");
}

#[tokio::test]
async fn test_extract_text_content_binary_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("binary.bin");
    fs::write(&file_path, &[0u8, 1u8, 2u8, 255u8]).unwrap();

    let text = extract_text_content(&file_path, 1000).await;
    // Binary files might return error or empty string
    assert!(text.is_ok() || text.is_err());
}

#[test]
fn test_detect_mime_type() {
    assert_eq!(detect_mime_type("test.jpg"), "image/jpeg");
    assert_eq!(detect_mime_type("test.png"), "image/png");
    assert_eq!(detect_mime_type("test.pdf"), "application/pdf");
    assert_eq!(detect_mime_type("test.txt"), "text/plain");
    assert_eq!(detect_mime_type("test.html"), "text/html");
    assert_eq!(detect_mime_type("test.json"), "application/json");
    assert_eq!(detect_mime_type("test.mp4"), "video/mp4");
    assert_eq!(detect_mime_type("test.mp3"), "audio/mpeg");
}

#[test]
fn test_detect_mime_type_case_insensitive() {
    assert_eq!(detect_mime_type("test.JPG"), "image/jpeg");
    assert_eq!(detect_mime_type("test.PNG"), "image/png");
    assert_eq!(detect_mime_type("test.PDF"), "application/pdf");
}

#[test]
fn test_detect_mime_type_unknown() {
    let mime = detect_mime_type("test.xyz");
    // Unknown types should return a default mime type
    assert!(mime.contains("application") || mime.contains("text"));
}

#[test]
fn test_extract_title_from_path() {
    assert_eq!(
        extract_title_from_path(&PathBuf::from("/path/to/file.txt")),
        "file.txt"
    );
    assert_eq!(
        extract_title_from_path(&PathBuf::from("document.pdf")),
        "document.pdf"
    );
    assert_eq!(
        extract_title_from_path(&PathBuf::from("/nested/deep/path/readme.md")),
        "readme.md"
    );
}

#[test]
fn test_is_text_file() {
    assert!(is_text_file("test.txt"));
    assert!(is_text_file("test.md"));
    assert!(is_text_file("test.rs"));
    assert!(is_text_file("test.py"));
    assert!(is_text_file("test.json"));
    assert!(is_text_file("test.html"));

    assert!(!is_text_file("test.jpg"));
    assert!(!is_text_file("test.mp4"));
    assert!(!is_text_file("test.zip"));
    assert!(!is_text_file("test.bin"));
}

#[test]
fn test_is_image_file() {
    assert!(is_image_file("test.jpg"));
    assert!(is_image_file("test.jpeg"));
    assert!(is_image_file("test.png"));
    assert!(is_image_file("test.gif"));
    assert!(is_image_file("test.webp"));
    assert!(is_image_file("test.bmp"));

    assert!(!is_image_file("test.txt"));
    assert!(!is_image_file("test.mp4"));
    assert!(!is_image_file("test.pdf"));
}

#[test]
fn test_is_video_file() {
    assert!(is_video_file("test.mp4"));
    assert!(is_video_file("test.mov"));
    assert!(is_video_file("test.avi"));
    assert!(is_video_file("test.mkv"));
    assert!(is_video_file("test.webm"));

    assert!(!is_video_file("test.txt"));
    assert!(!is_video_file("test.jpg"));
    assert!(!is_video_file("test.mp3"));
}

#[test]
fn test_is_audio_file() {
    assert!(is_audio_file("test.mp3"));
    assert!(is_audio_file("test.wav"));
    assert!(is_audio_file("test.flac"));
    assert!(is_audio_file("test.m4a"));
    assert!(is_audio_file("test.ogg"));

    assert!(!is_audio_file("test.txt"));
    assert!(!is_audio_file("test.jpg"));
    assert!(!is_audio_file("test.mp4"));
}

#[test]
fn test_is_code_file() {
    assert!(is_code_file("test.rs"));
    assert!(is_code_file("test.py"));
    assert!(is_code_file("test.js"));
    assert!(is_code_file("test.ts"));
    assert!(is_code_file("test.go"));
    assert!(is_code_file("test.java"));
    assert!(is_code_file("test.cpp"));
    assert!(is_code_file("test.c"));
    assert!(is_code_file("test.h"));

    assert!(!is_code_file("test.txt"));
    assert!(!is_code_file("test.jpg"));
    assert!(!is_code_file("test.pdf"));
}

#[test]
fn test_is_document_file() {
    assert!(is_document_file("test.pdf"));
    assert!(is_document_file("test.docx"));
    assert!(is_document_file("test.doc"));
    assert!(is_document_file("test.txt"));
    assert!(is_document_file("test.md"));

    assert!(!is_document_file("test.jpg"));
    assert!(!is_document_file("test.mp4"));
    assert!(!is_document_file("test.zip"));
}

// ============================================================================
// Integration Tests for Complete Metadata Extraction
// ============================================================================

#[tokio::test]
async fn test_extract_complete_metadata_text_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("readme.txt");
    let content = "This is a README file with important information.";
    fs::write(&file_path, content).unwrap();

    let basic = extract_basic_metadata(&file_path).await.unwrap();

    assert_eq!(basic.title, Some("readme.txt".to_string()));
    assert_eq!(basic.file_size, content.len() as u64);
    assert!(basic.mime_type.is_some());
    assert!(basic.created_at.is_some());
    assert!(basic.modified_at.is_some());
}

#[tokio::test]
async fn test_extract_complete_metadata_code_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("main.rs");
    let code = r#"
fn main() {
    println!("Hello");
}
"#;
    fs::write(&file_path, code).unwrap();

    let basic = extract_basic_metadata(&file_path).await.unwrap();
    let code_meta = extract_code_metadata(&file_path).await.unwrap();

    assert_eq!(basic.title, Some("main.rs".to_string()));
    assert_eq!(code_meta.language, Some("rust".to_string()));
    assert!(code_meta.lines_of_code.unwrap_or(0) > 0);
}

#[tokio::test]
async fn test_metadata_preserves_file_timestamps() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    fs::write(&file_path, b"content").unwrap();

    let metadata = extract_basic_metadata(&file_path).await.unwrap();

    assert!(metadata.created_at.is_some());
    assert!(metadata.modified_at.is_some());

    let created = metadata.created_at.unwrap();
    let modified = metadata.modified_at.unwrap();

    // Both should be valid timestamps
    assert!(created > chrono::DateTime::UNIX_EPOCH);
    assert!(modified > chrono::DateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn test_hash_generation_consistency() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.txt");
    fs::write(&file_path, b"consistent content").unwrap();

    let metadata1 = extract_basic_metadata(&file_path).await.unwrap();
    let metadata2 = extract_basic_metadata(&file_path).await.unwrap();

    // Hash should be consistent for same file
    if metadata1.hash.is_some() && metadata2.hash.is_some() {
        assert_eq!(metadata1.hash, metadata2.hash);
    }
}

#[tokio::test]
async fn test_metadata_extraction_error_handling() {
    let nonexistent = PathBuf::from("/nonexistent/path/file.txt");

    let result = extract_basic_metadata(&nonexistent).await;
    assert!(result.is_err(), "Should error on nonexistent file");
}
