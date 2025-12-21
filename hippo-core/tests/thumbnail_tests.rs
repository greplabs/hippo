//! Comprehensive tests for thumbnail generation and caching
//!
//! Tests image thumbnail generation using the public Hippo API.

use hippo_core::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// Image Support Tests
// ============================================================================

#[test]
fn test_is_supported_image_jpg() {
    assert!(is_supported_image(Path::new("photo.jpg")));
    assert!(is_supported_image(Path::new("photo.JPG")));
    assert!(is_supported_image(Path::new("photo.jpeg")));
    assert!(is_supported_image(Path::new("photo.JPEG")));
}

#[test]
fn test_is_supported_image_png() {
    assert!(is_supported_image(Path::new("image.png")));
    assert!(is_supported_image(Path::new("image.PNG")));
}

#[test]
fn test_is_supported_image_other_formats() {
    assert!(is_supported_image(Path::new("image.gif")));
    assert!(is_supported_image(Path::new("image.bmp")));
    assert!(is_supported_image(Path::new("image.webp")));
    assert!(is_supported_image(Path::new("image.tiff")));
    assert!(is_supported_image(Path::new("image.ico")));
}

#[test]
fn test_is_supported_image_unsupported() {
    assert!(!is_supported_image(Path::new("document.pdf")));
    assert!(!is_supported_image(Path::new("video.mp4")));
    assert!(!is_supported_image(Path::new("text.txt")));
    assert!(!is_supported_image(Path::new("archive.zip")));
}

#[test]
fn test_is_supported_video_common_formats() {
    assert!(is_supported_video(Path::new("video.mp4")));
    assert!(is_supported_video(Path::new("video.MP4")));
    assert!(is_supported_video(Path::new("video.mov")));
    assert!(is_supported_video(Path::new("video.MOV")));
}

#[test]
fn test_is_supported_video_other_formats() {
    assert!(is_supported_video(Path::new("video.avi")));
    assert!(is_supported_video(Path::new("video.mkv")));
    assert!(is_supported_video(Path::new("video.webm")));
    assert!(is_supported_video(Path::new("video.flv")));
    assert!(is_supported_video(Path::new("video.wmv")));
}

#[test]
fn test_is_supported_video_unsupported() {
    assert!(!is_supported_video(Path::new("photo.jpg")));
    assert!(!is_supported_video(Path::new("audio.mp3")));
    assert!(!is_supported_video(Path::new("text.txt")));
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_image(width: u32, height: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(width, height);

    // Create a gradient pattern for testing
    for y in 0..height {
        for x in 0..width {
            let r = (x * 255 / width) as u8;
            let g = (y * 255 / height) as u8;
            let b = 128;
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }

    img
}

// ============================================================================
// Thumbnail Generation Tests
// ============================================================================

#[tokio::test]
async fn test_thumbnail_manager_creation() {
    let manager = ThumbnailManager::new();
    assert!(manager.is_ok(), "Should create thumbnail manager");
}

#[tokio::test]
async fn test_generate_thumbnail_from_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("test.png");

    // Create a test image
    let img = create_test_image(1920, 1080);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();

    let result = manager.generate_thumbnail(&source_path).await;
    assert!(result.is_ok(), "Should generate thumbnail from image");

    let thumb_path = result.unwrap();
    assert!(thumb_path.exists(), "Thumbnail file should exist");
}

#[tokio::test]
async fn test_generate_thumbnail_size() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("large.png");

    // Create large image
    let img = create_test_image(4000, 3000);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let thumb_path = manager.generate_thumbnail(&source_path).await.unwrap();

    // Load thumbnail and check size
    let thumb_img = image::open(&thumb_path).unwrap();
    let (w, h) = (thumb_img.width(), thumb_img.height());

    // Should be scaled down to max 256x256
    assert!(
        w <= THUMBNAIL_SIZE && h <= THUMBNAIL_SIZE,
        "Thumbnail should be at most {}x{}, got {}x{}",
        THUMBNAIL_SIZE,
        THUMBNAIL_SIZE,
        w,
        h
    );

    // Should preserve aspect ratio
    let original_ratio = 4000.0 / 3000.0;
    let thumb_ratio = w as f32 / h as f32;
    let ratio_diff = (original_ratio - thumb_ratio).abs();
    assert!(
        ratio_diff < 0.01,
        "Should preserve aspect ratio, original: {}, thumb: {}",
        original_ratio,
        thumb_ratio
    );
}

#[tokio::test]
async fn test_generate_thumbnail_nonexistent_file() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();

    let source_path = PathBuf::from("/nonexistent/image.jpg");
    let result = manager.generate_thumbnail(&source_path).await;

    assert!(result.is_err(), "Should error on nonexistent file");
}

#[tokio::test]
async fn test_thumbnail_stats() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let stats = manager.get_stats();
    assert!(stats.is_ok(), "Should get stats");
}

#[tokio::test]
async fn test_hippo_thumbnail_integration() {
    let temp = TempDir::new().unwrap();
    let config = HippoConfig {
        data_dir: temp.path().join("hippo_data"),
        qdrant_url: "http://localhost:9999".to_string(),
        ..Default::default()
    };

    let hippo = Hippo::with_config(config).await.unwrap();

    // Create test image
    let image_path = temp.path().join("test.jpg");
    let img = create_test_image(800, 600);
    img.save(&image_path).unwrap();

    // Get thumbnail through Hippo API
    let result = hippo.get_thumbnail(&image_path);
    assert!(result.is_ok(), "Should get thumbnail via Hippo API");

    let thumb_path = result.unwrap();
    assert!(thumb_path.exists(), "Thumbnail should exist");
}

// Note: Most detailed thumbnail tests (caching, invalidation, etc.) should be
// unit tests in src/thumbnails/mod.rs. These integration tests verify the
// public API works correctly.
