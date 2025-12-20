//! Streaming AI chat commands for Tauri
//!
//! Provides real-time streaming chat functionality with cancellation support

use hippo_core::UnifiedAiClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// State for managing active streams
pub struct StreamState {
    pub active_streams: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl StreamState {
    pub fn new() -> Self {
        Self {
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Streaming chat chunk event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunkPayload {
    pub stream_id: String,
    pub chunk: String,
    pub done: bool,
}

/// Streaming error event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatErrorPayload {
    pub stream_id: String,
    pub error: String,
}

/// Start a streaming chat session
#[tauri::command]
pub async fn stream_chat(
    messages: Vec<(String, String)>,
    app: AppHandle,
    stream_state: State<'_, StreamState>,
) -> Result<String, String> {
    println!("[Hippo] Starting streaming chat with {} messages", messages.len());

    // Generate unique stream ID
    let stream_id = Uuid::new_v4().to_string();
    let stream_id_clone = stream_id.clone();

    // Create cancellation token
    let cancellation_token = CancellationToken::new();

    // Store the token so it can be cancelled later
    {
        let mut streams = stream_state.active_streams.write().await;
        streams.insert(stream_id.clone(), cancellation_token.clone());
    }

    // Create AI client
    let ai_client = UnifiedAiClient::with_ollama(None);

    if !ai_client.is_available().await {
        // Clean up
        let mut streams = stream_state.active_streams.write().await;
        streams.remove(&stream_id);
        return Err("Ollama is not running. Please start Ollama first.".to_string());
    }

    // Clone app handle for the closure
    let app_clone = app.clone();
    let stream_id_for_closure = stream_id.clone();

    // Stream the chat
    let result = ai_client
        .stream_chat(messages, cancellation_token.clone(), move |chunk| {
            // Emit chunk event to frontend
            let payload = ChatChunkPayload {
                stream_id: stream_id_for_closure.clone(),
                chunk,
                done: false,
            };

            if let Err(e) = app_clone.emit("chat-chunk", payload) {
                eprintln!("[Hippo] Failed to emit chunk event: {}", e);
            }
        })
        .await;

    // Clean up the stream from active streams
    {
        let mut streams = stream_state.active_streams.write().await;
        streams.remove(&stream_id_clone);
    }

    match result {
        Ok(full_response) => {
            // Emit final done event
            let payload = ChatChunkPayload {
                stream_id: stream_id_clone.clone(),
                chunk: String::new(),
                done: true,
            };
            let _ = app.emit("chat-chunk", payload);

            println!("[Hippo] Stream completed successfully");
            Ok(stream_id_clone)
        }
        Err(e) => {
            // Emit error event
            let error_payload = ChatErrorPayload {
                stream_id: stream_id_clone.clone(),
                error: e.to_string(),
            };
            let _ = app.emit("chat-error", error_payload);

            println!("[Hippo] Stream failed: {}", e);
            Err(format!("Chat stream failed: {}", e))
        }
    }
}

/// Cancel an active streaming chat session
#[tauri::command]
pub async fn cancel_stream(
    stream_id: String,
    stream_state: State<'_, StreamState>,
) -> Result<String, String> {
    println!("[Hippo] Cancelling stream: {}", stream_id);

    let mut streams = stream_state.active_streams.write().await;

    if let Some(token) = streams.remove(&stream_id) {
        token.cancel();
        println!("[Hippo] Stream {} cancelled successfully", stream_id);
        Ok(format!("Stream {} cancelled", stream_id))
    } else {
        Err(format!("Stream {} not found or already completed", stream_id))
    }
}

/// Get list of active stream IDs
#[tauri::command]
pub async fn get_active_streams(
    stream_state: State<'_, StreamState>,
) -> Result<Vec<String>, String> {
    let streams = stream_state.active_streams.read().await;
    let active_ids: Vec<String> = streams.keys().cloned().collect();

    println!("[Hippo] {} active streams", active_ids.len());
    Ok(active_ids)
}
