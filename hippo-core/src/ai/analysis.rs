//! Deep file analysis module
//!
//! Provides comprehensive analysis capabilities for different file types:
//! - Images: object detection, scene recognition, OCR, color analysis, face detection
//! - Documents: summarization, topic extraction, entity recognition, sentiment analysis
//! - Code: language detection, complexity analysis, function extraction, dependency analysis
//! - Videos: key frame extraction, scene detection, content summary

use crate::error::{HippoError, Result};
use crate::ollama::OllamaClient;
use crate::{Memory, MemoryKind};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, warn};

// ==================== Image Analysis ====================

/// Comprehensive image analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    /// Detected objects in the image
    pub objects: Vec<DetectedObject>,
    /// Scene classification (e.g., "indoor", "outdoor", "beach", "city")
    pub scenes: Vec<String>,
    /// Extracted text from image (OCR)
    pub text_content: Option<String>,
    /// Dominant colors in the image
    pub dominant_colors: Vec<Color>,
    /// Number of faces detected
    pub faces_detected: u32,
    /// AI-generated image caption
    pub caption: Option<String>,
    /// Confidence score (0-100)
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub name: String,
    pub confidence: f32,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub hex: String,
    pub percentage: f32,
}

// ==================== Document Analysis ====================

/// Comprehensive document analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAnalysis {
    /// AI-generated summary (2-3 sentences)
    pub summary: String,
    /// Key topics extracted from the document
    pub topics: Vec<String>,
    /// Named entities found in the document
    pub entities: ExtractedEntities,
    /// Word count
    pub word_count: u32,
    /// Estimated reading time in minutes
    pub reading_time: u32,
    /// Document sentiment
    pub sentiment: Option<String>,
    /// Document complexity level
    pub complexity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub people: Vec<String>,
    pub organizations: Vec<String>,
    pub locations: Vec<String>,
    pub technologies: Vec<String>,
    pub dates: Vec<String>,
}

// ==================== Code Analysis ====================

/// Comprehensive code file analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    /// Programming language
    pub language: String,
    /// Total lines of code
    pub lines: u32,
    /// Extracted functions/methods
    pub functions: Vec<FunctionInfo>,
    /// Import statements
    pub imports: Vec<String>,
    /// Cyclomatic complexity score
    pub complexity_score: u32,
    /// AI-generated summary of what the code does
    pub summary: String,
    /// Detected code patterns or design patterns
    pub patterns: Vec<String>,
    /// Dependencies identified
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub is_public: bool,
    pub parameters: Vec<String>,
    pub doc_comment: Option<String>,
}

// ==================== Video Analysis ====================

/// Comprehensive video analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalysis {
    /// Duration in seconds
    pub duration: u64,
    /// Key frames extracted (timestamps in seconds)
    pub key_frames: Vec<f64>,
    /// Scene descriptions from key frames
    pub scene_descriptions: Vec<String>,
    /// AI-generated content summary
    pub content_summary: Option<String>,
    /// Estimated transcript (if audio present)
    pub transcript: Option<String>,
}

// ==================== Unified Analysis Result ====================

/// Unified analysis result that can hold any file type analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnalysisResult {
    Image(ImageAnalysis),
    Document(DocumentAnalysis),
    Code(CodeAnalysis),
    Video(VideoAnalysis),
    Unsupported { message: String },
}

impl AnalysisResult {
    /// Check if the analysis was successful
    pub fn is_supported(&self) -> bool {
        !matches!(self, AnalysisResult::Unsupported { .. })
    }

    /// Get a human-readable summary of the analysis
    pub fn summary(&self) -> String {
        match self {
            AnalysisResult::Image(img) => {
                format!(
                    "Found {} objects, {} scenes, {} colors",
                    img.objects.len(),
                    img.scenes.len(),
                    img.dominant_colors.len()
                )
            }
            AnalysisResult::Document(doc) => {
                format!("{} words, {} topics", doc.word_count, doc.topics.len())
            }
            AnalysisResult::Code(code) => {
                format!(
                    "{} language, {} lines, {} functions",
                    code.language,
                    code.lines,
                    code.functions.len()
                )
            }
            AnalysisResult::Video(vid) => {
                format!("{}s duration, {} scenes", vid.duration, vid.scene_descriptions.len())
            }
            AnalysisResult::Unsupported { message } => message.clone(),
        }
    }
}

// ==================== Analysis Functions ====================

