//! AI-powered file analysis and auto-tagging
//!
//! This module provides intelligent file analysis, tag suggestion,
//! and organization recommendations using either:
//! - Anthropic's Claude API (cloud)
//! - Ollama (local, privacy-first)

#![allow(missing_docs)]

pub mod analysis;

// Re-export analysis types
pub use analysis::{
    analyze_code, analyze_document, analyze_file, analyze_image, analyze_video, AnalysisResult,
    CodeAnalysis, Color, DetectedObject, DocumentAnalysis, ExtractedEntities, FunctionInfo,
    ImageAnalysis, VideoAnalysis,
};

use crate::ollama::{
    ChatMessage, LocalAnalysis, OllamaClient, OllamaConfig, RagContext, RagDocument,
};
use crate::{HippoError, Memory, MemoryKind, Result, Tag, TagSource};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Claude API client for AI-powered features
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
}

/// Request to Claude API
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

/// Response from Claude API
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: Option<String>,
}

/// Tag suggestion from AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSuggestion {
    pub name: String,
    pub confidence: u8,
    pub reason: String,
}

/// Organization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSuggestion {
    pub suggested_folder: String,
    pub reason: String,
}

/// Collection suggestion for grouping similar files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSuggestion {
    pub name: String,
    pub description: String,
    pub memory_ids: Vec<crate::MemoryId>,
    pub confidence: u8,
    pub reason: String,
}

/// Duplicate match with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateMatch {
    pub memory_id: crate::MemoryId,
    pub path: std::path::PathBuf,
    pub similarity_type: DuplicateType,
    pub confidence: u8,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DuplicateType {
    ExactHash,
    SimilarName,
    SimilarContent,
    SimilarDimensions,
}

/// Similar file result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarFile {
    pub memory: crate::Memory,
    pub similarity_score: f32,
    pub reasons: Vec<String>,
}

/// Full AI analysis result for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub tags: Vec<TagSuggestion>,
    pub description: Option<String>,
    pub organization: Option<OrganizationSuggestion>,
}

impl ClaudeClient {
    /// Create a new Claude client with the given API key
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    /// Create a client with a specific model
    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    /// Analyze a file and suggest tags
    pub async fn analyze_file(&self, memory: &Memory) -> Result<FileAnalysis> {
        let file_name = memory
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let kind_str = Self::kind_to_string(&memory.kind);
        let existing_tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();

        // Build the prompt
        let prompt =
            self.build_analysis_prompt(&file_name, &kind_str, &memory.metadata, &existing_tags);

        // Check if we can analyze the image directly
        let content = if self.is_analyzable_image(&memory.path, &memory.kind) {
            self.build_image_request(&memory.path, &prompt).await?
        } else {
            vec![ContentBlock::Text { text: prompt }]
        };

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content,
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse response: {}", e)))?;

        // Parse the response
        self.parse_analysis_response(&claude_response)
    }

    /// Analyze multiple files in batch
    pub async fn analyze_batch(&self, memories: &[Memory]) -> Vec<Result<FileAnalysis>> {
        let mut results = Vec::new();

        for memory in memories {
            info!("Analyzing: {}", memory.path.display());
            let result = self.analyze_file(memory).await;
            results.push(result);

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        results
    }

    /// Get organization suggestions for a set of files
    pub async fn suggest_organization(
        &self,
        memories: &[Memory],
    ) -> Result<Vec<(String, OrganizationSuggestion)>> {
        if memories.is_empty() {
            return Ok(Vec::new());
        }

        // Build a summary of files
        let file_summary: Vec<String> = memories
            .iter()
            .take(50)
            .map(|m| {
                let name = m
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let kind = Self::kind_to_string(&m.kind);
                let tags: Vec<String> = m.tags.iter().map(|t| t.name.clone()).collect();
                format!(
                    "- {} ({}){}",
                    name,
                    kind,
                    if tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", tags.join(", "))
                    }
                )
            })
            .collect();

        let prompt = format!(
            r#"Analyze these files and suggest how to organize them into folders.

Files:
{}

Respond in this exact JSON format:
{{
  "suggestions": [
    {{
      "file": "filename.ext",
      "folder": "suggested/folder/path",
      "reason": "brief reason"
    }}
  ],
  "folder_structure": [
    "suggested folder 1",
    "suggested folder 2"
  ]
}}

Consider:
- Group by project, date, type, or topic
- Create meaningful folder names
- Keep related files together"#,
            file_summary.join("\n")
        );

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 2048,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: vec![ContentBlock::Text { text: prompt }],
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse response: {}", e)))?;

        self.parse_organization_response(&claude_response, memories)
    }

