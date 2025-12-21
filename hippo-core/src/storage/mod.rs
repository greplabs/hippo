//! Storage layer combining SQLite (metadata) and Qdrant (vectors)

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::qdrant::QdrantStorage;
use crate::HippoConfig;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

#[allow(dead_code)]
pub struct Storage {
    db: Mutex<Connection>,
    qdrant: Arc<QdrantStorage>,
    qdrant_url: String,
}

impl Storage {
    pub async fn new(config: &HippoConfig) -> Result<Self> {
        let db_path = config.data_dir.join("hippo.db");
        let conn = Connection::open(&db_path)?;

        // Initialize schema
        Self::init_schema(&conn)?;

        // Initialize Qdrant
        let qdrant = QdrantStorage::new(&config.qdrant_url).await?;
        if qdrant.is_available().await {
            qdrant.ensure_collections().await?;
            info!("Qdrant vector storage initialized");
        } else {
            warn!("Qdrant not available, using SQLite fallback for embeddings");
        }

        Ok(Self {
            db: Mutex::new(conn),
            qdrant: Arc::new(qdrant),
            qdrant_url: config.qdrant_url.clone(),
        })
    }

    /// Get the Qdrant storage for direct access
    pub fn qdrant(&self) -> &Arc<QdrantStorage> {
        &self.qdrant
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        // First, create core tables
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                source_json TEXT NOT NULL,
                kind_json TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                tags_json TEXT NOT NULL,
                embedding_id TEXT,
                connections_json TEXT NOT NULL,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                indexed_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_path ON memories(path);
            CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
            CREATE INDEX IF NOT EXISTS idx_memories_modified ON memories(modified_at);

            CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_sync TEXT
            );

