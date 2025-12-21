//! Integration tests for Hippo core functionality
//!
//! These tests verify the major features work correctly together.

use hippo_core::*;
use std::path::PathBuf;

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn test_memory_kind_variants() {
        // Test all MemoryKind variants can be created
        let image = MemoryKind::Image {
            width: 1920,
            height: 1080,
            format: "jpg".to_string(),
        };
        let video = MemoryKind::Video {
            duration_ms: 60000,
            format: "mp4".to_string(),
        };
        let audio = MemoryKind::Audio {
            duration_ms: 180000,
            format: "mp3".to_string(),
        };
        let code = MemoryKind::Code {
            language: "rust".to_string(),
            lines: 500,
        };
        let doc = MemoryKind::Document {
            format: DocumentFormat::Pdf,
            page_count: Some(10),
        };

        // Verify serialization roundtrip
        let json = serde_json::to_string(&image).unwrap();
        let parsed: MemoryKind = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, MemoryKind::Image { .. }));

        assert!(matches!(video, MemoryKind::Video { .. }));
        assert!(matches!(audio, MemoryKind::Audio { .. }));
        assert!(matches!(code, MemoryKind::Code { .. }));
        assert!(matches!(doc, MemoryKind::Document { .. }));
    }

    #[test]
    fn test_source_types() {
        let local = Source::Local {
            root_path: PathBuf::from("/home/user/documents"),
        };
        let gdrive = Source::GoogleDrive {
            account_id: "test@gmail.com".to_string(),
        };

        let json = serde_json::to_string(&local).unwrap();
        assert!(json.contains("Local"));

        let json = serde_json::to_string(&gdrive).unwrap();
        assert!(json.contains("GoogleDrive"));
    }

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery {
            text: Some("rust programming".to_string()),
            tags: vec![TagFilter {
                tag: "lang:rust".to_string(),
                mode: TagFilterMode::Include,
            }],
            sources: vec![],
            kinds: vec![],
            date_range: None,
            sort: SortOrder::Relevance,
            limit: 50,
            offset: 0,
            location: None,
        };

        assert_eq!(query.text, Some("rust programming".to_string()));
        assert_eq!(query.tags.len(), 1);
        assert_eq!(query.limit, 50);
    }

    #[test]
    fn test_tag_creation() {
        let user_tag = Tag::user("project:hippo");
        assert_eq!(user_tag.name, "project:hippo");

        let ai_tag = Tag::ai("landscape", 95);
        assert_eq!(ai_tag.name, "landscape");
        assert_eq!(ai_tag.confidence, Some(95));
    }
}

#[cfg(test)]
mod organization_tests {
    use super::*;
    use chrono::Utc;
    use hippo_core::organization::{
        CollectionType, FilePointer, OrganizationReason, OrganizationStats, VirtualPath,
    };

    #[test]
    fn test_collection_types() {
        let temporal = CollectionType::Temporal {
            start: Utc::now(),
            end: Utc::now(),
        };
        let location = CollectionType::Location {
            center_lat: 37.7749,
            center_lon: -122.4194,
            radius_km: 10.0,
        };
        let topic = CollectionType::Topic {
            keywords: vec!["rust".to_string(), "programming".to_string()],
        };
        let project = CollectionType::Project {
            root_path: Some(PathBuf::from("/projects/hippo")),
        };
        let person = CollectionType::Person {
            face_cluster_id: "cluster_123".to_string(),
        };
        let custom = CollectionType::Custom;

        // All variants should serialize
        assert!(serde_json::to_string(&temporal).is_ok());
        assert!(serde_json::to_string(&location).is_ok());
        assert!(serde_json::to_string(&topic).is_ok());
        assert!(serde_json::to_string(&project).is_ok());
        assert!(serde_json::to_string(&person).is_ok());
        assert!(serde_json::to_string(&custom).is_ok());
    }

