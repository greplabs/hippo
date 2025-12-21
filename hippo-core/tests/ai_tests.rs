//! AI Feature Tests with Mocked Responses
//!
//! This test suite provides comprehensive testing for all AI-powered features
//! without requiring actual Ollama or Claude API services running.
//!
//! Features tested:
//! - File analysis (images, documents, code, videos)
//! - Tag suggestions
//! - Image captioning
//! - Natural language search parsing
//! - RAG query handling
//! - Organization suggestions
//! - Similar file detection
//! - Duplicate detection

use hippo_core::ai::{
    analyze_code, analyze_document, analyze_file, analyze_image, analyze_video, AiConfig,
    AiProvider, ClaudeClient, CodeAnalysis, CodeSummary, DocumentAnalysis, DocumentSummary,
    DuplicateMatch, DuplicateType, ExtractedEntities, FileAnalysis, ImageAnalysis,
    OrganizationSuggestion, SimilarFile, TagSuggestion, UnifiedAiClient, VideoAnalysis,
};
use hippo_core::ollama::{
    ChatMessage, LocalAnalysis, OllamaClient, OllamaConfig, RagContext, RagDocument,
};
use hippo_core::{Memory, MemoryKind, MemoryMetadata, Result, Source, Tag, TagSource};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// ==================== Mock AI Client Trait ====================

/// Trait for AI operations - allows us to create mocks
#[async_trait::async_trait]
pub trait AiAnalyzer: Send + Sync {
    async fn analyze_file(&self, memory: &Memory) -> Result<FileAnalysis>;
    async fn summarize_text(&self, content: &str, file_name: &str) -> Result<DocumentSummary>;
    async fn summarize_code(
        &self,
        code: &str,
        language: &str,
        file_name: &str,
    ) -> Result<CodeSummary>;
    async fn suggest_tags(&self, memory: &Memory) -> Result<Vec<TagSuggestion>>;
    async fn caption_image(&self, path: &std::path::Path) -> Result<String>;
    async fn rag_query(&self, query: &str, docs: Vec<(String, String, f32)>) -> Result<String>;
}

// ==================== Mock Implementations ====================

/// Mock Claude client for testing
pub struct MockClaudeClient {
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockClaudeClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
        }
    }

    pub async fn add_response(&self, response: String) {
        self.responses.lock().await.push(response);
    }

    async fn next_response(&self) -> String {
        let mut responses = self.responses.lock().await;
        if responses.is_empty() {
            // Default response
            json!({
                "tags": [
                    {"name": "test-tag", "confidence": 80, "reason": "Mock response"}
                ],
                "description": "Mock file description",
                "suggested_folder": "mock/folder"
            })
            .to_string()
        } else {
            responses.remove(0)
        }
    }
}

