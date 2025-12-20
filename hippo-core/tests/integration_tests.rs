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
