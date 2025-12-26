//! Hippo Desktop Application
//!
//! 🦛 The memory that never forgets

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use chrono::Utc;
use hippo_core::{
    ClaudeClient, Hippo, MemoryId, OllamaClient, QdrantManager, SearchQuery, Source, Tag,
    UnifiedAiClient,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State};
use tokio::sync::RwLock;

struct AppState {
    hippo: Arc<RwLock<Option<Hippo>>>,
    qdrant_manager: Arc<QdrantManager>,
}

#[tauri::command]
async fn initialize(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Initializing...");

    // Ensure Qdrant is running
    println!("[Hippo] Starting Qdrant...");
    match state.qdrant_manager.ensure_running().await {
        Ok(_) => {
            let status = state.qdrant_manager.status().await;
            if status.managed {
                println!("[Hippo] Qdrant started successfully (managed)");
            } else {
                println!("[Hippo] Qdrant is running externally");
            }
        }
        Err(e) => {
            println!(
                "[Hippo] Warning: Qdrant not available ({}). Vector search will be limited.",
                e
            );
        }
    }

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
async fn bulk_add_tag(
    memory_ids: Vec<String>,
    tag: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!(
        "[Hippo] Bulk adding tag '{}' to {} memories",
        tag,
        memory_ids.len()
    );
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let mut success_count = 0;
    for memory_id in memory_ids {
        if let Ok(id) = memory_id.parse::<MemoryId>() {
            if hippo.add_tag(id, Tag::user(tag.clone())).await.is_ok() {
                success_count += 1;
            }
        }
    }

    Ok(format!("Tag added to {} memories", success_count))
}

#[tauri::command]
async fn bulk_delete(
    memory_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("[Hippo] Bulk deleting {} memories", memory_ids.len());
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let mut success_count = 0;
    for memory_id in memory_ids {
        if let Ok(id) = memory_id.parse::<MemoryId>() {
            if hippo.delete_memory(id).await.is_ok() {
                success_count += 1;
            }
        }
    }

    Ok(format!("Deleted {} memories", success_count))
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
async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    println!("[Hippo] Opening folder picker...");

    let (sender, receiver) = std::sync::mpsc::channel();

    app.dialog().file().pick_folder(move |folder_path| {
        let result = folder_path.map(|p| p.to_string());
        let _ = sender.send(result);
    });

    // Wait for the dialog result
    match receiver.recv() {
        Ok(Some(path)) => {
            println!("[Hippo] Folder selected: {}", path);
            Ok(Some(path))
        }
        Ok(None) => {
            println!("[Hippo] Folder picker cancelled");
            Ok(None)
        }
        Err(_) => Err("Failed to receive folder path".to_string()),
    }
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

    // Track relevant files for preview
    let mut relevant_files: Vec<serde_json::Value> = Vec::new();
    let mut context_parts: Vec<String> = Vec::new();

    // First, try semantic search to find most relevant documents
    let semantic_results = hippo.semantic_search(&query, 15).await;
    let mut semantic_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    if let Ok(results) = &semantic_results {
        println!("[Hippo] Found {} semantically relevant files", results.memories.len());
        for result in results.memories.iter().take(10) {
            let memory = &result.memory;
            let score = result.score;
            semantic_ids.insert(memory.id.to_string());
            let filename = memory.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Read file content for text/code files
            let content_preview = if matches!(&memory.kind,
                hippo_core::MemoryKind::Document { .. } | hippo_core::MemoryKind::Code { .. }) {
                std::fs::read_to_string(&memory.path)
                    .map(|c| c.chars().take(2000).collect::<String>())
                    .unwrap_or_else(|_| String::new())
            } else {
                String::new()
            };

            let file_type = match &memory.kind {
                hippo_core::MemoryKind::Code { language, .. } => format!("code:{}", language),
                hippo_core::MemoryKind::Document { .. } => "document".to_string(),
                hippo_core::MemoryKind::Image { width, height, .. } => format!("image ({}x{})", width, height),
                hippo_core::MemoryKind::Video { .. } => "video".to_string(),
                hippo_core::MemoryKind::Audio { .. } => "audio".to_string(),
                _ => "file".to_string(),
            };

            // Build context for this file
            let mut file_context = format!("📁 {} [{}] (relevance: {:.0}%)", filename, file_type, score * 100.0);
            if let Some(title) = &memory.metadata.title {
                file_context.push_str(&format!("\n   Title: {}", title));
            }
            if !memory.tags.is_empty() {
                let tags: Vec<String> = memory.tags.iter().map(|t| t.name.clone()).collect();
                file_context.push_str(&format!("\n   Tags: {}", tags.join(", ")));
            }
            if !content_preview.is_empty() {
                file_context.push_str(&format!("\n   Content:\n{}", content_preview));
            }
            context_parts.push(file_context);

            relevant_files.push(serde_json::json!({
                "id": memory.id.to_string(),
                "path": memory.path.to_string_lossy(),
                "name": filename,
                "type": file_type,
                "size": memory.metadata.file_size,
                "relevance": "high",
                "score": score
            }));
        }
    }

    // Also get all memories for statistics
    let memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();

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

    // Add collection summary at the start
    let summary = format!(
        "📊 Collection Overview: {} total files, {:.1} MB total size\nFile types: {}",
        memories.len(),
        total_size as f64 / 1_000_000.0,
        file_types.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")
    );
    context_parts.insert(0, summary);

    // If semantic search didn't find enough, add some keyword matches
    if context_parts.len() < 5 {
        for memory in memories.iter().take(100) {
            if semantic_ids.contains(&memory.id.to_string()) {
                continue;
            }

            let filename = memory.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let filename_lower = filename.to_lowercase();

            // Check keyword relevance
            let is_relevant = query_lower.split_whitespace()
                .any(|word| filename_lower.contains(word) ||
                     memory.tags.iter().any(|t| t.name.to_lowercase().contains(word)));

            if is_relevant && relevant_files.len() < 15 {
                let file_type = match &memory.kind {
                    hippo_core::MemoryKind::Code { language, .. } => format!("code:{}", language),
                    hippo_core::MemoryKind::Document { .. } => "document".to_string(),
                    hippo_core::MemoryKind::Image { .. } => "image".to_string(),
                    _ => "file".to_string(),
                };

                relevant_files.push(serde_json::json!({
                    "id": memory.id.to_string(),
                    "path": memory.path.to_string_lossy(),
                    "name": filename,
                    "type": file_type,
                    "size": memory.metadata.file_size,
                    "relevance": "medium"
                }));

                // Add brief context
                context_parts.push(format!("📁 {} [{}]", filename, file_type));
            }
        }
    }

    let context = context_parts.join("\n\n");

    // Build an improved prompt
    let prompt = format!(
        r#"Based on the user's file collection below, provide a helpful and detailed answer to their question.

## File Collection Context:
{}

## User's Question:
{}

## Instructions:
- Answer based ONLY on the files and information provided above
- Be specific and reference actual filenames when relevant
- If you can identify patterns, themes, or useful insights, share them
- If the answer cannot be determined from the files, say so clearly
- Keep the response focused and well-organized"#,
        context, query
    );

    println!("[Hippo] Sending to Ollama ({} chars context)", context.len());

    // Use generate API with improved system prompt
    let system = r#"You are Hippo, an intelligent file assistant that helps users understand and organize their files.
You have access to the user's file collection and can answer questions about their documents, code, images, and other files.
Be helpful, accurate, and specific. Reference actual filenames and provide actionable insights when possible."#;

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
    use hippo_core::ollama::RecommendedModels;
    Ok(serde_json::json!({
        "embedding": RecommendedModels::EMBEDDINGS,
        "generation": {
            "fast": RecommendedModels::FAST,
            "balanced": RecommendedModels::BALANCED,
            "quality": RecommendedModels::QUALITY
        },
        "vision": RecommendedModels::VISION
    }))
}

// ==================== Organization Features ====================

#[tauri::command]
async fn list_collections(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Listing collections...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let collections = hippo.list_collections().await.map_err(|e| e.to_string())?;

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

    let ids: Vec<MemoryId> = memory_ids.iter().filter_map(|s| s.parse().ok()).collect();

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
    let ids: Vec<MemoryId> = memory_ids.iter().filter_map(|s| s.parse().ok()).collect();

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

    println!(
        "[Hippo] Hybrid search returned {} results",
        results.memories.len()
    );
    serde_json::to_value(results).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_qdrant_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting Qdrant stats...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let stats = hippo.qdrant_stats().await.map_err(|e| e.to_string())?;

    serde_json::to_value(stats).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_qdrant_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting Qdrant status...");
    let status = state.qdrant_manager.status().await;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn install_qdrant(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Installing Qdrant...");
    state
        .qdrant_manager
        .install()
        .await
        .map_err(|e| e.to_string())?;
    Ok("Qdrant installed successfully".to_string())
}

#[tauri::command]
async fn start_qdrant(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Starting Qdrant...");
    state
        .qdrant_manager
        .start()
        .await
        .map_err(|e| e.to_string())?;
    Ok("Qdrant started successfully".to_string())
}

// ==================== AI Natural Language Features ====================

#[tauri::command]
async fn parse_natural_query(
    query: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Parsing natural query: {}", query);
    let hippo_lock = state.hippo.read().await;
    let _hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    // Access the searcher through the internal field
    // Since we can't access private fields, we'll implement this directly
    use chrono::{Duration, Utc};
    use regex::Regex;

    let query_lower = query.to_lowercase();
    let mut keywords = query.clone();
    let mut file_types = Vec::new();
    let mut date_range = None;
    let mut interpretations = Vec::new();

    // Extract file types
    let type_patterns = [
        (
            r"\b(image|images|photo|photos|picture|pictures|pic|pics)\b",
            "image",
        ),
        (r"\b(video|videos|movie|movies|clip|clips)\b", "video"),
        (r"\b(audio|music|song|songs|sound|sounds)\b", "audio"),
        (
            r"\b(document|documents|doc|docs|pdf|pdfs|text|texts)\b",
            "document",
        ),
        (r"\b(code|source|script|scripts|program|programs)\b", "code"),
    ];

    for (pattern, kind_name) in type_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&query_lower) {
                file_types.push(kind_name.to_string());
                keywords = re.replace_all(&keywords, "").to_string();
                interpretations.push(format!("file type: {}", kind_name));
            }
        }
    }

    // Extract date ranges
    let now = Utc::now();
    let date_patterns = [
        (r"\b(today|tonight)\b", 0, "today"),
        (r"\b(yesterday)\b", 1, "yesterday"),
        (r"\blast week\b", 7, "last week"),
        (r"\blast month\b", 30, "last month"),
        (r"\blast year\b", 365, "last year"),
        (r"\bthis week\b", 7, "this week"),
        (r"\bthis month\b", 30, "this month"),
        (r"\bthis year\b", 365, "this year"),
    ];

    for (pattern, days, desc) in date_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&query_lower) {
                let start = now - Duration::days(days);
                date_range = Some(serde_json::json!({
                    "start": start.to_rfc3339(),
                    "end": now.to_rfc3339()
                }));
                keywords = re.replace_all(&keywords, "").to_string();
                interpretations.push(format!("date range: {}", desc));
                break;
            }
        }
    }

    // Clean up keywords
    keywords = keywords
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    let parsed = serde_json::json!({
        "original": query,
        "keywords": if keywords.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(keywords) },
        "file_types": file_types,
        "date_range": date_range,
        "interpretation": if interpretations.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(interpretations.join(", "))
        }
    });

    println!("[Hippo] Parsed query: {:?}", parsed);
    Ok(parsed)
}

