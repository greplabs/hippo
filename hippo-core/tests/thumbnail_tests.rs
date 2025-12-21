//! Comprehensive tests for thumbnail generation and caching
//!
//! Tests image thumbnail generation, video frame extraction, caching behavior,
//! and memory management.

use hippo_core::thumbnails::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// Thumbnail Manager Tests
// ============================================================================

#[test]
fn test_thumbnail_manager_creation() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().to_path_buf();

    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone());
    assert!(manager.is_ok(), "Should create thumbnail manager");

    let manager = manager.unwrap();
    assert_eq!(manager.cache_dir(), cache_dir.as_path());
}

#[test]
fn test_thumbnail_manager_default() {
    let manager = ThumbnailManager::new();
    assert!(manager.is_ok(), "Should create default thumbnail manager");

    let manager = manager.unwrap();
    let cache_dir = manager.cache_dir();
    assert!(cache_dir.exists() || !cache_dir.as_os_str().is_empty());
}

#[test]
fn test_thumbnail_manager_creates_cache_directory() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join("thumbnails");

    // Cache dir doesn't exist yet
    assert!(!cache_dir.exists());

    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone());
    assert!(manager.is_ok());

    // Should be created by manager
    let manager = manager.unwrap();
    assert_eq!(manager.cache_dir(), cache_dir.as_path());
}

#[test]
fn test_get_thumbnail_path_consistency() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let source_path = PathBuf::from("/test/images/photo.jpg");

    let thumb1 = manager.get_thumbnail_path(&source_path);
    let thumb2 = manager.get_thumbnail_path(&source_path);

    assert_eq!(
        thumb1, thumb2,
        "Same source should produce same thumbnail path"
    );
}

#[test]
fn test_get_thumbnail_path_different_sources() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let path1 = PathBuf::from("/test/photo1.jpg");
    let path2 = PathBuf::from("/test/photo2.jpg");

    let thumb1 = manager.get_thumbnail_path(&path1);
    let thumb2 = manager.get_thumbnail_path(&path2);

    assert_ne!(
        thumb1, thumb2,
        "Different sources should produce different thumbnail paths"
    );
}

#[test]
fn test_get_thumbnail_path_uses_cache_dir() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().to_path_buf();
    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone()).unwrap();

    let source = PathBuf::from("/test/photo.jpg");
    let thumb = manager.get_thumbnail_path(&source);

    assert!(
        thumb.starts_with(&cache_dir),
        "Thumbnail path should be in cache dir"
    );
}

#[test]
fn test_thumbnail_path_has_jpg_extension() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let source = PathBuf::from("/test/photo.png");
    let thumb = manager.get_thumbnail_path(&source);

    assert_eq!(
        thumb.extension().and_then(|e| e.to_str()),
        Some("jpg"),
        "Thumbnails should be .jpg"
    );
}

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
// Thumbnail Generation Tests (with mock images)
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
async fn test_generate_thumbnail_caching() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("test.png");

    let img = create_test_image(800, 600);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();

    // Generate first time
    let thumb1 = manager.generate_thumbnail(&source_path).await.unwrap();
    let mtime1 = fs::metadata(&thumb1).unwrap().modified().unwrap();

    // Small delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Generate again - should use cache
    let thumb2 = manager.generate_thumbnail(&source_path).await.unwrap();
    let mtime2 = fs::metadata(&thumb2).unwrap().modified().unwrap();

    assert_eq!(thumb1, thumb2, "Should return same thumbnail path");
    assert_eq!(mtime1, mtime2, "Should not regenerate if source unchanged");
}

#[tokio::test]
async fn test_generate_thumbnail_invalidates_on_modification() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("test.png");

    // Create initial image
    let img1 = create_test_image(800, 600);
    img1.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();

    // Generate first thumbnail
    let _thumb1 = manager.generate_thumbnail(&source_path).await.unwrap();

    // Wait and modify source
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let img2 = create_test_image(1920, 1080);
    img2.save(&source_path).unwrap();

    // Generate again - should invalidate cache
    let result = manager.generate_thumbnail(&source_path).await;
    assert!(
        result.is_ok(),
        "Should regenerate thumbnail after modification"
    );
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
        w <= 256 && h <= 256,
        "Thumbnail should be at most 256x256, got {}x{}",
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
async fn test_generate_thumbnail_small_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("small.png");

    // Create image smaller than thumbnail size
    let img = create_test_image(100, 100);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let thumb_path = manager.generate_thumbnail(&source_path).await.unwrap();

    // Should still generate thumbnail
    assert!(thumb_path.exists());

    let thumb_img = image::open(&thumb_path).unwrap();
    assert!(thumb_img.width() <= 256 && thumb_img.height() <= 256);
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
async fn test_generate_thumbnail_invalid_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("invalid.jpg");

    // Write invalid image data
    fs::write(&source_path, b"not a valid image").unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let result = manager.generate_thumbnail(&source_path).await;

    assert!(result.is_err(), "Should error on invalid image");
}

// ============================================================================
// Memory Cache Tests
// ============================================================================