/// Analyze an image file using Ollama vision model
pub async fn analyze_image(path: &Path, ollama: &OllamaClient) -> Result<ImageAnalysis> {
    debug!("Analyzing image: {}", path.display());

    // Check if file exists
    if !path.exists() {
        return Err(HippoError::Other("Image file not found".to_string()));
    }

    // Check if Ollama is available
    if !ollama.is_available().await {
        return Err(HippoError::Other("Ollama is not running".to_string()));
    }

    // Use vision model to analyze the image
    let caption = match ollama.caption_image(path).await {
        Ok(cap) => Some(cap),
        Err(e) => {
            warn!("Failed to generate caption: {}", e);
            None
        }
    };

    // Extract dominant colors from the image
    let dominant_colors = extract_dominant_colors(path)?;

    // Parse objects and scenes from caption if available
    let (objects, scenes) = if let Some(ref caption_text) = caption {
        parse_objects_and_scenes(caption_text)
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(ImageAnalysis {
        objects,
        scenes,
        text_content: None, // OCR not implemented yet
        dominant_colors,
        faces_detected: 0, // Face detection not implemented yet
        caption,
        confidence: 75,
    })
}

/// Analyze a document file using Ollama
pub async fn analyze_document(path: &Path, ollama: &OllamaClient) -> Result<DocumentAnalysis> {
    debug!("Analyzing document: {}", path.display());

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| HippoError::Other(format!("Failed to read document: {}", e)))?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Use Ollama to analyze
    let analysis = ollama.analyze_document(&content, &file_name).await?;

    // Count words
    let word_count = content.split_whitespace().count() as u32;
    let reading_time = (word_count / 200).max(1); // ~200 words per minute

    Ok(DocumentAnalysis {
        summary: analysis.summary,
        topics: analysis.key_topics,
        entities: ExtractedEntities {
            people: Vec::new(),
            organizations: Vec::new(),
            locations: Vec::new(),
            technologies: Vec::new(),
            dates: Vec::new(),
        },
        word_count,
        reading_time,
        sentiment: None,
        complexity: analysis.document_type,
    })
}

/// Analyze a code file using Ollama and static analysis
pub async fn analyze_code(path: &Path, ollama: &OllamaClient) -> Result<CodeAnalysis> {
    debug!("Analyzing code: {}", path.display());

    // Read file content
    let code = std::fs::read_to_string(path)
        .map_err(|e| HippoError::Other(format!("Failed to read code file: {}", e)))?;

    // Detect language from extension
    let language = detect_language(path);

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Use Ollama to analyze
    let analysis = ollama.analyze_code(&code, &language, &file_name).await?;

    // Count lines
    let lines = code.lines().count() as u32;

    // Extract imports (simple regex-based for now)
    let imports = extract_imports(&code, &language);

    // Calculate basic complexity (count branches and loops)
    let complexity_score = calculate_complexity(&code);

    Ok(CodeAnalysis {
        language: language.clone(),
        lines,
        functions: Vec::new(), // Function extraction would need tree-sitter integration
        imports,
        complexity_score,
        summary: analysis.summary,
        patterns: Vec::new(),
        dependencies: Vec::new(),
    })
}

/// Analyze a video file
pub async fn analyze_video(path: &Path, _ollama: &OllamaClient) -> Result<VideoAnalysis> {
    debug!("Analyzing video: {}", path.display());

    // For now, just extract basic metadata
    // Full video analysis would require ffmpeg integration

    Ok(VideoAnalysis {
        duration: 0,
        key_frames: Vec::new(),
        scene_descriptions: Vec::new(),
        content_summary: Some("Video analysis not fully implemented yet".to_string()),
        transcript: None,
    })
}

/// Main dispatch function to analyze any file based on its type
pub async fn analyze_file(memory: &Memory, ollama: &OllamaClient) -> Result<AnalysisResult> {
    match &memory.kind {
        MemoryKind::Image { .. } => {
            let analysis = analyze_image(&memory.path, ollama).await?;
            Ok(AnalysisResult::Image(analysis))
        }
        MemoryKind::Document { .. } => {
            let analysis = analyze_document(&memory.path, ollama).await?;
            Ok(AnalysisResult::Document(analysis))
        }
        MemoryKind::Code { .. } => {
            let analysis = analyze_code(&memory.path, ollama).await?;
            Ok(AnalysisResult::Code(analysis))
        }
        MemoryKind::Video { .. } => {
            let analysis = analyze_video(&memory.path, ollama).await?;
            Ok(AnalysisResult::Video(analysis))
        }
        _ => Ok(AnalysisResult::Unsupported {
            message: format!("Analysis not supported for {:?}", memory.kind),
        }),
    }
}

