//! Hippo Desktop Application
//!
//! 🦛 The memory that never forgets

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use hippo_core::{
    ClaudeClient, Hippo, MemoryId, OllamaClient, SearchQuery, Source, Tag, UnifiedAiClient,
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

struct AppState {
    hippo: Arc<RwLock<Option<Hippo>>>,
}

#[tauri::command]
async fn initialize(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Initializing...");
    let mut hippo_lock = state.hippo.write().await;

    match Hippo::new().await {
        Ok(hippo) => {
            println!("[Hippo] Initialized successfully!");
            *hippo_lock = Some(hippo);
            Ok("Hippo initialized successfully".to_string())
        }
        Err(e) => {
            println!("[Hippo] Init failed: {}", e);
            Err(format!("Failed to initialize Hippo: {}", e))
        }
    }
}

#[tauri::command]
async fn search(
    query: String,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Search: query='{}', tags={:?}", query, tags);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let search_query = SearchQuery {
        text: if query.is_empty() { None } else { Some(query) },
        tags: tags
            .into_iter()
            .map(|t| hippo_core::TagFilter {
                tag: t,
                mode: hippo_core::TagFilterMode::Include,
            })
            .collect(),
        ..Default::default()
    };

    match hippo.search_advanced(search_query).await {
        Ok(results) => {
            println!("[Hippo] Search returned {} results", results.memories.len());
            serde_json::to_value(results).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Search failed: {}", e);
            Err(format!("Search failed: {}", e))
        }
    }
}

