//! Thumbnail generation and caching for fast image previews

use crate::error::{HippoError, Result};
use image::{DynamicImage, ImageFormat};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Default thumbnail size (width and height)
pub const THUMBNAIL_SIZE: u32 = 256;

/// Thumbnail manager for generating and caching image thumbnails
pub struct ThumbnailManager {
    cache_dir: PathBuf,
    size: u32,
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
        })
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
                if let (Ok(src_time), Ok(thumb_time)) =
                    (src_meta.modified(), thumb_meta.modified())
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
            for entry in fs::read_dir(&self.cache_dir)? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Err(e) = fs::remove_file(&path) {
                            warn!("Failed to delete thumbnail {:?}: {}", path, e);
                        }
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
            for entry in fs::read_dir(&self.cache_dir)? {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        count += 1;
                        if let Ok(meta) = entry.metadata() {
                            total_size += meta.len();
                        }
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
}

impl Default for ThumbnailManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ThumbnailManager")
    }
}

/// Statistics about the thumbnail cache
#[derive(Debug, Clone)]
pub struct ThumbnailStats {
    pub count: usize,
    pub total_size: u64,
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
}
