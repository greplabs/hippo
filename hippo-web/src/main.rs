//! # Hippo Web API
//!
//! REST API server for Hippo - intelligent file memory system.
//!
//! ## Configuration
//!
//! - `HIPPO_HOST` - Server host (default: "127.0.0.1")
//! - `HIPPO_PORT` - Server port (default: "3000")
//! - `HIPPO_DATA_DIR` - Data directory for Hippo (optional)
//!
//! ## Endpoints
//!
//! - `GET /api/health` - Health check
//! - `GET /api/stats` - Index statistics
//! - `GET /api/search` - Search memories with filters
//! - `GET /api/memories/:id` - Get single memory by ID
//! - `GET /api/thumbnails/:id` - Get thumbnail image for memory
//! - `GET /api/sources` - List configured sources
//! - `POST /api/sources` - Add a new source
//! - `DELETE /api/sources/:path` - Remove a source
//! - `GET /api/tags` - List all tags with counts
//! - `POST /api/memories/:id/tags` - Add tag to memory
//! - `DELETE /api/memories/:id/tags/:tag` - Remove tag from memory

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use hippo_core::{
    Hippo, HippoConfig, IndexStats, Memory, MemoryId, SearchQuery, SearchResults, Source,
    SourceConfig, Tag, TagFilter, TagFilterMode,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Shared application state
#[derive(Clone)]
struct AppState {
    hippo: Arc<Hippo>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hippo_web=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let host = std::env::var("HIPPO_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("HIPPO_PORT").unwrap_or_else(|_| "3000".to_string());

    let mut config = HippoConfig::default();
    if let Ok(data_dir) = std::env::var("HIPPO_DATA_DIR") {
        config.data_dir = PathBuf::from(data_dir);
    }

    info!(
        "Initializing Hippo with data directory: {:?}",
        config.data_dir
    );

    // Initialize Hippo instance
    let hippo = Arc::new(Hippo::with_config(config).await?);
    info!("Hippo initialized successfully");

    let state = AppState { hippo };

    // Build router with CORS and tracing middleware
    let app = Router::new()
        // Health check
        .route("/api/health", get(health_check))
        // Stats
        .route("/api/stats", get(get_stats))
        // Search
        .route("/api/search", get(search_memories))
        // Memories
        .route("/api/memories/:id", get(get_memory))
        // Thumbnails
        .route("/api/thumbnails/:id", get(get_thumbnail))
        // Sources
        .route("/api/sources", get(list_sources).post(add_source))
        .route("/api/sources/*path", delete(remove_source))
        // Tags
        .route("/api/tags", get(list_tags))
        .route("/api/memories/:id/tags", post(add_tag))
        .route("/api/memories/:id/tags/:tag", delete(remove_tag))
        // Apply middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Bind and serve
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Starting Hippo Web API server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// API Handlers
// ============================================================================

/// GET /api/health
///
/// Health check endpoint to verify the service is running.
///
/// ## Response
/// ```json
/// {
///   "status": "ok",
///   "version": "0.1.0"
/// }
/// ```
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// GET /api/stats
///
/// Get index statistics including total memories, breakdown by kind and source.
///
/// ## Response
/// Returns `IndexStats` containing:
/// - `total_memories`: Total number of indexed items
/// - `by_kind`: Breakdown by memory type (Image, Video, Code, etc.)
/// - `by_source`: Breakdown by source
/// - `total_size_bytes`: Total size of indexed files
/// - `index_size_bytes`: Database size
/// - `last_updated`: Last update timestamp
async fn get_stats(State(state): State<AppState>) -> Result<Json<IndexStats>, ApiError> {
    let stats = state.hippo.stats().await?;
    Ok(Json(stats))
}

/// GET /api/search
///
/// Search memories with optional filters and sorting.
///
/// ## Query Parameters
/// - `q` (optional): Search text for semantic/keyword search
/// - `tags` (optional): Comma-separated tags to filter by (prefix with `-` to exclude)
/// - `type` (optional): Filter by memory kind (Image, Video, Audio, Code, Document)
/// - `sort` (optional): Sort order (Relevance, DateNewest, DateOldest, NameAsc, NameDesc, SizeAsc, SizeDesc)
/// - `limit` (optional): Maximum results to return (default: 50)
/// - `offset` (optional): Offset for pagination (default: 0)
/// - `mode` (optional): Search mode - "keyword" (default), "semantic", or "hybrid"
///
/// ## Example
/// ```
/// GET /api/search?q=sunset&tags=vacation,-private&type=Image&sort=DateNewest&limit=20
/// GET /api/search?q=beautiful landscape&mode=hybrid&limit=20
/// ```
///
/// ## Response
/// Returns `SearchResults` containing:
/// - `memories`: Array of matching memories with scores
/// - `total_count`: Total matching results
/// - `suggested_tags`: Tag suggestions based on results
/// - `clusters`: Related clusters
async fn search_memories(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResults>, ApiError> {
    let mode = params.mode.as_deref().unwrap_or("keyword");
    let limit = params.limit.unwrap_or(50);

    // Use different search methods based on mode
    let results = match mode {
        "hybrid" => {
            // Hybrid search: 70% semantic + 30% keyword
            if let Some(ref q) = params.q {
                state.hippo.hybrid_search(q, limit).await?
            } else {
                let query = params.into_search_query()?;
                state.hippo.search_advanced(query).await?
            }
        }
        "semantic" => {
            // Pure semantic/embedding search
            if let Some(ref q) = params.q {
                state.hippo.semantic_search(q, limit).await?
            } else {
                let query = params.into_search_query()?;
                state.hippo.search_advanced(query).await?
            }
        }
        _ => {
            // Default: keyword-based search with filters
            let query = params.into_search_query()?;
            state.hippo.search_advanced(query).await?
        }
    };

    Ok(Json(results))
}

/// GET /api/memories/:id
///
/// Get a single memory by its UUID.
///
/// ## Path Parameters
/// - `id`: UUID of the memory
///
/// ## Response
/// Returns the full `Memory` object if found, or 404 if not found.
async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Memory>, ApiError> {
    let memory_id = MemoryId::from_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid memory ID".to_string()))?;

    let memory = state
        .hippo
        .get_memory(memory_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Memory not found".to_string()))?;

    Ok(Json(memory))
}

/// GET /api/thumbnails/:id
///
/// Get thumbnail image for a memory (images and videos only).
///
/// ## Path Parameters
/// - `id`: UUID of the memory
///
/// ## Response
/// Returns JPEG image data with appropriate content-type header.
/// Returns 404 if memory not found or thumbnail cannot be generated.
async fn get_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let memory_id = MemoryId::from_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid memory ID".to_string()))?;

    let memory = state
        .hippo
        .get_memory(memory_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Memory not found".to_string()))?;

    // Try to get thumbnail based on memory kind
    let thumbnail_path = match &memory.kind {
        hippo_core::MemoryKind::Image { .. } => state.hippo.get_thumbnail(&memory.path).ok(),
        hippo_core::MemoryKind::Video { .. } => state.hippo.get_video_thumbnail(&memory.path).ok(),
        _ => None,
    };

    let thumbnail_path =
        thumbnail_path.ok_or_else(|| ApiError::NotFound("Thumbnail not available".to_string()))?;

    // Read thumbnail file
    let data = tokio::fs::read(&thumbnail_path)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to read thumbnail: {}", e)))?;

    // Return image with proper content type
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], data).into_response())
}

/// GET /api/sources
///
/// List all configured sources.
///
/// ## Response
/// Returns array of `SourceConfig` objects containing source settings.
async fn list_sources(State(state): State<AppState>) -> Result<Json<Vec<SourceConfig>>, ApiError> {
    let sources = state.hippo.list_sources().await?;
    Ok(Json(sources))
}

/// POST /api/sources
///
/// Add a new source to index.
///
/// ## Request Body
/// ```json
/// {
///   "sourceType": "Local",
///   "path": "/Users/you/Documents"
/// }
/// ```
///
/// ## Response
/// Returns success message with source details.
async fn add_source(
    State(state): State<AppState>,
    Json(payload): Json<AddSourceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let source = match payload.source_type.as_str() {
        "Local" => Source::Local {
            root_path: PathBuf::from(&payload.path),
        },
        _ => {
            return Err(ApiError::BadRequest(
                "Only Local sources are currently supported".to_string(),
            ))
        }
    };

    state.hippo.add_source(source.clone()).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Source added successfully",
        "source": source
    })))
}