#[tauri::command]
async fn add_source(
    source_type: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Adding source: type={}, path={}", source_type, path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = match source_type.as_str() {
        "local" => Source::Local {
            root_path: path.into(),
        },
        _ => return Err(format!("Unknown source type: {}", source_type)),
    };

    match hippo.add_source(source).await {
        Ok(_) => {
            println!("[Hippo] Source added successfully, indexing started");
            Ok("Source added successfully".to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to add source: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn get_sources(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting sources...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.list_sources().await {
        Ok(sources) => {
            println!("[Hippo] Found {} sources", sources.len());
            serde_json::to_value(sources).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to get sources: {}", e);
            Err(format!("Failed to get sources: {}", e))
        }
    }
}

#[tauri::command]
async fn add_tag(
    memory_id: String,
    tag: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Adding tag '{}' to memory {}", tag, memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    hippo
        .add_tag(id, Tag::user(tag))
        .await
        .map_err(|e| e.to_string())?;
    Ok("Tag added".to_string())
}

#[tauri::command]
async fn toggle_favorite(memory_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    println!("[Hippo] Toggling favorite for memory {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    hippo.toggle_favorite(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_tags(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting tags...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.list_tags().await {
        Ok(tags) => {
            println!("[Hippo] Found {} tags", tags.len());
            serde_json::to_value(tags).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to get tags: {}", e);
            Err(format!("Failed to get tags: {}", e))
        }
    }
}

#[tauri::command]
async fn get_mind_map(
    memory_id: String,
    depth: usize,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting mind map for {} depth {}", memory_id, depth);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;

    match hippo.get_mind_map(id, depth).await {
        Ok(map) => serde_json::to_value(map).map_err(|e| e.to_string()),
        Err(e) => Err(format!("Failed to get mind map: {}", e)),
    }
}

#[tauri::command]
async fn get_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting stats...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.stats().await {
        Ok(stats) => {
            println!("[Hippo] Stats: {} memories", stats.total_memories);
            serde_json::to_value(stats).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to get stats: {}", e);
            Err(format!("Failed to get stats: {}", e))
        }
    }
}

#[tauri::command]
async fn reset_index(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Resetting index...");
    let mut hippo_lock = state.hippo.write().await;

    // Drop the current instance
    *hippo_lock = None;

    // Delete the database file
    let data_dir = directories::ProjectDirs::from("", "", "Hippo")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from(".hippo"));

    let db_path = data_dir.join("hippo.db");
    if db_path.exists() {
        if let Err(e) = std::fs::remove_file(&db_path) {
            println!("[Hippo] Failed to delete db: {}", e);
        } else {
            println!("[Hippo] Deleted database: {:?}", db_path);
        }
    }

    // Recreate Hippo
    match Hippo::new().await {
        Ok(hippo) => {
            *hippo_lock = Some(hippo);
            println!("[Hippo] Index reset complete");
            Ok("Index reset successfully".to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to reinitialize: {}", e);
            Err(format!("Failed to reinitialize: {}", e))
        }
    }
}

#[tauri::command]
async fn remove_source(
    path: String,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!(
        "[Hippo] Removing source: {} (delete_files={})",
        path, delete_files
    );
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local {
        root_path: path.into(),
    };

    match hippo.remove_source(&source, delete_files).await {
        Ok(_) => {
            println!("[Hippo] Source removed successfully");
            Ok("Source removed".to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to remove source: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn reindex_source(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Re-indexing source: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local {
        root_path: path.into(),
    };

    // Queue for indexing (will update existing entries)
    match hippo.indexer.queue_source(source).await {
        Ok(_) => {
            println!("[Hippo] Re-index queued");
            Ok("Re-index started".to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to queue re-index: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn open_in_finder(path: String) -> Result<(), String> {
    println!("[Hippo] Opening in finder: {}", path);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(
                std::path::Path::new(&path)
                    .parent()
                    .unwrap_or(std::path::Path::new(&path)),
            )
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    println!("[Hippo] Opening file: {}", path);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    println!("[Hippo] Opening folder picker...");
    Ok(None)
}

#[tauri::command]
async fn get_thumbnail(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let file_path = std::path::Path::new(&path);

    // Check if this is a supported image format
    if hippo_core::is_supported_image(file_path) {
        // Generate or get cached image thumbnail
        match hippo.get_thumbnail(file_path) {
            Ok(thumb_path) => return Ok(thumb_path.to_string_lossy().to_string()),
            Err(e) => return Err(format!("Failed to generate thumbnail: {}", e)),
        }
    }

    // Check if this is a supported video format
    if hippo_core::is_supported_video(file_path) {
        // Generate or get cached video thumbnail
        match hippo.get_video_thumbnail(file_path) {
            Ok(thumb_path) => return Ok(thumb_path.to_string_lossy().to_string()),
            Err(e) => return Err(format!("Failed to generate video thumbnail: {}", e)),
        }
    }

    Err("Not a supported image or video format".to_string())
}

#[tauri::command]
async fn get_thumbnail_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.thumbnail_stats() {
        Ok(stats) => serde_json::to_value(serde_json::json!({
            "count": stats.count,
            "total_size": stats.total_size
        }))
        .map_err(|e| e.to_string()),
        Err(e) => Err(format!("Failed to get thumbnail stats: {}", e)),
    }
}

#[tauri::command]
async fn clear_thumbnail_cache(state: State<'_, AppState>) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.clear_thumbnail_cache() {
        Ok(_) => Ok("Thumbnail cache cleared".to_string()),
        Err(e) => Err(format!("Failed to clear cache: {}", e)),
    }
}

// ==================== AI Features ====================

#[tauri::command]
async fn analyze_file(
    memory_id: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] AI analyzing file: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    let client = ClaudeClient::new(api_key);
    let analysis = client
        .analyze_file(&memory)
        .await
        .map_err(|e| format!("Analysis failed: {}", e))?;

    // Add AI tags to the memory
    for tag_suggestion in &analysis.tags {
        if tag_suggestion.confidence >= 70 {
            let tag = tag_suggestion.to_tag();
            let _ = hippo.add_tag(id, tag).await;
        }
    }

    println!(
        "[Hippo] AI analysis complete, found {} tags",
        analysis.tags.len()
    );
    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn summarize_document(
    memory_id: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Summarizing document: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Read file content
    let content =
        std::fs::read_to_string(&memory.path).map_err(|e| format!("Failed to read file: {}", e))?;

    let file_name = memory
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let client = ClaudeClient::new(api_key);
    let summary = client
        .summarize_text(&content, &file_name)
        .await
        .map_err(|e| format!("Summarization failed: {}", e))?;

    println!("[Hippo] Document summarized");
    serde_json::to_value(summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_organization_suggestions(
    api_key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting organization suggestions...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    // Get recent unorganized files
    let all_memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;

    // Take up to 50 files for analysis
    let to_analyze: Vec<_> = all_memories.into_iter().take(50).collect();

    let client = ClaudeClient::new(api_key);
    let suggestions = client
        .suggest_organization(&to_analyze)
        .await
        .map_err(|e| format!("Organization suggestion failed: {}", e))?;

    println!("[Hippo] Got {} organization suggestions", suggestions.len());
    serde_json::to_value(suggestions).map_err(|e| e.to_string())
}

#[tauri::command]
async fn semantic_search(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Semantic search: {}", query);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let limit = limit.unwrap_or(20);

    // Get query embedding
    let query_embedding = hippo
        .indexer
        .embedder()
        .embed_query(&query)
        .await
        .map_err(|e| format!("Embedding failed: {}", e))?;

    // Get all memories and their embeddings
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;

    // Generate embeddings for memories and find similar ones
    let mut scored: Vec<(hippo_core::Memory, f32)> = Vec::new();
    for memory in memories {
        if let Ok(emb) = hippo.indexer.embedder().embed_memory(&memory).await {
            let similarity = hippo_core::embeddings::Embedder::similarity(&query_embedding, &emb);
            scored.push((memory, similarity));
        }
    }

    // Sort by similarity and take top results
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let results: Vec<serde_json::Value> = scored
        .iter()
        .map(|(mem, score)| {
            serde_json::json!({
                "memory": mem,
                "similarity": score
            })
        })
        .collect();

    println!("[Hippo] Semantic search returned {} results", results.len());
    Ok(serde_json::Value::Array(results))
}

#[tauri::command]
async fn search_symbols(
    query: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Symbol search: {}", query);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();

    let mut results: Vec<serde_json::Value> = Vec::new();

    for memory in memories {
        if let hippo_core::MemoryKind::Code { .. } = &memory.kind {
            if let Some(code_info) = &memory.metadata.code_info {
                // Search functions
                for func in &code_info.functions {
                    if func.name.to_lowercase().contains(&query_lower) {
                        results.push(serde_json::json!({
                            "type": "function",
                            "name": func.name,
                            "file": memory.path.to_string_lossy(),
                            "line": func.line_start,
                            "is_public": func.is_public,
                            "doc": func.doc_comment,
                            "language": code_info.language
                        }));
                    }
                }

                // Search exports
                for export in &code_info.exports {
                    if export.to_lowercase().contains(&query_lower) {
                        results.push(serde_json::json!({
                            "type": "export",
                            "name": export,
                            "file": memory.path.to_string_lossy(),
                            "language": code_info.language
                        }));
                    }
                }

                // Search imports
                for import in &code_info.imports {
                    if import.to_lowercase().contains(&query_lower) {
                        results.push(serde_json::json!({
                            "type": "import",
                            "name": import,
                            "file": memory.path.to_string_lossy(),
                            "language": code_info.language
                        }));
                    }
                }
            }
        }
    }

    // Sort by relevance (exact match first)
    results.sort_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let exact_a = name_a.to_lowercase() == query_lower;
        let exact_b = name_b.to_lowercase() == query_lower;
        exact_b.cmp(&exact_a)
    });

    results.truncate(100); // Limit results

    println!("[Hippo] Symbol search found {} results", results.len());
    Ok(serde_json::Value::Array(results))
}

#[tauri::command]
async fn add_source_path(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Adding source path: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local {
        root_path: path.into(),
    };

    match hippo.add_source(source).await {
        Ok(_) => {
            println!("[Hippo] Source added successfully, indexing started");
            Ok("Source added successfully".to_string())
        }
        Err(e) => {
            println!("[Hippo] Failed to add source: {}", e);
            Err(e.to_string())
        }
    }
}

// ==================== Ollama / Local AI Features ====================

#[tauri::command]
async fn ollama_status() -> Result<serde_json::Value, String> {
    println!("[Hippo] Checking Ollama status...");
    let client = OllamaClient::new();

    let available = client.is_available().await;
    let version = if available {
        client.version().await.ok()
    } else {
        None
    };

    Ok(serde_json::json!({
        "available": available,
        "version": version,
        "url": hippo_core::ollama::DEFAULT_OLLAMA_URL
    }))
}

#[tauri::command]
async fn ollama_list_models() -> Result<serde_json::Value, String> {
    println!("[Hippo] Listing Ollama models...");
    let client = OllamaClient::new();

    if !client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let models = client
        .list_models()
        .await
        .map_err(|e| format!("Failed to list models: {}", e))?;

    let model_info: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "size": m.size,
                "size_formatted": format_bytes(m.size),
                "digest": &m.digest[..12.min(m.digest.len())],
                "modified_at": m.modified_at
            })
        })
        .collect();

    println!("[Hippo] Found {} models", model_info.len());
    Ok(serde_json::Value::Array(model_info))
}

#[tauri::command]
async fn ollama_pull_model(name: String) -> Result<String, String> {
    println!("[Hippo] Pulling Ollama model: {}", name);
    let client = OllamaClient::new();

    if !client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    client
        .pull_model(&name)
        .await
        .map_err(|e| format!("Failed to pull model: {}", e))?;

    Ok(format!("Model {} pulled successfully", name))
}

#[tauri::command]
async fn ollama_analyze(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Ollama analyzing file: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Use unified AI client with Ollama
    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let analysis = ai_client
        .analyze_file(&memory)
        .await
        .map_err(|e| format!("Analysis failed: {}", e))?;

    // Add AI tags to the memory
    for tag_suggestion in &analysis.tags {
        if tag_suggestion.confidence >= 70 {
            let tag = tag_suggestion.to_tag();
            let _ = hippo.add_tag(id, tag).await;
        }
    }

    println!(
        "[Hippo] Ollama analysis complete, found {} tags",
        analysis.tags.len()
    );
    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn ollama_summarize(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Ollama summarizing: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Read file content
    let content =
        std::fs::read_to_string(&memory.path).map_err(|e| format!("Failed to read file: {}", e))?;

    let file_name = memory
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let summary = ai_client
        .summarize(&content, &file_name)
        .await
        .map_err(|e| format!("Summarization failed: {}", e))?;

    println!("[Hippo] Ollama summarization complete");
    serde_json::to_value(summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn ollama_chat(messages: Vec<(String, String)>) -> Result<String, String> {
    println!("[Hippo] Ollama chat with {} messages", messages.len());

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let response = ai_client
        .chat(messages)
        .await
        .map_err(|e| format!("Chat failed: {}", e))?;

    Ok(response)
}

#[tauri::command]
async fn ollama_rag_query(
    query: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] RAG query: {}", query);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    // Get all memories
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();

    // Track relevant files for preview
    let mut relevant_files: Vec<serde_json::Value> = Vec::new();

    // Build context from files - use metadata and content
    let mut context_parts: Vec<String> = Vec::new();

    // Collect file statistics
    let mut file_types: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_size: u64 = 0;

    for memory in &memories {
        let type_name = match &memory.kind {
            hippo_core::MemoryKind::Image { .. } => "image",
            hippo_core::MemoryKind::Video { .. } => "video",
            hippo_core::MemoryKind::Audio { .. } => "audio",
            hippo_core::MemoryKind::Document { .. } => "document",
            hippo_core::MemoryKind::Code { .. } => "code",
            hippo_core::MemoryKind::Archive { .. } => "archive",
            _ => "other",
        };
        *file_types.entry(type_name.to_string()).or_insert(0) += 1;
        total_size += memory.metadata.file_size;
    }

    // Add collection summary
    let summary = format!(
        "File collection: {} total files, {:.1} MB total size. Types: {}",
        memories.len(),
        total_size as f64 / 1_000_000.0,
        file_types
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    );
    context_parts.push(summary);

    // For text/code files, include content samples
    let mut text_files_added = 0;
    for memory in memories.iter() {
        if text_files_added >= 10 {
            break;
        }

        let is_text_file = matches!(
            &memory.kind,
            hippo_core::MemoryKind::Document { .. } | hippo_core::MemoryKind::Code { .. }
        );

        if is_text_file {
            if let Ok(content) = std::fs::read_to_string(&memory.path) {
                let filename = memory
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Check if relevant to query
                let content_lower = content.to_lowercase();
                let filename_lower = filename.to_lowercase();
                let is_relevant = query_lower
                    .split_whitespace()
                    .any(|word| content_lower.contains(word) || filename_lower.contains(word));

                if is_relevant || text_files_added < 3 {
                    let preview: String = content.chars().take(1000).collect();
                    let lang = if let hippo_core::MemoryKind::Code { language, .. } = &memory.kind {
                        format!(" ({})", language)
                    } else {
                        String::new()
                    };
                    context_parts.push(format!("File: {}{}\n{}", filename, lang, preview));
                    text_files_added += 1;

                    // Add to relevant files
                    let file_type = match &memory.kind {
                        hippo_core::MemoryKind::Code { language, .. } => {
                            format!("code:{}", language)
                        }
                        hippo_core::MemoryKind::Document { .. } => "document".to_string(),
                        _ => "file".to_string(),
                    };
                    relevant_files.push(serde_json::json!({
                        "id": memory.id.to_string(),
                        "path": memory.path.to_string_lossy(),
                        "name": filename,
                        "type": file_type,
                        "size": memory.metadata.file_size,
                        "relevance": if is_relevant { "high" } else { "medium" }
                    }));
                }
            }
        }
    }

    // For images, include metadata and add to relevant files
    let mut image_info: Vec<String> = Vec::new();
    for memory in memories
        .iter()
        .filter(|m| matches!(m.kind, hippo_core::MemoryKind::Image { .. }))
        .take(20)
    {
        let filename = memory
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut details = vec![filename.clone()];
        let (width, height) =
            if let hippo_core::MemoryKind::Image { width, height, .. } = &memory.kind {
                details.push(format!("{}x{}", width, height));
                (*width, *height)
            } else {
                (0, 0)
            };
        // Add modified date from memory
        details.push(memory.modified_at.format("%Y-%m-%d").to_string());
        image_info.push(details.join(" - "));

        // Check relevance for images
        let filename_lower = filename.to_lowercase();
        let is_relevant = query_lower
            .split_whitespace()
            .any(|word| filename_lower.contains(word));

        if relevant_files.len() < 12 {
            relevant_files.push(serde_json::json!({
                "id": memory.id.to_string(),
                "path": memory.path.to_string_lossy(),
                "name": filename,
                "type": "image",
                "size": memory.metadata.file_size,
                "width": width,
                "height": height,
                "relevance": if is_relevant { "high" } else { "low" }
            }));
        }
    }

    if !image_info.is_empty() {
        context_parts.push(format!("Recent images:\n{}", image_info.join("\n")));
    }

    let context = context_parts.join("\n\n---\n\n");

    // Build the prompt
    let prompt = format!(
        "You are a helpful file assistant. Based on the user's file collection, answer their question.\n\n\
        USER'S FILES:\n{}\n\n\
        USER'S QUESTION: {}\n\n\
        Provide a helpful, concise answer based on the files above. If you can't answer from the files, say so.",
        context, query
    );

    println!(
        "[Hippo] Sending to Ollama ({} chars context)",
        context.len()
    );

    // Use generate API with system prompt
    let system = "You are a helpful file assistant. Answer questions about the user's files based on the provided context. Be concise and helpful.";
    let response = ai_client
        .ollama()
        .generate(&prompt, Some(system))
        .await
        .map_err(|e| format!("Generation failed: {}", e))?;

    // Sort relevant files by relevance (high first)
    relevant_files.sort_by(|a, b| {
        let rel_a = a["relevance"].as_str().unwrap_or("low");
        let rel_b = b["relevance"].as_str().unwrap_or("low");
        let score_a = match rel_a {
            "high" => 3,
            "medium" => 2,
            _ => 1,
        };
        let score_b = match rel_b {
            "high" => 3,
            "medium" => 2,
            _ => 1,
        };
        score_b.cmp(&score_a)
    });

    // Return response with relevant files
    Ok(serde_json::json!({
        "response": response,
        "files": relevant_files.into_iter().take(8).collect::<Vec<_>>(),
        "stats": {
            "total_files": memories.len(),
            "context_size": context.len()
        }
    }))
}

#[tauri::command]
async fn ai_suggest_tags(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] AI suggest tags for: {}", file_path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let ai_client = UnifiedAiClient::with_ollama(None);
    if !ai_client.is_available().await {
        return Err("Ollama is not running".to_string());
    }

    // Get memory for the file
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let memory = memories
        .iter()
        .find(|m| m.path.to_string_lossy() == file_path)
        .ok_or("File not found in index")?;

    let filename = memory
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Build context about the file
    let file_info = match &memory.kind {
        hippo_core::MemoryKind::Image {
            width,
            height,
            format,
        } => {
            format!("Image file: {}x{} {}", width, height, format)
        }
        hippo_core::MemoryKind::Code { language, lines } => {
            let content_preview = std::fs::read_to_string(&memory.path)
                .map(|c| c.chars().take(500).collect::<String>())
                .unwrap_or_default();
            format!(
                "Code file ({}, {} lines):\n{}",
                language, lines, content_preview
            )
        }
        hippo_core::MemoryKind::Document { format, .. } => {
            format!("Document file: {:?}", format)
        }
        _ => format!("File type: {:?}", memory.kind),
    };

    let existing_tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();

    let prompt = format!(
        "Suggest 3-5 relevant tags for this file. Only output the tags as a comma-separated list, nothing else.\n\n\
        Filename: {}\nFile info: {}\nExisting tags: {}\n\nSuggested tags:",
        filename, file_info, existing_tags.join(", ")
    );

    let response = ai_client
        .ollama()
        .generate(
            &prompt,
            Some("You are a file tagging assistant. Suggest concise, relevant tags."),
        )
        .await
        .map_err(|e| format!("Generation failed: {}", e))?;

    // Parse tags from response
    let suggested: Vec<String> = response
        .split(',')
        .map(|s| s.trim().to_lowercase().replace(['#', '"', '\'', '.'], ""))
        .filter(|s| !s.is_empty() && s.len() < 30)
        .take(5)
        .collect();

    Ok(serde_json::json!({
        "file": filename,
        "existing_tags": existing_tags,
        "suggested_tags": suggested
    }))
}

#[tauri::command]
async fn ai_find_similar(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] AI find similar to: {}", file_path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    // Get all memories
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let target = memories
        .iter()
        .find(|m| m.path.to_string_lossy() == file_path)
        .ok_or("File not found in index")?;

    let target_name = target
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Find similar files based on type, name patterns, and tags
    let mut similar_files: Vec<serde_json::Value> = Vec::new();

    let target_type = std::mem::discriminant(&target.kind);
    let target_ext = target
        .path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let target_name_lower = target_name.to_lowercase();

    for memory in &memories {
        if memory.path == target.path {
            continue;
        }

        let mut score: u32 = 0;
        let mut reasons: Vec<&str> = Vec::new();

        // Same type
        if std::mem::discriminant(&memory.kind) == target_type {
            score += 30;
            reasons.push("same type");
        }

        // Same extension
        let ext = memory
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext == target_ext && !target_ext.is_empty() {
            score += 20;
        }

        // Similar name
        let name = memory
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Check for common words in filename
        let target_words: std::collections::HashSet<&str> = target_name_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .collect();
        let name_words: std::collections::HashSet<&str> = name
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .collect();
        let common_words = target_words.intersection(&name_words).count();
        if common_words > 0 {
            score += (common_words * 15) as u32;
            reasons.push("similar name");
        }

        // Same directory
        if memory.path.parent() == target.path.parent() {
            score += 10;
            reasons.push("same folder");
        }

        // Shared tags
        let shared_tags: Vec<String> = memory
            .tags
            .iter()
            .filter(|t| target.tags.iter().any(|tt| tt.name == t.name))
            .map(|t| t.name.clone())
            .collect();
        if !shared_tags.is_empty() {
            score += (shared_tags.len() * 20) as u32;
            reasons.push("shared tags");
        }

        // Similar file size (within 50%)
        if target.metadata.file_size > 0 {
            let size_ratio = memory.metadata.file_size as f64 / target.metadata.file_size as f64;
            if size_ratio > 0.5 && size_ratio < 2.0 {
                score += 5;
            }
        }

        // Similar dimensions for images
        if let (
            hippo_core::MemoryKind::Image {
                width: tw,
                height: th,
                ..
            },
            hippo_core::MemoryKind::Image {
                width: mw,
                height: mh,
                ..
            },
        ) = (&target.kind, &memory.kind)
        {
            let w_ratio = *mw as f64 / *tw as f64;
            let h_ratio = *mh as f64 / *th as f64;
            if (0.8..1.2).contains(&w_ratio) && (0.8..1.2).contains(&h_ratio) {
                score += 15;
                reasons.push("similar dimensions");
            }
        }

        if score >= 25 {
            similar_files.push(serde_json::json!({
                "id": memory.id.to_string(),
                "path": memory.path.to_string_lossy(),
                "name": memory.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "score": score,
                "reasons": reasons,
                "type": format!("{:?}", memory.kind).split('{').next().unwrap_or("Unknown").trim()
            }));
        }
    }

    // Sort by score descending
    similar_files.sort_by(|a, b| {
        b["score"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["score"].as_u64().unwrap_or(0))
    });

    Ok(serde_json::json!({
        "target": target_name,
        "similar": similar_files.into_iter().take(10).collect::<Vec<_>>()
    }))
}

#[tauri::command]
async fn ai_smart_rename(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] AI smart rename for: {}", file_path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let ai_client = UnifiedAiClient::with_ollama(None);
    if !ai_client.is_available().await {
        return Err("Ollama is not running".to_string());
    }

    // Get memory for the file
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let memory = memories
        .iter()
        .find(|m| m.path.to_string_lossy() == file_path)
        .ok_or("File not found in index")?;

    let current_name = memory
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = memory
        .path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    // Build context about the file
    let file_info = match &memory.kind {
        hippo_core::MemoryKind::Image { width, height, .. } => {
            format!("Image ({}x{})", width, height)
        }
        hippo_core::MemoryKind::Code { language, lines } => {
            let content_preview = std::fs::read_to_string(&memory.path)
                .map(|c| c.chars().take(300).collect::<String>())
                .unwrap_or_default();
            format!("Code ({}, {} lines): {}", language, lines, content_preview)
        }
        hippo_core::MemoryKind::Document { .. } => "Document".to_string(),
        _ => "File".to_string(),
    };

    let date_str = memory.modified_at.format("%Y-%m-%d").to_string();
    let tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();

    let prompt = format!(
        "Suggest 3 better file names for this file. Keep the same extension (.{}). \
        Names should be descriptive, use-lowercase-with-dashes, and be concise. \
        Only output the 3 suggested names, one per line, nothing else.\n\n\
        Current name: {}\nFile type: {}\nDate: {}\nTags: {}\n\nSuggested names:",
        extension,
        current_name,
        file_info,
        date_str,
        tags.join(", ")
    );

    let response = ai_client
        .ollama()
        .generate(
            &prompt,
            Some("You are a file naming assistant. Suggest clean, descriptive file names."),
        )
        .await
        .map_err(|e| format!("Generation failed: {}", e))?;

    // Parse suggestions from response
    let suggestions: Vec<String> = response
        .lines()
        .map(|s| {
            s.trim()
                .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == '-')
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() < 100)
        .take(3)
        .collect();

    Ok(serde_json::json!({
        "current_name": current_name,
        "suggestions": suggestions
    }))
}

#[tauri::command]
async fn get_recommended_models() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "embedding": {
            "light": hippo_core::RecommendedModels::EMBEDDING_LIGHT,
            "standard": hippo_core::RecommendedModels::EMBEDDING_STANDARD
        },
        "generation": {
            "light": hippo_core::RecommendedModels::GENERATION_LIGHT,
            "standard": hippo_core::RecommendedModels::GENERATION_STANDARD
        },
        "code": hippo_core::RecommendedModels::CODE_MODELS
    }))
}