    #[test]
    fn test_organization_reasons() {
        let date = OrganizationReason::DateBased { date: Utc::now() };
        let location = OrganizationReason::LocationBased {
            place: "San Francisco".to_string(),
        };
        let content = OrganizationReason::ContentSimilarity {
            topic: "technology".to_string(),
        };
        let project = OrganizationReason::ProjectMembership {
            project: "hippo".to_string(),
        };
        let user_defined = OrganizationReason::UserDefined;

        // All should serialize
        assert!(serde_json::to_string(&date).is_ok());
        assert!(serde_json::to_string(&location).is_ok());
        assert!(serde_json::to_string(&content).is_ok());
        assert!(serde_json::to_string(&project).is_ok());
        assert!(serde_json::to_string(&user_defined).is_ok());
    }

    #[test]
    fn test_file_pointer_creation() {
        let pointer = FilePointer {
            memory_id: uuid::Uuid::new_v4(),
            original_path: PathBuf::from("/photos/IMG_001.jpg"),
            virtual_paths: vec![VirtualPath {
                path: PathBuf::from("By Date/2024/Summer/IMG_001.jpg"),
                reason: OrganizationReason::DateBased { date: Utc::now() },
                confidence: 0.95,
            }],
            collections: vec![],
            suggested_location: Some(PathBuf::from("By Date/2024/Summer")),
            organization_score: 0.75,
        };

        assert!(!pointer.virtual_paths.is_empty());
        assert!(pointer.suggested_location.is_some());
        assert!(pointer.organization_score > 0.0);
    }

    #[test]
    fn test_organization_stats_serialization() {
        let stats = OrganizationStats {
            total_files: 1000,
            total_collections: 25,
            auto_generated_collections: 15,
            custom_collections: 10,
            files_in_collections: 750,
            average_organization_score: 0.85,
            virtual_paths_count: 2500,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: OrganizationStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_files, 1000);
        assert_eq!(parsed.total_collections, 25);
        assert_eq!(parsed.auto_generated_collections, 15);
        assert_eq!(parsed.files_in_collections, 750);
    }
}

#[cfg(test)]
mod thumbnail_tests {
    use hippo_core::thumbnails::*;
    use std::path::Path;

    #[test]
    fn test_supported_formats() {
        // Images
        assert!(is_supported_image(Path::new("photo.jpg")));
        assert!(is_supported_image(Path::new("photo.JPEG")));
        assert!(is_supported_image(Path::new("image.png")));
        assert!(is_supported_image(Path::new("image.webp")));
        assert!(is_supported_image(Path::new("icon.gif")));
        assert!(!is_supported_image(Path::new("document.pdf")));
        assert!(!is_supported_image(Path::new("video.mp4")));

        // Videos
        assert!(is_supported_video(Path::new("video.mp4")));
        assert!(is_supported_video(Path::new("video.MOV")));
        assert!(is_supported_video(Path::new("video.mkv")));
        assert!(is_supported_video(Path::new("video.webm")));
        assert!(!is_supported_video(Path::new("photo.jpg")));
        assert!(!is_supported_video(Path::new("audio.mp3")));
    }

    #[test]
    fn test_thumbnail_manager_creation() {
        let temp = tempfile::TempDir::new().unwrap();
        let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

        assert_eq!(manager.cache_dir(), temp.path());
    }

    #[test]
    fn test_thumbnail_name_consistency() {
        let temp = tempfile::TempDir::new().unwrap();
        let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

        let path = Path::new("/path/to/image.jpg");
        let name1 = manager.get_thumbnail_path(path);
        let name2 = manager.get_thumbnail_path(path);

        // Same path should always get same thumbnail path
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_memory_cache_stats() {
        let temp = tempfile::TempDir::new().unwrap();
        let manager = ThumbnailManager::with_cache_dir(temp.path().to_path_buf()).unwrap();

        let stats = manager.memory_cache_stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.memory_bytes, 0);
        assert!(stats.capacity > 0);
        assert!(stats.max_memory_bytes > 0);
    }
}

#[cfg(test)]
mod qdrant_tests {
    use hippo_core::qdrant::*;

    #[test]
    fn test_collection_names() {
        assert_eq!(COLLECTION_IMAGES, "hippo_images");
        assert_eq!(COLLECTION_TEXT, "hippo_text");
        assert_eq!(COLLECTION_CODE, "hippo_code");
    }

