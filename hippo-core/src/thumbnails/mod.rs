//! Thumbnail generation and caching for fast image and video previews
//!
//! Features:
//! - Disk caching for persistence across restarts
//! - In-memory LRU cache for hot thumbnails
//! - Automatic cache eviction when memory limit reached
//! - Support for images and videos (via ffmpeg)

use crate::error::{HippoError, Result};
use image::{DynamicImage, ImageFormat};
use lru::LruCache;
use parking_lot::Mutex;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, warn};

/// Default thumbnail size (width and height)
pub const THUMBNAIL_SIZE: u32 = 256;

/// Default LRU cache capacity (number of thumbnails to keep in memory)
pub const DEFAULT_CACHE_CAPACITY: usize = 500;

/// Maximum memory usage for in-memory cache (50MB)
pub const MAX_CACHE_MEMORY_BYTES: usize = 50 * 1024 * 1024;

/// Thumbnail manager for generating and caching image thumbnails
pub struct ThumbnailManager {
    cache_dir: PathBuf,
    size: u32,
    /// In-memory LRU cache for recently accessed thumbnails
    memory_cache: Mutex<LruCache<String, CachedThumbnail>>,
    /// Current memory usage of the cache
    cache_memory_usage: Mutex<usize>,
}

/// A cached thumbnail with its data
#[derive(Clone)]
struct CachedThumbnail {
    data: Vec<u8>,
}

impl ThumbnailManager {
    /// Create a new thumbnail manager with the default cache directory
    pub fn new() -> Result<Self> {
        let cache_dir = directories::ProjectDirs::from("com", "hippo", "Hippo")
            .map(|dirs| dirs.cache_dir().join("thumbnails"))
            .unwrap_or_else(|| PathBuf::from(".hippo_cache/thumbnails"));

        Self::with_cache_dir(cache_dir)
    }