// ==================== Organization Features ====================

#[tauri::command]
async fn list_collections(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Listing collections...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let collections = hippo
        .list_collections()
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Found {} collections", collections.len());
    serde_json::to_value(collections).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_virtual_paths(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting virtual paths for: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let paths = hippo
        .get_virtual_paths(id)
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(paths).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_collections_for_memory(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting collections for memory: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let collections = hippo
        .get_collections_for_memory(id)
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(collections).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_collection(
    name: String,
    description: Option<String>,
    memory_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Creating collection: {}", name);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let ids: Vec<MemoryId> = memory_ids
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    let collection = hippo
        .create_collection(&name, description, ids)
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Collection created: {}", collection.id);
    serde_json::to_value(collection).map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_to_collection(
    collection_id: String,
    memory_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Adding to collection: {}", collection_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let cid: uuid::Uuid = collection_id.parse().map_err(|_| "Invalid collection ID")?;
    let ids: Vec<MemoryId> = memory_ids
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    hippo
        .add_to_collection(cid, ids)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Added to collection".to_string())
}

#[tauri::command]
async fn remove_collection(
    collection_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Removing collection: {}", collection_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let cid: uuid::Uuid = collection_id.parse().map_err(|_| "Invalid collection ID")?;
    hippo
        .remove_collection(cid)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Collection removed".to_string())
}

#[tauri::command]
async fn discover_collections(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Discovering collections...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let collections = hippo
        .discover_collections()
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Discovered {} collections", collections.len());
    serde_json::to_value(collections).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_organization_stats_internal(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting organization stats...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let stats = hippo
        .organization_stats()
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(stats).map_err(|e| e.to_string())
}

#[tauri::command]
async fn suggest_groupings(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Suggesting groupings for: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let suggestions = hippo
        .suggest_groupings(id)
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(suggestions).map_err(|e| e.to_string())
}

// ==================== Similarity Search ====================

#[tauri::command]
async fn find_similar_memories(
    memory_id: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Finding similar to: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let limit = limit.unwrap_or(10);

    let similar = hippo
        .find_similar(id, limit)
        .await
        .map_err(|e| e.to_string())?;

    // Fetch full memories for the results
    let mut results = Vec::new();
    for (sim_id, score) in similar {
        if let Ok(Some(memory)) = hippo.get_memory(sim_id).await {
            results.push(serde_json::json!({
                "memory": memory,
                "similarity": score
            }));
        }
    }

    println!("[Hippo] Found {} similar memories", results.len());
    Ok(serde_json::Value::Array(results))
}

#[tauri::command]
async fn hybrid_search(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Hybrid search: {}", query);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let limit = limit.unwrap_or(20);
    let results = hippo
        .hybrid_search(&query, limit)
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Hybrid search returned {} results", results.memories.len());
    serde_json::to_value(results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_qdrant_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting Qdrant stats...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let stats = hippo
        .qdrant_stats()
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(stats).map_err(|e| e.to_string())
}

// Helper function for cosine similarity (kept for future semantic search)
#[allow(dead_code)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// Helper function for formatting bytes
fn format_bytes(bytes: u64) -> String {
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

fn main() {
    println!("[Hippo] Starting application...");
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            hippo: Arc::new(RwLock::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            initialize,
            search,
            add_source,
            add_source_path,
            remove_source,
            reindex_source,
            pick_folder,
            get_sources,
            add_tag,
            toggle_favorite,
            get_tags,
            get_mind_map,
            get_stats,
            reset_index,
            open_in_finder,
            open_file,
            get_thumbnail,
            get_thumbnail_stats,
            clear_thumbnail_cache,
            // AI Features (Claude API)
            analyze_file,
            summarize_document,
            get_organization_suggestions,
            semantic_search,
            // Code Engine
            search_symbols,
            // Ollama / Local AI
            ollama_status,
            ollama_list_models,
            ollama_pull_model,
            ollama_analyze,
            ollama_summarize,
            ollama_chat,
            ollama_rag_query,
            get_recommended_models,
            // AI Actions
            ai_suggest_tags,
            ai_find_similar,
            ai_smart_rename,
            // Organization Features
            list_collections,
            get_virtual_paths,
            get_collections_for_memory,
            create_collection,
            add_to_collection,
            remove_collection,
            discover_collections,
            get_organization_stats_internal,
            suggest_groupings,
            // Similarity Search
            find_similar_memories,
            hybrid_search,
            get_qdrant_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