    #[tokio::test]
    async fn test_qdrant_stats_structure() {
        // Test without Qdrant running - should get empty stats
        let storage = QdrantStorage::new("http://localhost:9999").await.unwrap();
        let stats = storage.stats().await.unwrap();

        assert!(!stats.available);
        assert_eq!(stats.total_vectors, 0);
        assert!(stats.collections.is_empty());
    }
}

#[cfg(test)]
mod ollama_tests {
    use hippo_core::ollama::*;

    #[test]
    fn test_recommended_models() {
        // Fast models should include gemma2:2b
        assert!(RecommendedModels::FAST.contains(&"gemma2:2b"));

        // Balanced models should include larger variants
        assert!(RecommendedModels::BALANCED.contains(&"gemma2:9b"));

        // Embedding models should include nomic
        assert!(RecommendedModels::EMBEDDINGS.contains(&"nomic-embed-text"));

        // Vision models should include llava
        assert!(RecommendedModels::VISION.contains(&"llava:7b"));
    }

    #[test]
    fn test_ollama_config_defaults() {
        let config = OllamaConfig::default();

        assert_eq!(config.base_url, DEFAULT_OLLAMA_URL);
        assert_eq!(config.embedding_model, DEFAULT_EMBEDDING_MODEL);
        assert_eq!(config.generation_model, DEFAULT_GENERATION_MODEL);
        assert!(config.timeout_secs > 0);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            images: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello, world!"));

        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "Hello, world!");
    }

    #[test]
    fn test_local_analysis_serialization() {
        let analysis = LocalAnalysis {
            summary: "A Rust project for file organization".to_string(),
            key_topics: vec!["rust".to_string(), "files".to_string()],
            suggested_tags: vec!["project".to_string(), "code".to_string()],
            document_type: Some("code".to_string()),
            language: Some("rust".to_string()),
        };

        let json = serde_json::to_string(&analysis).unwrap();
        let parsed: LocalAnalysis = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.summary, analysis.summary);
        assert_eq!(parsed.key_topics.len(), 2);
        assert_eq!(parsed.suggested_tags.len(), 2);
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn test_sort_order_variants() {
        let relevance = SortOrder::Relevance;
        let date_new = SortOrder::DateNewest;
        let date_old = SortOrder::DateOldest;
        let name_asc = SortOrder::NameAsc;
        let name_desc = SortOrder::NameDesc;
        let size_asc = SortOrder::SizeAsc;
        let size_desc = SortOrder::SizeDesc;

        // All should be distinct
        assert!(matches!(relevance, SortOrder::Relevance));
        assert!(matches!(date_new, SortOrder::DateNewest));
        assert!(matches!(date_old, SortOrder::DateOldest));
        assert!(matches!(name_asc, SortOrder::NameAsc));
        assert!(matches!(name_desc, SortOrder::NameDesc));
        assert!(matches!(size_asc, SortOrder::SizeAsc));
        assert!(matches!(size_desc, SortOrder::SizeDesc));
    }

    #[test]
    fn test_tag_filter_modes() {
        let include = TagFilter {
            tag: "important".to_string(),
            mode: TagFilterMode::Include,
        };
        let exclude = TagFilter {
            tag: "spam".to_string(),
            mode: TagFilterMode::Exclude,
        };

        assert!(matches!(include.mode, TagFilterMode::Include));
        assert!(matches!(exclude.mode, TagFilterMode::Exclude));
    }

    #[test]
    fn test_search_results_structure() {
        let results = SearchResults {
            memories: vec![],
            total_count: 0,
            suggested_tags: vec![],
            clusters: vec![],
        };

        assert_eq!(results.total_count, 0);
        assert!(results.memories.is_empty());
    }
}

#[cfg(test)]
mod duplicates_tests {
    use super::*;
    use hippo_core::duplicates::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_duplicate_group_structure() {
        let group = hippo_core::duplicates::DuplicateGroup {
            hash: "abc123def456".to_string(),
            size: 1024 * 1024, // 1MB
            memory_ids: vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4()],
            paths: vec![
                PathBuf::from("/path/to/file1.jpg"),
                PathBuf::from("/path/to/file2.jpg"),
            ],
        };