#[tauri::command]
async fn natural_language_search(
    query: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Natural language search: {}", query);

    // First parse the query
    let parsed = parse_natural_query(query.clone(), state.clone()).await?;

    // Then perform search with extracted parameters
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let keywords = parsed
        .get("keywords")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut search_query = SearchQuery {
        text: keywords,
        ..Default::default()
    };

    // Add file type filters
    if let Some(types) = parsed.get("file_types").and_then(|v| v.as_array()) {
        for type_str in types.iter().filter_map(|v| v.as_str()) {
            use hippo_core::{DocumentFormat, MemoryKind};
            let kind = match type_str {
                "image" => MemoryKind::Image {
                    width: 0,
                    height: 0,
                    format: String::new(),
                },
                "video" => MemoryKind::Video {
                    duration_ms: 0,
                    format: String::new(),
                },
                "audio" => MemoryKind::Audio {
                    duration_ms: 0,
                    format: String::new(),
                },
                "document" => MemoryKind::Document {
                    format: DocumentFormat::Pdf,
                    page_count: None,
                },
                "code" => MemoryKind::Code {
                    language: String::new(),
                    lines: 0,
                },
                _ => continue,
            };
            search_query.kinds.push(kind);
        }
    }

    // Add date range filter
    if let Some(range) = parsed.get("date_range").and_then(|v| v.as_object()) {
        if let (Some(start_str), Some(end_str)) = (
            range.get("start").and_then(|v| v.as_str()),
            range.get("end").and_then(|v| v.as_str()),
        ) {
            if let (Ok(start), Ok(end)) = (
                chrono::DateTime::parse_from_rfc3339(start_str),
                chrono::DateTime::parse_from_rfc3339(end_str),
            ) {
                search_query.date_range = Some(hippo_core::DateRange {
                    start: Some(start.with_timezone(&Utc)),
                    end: Some(end.with_timezone(&Utc)),
                });
            }
        }
    }

    let results = hippo
        .search_advanced(search_query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "parsed": parsed,
        "results": results
    }))
}

