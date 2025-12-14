//! Hippo Desktop Application
//! 
//! 🦛 The memory that never forgets

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use hippo_core::{Hippo, SearchQuery, Tag, Source, MemoryId, ClaudeClient, OllamaClient, OllamaConfig, UnifiedAiClient, AiProvider};
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
        tags: tags.into_iter()
            .map(|t| hippo_core::TagFilter { 
                tag: t, 
                mode: hippo_core::TagFilterMode::Include 
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
        "local" => Source::Local { root_path: path.into() },
        _ => return Err(format!("Unknown source type: {}", source_type))
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
    hippo.add_tag(id, Tag::user(tag)).await.map_err(|e| e.to_string())?;
    Ok("Tag added".to_string())
}

#[tauri::command]
async fn toggle_favorite(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
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
        Err(e) => Err(format!("Failed to get mind map: {}", e))
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
    println!("[Hippo] Removing source: {} (delete_files={})", path, delete_files);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;
    
    let source = Source::Local { root_path: path.into() };
    
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
async fn reindex_source(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Re-indexing source: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;
    
    let source = Source::Local { root_path: path.into() };
    
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
            .arg(std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new(&path)))
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
async fn get_thumbnail(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let file_path = std::path::Path::new(&path);

    // Check if this is a supported image format
    if !hippo_core::is_supported_image(file_path) {
        return Err("Not a supported image format".to_string());
    }

    // Generate or get cached thumbnail
    match hippo.get_thumbnail(file_path) {
        Ok(thumb_path) => Ok(thumb_path.to_string_lossy().to_string()),
        Err(e) => Err(format!("Failed to generate thumbnail: {}", e))
    }
}

#[tauri::command]
async fn get_thumbnail_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.thumbnail_stats() {
        Ok(stats) => {
            serde_json::to_value(serde_json::json!({
                "count": stats.count,
                "total_size": stats.total_size
            })).map_err(|e| e.to_string())
        }
        Err(e) => Err(format!("Failed to get thumbnail stats: {}", e))
    }
}

#[tauri::command]
async fn clear_thumbnail_cache(state: State<'_, AppState>) -> Result<String, String> {
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.clear_thumbnail_cache() {
        Ok(_) => Ok("Thumbnail cache cleared".to_string()),
        Err(e) => Err(format!("Failed to clear cache: {}", e))
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
    let memory = hippo.get_memory(id).await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    let client = ClaudeClient::new(api_key);
    let analysis = client.analyze_file(&memory).await
        .map_err(|e| format!("Analysis failed: {}", e))?;

    // Add AI tags to the memory
    for tag_suggestion in &analysis.tags {
        if tag_suggestion.confidence >= 70 {
            let tag = tag_suggestion.to_tag();
            let _ = hippo.add_tag(id, tag).await;
        }
    }

    println!("[Hippo] AI analysis complete, found {} tags", analysis.tags.len());
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
    let memory = hippo.get_memory(id).await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Read file content
    let content = std::fs::read_to_string(&memory.path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let file_name = memory.path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let client = ClaudeClient::new(api_key);
    let summary = client.summarize_text(&content, &file_name).await
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
    let all_memories = hippo.get_all_memories().await
        .map_err(|e| e.to_string())?;

    // Take up to 50 files for analysis
    let to_analyze: Vec<_> = all_memories.into_iter().take(50).collect();

    let client = ClaudeClient::new(api_key);
    let suggestions = client.suggest_organization(&to_analyze).await
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
    let query_embedding = hippo.indexer.embedder().embed_query(&query).await
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

    let results: Vec<serde_json::Value> = scored.iter()
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

    results.truncate(100);  // Limit results

    println!("[Hippo] Symbol search found {} results", results.len());
    Ok(serde_json::Value::Array(results))
}

#[tauri::command]
async fn add_source_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Adding source path: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local { root_path: path.into() };

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

    let models = client.list_models().await
        .map_err(|e| format!("Failed to list models: {}", e))?;

    let model_info: Vec<serde_json::Value> = models.iter()
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

    client.pull_model(&name).await
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
    let memory = hippo.get_memory(id).await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Use unified AI client with Ollama
    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let analysis = ai_client.analyze_file(&memory).await
        .map_err(|e| format!("Analysis failed: {}", e))?;

    // Add AI tags to the memory
    for tag_suggestion in &analysis.tags {
        if tag_suggestion.confidence >= 70 {
            let tag = tag_suggestion.to_tag();
            let _ = hippo.add_tag(id, tag).await;
        }
    }

    println!("[Hippo] Ollama analysis complete, found {} tags", analysis.tags.len());
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
    let memory = hippo.get_memory(id).await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Read file content
    let content = std::fs::read_to_string(&memory.path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let file_name = memory.path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let summary = ai_client.summarize(&content, &file_name).await
        .map_err(|e| format!("Summarization failed: {}", e))?;

    println!("[Hippo] Ollama summarization complete");
    serde_json::to_value(summary).map_err(|e| e.to_string())
}

#[tauri::command]
async fn ollama_chat(
    messages: Vec<(String, String)>,
) -> Result<String, String> {
    println!("[Hippo] Ollama chat with {} messages", messages.len());

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let response = ai_client.chat(messages).await
        .map_err(|e| format!("Chat failed: {}", e))?;

    Ok(response)
}

#[tauri::command]
async fn ollama_rag_query(
    query: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] RAG query: {}", query);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    // Get query embedding
    let query_embedding = ai_client.embed_single(&query).await
        .map_err(|e| format!("Embedding failed: {}", e))?;

    // Get all memories and find similar ones
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;

    let mut context_docs: Vec<(String, String, f32)> = Vec::new();

    // Find similar files and extract their content
    for memory in memories.iter().take(100) {
        // For text-based files, compute similarity
        if matches!(memory.kind, hippo_core::MemoryKind::Document { .. } | hippo_core::MemoryKind::Code { .. }) {
            if let Ok(content) = std::fs::read_to_string(&memory.path) {
                // Truncate content for context
                let truncated: String = content.chars().take(2000).collect();
                if let Ok(emb) = ai_client.embed_single(&truncated).await {
                    let similarity = cosine_similarity(&query_embedding, &emb);
                    if similarity > 0.3 {
                        context_docs.push((
                            truncated,
                            memory.path.to_string_lossy().to_string(),
                            similarity
                        ));
                    }
                }
            }
        }
    }

    // Sort by similarity and take top results
    context_docs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    context_docs.truncate(5);

    if context_docs.is_empty() {
        return Ok("I couldn't find any relevant documents to answer your question. Try indexing more files or asking a different question.".to_string());
    }

    // Generate response using RAG
    let response = ai_client.rag_query(&query, context_docs).await
        .map_err(|e| format!("RAG query failed: {}", e))?;

    Ok(response)
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

// Helper function for cosine similarity
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