    fn build_analysis_prompt(
        &self,
        file_name: &str,
        kind: &str,
        metadata: &crate::MemoryMetadata,
        existing_tags: &[String],
    ) -> String {
        let size_str = Self::format_size(metadata.file_size);
        let existing = if existing_tags.is_empty() {
            "None".to_string()
        } else {
            existing_tags.join(", ")
        };

        format!(
            r#"Analyze this file and suggest relevant tags for organization.

File: {file_name}
Type: {kind}
Size: {size_str}
Existing tags: {existing}

Respond in this exact JSON format:
{{
  "tags": [
    {{"name": "tag-name", "confidence": 85, "reason": "brief reason"}}
  ],
  "description": "One sentence description of the file",
  "suggested_folder": "optional/folder/path"
}}

Tag guidelines:
- Use lowercase, hyphenated names (e.g., "work-project", "family-photo")
- Be specific but not too narrow
- Consider: content type, project, date/time context, people, location, purpose
- Confidence: 90+ for obvious, 70-89 for likely, 50-69 for possible
- Suggest 3-7 relevant tags
- Don't repeat existing tags"#
        )
    }

    async fn build_image_request(&self, path: &Path, prompt: &str) -> Result<Vec<ContentBlock>> {
        // Read and encode the image
        let image_data = tokio::fs::read(path)
            .await
            .map_err(|e| HippoError::Other(format!("Failed to read image: {}", e)))?;

        let base64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data);

