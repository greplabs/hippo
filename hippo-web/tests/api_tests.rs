//! Integration tests for Hippo Web API
//!
//! These tests start a real server instance and test the API endpoints.

use reqwest::Method;
use serde_json::json;

/// Helper to build a test client URL
fn api_url(endpoint: &str) -> String {
    let host = std::env::var("HIPPO_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("HIPPO_TEST_PORT").unwrap_or_else(|_| "3001".to_string());
    format!("http://{}:{}/api{}", host, port, endpoint)
}

#[tokio::test]
#[ignore] // Run with: cargo test --package hippo-web -- --ignored
async fn test_health_check() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/health"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
#[ignore]
async fn test_get_stats() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/stats"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["total_memories"].is_number());
    assert!(body["by_kind"].is_object());
    assert!(body["by_source"].is_object());
}

#[tokio::test]
#[ignore]
async fn test_search_no_params() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/search"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["memories"].is_array());
    assert!(body["total_count"].is_number());
    assert!(body["suggested_tags"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_search_with_query() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/search?q=test&limit=10"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["memories"].is_array());
    let memories = body["memories"].as_array().unwrap();
    assert!(memories.len() <= 10);
}

#[tokio::test]
#[ignore]
async fn test_search_with_tags() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/search?tags=important,-private&limit=20"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["memories"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_list_sources() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/sources"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body.is_array());
}

#[tokio::test]
#[ignore]
async fn test_add_and_remove_source() {
    let client = reqwest::Client::new();

    // Create a temporary directory for testing
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_path = temp_dir.path().to_string_lossy().to_string();

    // Add source
    let add_response = client
        .post(api_url("/sources"))
        .json(&json!({
            "sourceType": "Local",
            "path": test_path
        }))
        .send()
        .await
        .expect("Failed to add source");

    assert_eq!(add_response.status(), 200);
    let add_body: serde_json::Value = add_response.json().await.expect("Failed to parse JSON");
    assert_eq!(add_body["success"], true);

    // Remove source
    let remove_response = client
        .delete(api_url(&format!("/sources/{}", test_path)))
        .send()
        .await
        .expect("Failed to remove source");

    assert_eq!(remove_response.status(), 200);
    let remove_body: serde_json::Value =
        remove_response.json().await.expect("Failed to parse JSON");
    assert_eq!(remove_body["success"], true);
}

#[tokio::test]
#[ignore]
async fn test_list_tags() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/tags"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body.is_array());

    // Each tag should have name and count
    if let Some(tags) = body.as_array() {
        for tag in tags {
            assert!(tag["name"].is_string());
            assert!(tag["count"].is_number());
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_invalid_memory_id() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/memories/invalid-uuid"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
#[ignore]
async fn test_nonexistent_memory() {
    let client = reqwest::Client::new();
    // Use a valid UUID format that doesn't exist
    let response = client
        .get(api_url("/memories/00000000-0000-0000-0000-000000000000"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_cors_headers() {
    let client = reqwest::Client::new();
    let response = client
        .request(Method::OPTIONS, api_url("/health"))
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .expect("Failed to send request");

    // CORS middleware should allow the request
    assert!(response.status().is_success() || response.status() == 204);
}

#[tokio::test]
#[ignore]
async fn test_invalid_sort_order() {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url("/search?sort=InvalidSort"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["error"].is_string());
}

/// Test helper to start the server in background
/// Run with: HIPPO_TEST_SERVER=true cargo test --package hippo-web -- --ignored
#[cfg(feature = "test-server")]
mod server_tests {
    use super::*;

    #[tokio::test]
    async fn start_test_server() {
        // This would start a test server instance
        // For now, tests assume a server is running externally
        // TODO: Implement background server startup for tests
    }
}
