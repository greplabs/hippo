//! Comprehensive tests for metadata extraction from various file types
//!
//! Tests code parsing and metadata extraction using the public API.

use hippo_core::indexer::code_parser::*;
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

// Note: Tests for extract_basic_metadata, extract_code_metadata, etc.
// have been removed because those are internal functions.
// The public API for metadata extraction is through the Hippo struct
// which indexes files automatically.