        // Determine media type
        let media_type = match path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => "image/jpeg",
        };

        Ok(vec![
            ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: media_type.to_string(),
                    data: base64_data,
                },
            },
            ContentBlock::Text {
                text: prompt.to_string(),
            },
        ])
    }

    fn is_analyzable_image(&self, path: &Path, kind: &MemoryKind) -> bool {
        if !matches!(kind, MemoryKind::Image { .. }) {
            return false;
        }

        // Check file size (max 20MB for API)
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 20 * 1024 * 1024 {
                return false;
            }
        }

        // Check extension
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("webp")
        )
    }

    fn parse_analysis_response(&self, response: &ClaudeResponse) -> Result<FileAnalysis> {
        let text = response
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| HippoError::Other("Empty response from Claude".to_string()))?;

        // Try to extract JSON from the response
        let json_str = Self::extract_json(text);

        #[derive(Deserialize)]
        struct AnalysisJson {
            tags: Vec<TagJson>,
            description: Option<String>,
            suggested_folder: Option<String>,
        }

        #[derive(Deserialize)]
        struct TagJson {
            name: String,
            confidence: u8,
            reason: String,
        }

        match serde_json::from_str::<AnalysisJson>(&json_str) {
            Ok(parsed) => {
                let tags = parsed
                    .tags
                    .into_iter()
                    .map(|t| TagSuggestion {
                        name: t.name.to_lowercase().replace(' ', "-"),
                        confidence: t.confidence.min(100),
                        reason: t.reason,
                    })
                    .collect();

                let organization = parsed
                    .suggested_folder
                    .map(|folder| OrganizationSuggestion {
                        suggested_folder: folder,
                        reason: "AI suggested organization".to_string(),
                    });

                Ok(FileAnalysis {
                    tags,
                    description: parsed.description,
                    organization,
                })
            }
            Err(e) => {
                warn!(
                    "Failed to parse Claude response as JSON: {}. Raw: {}",
                    e, text
                );
                // Return basic analysis based on text
                Ok(FileAnalysis {
                    tags: Vec::new(),
                    description: Some(text.chars().take(200).collect()),
                    organization: None,
                })
            }
        }
    }

    fn parse_organization_response(
        &self,
        response: &ClaudeResponse,
        memories: &[Memory],
    ) -> Result<Vec<(String, OrganizationSuggestion)>> {
        let text = response
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| HippoError::Other("Empty response from Claude".to_string()))?;

        let json_str = Self::extract_json(text);

        #[derive(Deserialize)]
        struct OrgJson {
            suggestions: Vec<SuggestionJson>,
        }

        #[derive(Deserialize)]
        struct SuggestionJson {
            file: String,
            folder: String,
            reason: String,
        }

        match serde_json::from_str::<OrgJson>(&json_str) {
            Ok(parsed) => {
                let results: Vec<(String, OrganizationSuggestion)> = parsed
                    .suggestions
                    .into_iter()
                    .filter_map(|s| {
                        // Find the memory ID for this file
                        memories
                            .iter()
                            .find(|m| {
                                m.path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default()
                                    == s.file
                            })
                            .map(|m| {
                                (
                                    m.id.to_string(),
                                    OrganizationSuggestion {
                                        suggested_folder: s.folder,
                                        reason: s.reason,
                                    },
                                )
                            })
                    })
                    .collect();
                Ok(results)
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    fn extract_json(text: &str) -> String {
        // Try to find JSON in the response
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                return text[start..=end].to_string();
            }
        }
        text.to_string()
    }

    fn kind_to_string(kind: &MemoryKind) -> String {
        match kind {
            MemoryKind::Image {
                width,
                height,
                format,
            } => {
                format!("Image ({}x{}, {})", width, height, format)
            }
            MemoryKind::Video {
                duration_ms,
                format,
            } => {
                format!("Video ({:.1}s, {})", *duration_ms as f64 / 1000.0, format)
            }
            MemoryKind::Audio {
                duration_ms,
                format,
            } => {
                format!("Audio ({:.1}s, {})", *duration_ms as f64 / 1000.0, format)
            }
            MemoryKind::Code { language, lines } => {
                format!("Code ({}, {} lines)", language, lines)
            }
            MemoryKind::Document { format, page_count } => {
                let pages = page_count
                    .map(|p| format!(", {} pages", p))
                    .unwrap_or_default();
                format!("Document ({:?}{})", format, pages)
            }
            MemoryKind::Spreadsheet { sheet_count } => {
                format!("Spreadsheet ({} sheets)", sheet_count)
            }
            MemoryKind::Presentation { slide_count } => {
                format!("Presentation ({} slides)", slide_count)
            }
            MemoryKind::Archive { item_count } => {
                format!("Archive ({} items)", item_count)
            }
            MemoryKind::Database => "Database".to_string(),
            MemoryKind::Folder => "Folder".to_string(),
            MemoryKind::Unknown => "Unknown".to_string(),
        }
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// Convert AI suggestions to Tag structs
impl TagSuggestion {
    pub fn to_tag(&self) -> Tag {
        Tag {
            name: self.name.clone(),
            source: TagSource::Ai,
            confidence: Some(self.confidence),
        }
    }
}

impl ClaudeClient {
    /// Summarize a text document
    pub async fn summarize_text(&self, text: &str, file_name: &str) -> Result<DocumentSummary> {
        // Truncate text if too long (Claude has token limits)
        let max_chars = 50_000;
        let truncated = if text.len() > max_chars {
            &text[..max_chars]
        } else {
            text
        };

        let prompt = format!(
            r#"Analyze and summarize this document.

File: {file_name}

Content:
---
{truncated}
---

Respond in this exact JSON format:
{{
  "summary": "2-3 sentence summary of the document",
  "key_topics": ["topic1", "topic2", "topic3"],
  "entities": {{
    "people": ["name1", "name2"],
    "organizations": ["org1"],
    "locations": ["place1"],
    "technologies": ["tech1", "tech2"]
  }},
  "document_type": "article|code|notes|report|other",
  "sentiment": "positive|negative|neutral|mixed",
  "complexity": "simple|moderate|complex"
}}"#
        );

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: vec![ContentBlock::Text { text: prompt }],
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse response: {}", e)))?;

        self.parse_summary_response(&claude_response)
    }

    /// Summarize code file
    pub async fn summarize_code(
        &self,
        code: &str,
        language: &str,
        file_name: &str,
    ) -> Result<CodeSummary> {
        let max_chars = 30_000;
        let truncated = if code.len() > max_chars {
            &code[..max_chars]
        } else {
            code
        };

        let prompt = format!(
            r#"Analyze this {language} code file.

File: {file_name}

Code:
```{language}
{truncated}
```

Respond in this exact JSON format:
{{
  "summary": "Brief description of what this code does",
  "purpose": "main|library|utility|test|config|other",
  "main_functionality": ["function1 - description", "function2 - description"],
  "dependencies": ["dep1", "dep2"],
  "complexity": "simple|moderate|complex",
  "patterns": ["pattern1", "pattern2"],
  "suggested_tags": ["tag1", "tag2"]
}}"#
        );

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: vec![ContentBlock::Text { text: prompt }],
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse response: {}", e)))?;

        self.parse_code_summary_response(&claude_response)
    }

    fn parse_summary_response(&self, response: &ClaudeResponse) -> Result<DocumentSummary> {
        let text = response
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| HippoError::Other("Empty response from Claude".to_string()))?;

        let json_str = Self::extract_json(text);

        #[derive(Deserialize)]
        struct SummaryJson {
            summary: String,
            key_topics: Vec<String>,
            entities: Option<EntitiesJson>,
            document_type: Option<String>,
            sentiment: Option<String>,
            complexity: Option<String>,
        }

        #[derive(Deserialize)]
        struct EntitiesJson {
            people: Option<Vec<String>>,
            organizations: Option<Vec<String>>,
            locations: Option<Vec<String>>,
            technologies: Option<Vec<String>>,
            dates: Option<Vec<String>>,
        }

        match serde_json::from_str::<SummaryJson>(&json_str) {
            Ok(parsed) => {
                let entities = parsed.entities.map(|e| ExtractedEntities {
                    people: e.people.unwrap_or_default(),
                    organizations: e.organizations.unwrap_or_default(),
                    locations: e.locations.unwrap_or_default(),
                    technologies: e.technologies.unwrap_or_default(),
                    dates: e.dates.unwrap_or_default(),
                });

                Ok(DocumentSummary {
                    summary: parsed.summary,
                    key_topics: parsed.key_topics,
                    entities,
                    document_type: parsed.document_type,
                    sentiment: parsed.sentiment,
                    complexity: parsed.complexity,
                })
            }
            Err(e) => {
                warn!("Failed to parse summary response: {}. Raw: {}", e, text);
                Ok(DocumentSummary {
                    summary: text.chars().take(500).collect(),
                    key_topics: Vec::new(),
                    entities: None,
                    document_type: None,
                    sentiment: None,
                    complexity: None,
                })
            }
        }
    }

    fn parse_code_summary_response(&self, response: &ClaudeResponse) -> Result<CodeSummary> {
        let text = response
            .content
            .first()
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| HippoError::Other("Empty response from Claude".to_string()))?;

        let json_str = Self::extract_json(text);

        #[derive(Deserialize)]
        struct CodeJson {
            summary: String,
            purpose: Option<String>,
            main_functionality: Option<Vec<String>>,
            dependencies: Option<Vec<String>>,
            complexity: Option<String>,
            patterns: Option<Vec<String>>,
            suggested_tags: Option<Vec<String>>,
        }

        match serde_json::from_str::<CodeJson>(&json_str) {
            Ok(parsed) => Ok(CodeSummary {
                summary: parsed.summary,
                purpose: parsed.purpose,
                main_functionality: parsed.main_functionality.unwrap_or_default(),
                dependencies: parsed.dependencies.unwrap_or_default(),
                complexity: parsed.complexity,
                patterns: parsed.patterns.unwrap_or_default(),
                suggested_tags: parsed.suggested_tags.unwrap_or_default(),
            }),
            Err(e) => {
                warn!(
                    "Failed to parse code summary response: {}. Raw: {}",
                    e, text
                );
                Ok(CodeSummary {
                    summary: text.chars().take(500).collect(),
                    purpose: None,
                    main_functionality: Vec::new(),
                    dependencies: Vec::new(),
                    complexity: None,
                    patterns: Vec::new(),
                    suggested_tags: Vec::new(),
                })
            }
        }
    }
}