#[tauri::command]
async fn caption_image(path: String) -> Result<String, String> {
    println!("[Hippo] Captioning image: {}", path);

    let client = OllamaClient::new();

    if !client.is_available().await {
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    let image_path = std::path::Path::new(&path);
    if !image_path.exists() {
        return Err("Image file not found".to_string());
    }

    let caption = client
        .caption_image(image_path)
        .await
        .map_err(|e| format!("Failed to caption image: {}", e))?;

    println!("[Hippo] Image caption generated");
    Ok(caption)
}

// ==================== Indexing Progress and Control ====================

#[tauri::command]
async fn get_indexing_progress(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting indexing progress...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let progress = hippo.indexer.get_progress().await;
    serde_json::to_value(progress).map_err(|e| e.to_string())
}

#[tauri::command]
async fn pause_indexing(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Pausing indexing...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    hippo.indexer.pause().await.map_err(|e| e.to_string())?;

    Ok("Indexing paused".to_string())
}

#[tauri::command]
async fn resume_indexing(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Resuming indexing...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    hippo.indexer.resume().await.map_err(|e| e.to_string())?;

    Ok("Indexing resumed".to_string())
}

#[tauri::command]
async fn set_indexing_priority(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Setting indexing priority for: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let file_path = std::path::Path::new(&path);
    let source = Source::Local {
        root_path: file_path.to_path_buf(),
    };

    hippo
        .indexer
        .set_priority(file_path, source)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Priority set successfully".to_string())
}

// ==================== AI Smart Suggestions ====================

#[tauri::command]
async fn get_tag_suggestions(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting tag suggestions for: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Use Ollama for local AI suggestions
    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        // Return empty suggestions if Ollama is not available
        return Ok(serde_json::json!({
            "tags": [],
            "available": false
        }));
    }

    match ai_client.suggest_tags_for_memory(&memory).await {
        Ok(suggestions) => {
            println!("[Hippo] Found {} tag suggestions", suggestions.len());
            Ok(serde_json::json!({
                "tags": suggestions,
                "available": true
            }))
        }
        Err(e) => {
            println!("[Hippo] Tag suggestion failed: {}", e);
            Ok(serde_json::json!({
                "tags": [],
                "available": true,
                "error": e.to_string()
            }))
        }
    }
}