    /// Create a new thumbnail manager with a custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        Ok(Self {
            cache_dir,
            size: THUMBNAIL_SIZE,
            memory_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(DEFAULT_CACHE_CAPACITY).unwrap(),
            )),
            cache_memory_usage: Mutex::new(0),
        })
    }

    /// Get thumbnail data from memory cache or disk
    pub fn get_thumbnail_data(&self, image_path: &Path) -> Result<Vec<u8>> {
        let cache_key = self.get_thumbnail_name(image_path);

        // Check memory cache first
        {
            let mut cache = self.memory_cache.lock();
            if let Some(cached) = cache.get(&cache_key) {
                debug!("Thumbnail cache hit (memory): {}", cache_key);
                return Ok(cached.data.clone());
            }
        }

        // Not in memory, check disk cache
        let thumbnail_path = self.cache_dir.join(&cache_key);
        if thumbnail_path.exists() {
            let data = fs::read(&thumbnail_path)?;
            self.add_to_memory_cache(cache_key, data.clone());
            debug!("Thumbnail cache hit (disk): {:?}", thumbnail_path);
            return Ok(data);
        }

        // Need to generate
        let generated_path = self.generate_thumbnail(image_path)?;
        let data = fs::read(&generated_path)?;
        self.add_to_memory_cache(self.get_thumbnail_name(image_path), data.clone());
        Ok(data)
    }

    /// Add a thumbnail to the memory cache with memory limit enforcement
    fn add_to_memory_cache(&self, key: String, data: Vec<u8>) {
        let data_size = data.len();

        // Check if adding this would exceed memory limit
        let mut memory_usage = self.cache_memory_usage.lock();
        let mut cache = self.memory_cache.lock();

        // Evict entries if necessary to stay under memory limit
        while *memory_usage + data_size > MAX_CACHE_MEMORY_BYTES && !cache.is_empty() {
            if let Some((_, evicted)) = cache.pop_lru() {
                *memory_usage = memory_usage.saturating_sub(evicted.data.len());
            }
        }

        // Add to cache
        if let Some(old) = cache.put(key, CachedThumbnail { data }) {
            *memory_usage = memory_usage.saturating_sub(old.data.len());
        }
        *memory_usage += data_size;
    }

    /// Clear the in-memory cache
    pub fn clear_memory_cache(&self) {
        let mut cache = self.memory_cache.lock();
        let mut memory_usage = self.cache_memory_usage.lock();
        cache.clear();
        *memory_usage = 0;
        debug!("Memory cache cleared");
    }

    /// Get memory cache statistics
    pub fn memory_cache_stats(&self) -> MemoryCacheStats {
        let cache = self.memory_cache.lock();
        let memory_usage = self.cache_memory_usage.lock();
        MemoryCacheStats {
            entries: cache.len(),
            memory_bytes: *memory_usage,
            capacity: DEFAULT_CACHE_CAPACITY,
            max_memory_bytes: MAX_CACHE_MEMORY_BYTES,
        }
    }

    /// Set the thumbnail size
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Generate a thumbnail for an image file
    /// Returns the path to the thumbnail if successful
    pub fn generate_thumbnail(&self, image_path: &Path) -> Result<PathBuf> {
        // Generate a unique filename based on the original path
        let thumbnail_name = self.get_thumbnail_name(image_path);
        let thumbnail_path = self.cache_dir.join(&thumbnail_name);

        // Check if thumbnail already exists and is newer than source
        if thumbnail_path.exists() {
            if let (Ok(src_meta), Ok(thumb_meta)) =
                (fs::metadata(image_path), fs::metadata(&thumbnail_path))
            {
                if let (Ok(src_time), Ok(thumb_time)) = (src_meta.modified(), thumb_meta.modified())
                {
                    if thumb_time >= src_time {
                        debug!("Using cached thumbnail: {:?}", thumbnail_path);
                        return Ok(thumbnail_path);
                    }
                }
            }
        }

        // Load and resize the image
        debug!("Generating thumbnail for: {:?}", image_path);
        let img = image::open(image_path).map_err(|e| {
            HippoError::Other(format!("Failed to open image {:?}: {}", image_path, e))
        })?;

        // Create thumbnail (maintains aspect ratio)
        let thumbnail = self.create_thumbnail(&img);

        // Save thumbnail as JPEG for consistent format and good compression
        let mut output = fs::File::create(&thumbnail_path)?;

        thumbnail
            .write_to(&mut output, ImageFormat::Jpeg)
            .map_err(|e| HippoError::Other(format!("Failed to save thumbnail: {}", e)))?;

        debug!("Thumbnail saved: {:?}", thumbnail_path);
        Ok(thumbnail_path)
    }

    /// Get the thumbnail path for an image (without generating)
    pub fn get_thumbnail_path(&self, image_path: &Path) -> PathBuf {
        let thumbnail_name = self.get_thumbnail_name(image_path);
        self.cache_dir.join(thumbnail_name)
    }

    /// Check if a thumbnail exists for the given image
    pub fn has_thumbnail(&self, image_path: &Path) -> bool {
        self.get_thumbnail_path(image_path).exists()
    }

    /// Delete a thumbnail for the given image
    pub fn delete_thumbnail(&self, image_path: &Path) -> Result<()> {
        let thumbnail_path = self.get_thumbnail_path(image_path);
        if thumbnail_path.exists() {
            fs::remove_file(&thumbnail_path)?;
        }
        Ok(())
    }

    /// Clear all thumbnails from the cache
    pub fn clear_cache(&self) -> Result<()> {
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)?.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Err(e) = fs::remove_file(&path) {
                        warn!("Failed to delete thumbnail {:?}: {}", path, e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<ThumbnailStats> {
        let mut count = 0;
        let mut total_size = 0;

        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)?.flatten() {
                if entry.path().is_file() {
                    count += 1;
                    if let Ok(meta) = entry.metadata() {
                        total_size += meta.len();
                    }
                }
            }
        }

        Ok(ThumbnailStats { count, total_size })
    }

    /// Generate a unique thumbnail filename from the original path
    fn get_thumbnail_name(&self, image_path: &Path) -> String {
        use sha2::{Digest, Sha256};

        // Hash the full path to create a unique filename
        let path_str = image_path.to_string_lossy();
        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(&hash[..16]); // Use first 16 bytes (32 hex chars)

        format!("{}.jpg", hash_hex)
    }

    /// Create a thumbnail from a DynamicImage
    fn create_thumbnail(&self, img: &DynamicImage) -> DynamicImage {
        // Use thumbnail method which maintains aspect ratio
        img.thumbnail(self.size, self.size)
    }

    /// Generate a thumbnail for a video file by extracting a frame
    /// Returns the path to the thumbnail if successful
    pub fn generate_video_thumbnail(&self, video_path: &Path) -> Result<PathBuf> {
        // Check if ffmpeg is available
        if !is_ffmpeg_available() {
            return Err(HippoError::Other(
                "ffmpeg not available - cannot generate video thumbnails".to_string(),
            ));
        }

        // Generate a unique filename based on the original path
        let thumbnail_name = self.get_thumbnail_name(video_path);
        let thumbnail_path = self.cache_dir.join(&thumbnail_name);

        // Check if thumbnail already exists and is newer than source
        if thumbnail_path.exists() {
            if let (Ok(src_meta), Ok(thumb_meta)) =
                (fs::metadata(video_path), fs::metadata(&thumbnail_path))
            {
                if let (Ok(src_time), Ok(thumb_time)) = (src_meta.modified(), thumb_meta.modified())
                {
                    if thumb_time >= src_time {
                        debug!("Using cached video thumbnail: {:?}", thumbnail_path);
                        return Ok(thumbnail_path);
                    }
                }
            }
        }

        debug!("Generating video thumbnail for: {:?}", video_path);

        // Try to extract frame at 2 seconds, fall back to 0 if video is short
        let seek_times = ["2", "1", "0"];

        for seek_time in seek_times {
            let output = Command::new("ffmpeg")
                .args([
                    "-y", // Overwrite output
                    "-ss", seek_time, // Seek to timestamp
                    "-i",
                ])
                .arg(video_path)
                .args([
                    "-vframes",
                    "1", // Extract single frame
                    "-vf",
                    &format!(
                        "scale={}:{}:force_original_aspect_ratio=decrease",
                        self.size, self.size
                    ),
                    "-f",
                    "image2", // Output format
                ])
                .arg(&thumbnail_path)
                .output();

            match output {
                Ok(result) if result.status.success() && thumbnail_path.exists() => {
                    debug!("Video thumbnail saved: {:?}", thumbnail_path);
                    return Ok(thumbnail_path);
                }
                Ok(result) => {
                    debug!(
                        "ffmpeg failed at seek={}: {}",
                        seek_time,
                        String::from_utf8_lossy(&result.stderr)
                    );
                    // Try next seek time
                }
                Err(e) => {
                    warn!("Failed to run ffmpeg: {}", e);
                    break;
                }
            }
        }

        Err(HippoError::Other(format!(
            "Failed to generate video thumbnail for {:?}",
            video_path
        )))
    }
}