/// Document summary from AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub summary: String,
    pub key_topics: Vec<String>,
    pub entities: Option<ExtractedEntities>,
    pub document_type: Option<String>,
    pub sentiment: Option<String>,
    pub complexity: Option<String>,
}

/// Code analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSummary {
    pub summary: String,
    pub purpose: Option<String>,
    pub main_functionality: Vec<String>,
    pub dependencies: Vec<String>,
    pub complexity: Option<String>,
    pub patterns: Vec<String>,
    pub suggested_tags: Vec<String>,
}

// ==================== Unified AI Client ====================

/// AI provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AiProvider {
    /// Use Anthropic's Claude API (requires API key)
    Claude,
    /// Use local Ollama instance (privacy-first, no API key needed)
    #[default]
    Ollama,
}

/// Configuration for the unified AI client
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub claude_api_key: Option<String>,
    pub ollama_url: String,
    pub ollama_embedding_model: String,
    pub ollama_generation_model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Ollama,
            claude_api_key: None,
            ollama_url: crate::ollama::DEFAULT_OLLAMA_URL.to_string(),
            ollama_embedding_model: crate::ollama::DEFAULT_EMBEDDING_MODEL.to_string(),
            ollama_generation_model: crate::ollama::DEFAULT_GENERATION_MODEL.to_string(),
        }
    }
}

/// Unified AI client that supports both Claude and Ollama backends
pub struct UnifiedAiClient {
    config: AiConfig,
    claude: Option<ClaudeClient>,
    ollama: OllamaClient,
}