#[tauri::command]
async fn get_similar_files(
    memory_id: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting similar files for: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    let all_memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(6);

    let ai_client = UnifiedAiClient::with_ollama(None);
    let similar = ai_client.suggest_similar_files(&memory, &all_memories, limit);

    println!("[Hippo] Found {} similar files", similar.len());
    serde_json::to_value(similar).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_duplicate_suggestions(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting duplicate suggestions for: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    let all_memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;

    let ai_client = UnifiedAiClient::with_ollama(None);
    let duplicates = ai_client.suggest_duplicates(&memory, &all_memories);

    println!("[Hippo] Found {} potential duplicates", duplicates.len());
    serde_json::to_value(duplicates).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_organization_suggestions_for_memory(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!(
        "[Hippo] Getting organization suggestions for: {}",
        memory_id
    );
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let all_memories = hippo.get_all_memories().await.map_err(|e| e.to_string())?;

    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        return Ok(serde_json::json!({
            "suggestions": [],
            "available": false
        }));
    }

    match ai_client.suggest_groupings(&all_memories).await {
        Ok(suggestions) => {
            println!(
                "[Hippo] Found {} organization suggestions",
                suggestions.len()
            );
            Ok(serde_json::json!({
                "suggestions": suggestions,
                "available": true
            }))
        }
        Err(e) => {
            println!("[Hippo] Organization suggestion failed: {}", e);
            Ok(serde_json::json!({
                "suggestions": [],
                "available": true,
                "error": e.to_string()
            }))
        }
    }
}

// ==================== Deep File Analysis ====================

#[tauri::command]
async fn deep_analyze_file(
    memory_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Deep analyzing file: {}", memory_id);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let id: MemoryId = memory_id.parse().map_err(|_| "Invalid memory ID")?;
    let memory = hippo
        .get_memory(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Memory not found")?;

    // Use Ollama for analysis
    let ollama = OllamaClient::new();
    if !ollama.is_available().await {
        return Err("Ollama is not running. Please start Ollama with a vision model (llava) for image analysis.".to_string());
    }

    let analysis = hippo_core::analyze_file(&memory, &ollama)
        .await
        .map_err(|e| format!("Analysis failed: {}", e))?;

    println!("[Hippo] Deep analysis complete: {}", analysis.summary());
    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_image_deep(path: String) -> Result<serde_json::Value, String> {
    println!("[Hippo] Deep analyzing image: {}", path);

    let image_path = std::path::Path::new(&path);
    if !image_path.exists() {
        return Err("Image file not found".to_string());
    }

    let ollama = OllamaClient::new();
    if !ollama.is_available().await {
        return Err(
            "Ollama is not running. Please start Ollama with a vision model (llava).".to_string(),
        );
    }

    let analysis = hippo_core::analyze_image(image_path, &ollama)
        .await
        .map_err(|e| format!("Image analysis failed: {}", e))?;

    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_document_deep(path: String) -> Result<serde_json::Value, String> {
    println!("[Hippo] Deep analyzing document: {}", path);

    let doc_path = std::path::Path::new(&path);
    if !doc_path.exists() {
        return Err("Document file not found".to_string());
    }

    let ollama = OllamaClient::new();
    if !ollama.is_available().await {
        return Err("Ollama is not running".to_string());
    }

    let analysis = hippo_core::analyze_document(doc_path, &ollama)
        .await
        .map_err(|e| format!("Document analysis failed: {}", e))?;

    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_code_deep(path: String) -> Result<serde_json::Value, String> {
    println!("[Hippo] Deep analyzing code: {}", path);

    let code_path = std::path::Path::new(&path);
    if !code_path.exists() {
        return Err("Code file not found".to_string());
    }

    let ollama = OllamaClient::new();
    if !ollama.is_available().await {
        return Err("Ollama is not running".to_string());
    }

    let analysis = hippo_core::analyze_code(code_path, &ollama)
        .await
        .map_err(|e| format!("Code analysis failed: {}", e))?;

    serde_json::to_value(analysis).map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_file_content(path: String, max_lines: usize) -> Result<serde_json::Value, String> {
    println!(
        "[Hippo] Reading file content: {} (max {} lines)",
        path, max_lines
    );

    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("File not found".to_string());
    }

    // Check if file is binary by reading first few bytes
    let mut buffer = [0; 512];
    match std::fs::File::open(file_path) {
        Ok(mut file) => {
            use std::io::Read;
            let bytes_read = file.read(&mut buffer).unwrap_or(0);
            if bytes_read > 0 {
                // Check for null bytes (common in binary files)
                let has_null = buffer[..bytes_read].contains(&0);
                if has_null {
                    return Ok(serde_json::json!({
                        "is_binary": true,
                        "content": "",
                        "lines": 0
                    }));
                }
            }
        }
        Err(e) => return Err(format!("Failed to open file: {}", e)),
    }

    // Read as text
    match std::fs::read_to_string(file_path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();
            let limited_lines: Vec<String> = lines
                .iter()
                .take(max_lines)
                .map(|s| s.to_string())
                .collect();
            let limited_content = limited_lines.join("\n");

            Ok(serde_json::json!({
                "is_binary": false,
                "content": limited_content,
                "lines": total_lines,
                "truncated": total_lines > max_lines
            }))
        }
        Err(_) => {
            // If reading as UTF-8 fails, it's likely binary
            Ok(serde_json::json!({
                "is_binary": true,
                "content": "",
                "lines": 0
            }))
        }
    }
}

// ==================== Export/Import Features ====================

#[tauri::command]
async fn export_index(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Exporting index...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let export = hippo
        .export_index()
        .await
        .map_err(|e| format!("Export failed: {}", e))?;

    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    println!(
        "[Hippo] Export complete: {} memories, {} sources, {} tags, {} clusters",
        export.memories.len(),
        export.sources.len(),
        export.tags.len(),
        export.clusters.len()
    );

    Ok(json)
}

#[tauri::command]
async fn import_index(
    json: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!(
        "[Hippo] Importing index from JSON ({} bytes)...",
        json.len()
    );
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let export: hippo_core::IndexExport =
        serde_json::from_str(&json).map_err(|e| format!("JSON deserialization failed: {}", e))?;

    let stats = hippo
        .import_index(export)
        .await
        .map_err(|e| format!("Import failed: {}", e))?;

    println!(
        "[Hippo] Import complete: {} memories, {} sources, {} tags, {} clusters imported ({} duplicates skipped)",
        stats.memories_imported,
        stats.sources_imported,
        stats.tags_imported,
        stats.clusters_imported,
        stats.duplicates_skipped
    );

    serde_json::to_value(stats).map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_to_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Exporting index to file: {}", path);

    // Get the export JSON
    let json = export_index(state).await?;

    // Write to file
    std::fs::write(&path, json).map_err(|e| format!("Failed to write file: {}", e))?;

    println!("[Hippo] Export written to: {}", path);
    Ok(format!("Export written to {}", path))
}

#[tauri::command]
async fn import_from_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    println!("[Hippo] Importing index from file: {}", path);

    // Read the file
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Import the data
    import_index(json, state).await
}

// ==================== File Watching ====================

#[tauri::command]
async fn start_watching(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Starting file watcher for all sources...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    hippo.watch_all().await.map_err(|e| e.to_string())?;

    println!("[Hippo] File watcher started");
    Ok("File watcher started successfully".to_string())
}

#[tauri::command]
async fn stop_watching(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Stopping file watcher...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    hippo.unwatch_all().await.map_err(|e| e.to_string())?;

    println!("[Hippo] File watcher stopped");
    Ok("File watcher stopped successfully".to_string())
}

#[tauri::command]
async fn pause_watching(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Pausing file watcher...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    if let Some(watcher) = &hippo.watcher {
        watcher
            .read()
            .await
            .pause()
            .await
            .map_err(|e| e.to_string())?;
        println!("[Hippo] File watcher paused");
        Ok("File watcher paused successfully".to_string())
    } else {
        Err("File watcher not available".to_string())
    }
}

#[tauri::command]
async fn resume_watching(state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Resuming file watcher...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    if let Some(watcher) = &hippo.watcher {
        watcher
            .read()
            .await
            .resume()
            .await
            .map_err(|e| e.to_string())?;
        println!("[Hippo] File watcher resumed");
        Ok("File watcher resumed successfully".to_string())
    } else {
        Err("File watcher not available".to_string())
    }
}

#[tauri::command]
async fn get_watcher_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Getting watcher stats...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    if let Some(stats) = hippo.watcher_stats().await {
        serde_json::to_value(stats).map_err(|e| e.to_string())
    } else {
        Ok(serde_json::json!({
            "total_watched_paths": 0,
            "events_processed": 0,
            "files_created": 0,
            "files_modified": 0,
            "files_deleted": 0,
            "files_renamed": 0,
            "is_watching": false,
            "is_paused": false
        }))
    }
}

#[tauri::command]
async fn watch_source_path(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Watching source path: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local {
        root_path: path.into(),
    };

    hippo
        .watch_source(&source)
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Watching source path started");
    Ok("Source path watching started".to_string())
}

#[tauri::command]
async fn unwatch_source_path(path: String, state: State<'_, AppState>) -> Result<String, String> {
    println!("[Hippo] Unwatching source path: {}", path);
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    let source = Source::Local {
        root_path: path.into(),
    };

    hippo
        .unwatch_source(&source)
        .await
        .map_err(|e| e.to_string())?;

    println!("[Hippo] Unwatching source path completed");
    Ok("Source path unwatched".to_string())
}

#[tauri::command]
async fn optimize_storage(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Optimizing storage...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.optimize_storage().await {
        Ok(stats) => {
            println!(
                "[Hippo] Storage optimized: {} bytes reclaimed ({:.1}%)",
                stats.bytes_reclaimed,
                stats.reclaim_percentage()
            );
            serde_json::to_value(&stats).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Storage optimization failed: {}", e);
            Err(format!("Storage optimization failed: {}", e))
        }
    }
}

#[tauri::command]
async fn vacuum_database(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    println!("[Hippo] Vacuuming database...");
    let hippo_lock = state.hippo.read().await;
    let hippo = hippo_lock.as_ref().ok_or("Hippo not initialized")?;

    match hippo.vacuum().await {
        Ok(stats) => {
            println!(
                "[Hippo] Database vacuumed: {} bytes reclaimed ({:.1}%)",
                stats.bytes_reclaimed,
                stats.reclaim_percentage()
            );
            serde_json::to_value(&stats).map_err(|e| e.to_string())
        }
        Err(e) => {
            println!("[Hippo] Vacuum failed: {}", e);
            Err(format!("Vacuum failed: {}", e))
        }
    }
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

/// Set up system tray with menu and event handlers
fn setup_system_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Create tray menu items
    let show_item = MenuItemBuilder::with_id("show", "Show Hippo").build(app)?;
    let search_item = MenuItemBuilder::with_id("search", "Quick Search...").build(app)?;
    let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let stats_item = MenuItemBuilder::with_id("stats", "View Stats").build(app)?;
    let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit Hippo").build(app)?;

    // Build menu
    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .item(&search_item)
        .item(&separator1)
        .item(&stats_item)
        .item(&separator2)
        .item(&quit_item)
        .build()?;

    // Load tray icon from embedded bytes
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .unwrap_or_else(|_| Image::new_owned(vec![0, 0, 0, 255], 1, 1));

    // Build tray icon
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Hippo - The Memory That Never Forgets")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    // Left click: show/focus the main window
                    if let Some(window) = tray.app_handle().get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "search" => {
                    // Show window and trigger search focus
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Emit event to focus search input
                        let _ = window.emit("focus-search", ());
                    }
                }
                "stats" => {
                    // Show window and trigger stats view
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Emit event to show stats
                        let _ = window.emit("show-stats", ());
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    println!("[Hippo] System tray initialized");
    Ok(())
}

fn main() {
    println!("[Hippo] Starting application...");
    tracing_subscriber::fmt::init();

    // Get data directory for Qdrant
    let data_dir = directories::ProjectDirs::from("com", "hippo", "app")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".hippo"));

    // Create Qdrant manager
    let qdrant_manager = Arc::new(QdrantManager::new(data_dir, "http://localhost:6334"));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            hippo: Arc::new(RwLock::new(None)),
            qdrant_manager,
        })
        .setup(|app| {
            println!("[Hippo] Application started. Qdrant will be auto-managed.");

            // Set up system tray
            setup_system_tray(app)?;

            Ok(())
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
            bulk_add_tag,
            bulk_delete,
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
            // Qdrant Management
            get_qdrant_status,
            install_qdrant,
            start_qdrant,
            // AI Natural Language Features
            parse_natural_query,
            natural_language_search,
            caption_image,
            // Indexing Progress and Control
            get_indexing_progress,
            pause_indexing,
            resume_indexing,
            set_indexing_priority,
            // Deep File Analysis
            deep_analyze_file,
            analyze_image_deep,
            analyze_document_deep,
            analyze_code_deep,
            // AI Smart Suggestions
            get_tag_suggestions,
            get_similar_files,
            get_duplicate_suggestions,
            get_organization_suggestions_for_memory,
            // File Content
            read_file_content,
            // Export/Import
            export_index,
            import_index,
            export_to_file,
            import_from_file,
            // File Watching
            start_watching,
            stop_watching,
            pause_watching,
            resume_watching,
            get_watcher_stats,
            watch_source_path,
            unwatch_source_path,
            // Storage Optimization
            optimize_storage,
            vacuum_database,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