impl Default for ThumbnailManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ThumbnailManager")
    }
}

/// Statistics about the thumbnail disk cache
#[derive(Debug, Clone)]
pub struct ThumbnailStats {
    pub count: usize,
    pub total_size: u64,
}

/// Statistics about the in-memory thumbnail cache
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryCacheStats {
    pub entries: usize,
    pub memory_bytes: usize,
    pub capacity: usize,
    pub max_memory_bytes: usize,
}

/// Helper function to encode bytes as hex
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Check if a file is a supported image format for thumbnails
pub fn is_supported_image(path: &Path) -> bool {
    let supported_extensions = [
        "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if a file is a supported video format for thumbnails
pub fn is_supported_video(path: &Path) -> bool {
    let supported_extensions = ["mp4", "mov", "avi", "mkv", "webm", "m4v"];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if ffmpeg is available on the system
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_thumbnail_name_generation() {
        let temp = TempDir::new().unwrap();
        let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

        let name1 = manager.get_thumbnail_name(Path::new("/path/to/image1.jpg"));
        let name2 = manager.get_thumbnail_name(Path::new("/path/to/image2.jpg"));
        let name3 = manager.get_thumbnail_name(Path::new("/path/to/image1.jpg"));

        // Different paths should have different names
        assert_ne!(name1, name2);
        // Same path should have same name
        assert_eq!(name1, name3);
        // All should end with .jpg
        assert!(name1.ends_with(".jpg"));
    }

    #[test]
    fn test_is_supported_image() {
        assert!(is_supported_image(Path::new("test.jpg")));
        assert!(is_supported_image(Path::new("test.PNG")));
        assert!(is_supported_image(Path::new("test.webp")));
        assert!(!is_supported_image(Path::new("test.txt")));
        assert!(!is_supported_image(Path::new("test.mp4")));
    }

    #[test]
    fn test_is_supported_video() {
        assert!(is_supported_video(Path::new("test.mp4")));
        assert!(is_supported_video(Path::new("test.MOV")));
        assert!(is_supported_video(Path::new("test.mkv")));
        assert!(is_supported_video(Path::new("test.webm")));
        assert!(!is_supported_video(Path::new("test.jpg")));
        assert!(!is_supported_video(Path::new("test.txt")));
    }
}