impl UnifiedAiClient {
    /// Create a new unified AI client with default configuration (Ollama)
    pub fn new() -> Self {
        Self::with_config(AiConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: AiConfig) -> Self {
        let claude = config
            .claude_api_key
            .as_ref()
            .map(|key| ClaudeClient::new(key.clone()));

        let ollama_config = OllamaConfig {
            base_url: config.ollama_url.clone(),
            embedding_model: config.ollama_embedding_model.clone(),
            generation_model: config.ollama_generation_model.clone(),
            timeout_secs: 120,
        };
        let ollama = OllamaClient::with_config(ollama_config);

        Self {
            config,
            claude,
            ollama,
        }
    }

    /// Create with Claude API key
    pub fn with_claude(api_key: String) -> Self {
        Self::with_config(AiConfig {
            provider: AiProvider::Claude,
            claude_api_key: Some(api_key),
            ..Default::default()
        })
    }

    /// Create with Ollama only
    pub fn with_ollama(url: Option<String>) -> Self {
        Self::with_config(AiConfig {
            provider: AiProvider::Ollama,
            ollama_url: url.unwrap_or_else(|| crate::ollama::DEFAULT_OLLAMA_URL.to_string()),
            ..Default::default()
        })
    }

    /// Get the current provider
    pub fn provider(&self) -> AiProvider {
        self.config.provider
    }

    /// Set the AI provider
    pub fn set_provider(&mut self, provider: AiProvider) {
        self.config.provider = provider;
    }

    /// Set Claude API key
    pub fn set_claude_key(&mut self, api_key: String) {
        self.config.claude_api_key = Some(api_key.clone());
        self.claude = Some(ClaudeClient::new(api_key));
    }

    /// Set Ollama models
    pub fn set_ollama_models(
        &mut self,
        embedding_model: Option<String>,
        generation_model: Option<String>,
    ) {
        if let Some(model) = embedding_model {
            self.config.ollama_embedding_model = model;
        }
        if let Some(model) = generation_model {
            self.config.ollama_generation_model = model;
        }
        // Recreate Ollama client with new config
        let ollama_config = OllamaConfig {
            base_url: self.config.ollama_url.clone(),
            embedding_model: self.config.ollama_embedding_model.clone(),
            generation_model: self.config.ollama_generation_model.clone(),
            timeout_secs: 120,
        };
        self.ollama = OllamaClient::with_config(ollama_config);
    }

    /// Check if the current provider is available
    pub async fn is_available(&self) -> bool {
        match self.config.provider {
            AiProvider::Claude => self.claude.is_some(),
            AiProvider::Ollama => self.ollama.is_available().await,
        }
    }

    /// Get Ollama client reference
    pub fn ollama(&self) -> &OllamaClient {
        &self.ollama
    }

    /// Analyze a file and suggest tags
    pub async fn analyze_file(&self, memory: &Memory) -> Result<FileAnalysis> {
        match self.config.provider {
            AiProvider::Claude => {
                if let Some(claude) = &self.claude {
                    claude.analyze_file(memory).await
                } else {
                    Err(HippoError::Other(
                        "Claude API key not configured".to_string(),
                    ))
                }
            }
            AiProvider::Ollama => self.analyze_file_with_ollama(memory).await,
        }
    }

    /// Analyze file using Ollama
    async fn analyze_file_with_ollama(&self, memory: &Memory) -> Result<FileAnalysis> {
        let file_name = memory
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // For code files, read and analyze content
        if let MemoryKind::Code { language, .. } = &memory.kind {
            if let Ok(code) = std::fs::read_to_string(&memory.path) {
                let analysis = self
                    .ollama
                    .analyze_code(&code, language, &file_name)
                    .await?;
                return Ok(self.local_analysis_to_file_analysis(analysis));
            }
        }

        // For documents, try to read content
        if let MemoryKind::Document { .. } = &memory.kind {
            if let Ok(text) = std::fs::read_to_string(&memory.path) {
                let analysis = self.ollama.analyze_document(&text, &file_name).await?;
                return Ok(self.local_analysis_to_file_analysis(analysis));
            }
        }

        // For other files, analyze based on metadata
        let kind_str = ClaudeClient::kind_to_string(&memory.kind);
        let prompt = format!(
            "Analyze this file and suggest tags:\nFile: {}\nType: {}\nSize: {} bytes\n\nSuggest relevant tags for organization.",
            file_name,
            kind_str,
            memory.metadata.file_size
        );

        let response = self.ollama.generate(&prompt, Some(
            "You are a file organization assistant. Suggest relevant tags as a JSON object with 'tags' array containing objects with 'name', 'confidence' (0-100), and 'reason' fields. Also include 'description' field."
        )).await?;

        self.parse_ollama_analysis_response(&response)
    }

    fn local_analysis_to_file_analysis(&self, analysis: LocalAnalysis) -> FileAnalysis {
        let tags = analysis
            .suggested_tags
            .into_iter()
            .map(|name| TagSuggestion {
                name: name.to_lowercase().replace(' ', "-"),
                confidence: 75,
                reason: "AI suggested".to_string(),
            })
            .collect();

        FileAnalysis {
            tags,
            description: Some(analysis.summary),
            organization: None,
        }
    }

    fn parse_ollama_analysis_response(&self, response: &str) -> Result<FileAnalysis> {
        // Try to extract JSON from response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        #[derive(Deserialize)]
        struct AnalysisJson {
            tags: Option<Vec<TagJson>>,
            description: Option<String>,
        }

        #[derive(Deserialize)]
        struct TagJson {
            name: String,
            #[serde(default)]
            confidence: Option<u8>,
            #[serde(default)]
            reason: Option<String>,
        }

        match serde_json::from_str::<AnalysisJson>(json_str) {
            Ok(parsed) => {
                let tags = parsed
                    .tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| TagSuggestion {
                        name: t.name.to_lowercase().replace(' ', "-"),
                        confidence: t.confidence.unwrap_or(70).min(100),
                        reason: t.reason.unwrap_or_else(|| "AI suggested".to_string()),
                    })
                    .collect();

                Ok(FileAnalysis {
                    tags,
                    description: parsed.description,
                    organization: None,
                })
            }
            Err(_) => {
                // Return basic analysis from raw response
                Ok(FileAnalysis {
                    tags: Vec::new(),
                    description: Some(response.chars().take(300).collect()),
                    organization: None,
                })
            }
        }
    }