// ==================== Helper Functions ====================

/// Extract dominant colors from an image
fn extract_dominant_colors(path: &Path) -> Result<Vec<Color>> {
    use image::GenericImageView;

    let img = image::open(path)
        .map_err(|e| HippoError::Other(format!("Failed to open image: {}", e)))?;

    // Sample colors from the image (simple approach: sample every 10th pixel)
    let mut color_counts: std::collections::HashMap<(u8, u8, u8), u32> =
        std::collections::HashMap::new();

    let (width, height) = img.dimensions();
    let step = 10;

    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let pixel = img.get_pixel(x, y);
            let rgb = (pixel[0], pixel[1], pixel[2]);
            *color_counts.entry(rgb).or_insert(0) += 1;
        }
    }

    // Get top 5 most common colors
    let mut color_vec: Vec<_> = color_counts.into_iter().collect();
    color_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let total_samples = color_vec.iter().map(|(_, count)| count).sum::<u32>() as f32;

    Ok(color_vec
        .into_iter()
        .take(5)
        .map(|((r, g, b), count)| Color {
            r,
            g,
            b,
            hex: format!("#{:02x}{:02x}{:02x}", r, g, b),
            percentage: (count as f32 / total_samples) * 100.0,
        })
        .collect())
}

/// Parse objects and scenes from an AI-generated caption
fn parse_objects_and_scenes(caption: &str) -> (Vec<DetectedObject>, Vec<String>) {
    // Simple keyword-based extraction
    let caption_lower = caption.to_lowercase();

    let mut objects = Vec::new();
    let mut scenes = Vec::new();

    // Common objects
    let object_keywords = [
        "person", "people", "car", "tree", "building", "animal", "dog", "cat", "bird", "flower",
        "table", "chair", "book", "computer", "phone", "food", "drink",
    ];

    for keyword in &object_keywords {
        if caption_lower.contains(keyword) {
            objects.push(DetectedObject {
                name: keyword.to_string(),
                confidence: 0.7,
                category: "general".to_string(),
            });
        }
    }

    // Common scenes
    let scene_keywords = [
        "indoor", "outdoor", "beach", "city", "forest", "mountain", "office", "home", "street",
        "park", "restaurant",
    ];

    for keyword in &scene_keywords {
        if caption_lower.contains(keyword) {
            scenes.push(keyword.to_string());
        }
    }

    (objects, scenes)
}

/// Detect programming language from file extension
fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "Rust".to_string(),
        Some("py") => "Python".to_string(),
        Some("js") => "JavaScript".to_string(),
        Some("ts") => "TypeScript".to_string(),
        Some("go") => "Go".to_string(),
        Some("java") => "Java".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "C++".to_string(),
        Some("c") => "C".to_string(),
        Some("rb") => "Ruby".to_string(),
        Some("php") => "PHP".to_string(),
        Some("swift") => "Swift".to_string(),
        Some("kt") => "Kotlin".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Extract import statements from code
fn extract_imports(code: &str, language: &str) -> Vec<String> {
    let mut imports = Vec::new();

    for line in code.lines() {
        let line = line.trim();
        match language {
            "Rust" => {
                if line.starts_with("use ") || line.starts_with("extern crate ") {
                    imports.push(line.to_string());
                }
            }
            "Python" => {
                if line.starts_with("import ") || line.starts_with("from ") {
                    imports.push(line.to_string());
                }
            }
            "JavaScript" | "TypeScript" => {
                if line.starts_with("import ") || line.starts_with("require(") {
                    imports.push(line.to_string());
                }
            }
            "Go" => {
                if line.starts_with("import ") {
                    imports.push(line.to_string());
                }
            }
            _ => {}
        }
    }

    imports
}

/// Calculate basic cyclomatic complexity
fn calculate_complexity(code: &str) -> u32 {
    let mut complexity = 1; // Base complexity

    // Count decision points
    let keywords = [
        "if ", "else ", "for ", "while ", "case ", "catch ", "&&", "||", "?",
    ];

    for keyword in &keywords {
        complexity += code.matches(keyword).count() as u32;
    }

    complexity
}