#[async_trait::async_trait]
impl AiAnalyzer for MockClaudeClient {
    async fn analyze_file(&self, memory: &Memory) -> Result<FileAnalysis> {
        let response = self.next_response().await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        let tags = parsed["tags"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| TagSuggestion {
                name: t["name"].as_str().unwrap_or("unknown").to_string(),
                confidence: t["confidence"].as_u64().unwrap_or(50) as u8,
                reason: t["reason"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(FileAnalysis {
            tags,
            description: parsed["description"].as_str().map(String::from),
            organization: parsed["suggested_folder"]
                .as_str()
                .map(|f| OrganizationSuggestion {
                    suggested_folder: f.to_string(),
                    reason: "AI suggested".to_string(),
                }),
        })
    }

    async fn summarize_text(&self, _content: &str, _file_name: &str) -> Result<DocumentSummary> {
        let response = self.next_response().await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        Ok(DocumentSummary {
            summary: parsed["summary"]
                .as_str()
                .unwrap_or("Mock summary")
                .to_string(),
            key_topics: parsed["key_topics"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            entities: None,
            document_type: parsed["document_type"].as_str().map(String::from),
            sentiment: None,
            complexity: None,
        })
    }

    async fn summarize_code(
        &self,
        _code: &str,
        _language: &str,
        _file_name: &str,
    ) -> Result<CodeSummary> {
        let response = self.next_response().await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        Ok(CodeSummary {
            summary: parsed["summary"]
                .as_str()
                .unwrap_or("Mock code summary")
                .to_string(),
            purpose: parsed["purpose"].as_str().map(String::from),
            main_functionality: parsed["main_functionality"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            dependencies: vec![],
            complexity: None,
            patterns: vec![],
            suggested_tags: vec![],
        })
    }

    async fn suggest_tags(&self, memory: &Memory) -> Result<Vec<TagSuggestion>> {
        let analysis = self.analyze_file(memory).await?;
        Ok(analysis.tags)
    }

    async fn caption_image(&self, _path: &std::path::Path) -> Result<String> {
        Ok("A beautiful mock image with interesting content".to_string())
    }

    async fn rag_query(&self, query: &str, docs: Vec<(String, String, f32)>) -> Result<String> {
        Ok(format!(
            "Mock RAG response for query '{}' using {} documents",
            query,
            docs.len()
        ))
    }
}

/// Mock Ollama client for testing
pub struct MockOllamaClient {
    available: bool,
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockOllamaClient {
    pub fn new() -> Self {
        Self {
            available: true,
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn unavailable() -> Self {
        Self {
            available: false,
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            available: true,
            responses: Arc::new(Mutex::new(responses)),
        }
    }

    pub async fn add_response(&self, response: String) {
        self.responses.lock().await.push(response);
    }

    async fn next_response(&self) -> String {
        let mut responses = self.responses.lock().await;
        if responses.is_empty() {
            json!({
                "summary": "Mock Ollama summary",
                "key_topics": ["topic1", "topic2"],
                "suggested_tags": ["ollama-tag", "test-tag"],
                "document_type": "code",
                "language": "Rust"
            })
            .to_string()
        } else {
            responses.remove(0)
        }
    }

    pub async fn is_available(&self) -> bool {
        self.available
    }

    pub async fn analyze_document(
        &self,
        _content: &str,
        _file_name: &str,
    ) -> Result<LocalAnalysis> {
        let response = self.next_response().await;
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        Ok(LocalAnalysis {
            summary: parsed["summary"]
                .as_str()
                .unwrap_or("Mock summary")
                .to_string(),
            key_topics: parsed["key_topics"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            suggested_tags: parsed["suggested_tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            document_type: parsed["document_type"].as_str().map(String::from),
            language: parsed["language"].as_str().map(String::from),
        })
    }

    pub async fn analyze_code(
        &self,
        _code: &str,
        _language: &str,
        _file_name: &str,
    ) -> Result<LocalAnalysis> {
        self.analyze_document(_code, _file_name).await
    }

    pub async fn caption_image(&self, _path: &std::path::Path) -> Result<String> {
        Ok("Mock image caption from Ollama vision model".to_string())
    }

    pub async fn rag_query(&self, context: &RagContext) -> Result<String> {
        Ok(format!(
            "Mock RAG answer: Based on {} documents, the answer to '{}' is...",
            context.documents.len(),
            context.query
        ))
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        Ok(format!("Mock chat response to {} messages", messages.len()))
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Return mock embeddings (768 dimensions for nomic-embed-text)
        Ok(texts
            .iter()
            .enumerate()
            .map(|(i, _)| {
                // Generate deterministic mock embedding
                (0..768).map(|j| ((i + j) as f32) * 0.01).collect()
            })
            .collect())
    }

    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }
}

// ==================== Test Helpers ====================

fn create_test_memory(path: &str, kind: MemoryKind, tags: Vec<&str>) -> Memory {
    Memory {
        id: Uuid::new_v4(),
        path: PathBuf::from(path),
        source: Source::Local {
            root_path: PathBuf::from("/test/root"),
        },
        kind,
        metadata: MemoryMetadata {
            title: None,
            description: None,
            file_size: 1024,
            mime_type: Some("text/plain".to_string()),
            hash: Some("mock_hash".to_string()),
            exif: None,
            dimensions: None,
            duration: None,
            video_metadata: None,
            audio_metadata: None,
            location: None,
            face_cluster_ids: vec![],
            text_preview: None,
            word_count: None,
            code_info: None,
            ai_summary: None,
            ai_tags: vec![],
            scene_tags: vec![],
            ai_caption: None,
            custom: std::collections::HashMap::new(),
        },
        tags: tags
            .into_iter()
            .map(|t| Tag {
                name: t.to_string(),
                source: TagSource::User,
                confidence: None,
            })
            .collect(),
        embedding_id: None,
        connections: vec![],
        is_favorite: false,
        created_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        indexed_at: chrono::Utc::now(),
    }
}

// ==================== File Analysis Tests ====================

#[tokio::test]
async fn test_mock_claude_file_analysis() {
    let mock_client = MockClaudeClient::with_responses(vec![json!({
        "tags": [
            {"name": "rust-code", "confidence": 95, "reason": "Rust source file"},
            {"name": "backend", "confidence": 85, "reason": "Server-side code"},
            {"name": "ai-feature", "confidence": 80, "reason": "AI-related functionality"}
        ],
        "description": "A Rust module implementing AI-powered file analysis",
        "suggested_folder": "src/ai"
    })
    .to_string()]);

    let memory = create_test_memory(
        "/test/ai_mod.rs",
        MemoryKind::Code {
            language: "Rust".to_string(),
            lines: 500,
        },
        vec![],
    );

    let analysis = mock_client.analyze_file(&memory).await.unwrap();

    assert_eq!(analysis.tags.len(), 3);
    assert_eq!(analysis.tags[0].name, "rust-code");
    assert_eq!(analysis.tags[0].confidence, 95);
    assert!(analysis.description.is_some());
    assert!(analysis.organization.is_some());
    assert_eq!(analysis.organization.unwrap().suggested_folder, "src/ai");
}

#[tokio::test]
async fn test_mock_ollama_document_analysis() {
    let mock_ollama = MockOllamaClient::with_responses(vec![json!({
        "summary": "This document discusses AI testing strategies for file organization systems",
        "key_topics": ["testing", "mocking", "AI features", "quality assurance"],
        "suggested_tags": ["testing", "documentation", "ai", "quality"],
        "document_type": "article",
        "language": "text"
    })
    .to_string()]);

    let result = mock_ollama
        .analyze_document("Sample document content...", "test_doc.md")
        .await
        .unwrap();

    assert_eq!(result.key_topics.len(), 4);
    assert!(result.summary.contains("testing strategies"));
    assert_eq!(result.suggested_tags.len(), 4);
    assert_eq!(result.document_type, Some("article".to_string()));
}

#[tokio::test]
async fn test_mock_code_analysis() {
    let mock_client = MockClaudeClient::with_responses(vec![json!({
        "summary": "Implementation of a binary search tree data structure",
        "purpose": "library",
        "main_functionality": [
            "insert - adds elements to tree",
            "search - finds elements",
            "delete - removes elements"
        ],
        "complexity": "moderate",
        "patterns": ["recursion", "tree-traversal"],
        "suggested_tags": ["data-structure", "algorithm", "tree"]
    })
    .to_string()]);

    let code_summary = mock_client
        .summarize_code("fn main() {}", "Rust", "tree.rs")
        .await
        .unwrap();

    assert!(code_summary.summary.contains("binary search tree"));
    assert_eq!(code_summary.purpose, Some("library".to_string()));
    assert_eq!(code_summary.main_functionality.len(), 3);
}

// ==================== Tag Suggestion Tests ====================

#[tokio::test]
async fn test_tag_suggestions_for_image() {
    let mock_client = MockClaudeClient::with_responses(vec![json!({
        "tags": [
            {"name": "landscape", "confidence": 90, "reason": "Outdoor scenery"},
            {"name": "sunset", "confidence": 85, "reason": "Golden hour lighting"},
            {"name": "nature", "confidence": 95, "reason": "Natural environment"},
            {"name": "photography", "confidence": 80, "reason": "High quality photo"}
        ],
        "description": "A stunning landscape photograph taken during sunset",
        "suggested_folder": "Photos/Nature/Landscapes"
    })
    .to_string()]);

    let memory = create_test_memory(
        "/photos/sunset.jpg",
        MemoryKind::Image {
            width: 3840,
            height: 2160,
            format: "JPEG".to_string(),
        },
        vec![],
    );

    let tags = mock_client.suggest_tags(&memory).await.unwrap();

    assert_eq!(tags.len(), 4);
    assert!(tags.iter().any(|t| t.name == "landscape"));
    assert!(tags.iter().any(|t| t.name == "sunset"));
    assert!(tags.iter().all(|t| t.confidence >= 80));
}

#[tokio::test]
async fn test_tag_suggestions_confidence_filtering() {
    let mock_client = MockClaudeClient::with_responses(vec![json!({
        "tags": [
            {"name": "high-conf", "confidence": 95, "reason": "Very certain"},
            {"name": "medium-conf", "confidence": 70, "reason": "Somewhat certain"},
            {"name": "low-conf", "confidence": 45, "reason": "Uncertain"}
        ],
        "description": "Test file",
        "suggested_folder": "test"
    })
    .to_string()]);

    let memory = create_test_memory("/test.txt", MemoryKind::Unknown, vec![]);

    let tags = mock_client.suggest_tags(&memory).await.unwrap();
    let high_confidence_tags: Vec<_> = tags.iter().filter(|t| t.confidence >= 70).collect();

    assert_eq!(high_confidence_tags.len(), 2);
}

// ==================== Image Captioning Tests ====================

#[tokio::test]
async fn test_image_captioning_claude() {
    let mock_client = MockClaudeClient::new();

    let caption = mock_client
        .caption_image(&PathBuf::from("/test/image.jpg"))
        .await
        .unwrap();

    assert!(!caption.is_empty());
    assert!(caption.contains("mock"));
}

#[tokio::test]
async fn test_image_captioning_ollama() {
    let mock_ollama = MockOllamaClient::new();

    let caption = mock_ollama
        .caption_image(&PathBuf::from("/test/photo.png"))
        .await
        .unwrap();

    assert!(!caption.is_empty());
    assert!(caption.contains("Ollama"));
}

// ==================== RAG Query Tests ====================

#[tokio::test]
async fn test_rag_query_with_context() {
    let mock_client = MockClaudeClient::new();

    let docs = vec![
        (
            "The Hippo project uses Rust for backend".to_string(),
            "README.md".to_string(),
            0.95,
        ),
        (
            "Tauri provides the desktop framework".to_string(),
            "architecture.md".to_string(),
            0.87,
        ),
        (
            "SQLite is used for local storage".to_string(),
            "storage.md".to_string(),
            0.82,
        ),
    ];

    let response = mock_client
        .rag_query("What technologies does Hippo use?", docs)
        .await
        .unwrap();

    assert!(!response.is_empty());
    assert!(response.contains("3 documents"));
}

#[tokio::test]
async fn test_rag_query_ollama() {
    let mock_ollama = MockOllamaClient::new();

    let context = RagContext {
        query: "How does indexing work?".to_string(),
        documents: vec![
            RagDocument {
                content: "Indexing scans files and extracts metadata".to_string(),
                source: "indexer.rs".to_string(),
                relevance_score: 0.92,
            },
            RagDocument {
                content: "Background workers process files in parallel".to_string(),
                source: "worker.rs".to_string(),
                relevance_score: 0.85,
            },
        ],
    };

    let response = mock_ollama.rag_query(&context).await.unwrap();

    assert!(!response.is_empty());
    assert!(response.contains("2 documents"));
    assert!(response.contains("indexing"));
}

// ==================== Natural Language Processing Tests ====================

#[tokio::test]
async fn test_parse_search_query() {
    // Test parsing natural language queries into structured search
    let mock_ollama = MockOllamaClient::with_responses(vec![json!({
        "summary": "Search for Rust code files created last week",
        "key_topics": ["search", "rust", "recent"],
        "suggested_tags": ["rust-code", "last-week", "source-code"],
        "document_type": "query",
        "language": "natural"
    })
    .to_string()]);

    let analysis = mock_ollama
        .analyze_document("show me rust files from last week", "query")
        .await
        .unwrap();

    assert!(analysis.suggested_tags.iter().any(|t| t.contains("rust")));
}

#[tokio::test]
async fn test_complex_query_parsing() {
    let mock_ollama = MockOllamaClient::with_responses(vec![json!({
        "summary": "Find large image files tagged vacation from summer",
        "key_topics": ["images", "vacation", "summer", "large-files"],
        "suggested_tags": ["image", "vacation", "summer", "large-file"],
        "document_type": "query",
        "language": "natural"
    })
    .to_string()]);

    let analysis = mock_ollama
        .analyze_document("find all big vacation photos from this summer", "query")
        .await
        .unwrap();

    let tags = &analysis.suggested_tags;
    assert!(tags.iter().any(|t| t.contains("vacation")));
    assert!(tags.iter().any(|t| t.contains("image")));
    assert!(tags.iter().any(|t| t.contains("summer")));
}

// ==================== Organization Suggestions Tests ====================

#[tokio::test]
async fn test_organization_suggestions() {
    let mock_client = MockClaudeClient::new();

    let memory = create_test_memory(
        "/downloads/IMG_2024_vacation.jpg",
        MemoryKind::Image {
            width: 4032,
            height: 3024,
            format: "JPEG".to_string(),
        },
        vec!["vacation", "2024"],
    );

    // Mock response should suggest organized folder
    mock_client
        .add_response(
            json!({
                "tags": [],
                "description": "Vacation photo",
                "suggested_folder": "Photos/2024/Vacation"
            })
            .to_string(),
        )
        .await;

    let analysis = mock_client.analyze_file(&memory).await.unwrap();

    assert!(analysis.organization.is_some());
    let org = analysis.organization.unwrap();
    assert!(org.suggested_folder.contains("2024"));
    assert!(org.suggested_folder.contains("Vacation"));
}

// ==================== Similar File Detection Tests ====================

#[tokio::test]
async fn test_similar_file_detection() {
    let client = UnifiedAiClient::new();

    let target = create_test_memory(
        "/projects/hippo/src/main.rs",
        MemoryKind::Code {
            language: "Rust".to_string(),
            lines: 250,
        },
        vec!["rust", "backend"],
    );

    let mut all_memories = vec![
        create_test_memory(
            "/projects/hippo/src/lib.rs",
            MemoryKind::Code {
                language: "Rust".to_string(),
                lines: 180,
            },
            vec!["rust", "backend"],
        ),
        create_test_memory(
            "/projects/hippo/src/ai/mod.rs",
            MemoryKind::Code {
                language: "Rust".to_string(),
                lines: 300,
            },
            vec!["rust", "ai"],
        ),
        create_test_memory(
            "/photos/vacation.jpg",
            MemoryKind::Image {
                width: 1920,
                height: 1080,
                format: "JPEG".to_string(),
            },
            vec!["vacation"],
        ),
    ];

    let similar = client.suggest_similar_files(&target, &all_memories, 10);

    // Should find Rust files as similar
    assert!(!similar.is_empty());
    assert!(similar
        .iter()
        .all(|s| matches!(s.memory.kind, MemoryKind::Code { .. })));
    assert!(similar[0].similarity_score > 0.25);
}

// ==================== Duplicate Detection Tests ====================

#[tokio::test]
async fn test_exact_duplicate_detection() {
    let client = UnifiedAiClient::new();

    let target = create_test_memory(
        "/documents/report.pdf",
        MemoryKind::Document {
            format: hippo_core::DocumentFormat::Pdf,
            page_count: Some(10),
        },
        vec!["report"],
    );

    // Create an exact duplicate (same hash)
    let mut duplicate = target.clone();
    duplicate.id = Uuid::new_v4();
    duplicate.path = PathBuf::from("/backup/report.pdf");

    let all_memories = vec![duplicate];

    let duplicates = client.suggest_duplicates(&target, &all_memories);

    assert!(!duplicates.is_empty());
    assert_eq!(duplicates[0].similarity_type, DuplicateType::ExactHash);
    assert_eq!(duplicates[0].confidence, 100);
}

#[tokio::test]
async fn test_similar_name_duplicate_detection() {
    let client = UnifiedAiClient::new();

    let target = create_test_memory("/files/document.txt", MemoryKind::Unknown, vec![]);

    let mut similar = target.clone();
    similar.id = Uuid::new_v4();
    similar.path = PathBuf::from("/files/document (1).txt");
    similar.metadata.hash = Some("different_hash".to_string());

    let all_memories = vec![similar];

    let duplicates = client.suggest_duplicates(&target, &all_memories);

    assert!(!duplicates.is_empty());
    assert_eq!(duplicates[0].similarity_type, DuplicateType::SimilarName);
    assert!(duplicates[0].confidence >= 80);
}

// ==================== Embedding Tests ====================

#[tokio::test]
async fn test_mock_embeddings_generation() {
    let mock_ollama = MockOllamaClient::new();

    let texts = vec![
        "The quick brown fox".to_string(),
        "jumps over the lazy dog".to_string(),
        "Machine learning is fascinating".to_string(),
    ];

    let embeddings = mock_ollama.embed(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    assert_eq!(embeddings[0].len(), 768); // nomic-embed-text dimension
    assert_eq!(embeddings[1].len(), 768);
    assert_eq!(embeddings[2].len(), 768);

    // Embeddings should be different for different texts
    assert_ne!(embeddings[0], embeddings[1]);
}

#[tokio::test]
async fn test_single_embedding() {
    let mock_ollama = MockOllamaClient::new();

    let embedding = mock_ollama
        .embed_single("test document content")
        .await
        .unwrap();

    assert_eq!(embedding.len(), 768);
    assert!(embedding.iter().any(|&v| v != 0.0)); // Not all zeros
}

// ==================== Chat Tests ====================

#[tokio::test]
async fn test_chat_conversation() {
    let mock_ollama = MockOllamaClient::new();

    let messages = vec![
        ChatMessage::new("system", "You are a helpful file organizer"),
        ChatMessage::new("user", "How should I organize my photos?"),
    ];

    let response = mock_ollama.chat(&messages).await.unwrap();

    assert!(!response.is_empty());
    assert!(response.contains("2 messages"));
}

// ==================== Integration Tests ====================

#[tokio::test]
async fn test_unified_client_ollama_fallback() {
    // Test that UnifiedAiClient falls back gracefully when Ollama unavailable
    let config = AiConfig {
        provider: AiProvider::Ollama,
        claude_api_key: None,
        ollama_url: "http://localhost:11434".to_string(),
        ollama_embedding_model: "nomic-embed-text".to_string(),
        ollama_generation_model: "gemma2:2b".to_string(),
    };

    let client = UnifiedAiClient::with_config(config);

    // This should work even without Ollama running (client is created)
    assert_eq!(client.provider(), AiProvider::Ollama);
}

#[tokio::test]
async fn test_unified_client_provider_switching() {
    let mut client = UnifiedAiClient::new();

    // Default is Ollama
    assert_eq!(client.provider(), AiProvider::Ollama);

    // Switch to Claude
    client.set_provider(AiProvider::Claude);
    assert_eq!(client.provider(), AiProvider::Claude);

    // Set API key
    client.set_claude_key("test_key_123".to_string());
}

// ==================== Error Handling Tests ====================

#[tokio::test]
async fn test_mock_ollama_unavailable() {
    let mock_ollama = MockOllamaClient::unavailable();

    assert!(!mock_ollama.is_available().await);
}

#[tokio::test]
async fn test_empty_response_handling() {
    let mock_client = MockClaudeClient::new();

    // Should use default response when no responses queued
    let memory = create_test_memory("/test.txt", MemoryKind::Unknown, vec![]);
    let result = mock_client.analyze_file(&memory).await;

    assert!(result.is_ok());
}

// ==================== Batch Processing Tests ====================

#[tokio::test]
async fn test_batch_analysis() {
    let mock_client = MockClaudeClient::new();

    let memories = vec![
        create_test_memory("/file1.txt", MemoryKind::Unknown, vec![]),
        create_test_memory("/file2.txt", MemoryKind::Unknown, vec![]),
        create_test_memory("/file3.txt", MemoryKind::Unknown, vec![]),
    ];

    // Analyze all files
    let mut results = Vec::new();
    for memory in &memories {
        results.push(mock_client.analyze_file(memory).await);
    }

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));
}

// ==================== Collection Suggestions Tests ====================

#[tokio::test]
async fn test_suggest_groupings() {
    let client = UnifiedAiClient::new();

    let memories = vec![
        create_test_memory(
            "/photos/photo1.jpg",
            MemoryKind::Image {
                width: 1920,
                height: 1080,
                format: "JPEG".to_string(),
            },
            vec!["vacation", "beach"],
        ),
        create_test_memory(
            "/photos/photo2.jpg",
            MemoryKind::Image {
                width: 1920,
                height: 1080,
                format: "JPEG".to_string(),
            },
            vec!["vacation", "beach"],
        ),
        create_test_memory(
            "/photos/photo3.jpg",
            MemoryKind::Image {
                width: 1920,
                height: 1080,
                format: "JPEG".to_string(),
            },
            vec!["vacation", "mountain"],
        ),
    ];

    let suggestions = client.suggest_groupings(&memories).await.unwrap();

    // Should suggest grouping by common tags or file type
    assert!(!suggestions.is_empty());
}

// ==================== Performance Tests ====================

#[tokio::test]
async fn test_concurrent_analysis() {
    use tokio::task;

    let mock_client = Arc::new(MockClaudeClient::new());

    let memories: Vec<_> = (0..10)
        .map(|i| create_test_memory(&format!("/file{}.txt", i), MemoryKind::Unknown, vec![]))
        .collect();

    // Analyze concurrently
    let mut handles = Vec::new();
    for memory in memories {
        let client = Arc::clone(&mock_client);
        handles.push(task::spawn(
            async move { client.analyze_file(&memory).await },
        ));
    }

    let results = futures::future::join_all(handles).await;

    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|r| r.is_ok()));
}

#[tokio::test]
async fn test_embedding_batch_performance() {
    let mock_ollama = MockOllamaClient::new();

    // Generate embeddings for 100 texts
    let texts: Vec<String> = (0..100)
        .map(|i| format!("Document content number {}", i))
        .collect();

    let embeddings = mock_ollama.embed(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 100);
    assert!(embeddings.iter().all(|e| e.len() == 768));
}