    /// Summarize text content
    pub async fn summarize(&self, content: &str, file_name: &str) -> Result<DocumentSummary> {
        match self.config.provider {
            AiProvider::Claude => {
                if let Some(claude) = &self.claude {
                    claude.summarize_text(content, file_name).await
                } else {
                    Err(HippoError::Other(
                        "Claude API key not configured".to_string(),
                    ))
                }
            }
            AiProvider::Ollama => {
                let analysis = self.ollama.analyze_document(content, file_name).await?;
                Ok(DocumentSummary {
                    summary: analysis.summary,
                    key_topics: analysis.key_topics,
                    entities: None,
                    document_type: analysis.document_type,
                    sentiment: None,
                    complexity: None,
                })
            }
        }
    }

    /// Summarize code
    pub async fn summarize_code(
        &self,
        code: &str,
        language: &str,
        file_name: &str,
    ) -> Result<CodeSummary> {
        match self.config.provider {
            AiProvider::Claude => {
                if let Some(claude) = &self.claude {
                    claude.summarize_code(code, language, file_name).await
                } else {
                    Err(HippoError::Other(
                        "Claude API key not configured".to_string(),
                    ))
                }
            }
            AiProvider::Ollama => {
                let analysis = self.ollama.analyze_code(code, language, file_name).await?;
                Ok(CodeSummary {
                    summary: analysis.summary,
                    purpose: analysis.document_type,
                    main_functionality: analysis.key_topics,
                    dependencies: Vec::new(),
                    complexity: None,
                    patterns: Vec::new(),
                    suggested_tags: analysis.suggested_tags,
                })
            }
        }
    }