/// DELETE /api/sources/:path
///
/// Remove a source and optionally delete its indexed memories.
///
/// ## Path Parameters
/// - `path`: URL-encoded path of the source to remove
///
/// ## Query Parameters
/// - `deleteFiles` (optional): If "true", also delete indexed memories (default: false)
///
/// ## Example
/// ```
/// DELETE /api/sources/Users/you/Documents?deleteFiles=true
/// ```
async fn remove_source(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<RemoveSourceParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let source = Source::Local {
        root_path: PathBuf::from(&path),
    };

    let delete_memories = params.delete_files.unwrap_or(false);
    state.hippo.remove_source(&source, delete_memories).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Source removed successfully"
    })))
}

/// GET /api/tags
///
/// List all tags with their usage counts.
///
/// ## Response
/// Returns array of tag objects with name and count.
/// ```json
/// [
///   { "name": "vacation", "count": 42 },
///   { "name": "work", "count": 128 }
/// ]
/// ```
async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<TagInfo>>, ApiError> {
    let tags = state.hippo.list_tags().await?;
    let tag_info: Vec<TagInfo> = tags
        .into_iter()
        .map(|(name, count)| TagInfo { name, count })
        .collect();
    Ok(Json(tag_info))
}

/// POST /api/memories/:id/tags
///
/// Add a tag to a memory.
///
/// ## Path Parameters
/// - `id`: UUID of the memory
///
/// ## Request Body
/// ```json
/// {
///   "tag": "vacation"
/// }
/// ```
async fn add_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AddTagRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let memory_id = MemoryId::from_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid memory ID".to_string()))?;

    let tag = Tag::user(&payload.tag);
    state.hippo.add_tag(memory_id, tag).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Tag added successfully"
    })))
}

