//! AI-powered file analysis and auto-tagging using Claude API
//!
//! This module provides intelligent file analysis, tag suggestion,
//! and organization recommendations using Anthropic's Claude API.

use crate::{Memory, MemoryKind, Tag, TagSource, Result, HippoError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
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
#[derive(Debug, Clone)]
pub struct TagSuggestion {
    pub name: String,
    pub confidence: u8,
    pub reason: String,
}

/// Organization suggestion
#[derive(Debug, Clone)]
pub struct OrganizationSuggestion {
    pub suggested_folder: String,
    pub reason: String,
}

/// Full AI analysis result for a file
#[derive(Debug, Clone)]
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
        let file_name = memory.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let kind_str = Self::kind_to_string(&memory.kind);
        let existing_tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();

        // Build the prompt
        let prompt = self.build_analysis_prompt(&file_name, &kind_str, &memory.metadata, &existing_tags);

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

        let response = self.client
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
            return Err(HippoError::Other(format!("Claude API error {}: {}", status, body)));
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
    pub async fn suggest_organization(&self, memories: &[Memory]) -> Result<Vec<(String, OrganizationSuggestion)>> {
        if memories.is_empty() {
            return Ok(Vec::new());
        }

        // Build a summary of files
        let file_summary: Vec<String> = memories.iter()
            .take(50)
            .map(|m| {
                let name = m.path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let kind = Self::kind_to_string(&m.kind);
                let tags: Vec<String> = m.tags.iter().map(|t| t.name.clone()).collect();
                format!("- {} ({}){}", name, kind,
                    if tags.is_empty() { String::new() } else { format!(" [{}]", tags.join(", ")) })
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

        let response = self.client
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
            return Err(HippoError::Other(format!("Claude API error {}: {}", status, body)));
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

        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data);

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
            ContentBlock::Text { text: prompt.to_string() },
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
        let text = response.content
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
                let tags = parsed.tags
                    .into_iter()
                    .map(|t| TagSuggestion {
                        name: t.name.to_lowercase().replace(' ', "-"),
                        confidence: t.confidence.min(100),
                        reason: t.reason,
                    })
                    .collect();

                let organization = parsed.suggested_folder.map(|folder| {
                    OrganizationSuggestion {
                        suggested_folder: folder,
                        reason: "AI suggested organization".to_string(),
                    }
                });

                Ok(FileAnalysis {
                    tags,
                    description: parsed.description,
                    organization,
                })
            }
            Err(e) => {
                warn!("Failed to parse Claude response as JSON: {}. Raw: {}", e, text);
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
        memories: &[Memory]
    ) -> Result<Vec<(String, OrganizationSuggestion)>> {
        let text = response.content
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
                let results: Vec<(String, OrganizationSuggestion)> = parsed.suggestions
                    .into_iter()
                    .filter_map(|s| {
                        // Find the memory ID for this file
                        memories.iter()
                            .find(|m| {
                                m.path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default() == s.file
                            })
                            .map(|m| {
                                (m.id.to_string(), OrganizationSuggestion {
                                    suggested_folder: s.folder,
                                    reason: s.reason,
                                })
                            })
                    })
                    .collect();
                Ok(results)
            }
            Err(_) => Ok(Vec::new())
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
            MemoryKind::Image { width, height, format } => {
                format!("Image ({}x{}, {})", width, height, format)
            }
            MemoryKind::Video { duration_ms, format } => {
                format!("Video ({:.1}s, {})", *duration_ms as f64 / 1000.0, format)
            }
            MemoryKind::Audio { duration_ms, format } => {
                format!("Audio ({:.1}s, {})", *duration_ms as f64 / 1000.0, format)
            }
            MemoryKind::Code { language, lines } => {
                format!("Code ({}, {} lines)", language, lines)
            }
            MemoryKind::Document { format, page_count } => {
                let pages = page_count.map(|p| format!(", {} pages", p)).unwrap_or_default();
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
