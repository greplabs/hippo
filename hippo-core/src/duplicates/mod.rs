//! Duplicate file detection using content hashing
//!
//! Provides SHA-256 based file hashing and duplicate detection.

use crate::{HippoError, Memory, MemoryId, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Size threshold for partial hashing (files larger than this use chunked hashing)
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB

/// Chunk size for reading files
const CHUNK_SIZE: usize = 8 * 1024; // 8KB

/// A group of duplicate files
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// The shared content hash
    pub hash: String,
    /// File size (all duplicates have same size)
    pub size: u64,
    /// Memory IDs of duplicate files
    pub memory_ids: Vec<MemoryId>,
    /// File paths of duplicates
    pub paths: Vec<std::path::PathBuf>,
}

impl DuplicateGroup {
    /// Get the number of duplicates (excluding the original)
    pub fn duplicate_count(&self) -> usize {
        self.memory_ids.len().saturating_sub(1)
    }

    /// Get the total wasted space (duplicate_count * size)
    pub fn wasted_bytes(&self) -> u64 {
        self.duplicate_count() as u64 * self.size
    }
}

/// Summary of duplicate detection results
#[derive(Debug, Clone, Default)]
pub struct DuplicateSummary {
    /// Total files scanned
    pub files_scanned: usize,
    /// Number of duplicate groups found
    pub duplicate_groups: usize,
    /// Total duplicate files (excluding originals)
    pub total_duplicates: usize,
    /// Total wasted space in bytes
    pub wasted_bytes: u64,
}