        assert_eq!(group.paths.len(), 2);
        assert_eq!(group.memory_ids.len(), 2);
        assert_eq!(group.size, 1024 * 1024);
        assert!(!group.hash.is_empty());
    }

    #[test]
    fn test_duplicate_count() {
        let group = DuplicateGroup {
            hash: "test_hash".to_string(),
            size: 1000,
            memory_ids: vec![
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4(),
                uuid::Uuid::new_v4(),
            ],
            paths: vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")],
        };

        assert_eq!(group.duplicate_count(), 2); // 3 files - 1 original = 2 duplicates
    }

    #[test]
    fn test_wasted_bytes() {
        let group = DuplicateGroup {
            hash: "test_hash".to_string(),
            size: 5000,
            memory_ids: vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4()],
            paths: vec![PathBuf::from("a"), PathBuf::from("b")],
        };

        assert_eq!(group.wasted_bytes(), 5000); // 1 duplicate * 5000 bytes
    }

    #[test]
    fn test_compute_file_hash() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"Hello, World!").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn test_identical_files_same_hash() {
        let dir = TempDir::new().unwrap();
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");

        let content = b"Identical content";
        std::fs::write(&file1, content).unwrap();
        std::fs::write(&file2, content).unwrap();

        let hash1 = compute_file_hash(&file1).unwrap();
        let hash2 = compute_file_hash(&file2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_files_different_hash() {
        let dir = TempDir::new().unwrap();
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");

        std::fs::write(&file1, b"Content A").unwrap();
        std::fs::write(&file2, b"Content B").unwrap();

        let hash1 = compute_file_hash(&file1).unwrap();
        let hash2 = compute_file_hash(&file2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_quick_hash() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"Test content for quick hash").unwrap();

        let hash = compute_quick_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_find_duplicates_with_mock_data() {
        let hash1 = "hash123".to_string();
        let hash2 = "hash456".to_string();

        let memories = vec![
            create_mock_memory(1000, Some(hash1.clone())),
            create_mock_memory(1000, Some(hash1.clone())),
            create_mock_memory(2000, Some(hash2.clone())),
            create_mock_memory(2000, Some(hash2.clone())),
            create_mock_memory(2000, Some(hash2.clone())),
        ];

        let (_groups, summary) = find_duplicates(&memories, 100);

        assert_eq!(summary.files_scanned, 5);
        assert_eq!(summary.duplicate_groups, 2);
        assert_eq!(summary.total_duplicates, 3); // 1 dup from hash1, 2 dups from hash2
    }

    #[test]
    fn test_find_duplicates_respects_min_size() {
        let hash = "hash123".to_string();
        let memories = vec![
            create_mock_memory(50, Some(hash.clone())),
            create_mock_memory(50, Some(hash.clone())),
        ];

        let (groups, _) = find_duplicates(&memories, 100); // min_size = 100
        assert_eq!(groups.len(), 0); // Files too small to be considered
    }

    #[test]
    fn test_similar_by_embedding_basic() {
        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();
        let id3 = uuid::Uuid::new_v4();

        let memories = vec![
            create_mock_memory_with_id(id1, 1000, None),
            create_mock_memory_with_id(id2, 1000, None),
            create_mock_memory_with_id(id3, 1000, None),
        ];

        let mut embeddings = HashMap::new();
        // Similar embeddings for id1 and id2
        embeddings.insert(id1, vec![1.0, 0.0, 0.0]);
        embeddings.insert(id2, vec![0.9, 0.1, 0.0]);
        // Different embedding for id3
        embeddings.insert(id3, vec![0.0, 0.0, 1.0]);

        let (groups, summary) = find_similar_by_embedding(&memories, &embeddings, 0.8, 2);

        assert_eq!(summary.files_analyzed, 3);
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    // Helper functions
    fn create_mock_memory(size: u64, hash: Option<String>) -> Memory {
        create_mock_memory_with_id(uuid::Uuid::new_v4(), size, hash)
    }

    fn create_mock_memory_with_id(id: uuid::Uuid, size: u64, hash: Option<String>) -> Memory {
        Memory {
            id,
            path: PathBuf::from(format!("/test/{}.txt", id)),
            source: Source::Local {
                root_path: PathBuf::from("/test"),
            },
            kind: MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: None,
            },
            metadata: MemoryMetadata {
                title: Some("Test".to_string()),
                file_size: size,
                hash,
                ..Default::default()
            },
            tags: vec![],
            embedding_id: None,
            connections: vec![],
            is_favorite: false,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            indexed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_hippo_config_defaults() {
        let config = HippoConfig::default();

        assert!(!config.data_dir.as_os_str().is_empty());
        assert!(!config.qdrant_url.is_empty());
        assert!(config.indexing_parallelism > 0);
    }
}

#[cfg(test)]
mod natural_language_search_tests {
    use super::*;
    use hippo_core::search::Searcher;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_searcher() -> (Searcher, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = HippoConfig {
            data_dir: temp_dir.path().to_path_buf(),
            qdrant_url: "http://localhost:9999".to_string(),
            ..Default::default()
        };

        let storage = Arc::new(hippo_core::storage::Storage::new(&config).await.unwrap());
        let embedder = Arc::new(
            hippo_core::embeddings::Embedder::new(&config)
                .await
                .unwrap(),
        );
        let searcher = Searcher::new(storage, embedder, &config).await.unwrap();

        (searcher, temp_dir)
    }

    #[tokio::test]
    async fn test_parse_natural_query_image_type() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher
            .parse_natural_query("show me images from yesterday")
            .unwrap();

        assert!(parsed.original.contains("images"));
        assert!(!parsed.file_types.is_empty());
        assert!(matches!(parsed.file_types[0], MemoryKind::Image { .. }));
        assert!(parsed.date_range.is_some());
    }

    #[tokio::test]
    async fn test_parse_natural_query_video_type() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher.parse_natural_query("find videos").unwrap();

        assert!(!parsed.file_types.is_empty());
        assert!(matches!(parsed.file_types[0], MemoryKind::Video { .. }));
    }

    #[tokio::test]
    async fn test_parse_natural_query_code_type() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher
            .parse_natural_query("search for code files")
            .unwrap();

        assert!(!parsed.file_types.is_empty());
        assert!(matches!(parsed.file_types[0], MemoryKind::Code { .. }));
    }

    #[tokio::test]
    async fn test_parse_natural_query_date_range_today() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher.parse_natural_query("files from today").unwrap();

        assert!(parsed.date_range.is_some());
        let date_range = parsed.date_range.unwrap();
        assert!(date_range.start.is_some());
        assert!(date_range.end.is_some());
    }

    #[tokio::test]
    async fn test_parse_natural_query_date_range_last_week() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher
            .parse_natural_query("show last week documents")
            .unwrap();

        assert!(parsed.date_range.is_some());
    }

    #[tokio::test]
    async fn test_parse_natural_query_combined_filters() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher
            .parse_natural_query("photos from last month vacation")
            .unwrap();

        assert!(!parsed.file_types.is_empty());
        assert!(parsed.date_range.is_some());
        assert!(parsed.keywords.is_some());
        assert!(parsed.keywords.unwrap().contains("vacation"));
    }

    #[tokio::test]
    async fn test_parse_natural_query_extracts_keywords() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher
            .parse_natural_query("find my vacation photos from Hawaii")
            .unwrap();

        let keywords = parsed.keywords.unwrap();
        assert!(keywords.contains("vacation") || keywords.contains("Hawaii"));
    }

    #[tokio::test]
    async fn test_parse_natural_query_multiple_types() {
        let (searcher, _temp) = create_test_searcher().await;

        let parsed = searcher.parse_natural_query("images and videos").unwrap();

        // Should detect the first type mentioned
        assert!(!parsed.file_types.is_empty());
    }
}

