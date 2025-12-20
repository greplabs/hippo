//! Ollama integration for local AI capabilities
//!
//! Supports local embeddings, text generation, and RAG pipelines.
//! Includes support for high-quality models like Gemma2, Llama3.2, and Qwen2.5.

use base64::Engine;
use crate::error::{HippoError, Result};
use futures_util::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Stream chunk from Ollama
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub text: String,
    pub done: bool,
}

/// Default Ollama API endpoint
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default embedding model
pub const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";

/// Default generation model - Gemma2 2B is fast, smart, and lightweight (~1.6GB)
pub const DEFAULT_GENERATION_MODEL: &str = "gemma2:2b";

/// Recommended models for different use cases
pub struct RecommendedModels;

impl RecommendedModels {
    /// Fast and lightweight models (~1-2GB) - good for quick responses
    pub const FAST: &'static [&'static str] = &[
        "gemma2:2b",    // Google's Gemma2 2B - excellent quality/speed ratio
        "qwen2.5:1.5b", // Alibaba's Qwen 2.5 - very fast
        "llama3.2:1b",  // Meta's Llama 3.2 1B - smallest Llama
        "phi3:mini",    // Microsoft's Phi-3 mini
    ];

    /// Balanced models (~4-8GB) - good quality with reasonable speed
    pub const BALANCED: &'static [&'static str] = &[
        "gemma2:9b",   // Google's Gemma2 9B - high quality
        "llama3.2:3b", // Meta's Llama 3.2 3B
        "qwen2.5:7b",  // Alibaba's Qwen 2.5 7B
        "mistral:7b",  // Mistral 7B - great all-rounder
    ];

    /// High quality models (~12-20GB) - best results, slower
    pub const QUALITY: &'static [&'static str] = &[
        "llama3.1:8b",         // Meta's Llama 3.1 8B
        "gemma2:27b",          // Google's Gemma2 27B
        "qwen2.5:14b",         // Alibaba's Qwen 2.5 14B
        "deepseek-coder:6.7b", // DeepSeek for code
    ];

    /// Best embedding models
    pub const EMBEDDINGS: &'static [&'static str] = &[
        "nomic-embed-text",  // Good general purpose
        "mxbai-embed-large", // Higher quality embeddings
        "all-minilm",        // Fast, smaller embeddings
    ];

    /// Vision models for image understanding
    pub const VISION: &'static [&'static str] = &[
        "llava:7b",  // LLaVA for image understanding
        "llava:13b", // Larger LLaVA
        "bakllava",  // BakLLaVA variant
    ];

    /// Get the best available model from a category
    pub async fn get_best_available(client: &OllamaClient, category: &[&str]) -> Option<String> {
        if let Ok(models) = client.list_models().await {
            let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
            for preferred in category {
                // Check exact match or prefix match (for versioned models)
                if model_names
                    .iter()
                    .any(|m| m == *preferred || m.starts_with(&format!("{}:", preferred)))
                {
                    return Some(preferred.to_string());
                }
            }
        }
        None
    }
}

/// Embedding dimension for nomic-embed-text
pub const NOMIC_EMBED_DIM: usize = 768;

/// Ollama client for local AI operations
pub struct OllamaClient {
    client: Client,
    base_url: String,
    embedding_model: String,
    generation_model: String,
}

/// Configuration for Ollama client
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub embedding_model: String,
    pub generation_model: String,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            generation_model: DEFAULT_GENERATION_MODEL.to_string(),
            timeout_secs: 120,
        }
    }
}

/// Information about an Ollama model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    pub format: Option<String>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

/// Response from Ollama /api/tags endpoint
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<OllamaModel>,
}

/// Request body for embedding generation
#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
}

/// Response from embedding endpoint
#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Request body for text generation
#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i64>>,
    options: GenerateOptions,
}

#[derive(Debug, Serialize, Default)]
struct GenerateOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Response from generate endpoint
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields required for API deserialization
struct GenerateResponse {
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    context: Option<Vec<i64>>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Chat message for conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>, // Base64-encoded images
}

impl ChatMessage {
    /// Create a new text message
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            images: None,
        }
    }

    /// Create a new message with images
    pub fn with_images(role: &str, content: &str, images: Vec<String>) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            images: Some(images),
        }
    }
}

/// Request body for chat endpoint
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    options: GenerateOptions,
}

/// Response from chat endpoint
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields required for API deserialization
struct ChatResponse {
    message: ChatMessage,
    #[serde(default)]
    done: bool,
}

/// Pull request for downloading models
#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
    stream: bool,
}

/// Pull progress response
#[derive(Debug, Deserialize)]
pub struct PullProgress {
    pub status: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub completed: Option<u64>,
}