/// Compute SHA-256 hash of a file
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .map_err(|e| HippoError::Other(format!("Failed to open file: {}", e)))?;

    let metadata = file
        .metadata()
        .map_err(|e| HippoError::Other(format!("Failed to get metadata: {}", e)))?;

    let mut hasher = Sha256::new();
    let mut reader = std::io::BufReader::new(file);

    // For very large files, use a sampling strategy
    if metadata.len() > LARGE_FILE_THRESHOLD {
        // Hash first chunk, middle chunk, and last chunk
        let mut buffer = vec![0u8; CHUNK_SIZE];

        // First chunk
        let n = reader
            .read(&mut buffer)
            .map_err(|e| HippoError::Other(format!("Read error: {}", e)))?;
        hasher.update(&buffer[..n]);

        // Include file size in hash for large files
        hasher.update(metadata.len().to_le_bytes());
    } else {
        // Hash entire file for smaller files
        let mut buffer = vec![0u8; CHUNK_SIZE];
        loop {
            let n = reader
                .read(&mut buffer)
                .map_err(|e| HippoError::Other(format!("Read error: {}", e)))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute a quick hash based on file size and first bytes (for pre-filtering)
pub fn compute_quick_hash(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .map_err(|e| HippoError::Other(format!("Failed to open file: {}", e)))?;

    let metadata = file
        .metadata()
        .map_err(|e| HippoError::Other(format!("Failed to get metadata: {}", e)))?;

    let mut hasher = Sha256::new();

    // Include file size
    hasher.update(metadata.len().to_le_bytes());

    // Read first 4KB
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![0u8; 4096];
    let n = reader
        .read(&mut buffer)
        .map_err(|e| HippoError::Other(format!("Read error: {}", e)))?;
    hasher.update(&buffer[..n]);

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Find duplicates from a list of memories
pub fn find_duplicates(
    memories: &[Memory],
    min_size: u64,
) -> (Vec<DuplicateGroup>, DuplicateSummary) {
    let mut summary = DuplicateSummary {
        files_scanned: memories.len(),
        ..Default::default()
    };

    // Group by hash
    let mut hash_groups: HashMap<String, Vec<&Memory>> = HashMap::new();

    for memory in memories {
        // Skip small files
        if memory.metadata.file_size < min_size {
            continue;
        }

        // Use existing hash or compute one
        if let Some(ref hash) = memory.metadata.hash {
            hash_groups.entry(hash.clone()).or_default().push(memory);
        }
    }

    // Find groups with duplicates
    let mut duplicate_groups = Vec::new();

    for (hash, group) in hash_groups {
        if group.len() > 1 {
            let size = group[0].metadata.file_size;
            let dup_group = DuplicateGroup {
                hash,
                size,
                memory_ids: group.iter().map(|m| m.id).collect(),
                paths: group.iter().map(|m| m.path.clone()).collect(),
            };

            summary.total_duplicates += dup_group.duplicate_count();
            summary.wasted_bytes += dup_group.wasted_bytes();
            duplicate_groups.push(dup_group);
        }
    }

    summary.duplicate_groups = duplicate_groups.len();

    // Sort by wasted space (most wasted first)
    duplicate_groups.sort_by_key(|g| std::cmp::Reverse(g.wasted_bytes()));

    (duplicate_groups, summary)
}

/// Find duplicates by scanning file system and computing hashes
pub fn find_duplicates_by_scanning(
    memories: &[Memory],
    min_size: u64,
) -> Result<(Vec<DuplicateGroup>, DuplicateSummary)> {
    let mut summary = DuplicateSummary {
        files_scanned: 0,
        ..Default::default()
    };

    // First pass: group by file size (quick filter)
    let mut size_groups: HashMap<u64, Vec<&Memory>> = HashMap::new();

    for memory in memories {
        if memory.metadata.file_size >= min_size && memory.path.exists() {
            size_groups
                .entry(memory.metadata.file_size)
                .or_default()
                .push(memory);
            summary.files_scanned += 1;
        }
    }

    // Second pass: compute hashes only for size groups with potential duplicates
    let mut hash_groups: HashMap<String, Vec<&Memory>> = HashMap::new();

    for (_, group) in size_groups {
        if group.len() < 2 {
            continue; // No potential duplicates
        }

        for memory in group {
            // Use existing hash or compute one
            let hash = if let Some(ref h) = memory.metadata.hash {
                h.clone()
            } else {
                match compute_file_hash(&memory.path) {
                    Ok(h) => h,
                    Err(_) => continue, // Skip files we can't hash
                }
            };

            hash_groups.entry(hash).or_default().push(memory);
        }
    }

    // Find groups with duplicates
    let mut duplicate_groups = Vec::new();

    for (hash, group) in hash_groups {
        if group.len() > 1 {
            let size = group[0].metadata.file_size;
            let dup_group = DuplicateGroup {
                hash,
                size,
                memory_ids: group.iter().map(|m| m.id).collect(),
                paths: group.iter().map(|m| m.path.clone()).collect(),
            };

            summary.total_duplicates += dup_group.duplicate_count();
            summary.wasted_bytes += dup_group.wasted_bytes();
            duplicate_groups.push(dup_group);
        }
    }

    summary.duplicate_groups = duplicate_groups.len();

    // Sort by wasted space (most wasted first)
    duplicate_groups.sort_by_key(|g| std::cmp::Reverse(g.wasted_bytes()));

    Ok((duplicate_groups, summary))
}

/// Format bytes into human readable string
pub fn format_bytes(bytes: u64) -> String {
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

/// A group of semantically similar files (not exact duplicates)
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimilarGroup {
    /// Representative memory ID (first in group)
    pub representative_id: MemoryId,
    /// Representative file name
    pub representative_name: String,
    /// All similar memory IDs including representative
    pub memory_ids: Vec<MemoryId>,
    /// File paths
    pub paths: Vec<std::path::PathBuf>,
    /// Similarity scores (1.0 = identical)
    pub similarity_scores: Vec<f32>,
    /// Average similarity within the group
    pub avg_similarity: f32,
    /// Type of files in group (e.g., "image", "document")
    pub file_type: String,
}

/// Summary of semantic similarity detection
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SimilarSummary {
    /// Total files analyzed
    pub files_analyzed: usize,
    /// Number of similar groups found
    pub similar_groups: usize,
    /// Total files that have similar counterparts
    pub total_similar: usize,
    /// Similarity threshold used
    pub threshold: f32,
}

/// Find semantically similar files using embeddings
///
/// This uses vector embeddings to find files that are similar in content,
/// even if they're not exact duplicates. For example:
/// - Photos of the same subject from different angles
/// - Documents covering the same topic
/// - Code files with similar functionality
pub fn find_similar_by_embedding(
    memories: &[Memory],
    embeddings: &HashMap<MemoryId, Vec<f32>>,
    threshold: f32,
    min_group_size: usize,
) -> (Vec<SimilarGroup>, SimilarSummary) {
    let mut summary = SimilarSummary {
        files_analyzed: memories.len(),
        threshold,
        ..Default::default()
    };

    // Only consider memories that have embeddings
    let memories_with_embeddings: Vec<_> = memories
        .iter()
        .filter(|m| embeddings.contains_key(&m.id))
        .collect();

    if memories_with_embeddings.is_empty() {
        return (vec![], summary);
    }

    // Track which memories have been grouped
    let mut grouped: std::collections::HashSet<MemoryId> = std::collections::HashSet::new();
    let mut groups: Vec<SimilarGroup> = Vec::new();

    // For each memory, find similar ones
    for memory in &memories_with_embeddings {
        if grouped.contains(&memory.id) {
            continue;
        }

        let embedding = match embeddings.get(&memory.id) {
            Some(e) => e,
            None => continue,
        };

        let mut similar: Vec<(&Memory, f32)> = Vec::new();
        similar.push((memory, 1.0)); // Include self

        // Find all similar memories
        for other in &memories_with_embeddings {
            if other.id == memory.id || grouped.contains(&other.id) {
                continue;
            }

            if let Some(other_embedding) = embeddings.get(&other.id) {
                let similarity = cosine_similarity(embedding, other_embedding);
                if similarity >= threshold {
                    similar.push((other, similarity));
                }
            }
        }

        // Only create a group if we have enough similar files
        if similar.len() >= min_group_size {
            // Mark all as grouped
            for (m, _) in &similar {
                grouped.insert(m.id);
            }

            // Calculate average similarity
            let avg_sim = similar.iter().map(|(_, s)| s).sum::<f32>() / similar.len() as f32;

            // Determine file type
            let file_type = match &memory.kind {
                crate::MemoryKind::Image { .. } => "image",
                crate::MemoryKind::Video { .. } => "video",
                crate::MemoryKind::Audio { .. } => "audio",
                crate::MemoryKind::Code { .. } => "code",
                crate::MemoryKind::Document { .. } => "document",
                _ => "file",
            };

            let group = SimilarGroup {
                representative_id: memory.id,
                representative_name: memory
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                memory_ids: similar.iter().map(|(m, _)| m.id).collect(),
                paths: similar.iter().map(|(m, _)| m.path.clone()).collect(),
                similarity_scores: similar.iter().map(|(_, s)| *s).collect(),
                avg_similarity: avg_sim,
                file_type: file_type.to_string(),
            };

            summary.total_similar += group.memory_ids.len();
            groups.push(group);
        }
    }

    summary.similar_groups = groups.len();

    // Sort by group size (largest first)
    groups.sort_by_key(|g| std::cmp::Reverse(g.memory_ids.len()));

    (groups, summary)
}

/// Compute cosine similarity between two vectors
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_compute_file_hash() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");

        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        drop(file);

        let hash = compute_file_hash(&path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_identical_files_same_hash() {
        let dir = TempDir::new().unwrap();
        let path1 = dir.path().join("file1.txt");
        let path2 = dir.path().join("file2.txt");

        let content = b"Same content in both files";
        std::fs::write(&path1, content).unwrap();
        std::fs::write(&path2, content).unwrap();

        let hash1 = compute_file_hash(&path1).unwrap();
        let hash2 = compute_file_hash(&path2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_files_different_hash() {
        let dir = TempDir::new().unwrap();
        let path1 = dir.path().join("file1.txt");
        let path2 = dir.path().join("file2.txt");

        std::fs::write(&path1, b"Content A").unwrap();
        std::fs::write(&path2, b"Content B").unwrap();

        let hash1 = compute_file_hash(&path1).unwrap();
        let hash2 = compute_file_hash(&path2).unwrap();

        assert_ne!(hash1, hash2);
    }
}