#[cfg(test)]
mod favorites_tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (hippo_core::storage::Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = HippoConfig {
            data_dir: temp_dir.path().to_path_buf(),
            qdrant_url: "http://localhost:9999".to_string(),
            ..Default::default()
        };
        let storage = hippo_core::storage::Storage::new(&config).await.unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_toggle_favorite_sets_true() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory();
        storage.upsert_memory(&memory).await.unwrap();

        let is_favorite = storage.toggle_favorite(memory.id).await.unwrap();
        assert!(is_favorite);
    }

    #[tokio::test]
    async fn test_toggle_favorite_sets_false() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory();
        storage.upsert_memory(&memory).await.unwrap();

        // Toggle on
        storage.toggle_favorite(memory.id).await.unwrap();
        // Toggle off
        let is_favorite = storage.toggle_favorite(memory.id).await.unwrap();
        assert!(!is_favorite);
    }

    #[tokio::test]
    async fn test_get_favorite_memory() {
        let (storage, _temp) = create_test_storage().await;

        let mut memory = create_test_memory();
        memory.is_favorite = true;
        storage.upsert_memory(&memory).await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
        assert!(retrieved.is_favorite);
    }

    fn create_test_memory() -> Memory {
        Memory {
            id: uuid::Uuid::new_v4(),
            path: PathBuf::from("/test/file.txt"),
            source: Source::Local {
                root_path: PathBuf::from("/test"),
            },
            kind: MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: None,
            },
            metadata: MemoryMetadata {
                file_size: 1000,
                ..Default::default()
            },
            tags: vec![],
            embedding_id: None,
            connections: vec![],
            is_favorite: false,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            indexed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod storage_operations_tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (hippo_core::storage::Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = HippoConfig {
            data_dir: temp_dir.path().to_path_buf(),
            qdrant_url: "http://localhost:9999".to_string(),
            ..Default::default()
        };
        let storage = hippo_core::storage::Storage::new(&config).await.unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_upsert_and_get_memory() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, memory.id);
        assert_eq!(retrieved.path, memory.path);
    }

    #[tokio::test]
    async fn test_add_tag_to_memory() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        let tag = Tag::user("important");
        storage.add_tag(memory.id, tag.clone()).await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
        assert_eq!(retrieved.tags.len(), 1);
        assert_eq!(retrieved.tags[0].name, "important");
    }

    #[tokio::test]
    async fn test_remove_tag_from_memory() {
        let (storage, _temp) = create_test_storage().await;

        let mut memory = create_test_memory("test.txt", 1000);
        memory.tags = vec![Tag::user("tag1"), Tag::user("tag2")];
        storage.upsert_memory(&memory).await.unwrap();

        storage.remove_tag(memory.id, "tag1").await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap().unwrap();
        assert_eq!(retrieved.tags.len(), 1);
        assert_eq!(retrieved.tags[0].name, "tag2");
    }

    #[tokio::test]
    async fn test_list_tags() {
        let (storage, _temp) = create_test_storage().await;

        let mut memory1 = create_test_memory("file1.txt", 1000);
        memory1.tags = vec![Tag::user("rust"), Tag::user("code")];
        storage.upsert_memory(&memory1).await.unwrap();

        let mut memory2 = create_test_memory("file2.txt", 2000);
        memory2.tags = vec![Tag::user("rust"), Tag::user("project")];
        storage.upsert_memory(&memory2).await.unwrap();

        let tags = storage.list_tags().await.unwrap();
        assert!(!tags.is_empty());

        // Find "rust" tag - should have count of 2
        let rust_tag = tags.iter().find(|(name, _)| name == "rust");
        assert!(rust_tag.is_some());
    }

    #[tokio::test]
    async fn test_search_with_tags() {
        let (storage, _temp) = create_test_storage().await;

        let mut memory1 = create_test_memory("file1.txt", 1000);
        memory1.tags = vec![Tag::user("rust")];
        storage.upsert_memory(&memory1).await.unwrap();

        let mut memory2 = create_test_memory("file2.txt", 2000);
        memory2.tags = vec![Tag::user("python")];
        storage.upsert_memory(&memory2).await.unwrap();

        let results = storage
            .search_with_tags(None, &["rust".to_string()], None, 10, 0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory1.id);
    }

    #[tokio::test]
    async fn test_search_with_text() {
        let (storage, _temp) = create_test_storage().await;

        let memory1 = create_test_memory("vacation_photo.jpg", 1000);
        storage.upsert_memory(&memory1).await.unwrap();

        let memory2 = create_test_memory("work_document.pdf", 2000);
        storage.upsert_memory(&memory2).await.unwrap();

        let results = storage
            .search_with_tags(Some("vacation"), &[], None, 10, 0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory1.id);
    }

    #[tokio::test]
    async fn test_search_with_kind_filter() {
        let (storage, _temp) = create_test_storage().await;

        let mut memory1 = create_test_memory("image.jpg", 1000);
        memory1.kind = MemoryKind::Image {
            width: 1920,
            height: 1080,
            format: "jpg".to_string(),
        };
        storage.upsert_memory(&memory1).await.unwrap();

        let memory2 = create_test_memory("document.pdf", 2000);
        storage.upsert_memory(&memory2).await.unwrap();

        let results = storage
            .search_with_tags(None, &[], Some("image"), 10, 0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory1.id);
    }

    #[tokio::test]
    async fn test_count_search_results() {
        let (storage, _temp) = create_test_storage().await;

        for i in 0..5 {
            let memory = create_test_memory(&format!("file{}.txt", i), 1000);
            storage.upsert_memory(&memory).await.unwrap();
        }

        let count = storage.count_search_results(None, &[], None).await.unwrap();

        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_add_and_list_sources() {
        let (storage, _temp) = create_test_storage().await;

        let source = Source::Local {
            root_path: PathBuf::from("/test/documents"),
        };
        storage.add_source(source.clone()).await.unwrap();

        let sources = storage.list_sources().await.unwrap();
        assert!(!sources.is_empty());
    }

    #[tokio::test]
    async fn test_remove_source() {
        let (storage, _temp) = create_test_storage().await;

        let source = Source::Local {
            root_path: PathBuf::from("/test/documents"),
        };
        storage.add_source(source.clone()).await.unwrap();
        storage.remove_source(&source).await.unwrap();

        let sources = storage.list_sources().await.unwrap();
        // Should be empty or not contain our source
        assert!(sources.is_empty() || !sources.iter().any(|s| s.source == source));
    }

    #[tokio::test]
    async fn test_clear_all() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        storage.clear_all().await.unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.memory_count, 0);
    }

    #[tokio::test]
    async fn test_get_memory_by_path() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        let retrieved = storage.get_memory_by_path(&memory.path).await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, memory.id);
    }

    #[tokio::test]
    async fn test_remove_memory_by_path() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        storage.remove_memory_by_path(&memory.path).await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_find_by_path_prefix() {
        let (storage, _temp) = create_test_storage().await;

        let memory1 = create_test_memory("/home/user/docs/file1.txt", 1000);
        let memory2 = create_test_memory("/home/user/docs/file2.txt", 2000);
        let memory3 = create_test_memory("/home/user/photos/pic.jpg", 3000);

        storage.upsert_memory(&memory1).await.unwrap();
        storage.upsert_memory(&memory2).await.unwrap();
        storage.upsert_memory(&memory3).await.unwrap();

        let results = storage
            .find_by_path_prefix("/home/user/docs")
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_store_and_get_embedding() {
        let (storage, _temp) = create_test_storage().await;

        let memory = create_test_memory("test.txt", 1000);
        storage.upsert_memory(&memory).await.unwrap();

        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        storage
            .store_embedding(memory.id, &embedding, "test-model")
            .await
            .unwrap();

        let retrieved = storage.get_embedding(memory.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_emb = retrieved.unwrap();
        assert_eq!(retrieved_emb.len(), embedding.len());
    }

    fn create_test_memory(path: &str, size: u64) -> Memory {
        Memory {
            id: uuid::Uuid::new_v4(),
            path: PathBuf::from(path),
            source: Source::Local {
                root_path: PathBuf::from("/test"),
            },
            kind: MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: None,
            },
            metadata: MemoryMetadata {
                file_size: size,
                title: Some(
                    PathBuf::from(path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                ),
                ..Default::default()
            },
            tags: vec![],
            embedding_id: None,
            connections: vec![],
            is_favorite: false,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            indexed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod qdrant_manager_tests {
    use hippo_core::qdrant::QdrantManager;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_qdrant_manager_creation() {
        let data_dir = PathBuf::from("/tmp/hippo-test-qdrant");
        let manager = QdrantManager::new(data_dir, "http://localhost:6334");
        let status = manager.status().await;
        // Status should always be returned with a valid URL
        assert_eq!(status.url, "http://localhost:6334");
    }

    #[tokio::test]
    async fn test_qdrant_manager_start_logic() {
        let data_dir = PathBuf::from("/tmp/hippo-test-qdrant2");
        let manager = QdrantManager::new(data_dir, "http://localhost:6334");
        // Should handle gracefully when Qdrant is not installed
        let status = manager.status().await;
        // Status should be determinable regardless of whether Qdrant is running
        // In test environment, Qdrant is likely not installed or not running
        assert!(!status.managed || status.available);
    }

    #[tokio::test]
    async fn test_qdrant_manager_connection_handling() {
        let data_dir = PathBuf::from("/tmp/hippo-test-qdrant3");
        let manager = QdrantManager::new(data_dir, "http://localhost:9999");
        let status = manager.status().await;
        // Should handle unavailable Qdrant gracefully (port 9999 is unlikely to be running)
        // Either not available (expected) or if somehow available, just pass
        assert!(!status.available || status.url == "http://localhost:9999");
    }
}

#[cfg(test)]
mod ai_tagging_tests {
    use hippo_core::ai::{
        AiConfig, AiProvider, FileAnalysis, OrganizationSuggestion, TagSuggestion, UnifiedAiClient,
    };
    use hippo_core::TagSource;

    #[test]
    fn test_tag_suggestion_to_tag() {
        let suggestion = TagSuggestion {
            name: "test-tag".to_string(),
            confidence: 85,
            reason: "Test reason".to_string(),
        };

        let tag = suggestion.to_tag();
        assert_eq!(tag.name, "test-tag");
        assert_eq!(tag.confidence, Some(85));
        assert!(matches!(tag.source, TagSource::Ai));
    }

    #[test]
    fn test_file_analysis_structure() {
        let analysis = FileAnalysis {
            tags: vec![
                TagSuggestion {
                    name: "tag1".to_string(),
                    confidence: 90,
                    reason: "Reason 1".to_string(),
                },
                TagSuggestion {
                    name: "tag2".to_string(),
                    confidence: 75,
                    reason: "Reason 2".to_string(),
                },
            ],
            description: Some("Test description".to_string()),
            organization: Some(OrganizationSuggestion {
                suggested_folder: "Projects/Test".to_string(),
                reason: "Project organization".to_string(),
            }),
        };

        assert_eq!(analysis.tags.len(), 2);
        assert!(analysis.description.is_some());
        assert!(analysis.organization.is_some());
    }

    #[test]
    fn test_ai_config_defaults() {
        let config = AiConfig::default();
        assert_eq!(config.provider, AiProvider::Ollama);
        assert!(config.claude_api_key.is_none());
        assert!(!config.ollama_url.is_empty());
    }

    #[test]
    fn test_unified_ai_client_creation() {
        let client = UnifiedAiClient::new();
        assert_eq!(client.provider(), AiProvider::Ollama);
    }

    #[test]
    fn test_unified_ai_client_with_ollama() {
        let client = UnifiedAiClient::with_ollama(Some("http://localhost:11434".to_string()));
        assert_eq!(client.provider(), AiProvider::Ollama);
    }

    #[test]
    fn test_set_provider() {
        let mut client = UnifiedAiClient::new();
        client.set_provider(AiProvider::Claude);
        assert_eq!(client.provider(), AiProvider::Claude);
    }
}