/// Result of an AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAnalysis {
    pub summary: String,
    pub key_topics: Vec<String>,
    pub suggested_tags: Vec<String>,
    pub document_type: Option<String>,
    pub language: Option<String>,
}

/// RAG context for generation
#[derive(Debug, Clone)]
pub struct RagContext {
    pub query: String,
    pub documents: Vec<RagDocument>,
}

#[derive(Debug, Clone)]
pub struct RagDocument {
    pub content: String,
    pub source: String,
    pub relevance_score: f32,
}

impl OllamaClient {
    /// Create a new Ollama client with default configuration
    pub fn new() -> Self {
        Self::with_config(OllamaConfig::default())
    }

    /// Create a new Ollama client with custom configuration
    pub fn with_config(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: config.base_url,
            embedding_model: config.embedding_model,
            generation_model: config.generation_model,
        }
    }

    /// Check if Ollama is running and accessible
    pub async fn is_available(&self) -> bool {
        match self.client.get(&self.base_url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get Ollama version info
    pub async fn version(&self) -> Result<String> {
        let resp = self
            .client
            .get(&self.base_url)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to connect to Ollama: {}", e)))?;

        if resp.status().is_success() {
            Ok(resp
                .text()
                .await
                .unwrap_or_else(|_| "Ollama is running".to_string()))
        } else {
            Err(HippoError::Other("Ollama not available".to_string()))
        }
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to list models: {}", e)))?;

        if !resp.status().is_success() {
            return Err(HippoError::Other("Failed to list models".to_string()));
        }

        let models: ModelsResponse = resp
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse models: {}", e)))?;

        Ok(models.models)
    }

    /// Check if a specific model is available
    pub async fn has_model(&self, name: &str) -> bool {
        match self.list_models().await {
            Ok(models) => models.iter().any(|m| m.name.starts_with(name)),
            Err(_) => false,
        }
    }

    /// Pull (download) a model
    pub async fn pull_model(&self, name: &str) -> Result<()> {
        info!("Pulling model: {}", name);
        let url = format!("{}/api/pull", self.base_url);

        let request = PullRequest {
            name: name.to_string(),
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to pull model: {}", e)))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Failed to pull model: {}",
                error
            )));
        }

        info!("Model {} pulled successfully", name);
        Ok(())
    }

    /// Generate embeddings for a list of texts
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let url = format!("{}/api/embed", self.base_url);

        let request = EmbedRequest {
            model: self.embedding_model.clone(),
            input: texts.to_vec(),
        };

        debug!("Generating embeddings for {} texts", texts.len());

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Embedding request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Embedding failed ({}): {}. Make sure '{}' model is installed.",
                status, error, self.embedding_model
            )));
        }

        let embed_resp: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse embeddings: {}", e)))?;

        Ok(embed_resp.embeddings)
    }

    /// Generate embedding for a single text
    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed(&[text.to_string()]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| HippoError::Other("No embedding returned".to_string()))
    }

    /// Generate text completion
    pub async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: self.generation_model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            system: system.map(String::from),
            context: None,
            options: GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(1024),
                top_p: Some(0.9),
            },
        };

        debug!("Generating text with model: {}", self.generation_model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Generate request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Generation failed ({}): {}. Make sure '{}' model is installed.",
                status, error, self.generation_model
            )));
        }

        let gen_resp: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse response: {}", e)))?;

        Ok(gen_resp.response)
    }

    /// Chat with conversation history
    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: self.generation_model.clone(),
            messages: messages.to_vec(),
            stream: false,
            options: GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(2048),
                top_p: Some(0.9),
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Chat request failed: {}", e)))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!("Chat failed: {}", error)));
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse chat response: {}", e)))?;

        Ok(chat_resp.message.content)
    }

    /// Stream chat responses (yields chunks as they arrive)
    pub async fn stream_chat<F>(
        &self,
        messages: &[ChatMessage],
        cancellation_token: CancellationToken,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: self.generation_model.clone(),
            messages: messages.to_vec(),
            stream: true, // Enable streaming
            options: GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(2048),
                top_p: Some(0.9),
            },
        };

        debug!("Starting streaming chat with model: {}", self.generation_model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(300)) // 5 minute timeout for streaming
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Chat request failed: {}", e)))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!("Chat failed: {}", error)));
        }

        let mut stream = resp.bytes_stream();
        let mut full_response = String::new();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancellation_token.is_cancelled() {
                debug!("Stream cancelled by user");
                return Err(HippoError::Other("Stream cancelled".to_string()));
            }

            let chunk = chunk_result
                .map_err(|e| HippoError::Other(format!("Stream error: {}", e)))?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete JSON lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Parse the streaming response
                match serde_json::from_str::<ChatResponse>(&line) {
                    Ok(chat_resp) => {
                        let content = &chat_resp.message.content;
                        if !content.is_empty() {
                            full_response.push_str(content);
                            on_chunk(content.clone());
                        }

                        if chat_resp.done {
                            debug!("Stream completed successfully");
                            return Ok(full_response);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse streaming chunk: {} - Line: {}", e, line);
                    }
                }
            }
        }

        Ok(full_response)
    }

    /// Stream text generation (yields chunks as they arrive)
    pub async fn stream_generate<F>(
        &self,
        prompt: &str,
        system: Option<&str>,
        cancellation_token: CancellationToken,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) + Send,
    {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: self.generation_model.clone(),
            prompt: prompt.to_string(),
            stream: true, // Enable streaming
            system: system.map(String::from),
            context: None,
            options: GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(1024),
                top_p: Some(0.9),
            },
        };

        debug!("Starting streaming generation with model: {}", self.generation_model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(300)) // 5 minute timeout for streaming
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Generate request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!(
                "Generation failed ({}): {}. Make sure '{}' model is installed.",
                status, error, self.generation_model
            )));
        }

        let mut stream = resp.bytes_stream();
        let mut full_response = String::new();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancellation_token.is_cancelled() {
                debug!("Stream cancelled by user");
                return Err(HippoError::Other("Stream cancelled".to_string()));
            }

            let chunk = chunk_result
                .map_err(|e| HippoError::Other(format!("Stream error: {}", e)))?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete JSON lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Parse the streaming response
                match serde_json::from_str::<GenerateResponse>(&line) {
                    Ok(gen_resp) => {
                        let content = &gen_resp.response;
                        if !content.is_empty() {
                            full_response.push_str(content);
                            on_chunk(content.clone());
                        }

                        if gen_resp.done {
                            debug!("Stream completed successfully");
                            return Ok(full_response);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse streaming chunk: {} - Line: {}", e, line);
                    }
                }
            }
        }

        Ok(full_response)
    }

    /// Analyze a document and extract structured information
    pub async fn analyze_document(&self, content: &str, file_name: &str) -> Result<LocalAnalysis> {
        let system_prompt = r#"You are a document analysis assistant. Analyze the provided document and return a JSON response with:
- summary: A concise 2-3 sentence summary
- key_topics: Array of 3-5 main topics/themes
- suggested_tags: Array of 5-10 relevant tags for organization
- document_type: The type of document (e.g., "code", "article", "notes", "report")
- language: Programming language if code, or "text" for documents

Respond ONLY with valid JSON, no other text."#;

        let prompt = format!(
            "Analyze this file named '{}':\n\n{}\n\nRespond with JSON only.",
            file_name,
            &content[..content.len().min(8000)] // Truncate for context limits
        );

        let response = self.generate(&prompt, Some(system_prompt)).await?;

        // Try to parse JSON from response
        self.parse_analysis_response(&response)
    }

    /// Analyze code and extract information
    pub async fn analyze_code(
        &self,
        code: &str,
        language: &str,
        file_name: &str,
    ) -> Result<LocalAnalysis> {
        let system_prompt = r#"You are a code analysis assistant. Analyze the provided code and return a JSON response with:
- summary: What this code does (2-3 sentences)
- key_topics: Main concepts/patterns used (3-5 items)
- suggested_tags: Relevant tags for categorization (5-10 items)
- document_type: "code"
- language: The programming language

Respond ONLY with valid JSON, no other text."#;

        let prompt = format!(
            "Analyze this {} code file '{}':\n\n```{}\n{}\n```\n\nRespond with JSON only.",
            language,
            file_name,
            language,
            &code[..code.len().min(6000)]
        );

        let response = self.generate(&prompt, Some(system_prompt)).await?;
        self.parse_analysis_response(&response)
    }

    /// Summarize text content
    pub async fn summarize(&self, content: &str, max_length: usize) -> Result<String> {
        let system_prompt = "You are a summarization assistant. Provide a clear, concise summary of the given text. Be informative but brief.";

        let prompt = format!(
            "Summarize the following text in {} words or less:\n\n{}",
            max_length,
            &content[..content.len().min(10000)]
        );

        self.generate(&prompt, Some(system_prompt)).await
    }

    /// Answer a question using RAG context
    pub async fn rag_query(&self, context: &RagContext) -> Result<String> {
        let mut context_text = String::new();
        for (i, doc) in context.documents.iter().enumerate() {
            context_text.push_str(&format!(
                "\n--- Document {} (source: {}, relevance: {:.2}) ---\n{}\n",
                i + 1,
                doc.source,
                doc.relevance_score,
                doc.content
            ));
        }

        let system_prompt = r#"You are a helpful assistant that answers questions based on the provided context documents.
Use the information from the documents to answer the user's question accurately.
If the answer cannot be found in the documents, say so clearly.
Always cite which document(s) you used for your answer."#;

        let prompt = format!(
            "Context documents:{}\n\nUser question: {}\n\nAnswer based on the context above:",
            context_text, context.query
        );

        self.generate(&prompt, Some(system_prompt)).await
    }

    /// Extract entities from text
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<String>> {
        let system_prompt = "Extract named entities (people, organizations, locations, products, technologies) from the text. Return only a JSON array of strings with the entity names.";

        let prompt = format!(
            "Extract entities from:\n\n{}",
            &text[..text.len().min(4000)]
        );
        let response = self.generate(&prompt, Some(system_prompt)).await?;

        // Try to parse as JSON array
        if let Ok(entities) = serde_json::from_str::<Vec<String>>(&response) {
            Ok(entities)
        } else {
            // Fallback: split by newlines/commas
            Ok(response
                .lines()
                .flat_map(|line| line.split(','))
                .map(|s| {
                    s.trim()
                        .trim_matches(|c| c == '"' || c == '[' || c == ']')
                        .to_string()
                })
                .filter(|s| !s.is_empty() && s.len() > 1)
                .collect())
        }
    }

    /// Suggest how to organize files
    pub async fn suggest_organization(&self, file_descriptions: &[(&str, &str)]) -> Result<String> {
        let system_prompt = r#"You are a file organization expert. Based on the file descriptions provided, suggest a logical folder structure and organization scheme. Consider grouping by topic, type, project, or date as appropriate."#;

        let mut descriptions = String::new();
        for (name, desc) in file_descriptions.iter().take(20) {
            descriptions.push_str(&format!("- {}: {}\n", name, desc));
        }

        let prompt = format!(
            "Suggest how to organize these files:\n\n{}\n\nProvide a clear folder structure recommendation.",
            descriptions
        );

        self.generate(&prompt, Some(system_prompt)).await
    }

    /// Parse analysis response from JSON
    fn parse_analysis_response(&self, response: &str) -> Result<LocalAnalysis> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(json) => Ok(LocalAnalysis {
                summary: json
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No summary available")
                    .to_string(),
                key_topics: json
                    .get("key_topics")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                suggested_tags: json
                    .get("suggested_tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                document_type: json
                    .get("document_type")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                language: json
                    .get("language")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            }),
            Err(e) => {
                warn!("Failed to parse analysis JSON: {}", e);
                // Return a basic analysis from the raw response
                Ok(LocalAnalysis {
                    summary: response.chars().take(500).collect(),
                    key_topics: vec![],
                    suggested_tags: vec![],
                    document_type: None,
                    language: None,
                })
            }
        }
    }

    /// Set the embedding model
    pub fn set_embedding_model(&mut self, model: &str) {
        self.embedding_model = model.to_string();
    }

    /// Set the generation model
    pub fn set_generation_model(&mut self, model: &str) {
        self.generation_model = model.to_string();
    }

    /// Get current embedding model
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }

    /// Get current generation model
    pub fn generation_model(&self) -> &str {
        &self.generation_model
    }

    /// Caption an image using a vision model (llava:7b)
    pub async fn caption_image(&self, image_path: &std::path::Path) -> Result<String> {
        // Read image file and convert to base64
        let image_data = std::fs::read(image_path)
            .map_err(|e| HippoError::Other(format!("Failed to read image: {}", e)))?;
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_data);

        // Use llava:7b model for vision
        let vision_model = "llava:7b";

        // Check if vision model is available
        if !self.has_model(vision_model).await {
            return Err(HippoError::Other(format!(
                "Vision model '{}' not installed. Run: ollama pull {}",
                vision_model, vision_model
            )));
        }

        // Create chat request with image
        let url = format!("{}/api/chat", self.base_url);

        let messages = vec![ChatMessage::with_images(
            "user",
            "Describe this image in detail. What do you see?",
            vec![base64_image],
        )];

        let request = ChatRequest {
            model: vision_model.to_string(),
            messages,
            stream: false,
            options: GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(512),
                top_p: Some(0.9),
            },
        };

        debug!("Generating image caption with model: {}", vision_model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Vision request failed: {}", e)))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(HippoError::Other(format!("Vision failed: {}", error)));
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to parse vision response: {}", e)))?;

        Ok(chat_resp.message.content)
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_config() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, DEFAULT_OLLAMA_URL);
        assert_eq!(config.embedding_model, DEFAULT_EMBEDDING_MODEL);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = OllamaClient::new();
        assert_eq!(client.embedding_model(), DEFAULT_EMBEDDING_MODEL);
        assert_eq!(client.generation_model(), DEFAULT_GENERATION_MODEL);
    }
}