/// DELETE /api/memories/:id/tags/:tag
///
/// Remove a tag from a memory.
///
/// ## Path Parameters
/// - `id`: UUID of the memory
/// - `tag`: Name of the tag to remove
async fn remove_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let memory_id = MemoryId::from_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid memory ID".to_string()))?;

    state.hippo.remove_tag(memory_id, &tag).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Tag removed successfully"
    })))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
    tags: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    sort: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    /// Search mode: "keyword" (default), "semantic", or "hybrid"
    mode: Option<String>,
}

impl SearchParams {
    fn into_search_query(self) -> Result<SearchQuery, ApiError> {
        let mut query = SearchQuery {
            text: self.q,
            limit: self.limit.unwrap_or(50),
            offset: self.offset.unwrap_or(0),
            ..Default::default()
        };

        // Parse tags (comma-separated, prefix with - for exclude)
        if let Some(tags_str) = self.tags {
            query.tags = tags_str
                .split(',')
                .map(|t| {
                    let tag = t.trim();
                    if let Some(tag_name) = tag.strip_prefix('-') {
                        TagFilter {
                            tag: tag_name.to_string(),
                            mode: TagFilterMode::Exclude,
                        }
                    } else {
                        TagFilter {
                            tag: tag.to_string(),
                            mode: TagFilterMode::Include,
                        }
                    }
                })
                .collect();
        }

        // Parse type/kind filter
        if let Some(kind_str) = self.kind {
            query.kinds = kind_str
                .split(',')
                .filter_map(|k| match k.trim() {
                    "Image" | "image" => Some(hippo_core::MemoryKind::Image {
                        width: 0,
                        height: 0,
                        format: String::new(),
                    }),
                    "Video" | "video" => Some(hippo_core::MemoryKind::Video {
                        duration_ms: 0,
                        format: String::new(),
                    }),
                    "Audio" | "audio" => Some(hippo_core::MemoryKind::Audio {
                        duration_ms: 0,
                        format: String::new(),
                    }),
                    "Document" | "document" => Some(hippo_core::MemoryKind::Document {
                        format: hippo_core::DocumentFormat::Other(String::new()),
                        page_count: None,
                    }),
                    "Code" | "code" => Some(hippo_core::MemoryKind::Code {
                        language: String::new(),
                        lines: 0,
                    }),
                    _ => None,
                })
                .collect();
        }

        // Parse sort order
        if let Some(sort_str) = self.sort {
            query.sort = match sort_str.as_str() {
                "Relevance" => hippo_core::SortOrder::Relevance,
                "DateNewest" => hippo_core::SortOrder::DateNewest,
                "DateOldest" => hippo_core::SortOrder::DateOldest,
                "NameAsc" => hippo_core::SortOrder::NameAsc,
                "NameDesc" => hippo_core::SortOrder::NameDesc,
                "SizeAsc" => hippo_core::SortOrder::SizeAsc,
                "SizeDesc" => hippo_core::SortOrder::SizeDesc,
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid sort order: {}",
                        sort_str
                    )))
                }
            };
        }

        Ok(query)
    }
}

#[derive(Debug, Deserialize)]
struct AddSourceRequest {
    #[serde(rename = "sourceType")]
    source_type: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct RemoveSourceParams {
    #[serde(rename = "deleteFiles")]
    delete_files: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AddTagRequest {
    tag: String,
}

#[derive(Debug, Serialize)]
struct TagInfo {
    name: String,
    count: u64,
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
    HippoError(hippo_core::HippoError),
}

impl From<hippo_core::HippoError> for ApiError {
    fn from(err: hippo_core::HippoError) -> Self {
        ApiError::HippoError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::HippoError(err) => {
                error!("Hippo error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal error: {}", err),
                )
            }
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