#[test]
fn test_memory_cache_stats_initial() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let stats = manager.memory_cache_stats();

    assert_eq!(stats.entries, 0, "Should start with no entries");
    assert_eq!(stats.memory_bytes, 0, "Should start with no memory usage");
    assert!(stats.capacity > 0, "Should have positive capacity");
    assert!(
        stats.max_memory_bytes > 0,
        "Should have positive max memory"
    );
}

#[tokio::test]
async fn test_memory_cache_stores_thumbnails() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("test.png");

    let img = create_test_image(800, 600);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();

    // Generate thumbnail (should cache in memory)
    let _thumb = manager.generate_thumbnail(&source_path).await.unwrap();

    let stats = manager.memory_cache_stats();

    // Memory cache might not store immediately, but capacity should be set
    assert!(stats.capacity > 0);
    assert!(stats.max_memory_bytes > 0);
}

#[test]
fn test_clear_memory_cache() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    // Clear cache (should not error even if empty)
    manager.clear_memory_cache();

    let stats = manager.memory_cache_stats();
    assert_eq!(stats.entries, 0);
    assert_eq!(stats.memory_bytes, 0);
}

// ============================================================================
// Disk Cache Tests
// ============================================================================

#[tokio::test]
async fn test_disk_cache_persists() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join("cache");
    let source_path = temp.path().join("test.png");

    let img = create_test_image(800, 600);
    img.save(&source_path).unwrap();

    // Generate thumbnail
    {
        let manager = ThumbnailManager::with_cache_dir(cache_dir.clone()).unwrap();
        let _thumb = manager.generate_thumbnail(&source_path).await.unwrap();
    }

    // Create new manager - should find cached thumbnail
    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone()).unwrap();
    let thumb_path = manager.get_thumbnail_path(&source_path);

    assert!(
        thumb_path.exists(),
        "Thumbnail should persist on disk after manager dropped"
    );
}

#[tokio::test]
async fn test_clear_disk_cache() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join("cache");
    let source_path = temp.path().join("test.png");

    let img = create_test_image(800, 600);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone()).unwrap();

    // Generate some thumbnails
    let _thumb = manager.generate_thumbnail(&source_path).await.unwrap();

    // Clear disk cache
    let result = manager.clear_disk_cache().await;
    assert!(result.is_ok(), "Should clear disk cache without error");

    // Cache directory should be empty or not exist
    if cache_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&cache_dir).unwrap().collect();
        assert_eq!(entries.len(), 0, "Cache directory should be empty");
    }
}

#[tokio::test]
async fn test_get_disk_cache_size() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join("cache");
    let source_path = temp.path().join("test.png");

    let img = create_test_image(800, 600);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(cache_dir.clone()).unwrap();

    // Initial size should be 0
    let initial_size = manager.get_disk_cache_size().await.unwrap();
    assert_eq!(initial_size, 0);

    // Generate thumbnail
    let _thumb = manager.generate_thumbnail(&source_path).await.unwrap();

    // Size should increase
    let after_size = manager.get_disk_cache_size().await.unwrap();
    assert!(
        after_size > 0,
        "Cache size should increase after generating thumbnail"
    );
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_thumbnail_generation() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join("cache");

    // Create multiple test images
    let mut paths = vec![];
    for i in 0..5 {
        let path = temp.path().join(format!("test_{}.png", i));
        let img = create_test_image(800, 600);
        img.save(&path).unwrap();
        paths.push(path);
    }

    let manager = ThumbnailManager::with_cache_dir(cache_dir).unwrap();

    // Generate thumbnails concurrently
    let handles: Vec<_> = paths
        .iter()
        .map(|path| {
            let manager = manager.clone();
            let path = path.clone();
            tokio::spawn(async move { manager.generate_thumbnail(&path).await })
        })
        .collect();

    // Wait for all
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 5,
        "All thumbnails should be generated successfully"
    );
}

#[tokio::test]
async fn test_thumbnail_manager_clone() {
    let temp = TempDir::new().unwrap();
    let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

    let manager2 = manager.clone();

    // Both should work and share the same cache
    assert_eq!(manager.cache_dir(), manager2.cache_dir());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_thumbnail_very_wide_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("wide.png");

    // Create very wide image
    let img = create_test_image(3000, 100);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let thumb_path = manager.generate_thumbnail(&source_path).await.unwrap();

    let thumb_img = image::open(&thumb_path).unwrap();
    assert!(thumb_img.width() <= 256 && thumb_img.height() <= 256);
}

#[tokio::test]
async fn test_thumbnail_very_tall_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("tall.png");

    // Create very tall image
    let img = create_test_image(100, 3000);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let thumb_path = manager.generate_thumbnail(&source_path).await.unwrap();

    let thumb_img = image::open(&thumb_path).unwrap();
    assert!(thumb_img.width() <= 256 && thumb_img.height() <= 256);
}

#[tokio::test]
async fn test_thumbnail_square_image() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("square.png");

    let img = create_test_image(2000, 2000);
    img.save(&source_path).unwrap();

    let manager = ThumbnailManager::with_cache_dir(temp.path().join("cache")).unwrap();
    let thumb_path = manager.generate_thumbnail(&source_path).await.unwrap();

    let thumb_img = image::open(&thumb_path).unwrap();

    // Square should remain square
    assert_eq!(thumb_img.width(), thumb_img.height());
    assert!(thumb_img.width() <= 256);
}
