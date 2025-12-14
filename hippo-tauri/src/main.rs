//! Hippo Desktop Application
//! 
//! 🦛 The memory that never forgets

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use hippo_core::{Hippo, SearchQuery, Tag, Source, MemoryId, ClaudeClient};
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
            // AI Features
            analyze_file,
            summarize_document,
            get_organization_suggestions,
            semantic_search,
            // Code Engine
            search_symbols,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