    /// Generate embeddings for texts
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Embeddings always use Ollama for local processing
        self.ollama.embed(texts).await
    }

    /// Generate embedding for a single text
    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        self.ollama.embed_single(text).await
    }

    /// RAG query - answer questions using context from indexed files
    pub async fn rag_query(
        &self,
        query: &str,
        documents: Vec<(String, String, f32)>,
    ) -> Result<String> {
        let rag_docs: Vec<RagDocument> = documents
            .into_iter()
            .map(|(content, source, score)| RagDocument {
                content,
                source,
                relevance_score: score,
            })
            .collect();

        let context = RagContext {
            query: query.to_string(),
            documents: rag_docs,
        };

        self.ollama.rag_query(&context).await
    }

    /// Chat with conversation history
    pub async fn chat(&self, messages: Vec<(String, String)>) -> Result<String> {
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .map(|(role, content)| ChatMessage::new(&role, &content))
            .collect();

        self.ollama.chat(&chat_messages).await
    }

    /// Stream chat with conversation history (yields chunks as they arrive)
    pub async fn stream_chat<F>(
        &self,
        messages: Vec<(String, String)>,
        cancellation_token: CancellationToken,
        on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .map(|(role, content)| ChatMessage::new(&role, &content))
            .collect();

        // For now, streaming is only supported via Ollama
        self.ollama
            .stream_chat(&chat_messages, cancellation_token, on_chunk)
            .await
    }

    /// Get organization suggestions
    pub async fn suggest_organization(
        &self,
        memories: &[Memory],
    ) -> Result<Vec<(String, OrganizationSuggestion)>> {
        match self.config.provider {
            AiProvider::Claude => {
                if let Some(claude) = &self.claude {
                    claude.suggest_organization(memories).await
                } else {
                    Err(HippoError::Other(
                        "Claude API key not configured".to_string(),
                    ))
                }
            }
            AiProvider::Ollama => {
                let descriptions: Vec<(&str, &str)> = memories
                    .iter()
                    .take(20)
                    .filter_map(|m| {
                        let name = m.path.file_name()?.to_str()?;
                        let desc = ClaudeClient::kind_to_string(&m.kind);
                        Some((name, desc.leak() as &str))
                    })
                    .collect();

                let suggestion = self.ollama.suggest_organization(&descriptions).await?;

                // Parse the response and create suggestions
                Ok(memories
                    .iter()
                    .take(20)
                    .map(|m| {
                        (
                            m.id.to_string(),
                            OrganizationSuggestion {
                                suggested_folder: "Organized".to_string(),
                                reason: suggestion.clone(),
                            },
                        )
                    })
                    .collect())
            }
        }
    }

    /// Suggest tags for a memory (smart tag suggestions)
    pub async fn suggest_tags_for_memory(&self, memory: &Memory) -> Result<Vec<TagSuggestion>> {
        // Use the existing analyze_file method but only return tags
        let analysis = self.analyze_file(memory).await?;
        Ok(analysis.tags)
    }

    /// Find similar files using various heuristics (name, type, tags, content)
    pub fn suggest_similar_files(
        &self,
        target: &Memory,
        all_memories: &[Memory],
        limit: usize,
    ) -> Vec<SimilarFile> {
        let mut scored: Vec<(Memory, f32, Vec<String>)> = Vec::new();

        let target_type = std::mem::discriminant(&target.kind);
        let target_ext = target
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let target_name = target
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        for memory in all_memories {
            if memory.id == target.id {
                continue;
            }

            let mut score: f32 = 0.0;
            let mut reasons: Vec<String> = Vec::new();

            // Same type (30 points)
            if std::mem::discriminant(&memory.kind) == target_type {
                score += 0.3;
                reasons.push("same type".to_string());
            }

            // Same extension (20 points)
            let ext = memory
                .path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext == target_ext && !target_ext.is_empty() {
                score += 0.2;
            }

            // Similar name (up to 25 points)
            let name = memory
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let target_words: std::collections::HashSet<&str> = target_name
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() > 2)
                .collect();
            let name_words: std::collections::HashSet<&str> = name
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() > 2)
                .collect();
            let common_words = target_words.intersection(&name_words).count();
            if common_words > 0 {
                score += (common_words as f32 * 0.1).min(0.25);
                reasons.push(format!("{} shared keywords", common_words));
            }

            // Same folder (10 points)
            if memory.path.parent() == target.path.parent() {
                score += 0.1;
                reasons.push("same folder".to_string());
            }

            // Shared tags (up to 20 points)
            let shared_tags: Vec<String> = memory
                .tags
                .iter()
                .filter(|t| target.tags.iter().any(|tt| tt.name == t.name))
                .map(|t| t.name.clone())
                .collect();
            if !shared_tags.is_empty() {
                score += (shared_tags.len() as f32 * 0.1).min(0.2);
                reasons.push(format!("{} shared tags", shared_tags.len()));
            }

            // Similar file size (5 points)
            if target.metadata.file_size > 0 {
                let size_ratio =
                    memory.metadata.file_size as f64 / target.metadata.file_size as f64;
                if (0.7..1.3).contains(&size_ratio) {
                    score += 0.05;
                }
            }

            // Similar dimensions for images (15 points)
            if let (
                crate::MemoryKind::Image {
                    width: tw,
                    height: th,
                    ..
                },
                crate::MemoryKind::Image {
                    width: mw,
                    height: mh,
                    ..
                },
            ) = (&target.kind, &memory.kind)
            {
                let w_ratio = *mw as f64 / *tw as f64;
                let h_ratio = *mh as f64 / *th as f64;
                if (0.8..1.2).contains(&w_ratio) && (0.8..1.2).contains(&h_ratio) {
                    score += 0.15;
                    reasons.push("similar dimensions".to_string());
                }
            }

            // Only include if score is significant
            if score >= 0.25 {
                scored.push((memory.clone(), score, reasons));
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        scored
            .into_iter()
            .take(limit)
            .map(|(memory, score, reasons)| SimilarFile {
                memory,
                similarity_score: score,
                reasons,
            })
            .collect()
    }

    /// Suggest duplicate files with confidence scoring
    pub fn suggest_duplicates(
        &self,
        target: &Memory,
        all_memories: &[Memory],
    ) -> Vec<DuplicateMatch> {
        let mut duplicates: Vec<DuplicateMatch> = Vec::new();

        for memory in all_memories {
            if memory.id == target.id {
                continue;
            }

            // Check for exact hash match
            if let (Some(target_hash), Some(memory_hash)) =
                (&target.metadata.hash, &memory.metadata.hash)
            {
                if target_hash == memory_hash {
                    duplicates.push(DuplicateMatch {
                        memory_id: memory.id,
                        path: memory.path.clone(),
                        similarity_type: DuplicateType::ExactHash,
                        confidence: 100,
                        reason: "Identical file content (hash match)".to_string(),
                    });
                    continue;
                }
            }

            // Check for very similar names
            let target_name = target
                .path
                .file_stem()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let memory_name = memory
                .path
                .file_stem()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            // Remove common suffixes like (1), _copy, etc.
            let clean_target = target_name
                .replace(" (1)", "")
                .replace("_copy", "")
                .replace("-copy", "")
                .trim()
                .to_string();
            let clean_memory = memory_name
                .replace(" (1)", "")
                .replace("_copy", "")
                .replace("-copy", "")
                .trim()
                .to_string();

            if clean_target == clean_memory && !clean_target.is_empty() {
                duplicates.push(DuplicateMatch {
                    memory_id: memory.id,
                    path: memory.path.clone(),
                    similarity_type: DuplicateType::SimilarName,
                    confidence: 85,
                    reason: "Very similar name (possible copy)".to_string(),
                });
                continue;
            }

            // Check for similar dimensions in images
            if let (
                crate::MemoryKind::Image {
                    width: tw,
                    height: th,
                    ..
                },
                crate::MemoryKind::Image {
                    width: mw,
                    height: mh,
                    ..
                },
            ) = (&target.kind, &memory.kind)
            {
                if tw == mw && th == mh && target.metadata.file_size == memory.metadata.file_size {
                    duplicates.push(DuplicateMatch {
                        memory_id: memory.id,
                        path: memory.path.clone(),
                        similarity_type: DuplicateType::SimilarDimensions,
                        confidence: 70,
                        reason: "Same dimensions and size".to_string(),
                    });
                }
            }
        }

        // Sort by confidence descending
        duplicates.sort_by_key(|d| std::cmp::Reverse(d.confidence));
        duplicates.into_iter().take(5).collect()
    }

    /// Suggest organization groupings based on file patterns
    pub async fn suggest_groupings(
        &self,
        memories: &[Memory],
    ) -> Result<Vec<CollectionSuggestion>> {
        let mut suggestions: Vec<CollectionSuggestion> = Vec::new();

        // Group by common tags
        let mut tag_groups: std::collections::HashMap<String, Vec<crate::MemoryId>> =
            std::collections::HashMap::new();

        for memory in memories {
            for tag in &memory.tags {
                tag_groups
                    .entry(tag.name.clone())
                    .or_default()
                    .push(memory.id);
            }
        }

        // Create suggestions for tag groups with 3+ files
        for (tag_name, ids) in tag_groups {
            if ids.len() >= 3 {
                suggestions.push(CollectionSuggestion {
                    name: format!("{} Collection", tag_name),
                    description: format!("Files tagged with '{}'", tag_name),
                    memory_ids: ids,
                    confidence: 80,
                    reason: "Common tag".to_string(),
                });
            }
        }

        // Group by file type
        let mut type_groups: std::collections::HashMap<String, Vec<crate::MemoryId>> =
            std::collections::HashMap::new();

        for memory in memories {
            let type_name = match &memory.kind {
                crate::MemoryKind::Image { .. } => "Images",
                crate::MemoryKind::Video { .. } => "Videos",
                crate::MemoryKind::Audio { .. } => "Audio",
                crate::MemoryKind::Code { .. } => "Code",
                crate::MemoryKind::Document { .. } => "Documents",
                _ => continue,
            };
            type_groups
                .entry(type_name.to_string())
                .or_default()
                .push(memory.id);
        }

        for (type_name, ids) in type_groups {
            if ids.len() >= 5 {
                suggestions.push(CollectionSuggestion {
                    name: type_name.clone(),
                    description: format!("All {} files", type_name.to_lowercase()),
                    memory_ids: ids,
                    confidence: 90,
                    reason: "Same file type".to_string(),
                });
            }
        }

        // Sort by confidence
        suggestions.sort_by_key(|s| std::cmp::Reverse(s.confidence));
        Ok(suggestions.into_iter().take(5).collect())
    }
}

impl Default for UnifiedAiClient {
    fn default() -> Self {
        Self::new()
    }
}