            CREATE TABLE IF NOT EXISTS clusters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                memory_ids_json TEXT NOT NULL,
                cover_id TEXT,
                auto_generated INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                metadata_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY,
                count INTEGER NOT NULL DEFAULT 0
            );

            -- Embeddings table for semantic search
            CREATE TABLE IF NOT EXISTS embeddings (
                memory_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
        "#,
        )?;

        // Migration: Add is_favorite column if it doesn't exist (for existing databases)
        let _ = conn.execute(
            "ALTER TABLE memories ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Migration: Add searchable columns
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN title TEXT", []);
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN filename TEXT", []);
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN extension TEXT", []);
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN kind_name TEXT", []);
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN tags_text TEXT", []);

        // Create indexes for new columns (after migration)
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_favorite ON memories(is_favorite)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_kind ON memories(kind_name)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_extension ON memories(extension)",
            [],
        );

        // Populate new columns for existing data
        let _ = conn.execute(
            r#"
            UPDATE memories
            SET filename = SUBSTR(path, INSTR(path, '/') + 1),
                kind_name = CASE
                    WHEN kind_json LIKE '%"Image"%' THEN 'image'
                    WHEN kind_json LIKE '%"Video"%' THEN 'video'
                    WHEN kind_json LIKE '%"Audio"%' THEN 'audio'
                    WHEN kind_json LIKE '%"Document"%' THEN 'document'
                    WHEN kind_json LIKE '%"Code"%' THEN 'code'
                    WHEN kind_json LIKE '%"Spreadsheet"%' THEN 'spreadsheet'
                    WHEN kind_json LIKE '%"Presentation"%' THEN 'presentation'
                    WHEN kind_json LIKE '%"Archive"%' THEN 'archive'
                    ELSE 'unknown'
                END
            WHERE filename IS NULL OR kind_name IS NULL
        "#,
            [],
        );

        Ok(())
    }

    /// Get kind name for indexing
    fn get_kind_name(kind: &MemoryKind) -> &'static str {
        match kind {
            MemoryKind::Image { .. } => "image",
            MemoryKind::Video { .. } => "video",
            MemoryKind::Audio { .. } => "audio",
            MemoryKind::Document { .. } => "document",
            MemoryKind::Spreadsheet { .. } => "spreadsheet",
            MemoryKind::Presentation { .. } => "presentation",
            MemoryKind::Code { .. } => "code",
            MemoryKind::Archive { .. } => "archive",
            MemoryKind::Database => "database",
            MemoryKind::Folder => "folder",
            MemoryKind::Unknown => "unknown",
        }
    }

    // === Memory Operations ===

    pub async fn upsert_memory(&self, memory: &Memory) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        // Extract searchable fields
        let title = memory.metadata.title.clone().unwrap_or_default();
        let filename = memory
            .path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = memory
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let kind_name = Self::get_kind_name(&memory.kind);
        let tags_text = memory
            .tags
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join(" ");

        db.execute(
            r#"INSERT OR REPLACE INTO memories
               (id, path, source_json, kind_json, metadata_json, tags_json,
                embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at,
                title, filename, extension, kind_name, tags_text)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"#,
            params![
                memory.id.to_string(),
                memory.path.to_string_lossy(),
                serde_json::to_string(&memory.source)?,
                serde_json::to_string(&memory.kind)?,
                serde_json::to_string(&memory.metadata)?,
                serde_json::to_string(&memory.tags)?,
                memory.embedding_id,
                serde_json::to_string(&memory.connections)?,
                if memory.is_favorite { 1 } else { 0 },
                memory.created_at.to_rfc3339(),
                memory.modified_at.to_rfc3339(),
                memory.indexed_at.to_rfc3339(),
                title,
                filename,
                extension,
                kind_name,
                tags_text,
            ],
        )?;

        // Update tag counts
        for tag in &memory.tags {
            db.execute(
                "INSERT INTO tags (name, count) VALUES (?1, 1) ON CONFLICT(name) DO UPDATE SET count = count + 1",
                params![tag.name],
            )?;
        }

        Ok(())
    }

    pub async fn get_memory(&self, id: MemoryId) -> Result<Option<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare(
            "SELECT path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(Memory {
                id,
                path: PathBuf::from(row.get::<_, String>(0)?),
                source: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                metadata: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                embedding_id: row.get(5)?,
                connections: serde_json::from_str(&row.get::<_, String>(6)?).unwrap(),
                is_favorite: row.get::<_, i32>(7).unwrap_or(0) == 1,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn toggle_favorite(&self, id: MemoryId) -> Result<bool> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        // Get current state
        let current: i32 = db
            .query_row(
                "SELECT COALESCE(is_favorite, 0) FROM memories WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let new_state = if current == 1 { 0 } else { 1 };

        db.execute(
            "UPDATE memories SET is_favorite = ?1 WHERE id = ?2",
            params![new_state, id.to_string()],
        )?;

        Ok(new_state == 1)
    }

    pub async fn delete_memories_by_source(&self, source: &Source) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let source_json = serde_json::to_string(source)?;
        db.execute(
            "DELETE FROM memories WHERE source_json = ?1",
            params![source_json],
        )?;
        Ok(())
    }

    /// Remove a memory by its ID
    pub async fn delete_memory(&self, id: MemoryId) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        db.execute(
            "DELETE FROM memories WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    /// Remove a memory by its file path
    pub async fn remove_memory_by_path(&self, path: &std::path::Path) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let path_str = path.to_string_lossy();
        db.execute(
            "DELETE FROM memories WHERE path = ?1",
            params![path_str.as_ref()],
        )?;
        Ok(())
    }

    /// Get a memory by its file path
    pub async fn get_memory_by_path(&self, path: &std::path::Path) -> Result<Option<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let path_str = path.to_string_lossy();

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE path = ?1",
        )?;

        let result = stmt.query_row(params![path_str.as_ref()], |row| {
            Ok(Memory {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                path: PathBuf::from(row.get::<_, String>(1)?),
                source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                embedding_id: row.get(6)?,
                connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn find_by_path_prefix(&self, prefix: &str) -> Result<Vec<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE path LIKE ?1",
        )?;

        let pattern = format!("{}%", prefix);
        let memories = stmt
            .query_map(params![pattern], |row| {
                Ok(Memory {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                    embedding_id: row.get(6)?,
                    connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                    is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    // === Tag Operations ===

    pub async fn add_tag(&self, memory_id: MemoryId, tag: Tag) -> Result<()> {
        if let Some(mut memory) = self.get_memory(memory_id).await? {
            if !memory.tags.iter().any(|t| t.name == tag.name) {
                memory.tags.push(tag);
                self.upsert_memory(&memory).await?;
            }
        }
        Ok(())
    }

    pub async fn remove_tag(&self, memory_id: MemoryId, tag_name: &str) -> Result<()> {
        if let Some(mut memory) = self.get_memory(memory_id).await? {
            memory.tags.retain(|t| t.name != tag_name);
            self.upsert_memory(&memory).await?;
        }
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<(String, u64)>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare("SELECT name, count FROM tags ORDER BY count DESC")?;
        let tags = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tags)
    }

    // === Source Operations ===

    pub async fn add_source(&self, source: Source) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let id = uuid::Uuid::new_v4().to_string();
        let config = SourceConfig {
            source,
            enabled: true,
            sync_interval_secs: 3600,
            last_sync: None,
            include_patterns: vec![],
            exclude_patterns: vec![],
        };

        db.execute(
            "INSERT INTO sources (id, config_json, enabled) VALUES (?1, ?2, 1)",
            params![id, serde_json::to_string(&config)?],
        )?;
        Ok(())
    }

    pub async fn remove_source(&self, source: &Source) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let source_json = serde_json::to_string(source)?;
        db.execute(
            "DELETE FROM sources WHERE config_json LIKE ?1",
            params![format!("%{}%", source_json)],
        )?;
        Ok(())
    }

    pub async fn remove_memories_by_path_prefix(&self, path_prefix: &str) -> Result<usize> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        let count = db.execute(
            "DELETE FROM memories WHERE path LIKE ?1",
            params![format!("{}%", path_prefix)],
        )?;
        Ok(count)
    }

    pub async fn clear_all(&self) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        db.execute_batch(
            r#"
            DELETE FROM memories;
            DELETE FROM sources;
            DELETE FROM clusters;
            DELETE FROM tags;
        "#,
        )?;
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<StorageStats> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let memory_count: i64 =
            db.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        let source_count: i64 =
            db.query_row("SELECT COUNT(*) FROM sources", [], |row| row.get(0))?;
        let tag_count: i64 = db.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
        let cluster_count: i64 =
            db.query_row("SELECT COUNT(*) FROM clusters", [], |row| row.get(0))?;

        Ok(StorageStats {
            memory_count: memory_count as usize,
            source_count: source_count as usize,
            tag_count: tag_count as usize,
            cluster_count: cluster_count as usize,
        })
    }

    pub async fn list_sources(&self) -> Result<Vec<SourceConfig>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare("SELECT config_json FROM sources")?;
        let sources = stmt
            .query_map([], |row| {
                Ok(serde_json::from_str::<SourceConfig>(&row.get::<_, String>(0)?).unwrap())
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sources)
    }

    // === Cluster Operations ===

    pub async fn list_clusters(&self) -> Result<Vec<Cluster>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare(
            "SELECT id, name, kind, memory_ids_json, cover_id, auto_generated, created_at, metadata_json
             FROM clusters"
        )?;

        let clusters = stmt
            .query_map([], |row| {
                Ok(Cluster {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    kind: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    memory_ids: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    cover_memory_id: row
                        .get::<_, Option<String>>(4)?
                        .map(|s| uuid::Uuid::parse_str(&s).unwrap()),
                    auto_generated: row.get::<_, i32>(5)? != 0,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(clusters)
    }

    pub async fn create_cluster(&self, name: &str, kind: ClusterKind) -> Result<Cluster> {
        let cluster = Cluster {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            kind,
            memory_ids: vec![],
            cover_memory_id: None,
            auto_generated: false,
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };

        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;
        db.execute(
            "INSERT INTO clusters (id, name, kind, memory_ids_json, auto_generated, created_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                cluster.id.to_string(),
                cluster.name,
                serde_json::to_string(&cluster.kind)?,
                serde_json::to_string(&cluster.memory_ids)?,
                0,
                cluster.created_at.to_rfc3339(),
                serde_json::to_string(&cluster.metadata)?,
            ],
        )?;

        Ok(cluster)
    }

    pub async fn add_to_cluster(
        &self,
        cluster_id: uuid::Uuid,
        memory_ids: Vec<MemoryId>,
    ) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare("SELECT memory_ids_json FROM clusters WHERE id = ?1")?;
        let current: Vec<MemoryId> = stmt.query_row(params![cluster_id.to_string()], |row| {
            Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap())
        })?;

        let mut new_ids = current;
        for id in memory_ids {
            if !new_ids.contains(&id) {
                new_ids.push(id);
            }
        }

        db.execute(
            "UPDATE clusters SET memory_ids_json = ?1 WHERE id = ?2",
            params![serde_json::to_string(&new_ids)?, cluster_id.to_string()],
        )?;

        Ok(())
    }

    // === Vector Operations ===

    /// Find similar memories using Qdrant vector search
    pub async fn find_similar(
        &self,
        memory_id: MemoryId,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        // Try Qdrant first
        if self.qdrant.is_available().await {
            // Get the memory to know its kind
            if let Some(memory) = self.get_memory(memory_id).await? {
                // Get embedding from SQLite
                if let Some(embedding) = self.get_embedding(memory_id).await? {
                    return self
                        .qdrant
                        .find_similar(memory_id, embedding, &memory.kind, limit)
                        .await;
                }
            }
        }

        // Fallback to SQLite-based similarity search
        self.find_similar_sqlite(memory_id, limit).await
    }

    /// SQLite fallback for similarity search
    async fn find_similar_sqlite(
        &self,
        memory_id: MemoryId,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        // Get target embedding
        let target = match self.get_embedding(memory_id).await? {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        // Get all embeddings
        let all = self.get_all_embeddings().await?;

        // Calculate cosine similarity for each
        let mut scored: Vec<(MemoryId, f32)> = all
            .into_iter()
            .filter(|(id, _)| *id != memory_id)
            .map(|(id, emb)| {
                let similarity = cosine_similarity(&target, &emb);
                (id, similarity)
            })
            .collect();

        // Sort by similarity descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }

    /// Search vectors using Qdrant
    pub async fn search_vectors(
        &self,
        query_embedding: Vec<f32>,
        kind_filter: Option<&MemoryKind>,
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        // Try Qdrant first
        if self.qdrant.is_available().await {
            let kind_str = kind_filter.map(|k| match k {
                MemoryKind::Image { .. } => "image",
                MemoryKind::Code { .. } => "code",
                _ => "text",
            });
            return self
                .qdrant
                .search(query_embedding.clone(), kind_str, limit)
                .await;
        }

        // Fallback to SQLite-based search
        self.search_vectors_sqlite(&query_embedding, limit).await
    }

    /// SQLite fallback for vector search
    async fn search_vectors_sqlite(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>> {
        let all = self.get_all_embeddings().await?;

        let mut scored: Vec<(MemoryId, f32)> = all
            .into_iter()
            .map(|(id, emb)| {
                let similarity = cosine_similarity(query, &emb);
                (id, similarity)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }

    /// Store embedding in both SQLite and Qdrant
    pub async fn store_embedding_with_qdrant(
        &self,
        memory_id: MemoryId,
        embedding: &[f32],
        model: &str,
        kind: &MemoryKind,
    ) -> Result<()> {
        // Store in SQLite
        self.store_embedding(memory_id, embedding, model).await?;

        // Store in Qdrant if available
        if self.qdrant.is_available().await {
            self.qdrant
                .upsert(memory_id, embedding.to_vec(), kind)
                .await?;
        }

        Ok(())
    }

    // === Fast SQL Search ===

    /// Search memories using FTS5 full-text search
    pub async fn search_fts(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        // Prepare query for FTS5 - escape special characters and add wildcards
        let fts_query = query
            .split_whitespace()
            .map(|word| format!("{}*", word.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");

        let mut stmt = db.prepare(
            r#"SELECT m.id, m.path, m.source_json, m.kind_json, m.metadata_json, m.tags_json,
                      m.embedding_id, m.connections_json, m.is_favorite, m.created_at, m.modified_at, m.indexed_at
               FROM memories m
               JOIN memories_fts fts ON m.id = fts.id
               WHERE memories_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2 OFFSET ?3"#
        )?;

        let memories = stmt
            .query_map(params![fts_query, limit as i64, offset as i64], |row| {
                Ok(Memory {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                    embedding_id: row.get(6)?,
                    connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                    is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Search memories using LIKE pattern matching (fallback when FTS fails)
    pub async fn search_like(
        &self,
        query: &str,
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let pattern = format!("%{}%", query.to_lowercase());

        let sql = if kind.is_some() {
            r#"SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                      embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
               FROM memories
               WHERE kind_name = ?1 AND (
                   LOWER(filename) LIKE ?2 OR
                   LOWER(title) LIKE ?2 OR
                   LOWER(path) LIKE ?2 OR
                   LOWER(tags_text) LIKE ?2
               )
               ORDER BY modified_at DESC
               LIMIT ?3 OFFSET ?4"#
        } else {
            r#"SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                      embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
               FROM memories
               WHERE LOWER(filename) LIKE ?1 OR
                     LOWER(title) LIKE ?1 OR
                     LOWER(path) LIKE ?1 OR
                     LOWER(tags_text) LIKE ?1
               ORDER BY modified_at DESC
               LIMIT ?2 OFFSET ?3"#
        };

        let mut stmt = db.prepare(sql)?;
        let memories: Vec<Memory> = if let Some(k) = kind {
            let rows = stmt.query_map(
                params![k, pattern, limit as i64, offset as i64],
                Self::row_to_memory,
            )?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let rows = stmt.query_map(
                params![pattern, limit as i64, offset as i64],
                Self::row_to_memory,
            )?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(memories)
    }

    /// Helper to convert a row to Memory
    fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
        Ok(Memory {
            id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            path: PathBuf::from(row.get::<_, String>(1)?),
            source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
            kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
            metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
            tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
            embedding_id: row.get(6)?,
            connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
            is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
            indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                .unwrap()
                .with_timezone(&chrono::Utc),
        })
    }

    /// Search with tag filters
    pub async fn search_with_tags(
        &self,
        query: Option<&str>,
        tags: &[String],
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        // Build dynamic SQL based on filters
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Text search condition
        if let Some(q) = query {
            if !q.trim().is_empty() {
                let pattern = format!("%{}%", q.to_lowercase());
                conditions.push("(LOWER(filename) LIKE ? OR LOWER(title) LIKE ? OR LOWER(path) LIKE ? OR LOWER(tags_text) LIKE ?)".to_string());
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }
        }

        // Tag filters
        for tag in tags {
            conditions.push("LOWER(tags_text) LIKE ?".to_string());
            params_vec.push(Box::new(format!("%{}%", tag.to_lowercase())));
        }

        // Kind filter
        if let Some(k) = kind {
            conditions.push("kind_name = ?".to_string());
            params_vec.push(Box::new(k.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" AND ")
        };

        let sql = format!(
            r#"SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                      embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
               FROM memories
               WHERE {}
               ORDER BY modified_at DESC
               LIMIT ? OFFSET ?"#,
            where_clause
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = db.prepare(&sql)?;
        let memories = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(Memory {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                    embedding_id: row.get(6)?,
                    connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                    is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Count total memories matching a search
    pub async fn count_search_results(
        &self,
        query: Option<&str>,
        tags: &[String],
        kind: Option<&str>,
    ) -> Result<usize> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(q) = query {
            if !q.trim().is_empty() {
                let pattern = format!("%{}%", q.to_lowercase());
                conditions.push("(LOWER(filename) LIKE ? OR LOWER(title) LIKE ? OR LOWER(path) LIKE ? OR LOWER(tags_text) LIKE ?)".to_string());
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }
        }

        for tag in tags {
            conditions.push("LOWER(tags_text) LIKE ?".to_string());
            params_vec.push(Box::new(format!("%{}%", tag.to_lowercase())));
        }

        if let Some(k) = kind {
            conditions.push("kind_name = ?".to_string());
            params_vec.push(Box::new(k.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" AND ")
        };

        let sql = format!("SELECT COUNT(*) FROM memories WHERE {}", where_clause);
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = db.prepare(&sql)?;
        let count: i64 = stmt.query_row(params_refs.as_slice(), |row| row.get(0))?;

        Ok(count as usize)
    }

    /// Store embedding for a memory
    pub async fn store_embedding(
        &self,
        memory_id: MemoryId,
        embedding: &[f32],
        model: &str,
    ) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        // Convert embedding to bytes
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        db.execute(
            r#"INSERT OR REPLACE INTO embeddings (memory_id, embedding, model, created_at)
               VALUES (?1, ?2, ?3, ?4)"#,
            params![
                memory_id.to_string(),
                embedding_bytes,
                model,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get embedding for a memory
    pub async fn get_embedding(&self, memory_id: MemoryId) -> Result<Option<Vec<f32>>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare("SELECT embedding FROM embeddings WHERE memory_id = ?1")?;
        let result = stmt.query_row(params![memory_id.to_string()], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        });

        match result {
            Ok(bytes) => {
                // Convert bytes back to f32 vector
                let floats: Vec<f32> = bytes
                    .chunks(4)
                    .map(|chunk| {
                        let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                        f32::from_le_bytes(arr)
                    })
                    .collect();
                Ok(Some(floats))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all embeddings for similarity search
    pub async fn get_all_embeddings(&self) -> Result<Vec<(MemoryId, Vec<f32>)>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare("SELECT memory_id, embedding FROM embeddings")?;
        let embeddings = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((id_str, bytes))
            })?
            .filter_map(|r| r.ok())
            .map(|(id_str, bytes)| {
                let id = uuid::Uuid::parse_str(&id_str).unwrap();
                let floats: Vec<f32> = bytes
                    .chunks(4)
                    .map(|chunk| {
                        let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                        f32::from_le_bytes(arr)
                    })
                    .collect();
                (id, floats)
            })
            .collect();

        Ok(embeddings)
    }

    // === Stats ===

    pub async fn get_all_memories(&self) -> Result<Vec<Memory>> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories ORDER BY modified_at DESC LIMIT 5000",
        )?;

        let memories = stmt
            .query_map([], |row| {
                Ok(Memory {
                    id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    kind: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                    embedding_id: row.get(6)?,
                    connections: serde_json::from_str(&row.get::<_, String>(7)?).unwrap(),
                    is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    pub async fn stats(&self) -> Result<IndexStats> {
        let db = self
            .db
            .lock()
            .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

        let total: i64 = db.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;

        Ok(IndexStats {
            total_memories: total as u64,
            by_kind: std::collections::HashMap::new(),
            by_source: std::collections::HashMap::new(),
            total_size_bytes: 0,
            index_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Get Qdrant statistics
    pub async fn qdrant_stats(&self) -> Result<crate::qdrant::QdrantStats> {
        self.qdrant.stats().await
    }

    // === Export/Import Operations ===

    /// Export all index data to a serializable structure
    pub async fn export_index(&self) -> Result<IndexExport> {
        info!("Exporting index...");

        let memories = self.get_all_memories().await?;
        let sources = self.list_sources().await?;
        let tags = self.list_tags().await?;
        let clusters = self.list_clusters().await?;

        let export = IndexExport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            export_date: chrono::Utc::now(),
            memories,
            sources,
            tags,
            clusters,
        };

        info!(
            "Export complete: {} memories, {} sources, {} tags, {} clusters",
            export.memories.len(),
            export.sources.len(),
            export.tags.len(),
            export.clusters.len()
        );

        Ok(export)
    }

    /// Import index data from an export structure
    pub async fn import_index(&self, data: IndexExport) -> Result<ImportStats> {
        info!("Importing index from export dated {}", data.export_date);

        let mut stats = ImportStats {
            memories_imported: 0,
            tags_imported: 0,
            sources_imported: 0,
            clusters_imported: 0,
            duplicates_skipped: 0,
            errors: Vec::new(),
        };

        // Validate version compatibility (simple check for now)
        let current_version = env!("CARGO_PKG_VERSION");
        if data.version != current_version {
            warn!(
                "Import version mismatch: export={}, current={}",
                data.version, current_version
            );
            stats.errors.push(format!(
                "Version mismatch: export={}, current={}",
                data.version, current_version
            ));
        }

        // Import sources first
        for source_config in data.sources {
            match self.add_source(source_config.source.clone()).await {
                Ok(_) => stats.sources_imported += 1,
                Err(e) => {
                    stats.errors.push(format!(
                        "Failed to import source {:?}: {}",
                        source_config.source, e
                    ));
                }
            }
        }

        // Import memories
        for memory in data.memories {
            // Check if memory already exists by path
            match self.get_memory_by_path(&memory.path).await {
                Ok(Some(_)) => {
                    stats.duplicates_skipped += 1;
                }
                Ok(None) => {
                    // Memory doesn't exist, import it
                    match self.upsert_memory(&memory).await {
                        Ok(_) => stats.memories_imported += 1,
                        Err(e) => {
                            stats
                                .errors
                                .push(format!("Failed to import memory {:?}: {}", memory.path, e));
                        }
                    }
                }
                Err(e) => {
                    stats
                        .errors
                        .push(format!("Failed to check memory {:?}: {}", memory.path, e));
                }
            }
        }

        // Import clusters
        for cluster in data.clusters {
            let db = self
                .db
                .lock()
                .map_err(|_| HippoError::Database(rusqlite::Error::InvalidQuery))?;

            match db.execute(
                "INSERT OR REPLACE INTO clusters (id, name, kind, memory_ids_json, cover_id, auto_generated, created_at, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    cluster.id.to_string(),
                    cluster.name,
                    serde_json::to_string(&cluster.kind)?,
                    serde_json::to_string(&cluster.memory_ids)?,
                    cluster.cover_memory_id.map(|id| id.to_string()),
                    if cluster.auto_generated { 1 } else { 0 },
                    cluster.created_at.to_rfc3339(),
                    serde_json::to_string(&cluster.metadata)?,
                ],
            ) {
                Ok(_) => stats.clusters_imported += 1,
                Err(e) => {
                    stats.errors.push(format!(
                        "Failed to import cluster {}: {}",
                        cluster.name, e
                    ));
                }
            }
        }

        // Tags are automatically imported with memories via upsert_memory
        stats.tags_imported = data.tags.len();

        info!(
            "Import complete: {} memories, {} sources, {} tags, {} clusters imported ({} duplicates skipped, {} errors)",
            stats.memories_imported,
            stats.sources_imported,
            stats.tags_imported,
            stats.clusters_imported,
            stats.duplicates_skipped,
            stats.errors.len()
        );

        Ok(stats)
    }
}

/// Calculate cosine similarity between two vectors
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

    #[test]
    fn test_cosine_similarity_identical() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert!((similarity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![-1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert!((similarity + 1.0).abs() < 0.001); // Should be -1.0
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert_eq!(similarity, 0.0); // Different lengths
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let vec1: Vec<f32> = vec![];
        let vec2: Vec<f32> = vec![];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vectors() {
        let vec1 = vec![0.0, 0.0, 0.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&vec1, &vec2);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_get_kind_name() {
        use crate::models::DocumentFormat;

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Image {
                width: 1920,
                height: 1080,
                format: "jpg".to_string()
            }),
            "image"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Video {
                duration_ms: 1000,
                format: "mp4".to_string()
            }),
            "video"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Audio {
                duration_ms: 1000,
                format: "mp3".to_string()
            }),
            "audio"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Code {
                language: "rust".to_string(),
                lines: 100
            }),
            "code"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Document {
                format: DocumentFormat::Pdf,
                page_count: Some(10)
            }),
            "document"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Spreadsheet { sheet_count: 5 }),
            "spreadsheet"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Presentation { slide_count: 20 }),
            "presentation"
        );

        assert_eq!(
            Storage::get_kind_name(&MemoryKind::Archive { item_count: 100 }),
            "archive"
        );

        assert_eq!(Storage::get_kind_name(&MemoryKind::Database), "database");
        assert_eq!(Storage::get_kind_name(&MemoryKind::Folder), "folder");
        assert_eq!(Storage::get_kind_name(&MemoryKind::Unknown), "unknown");
    }
}
