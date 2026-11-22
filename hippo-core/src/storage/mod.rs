//! Storage layer combining SQLite (metadata) and Qdrant (vectors)

use crate::error::{HippoError, Result};
use crate::models::*;
use crate::qdrant::QdrantStorage;
use crate::HippoConfig;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// An FTS5 search result with snippet data and BM25 ranking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Fts5SearchResult {
    pub memory: Memory,
    pub fts5_rank: f64,
    pub title_snippet: Option<String>,
    pub filename_snippet: Option<String>,
    pub tags_snippet: Option<String>,
    pub content_snippet: Option<String>,
}

/// A parent memory surfaced by chunk-level semantic retrieval, already rolled up
/// (deduplicated to one row per memory) with its best-matching chunk attached for
/// snippet offsets. `best_chunk` is `None` when the parent was matched only by its
/// whole-document embedding (no chunks).
#[derive(Debug, Clone)]
pub struct ChunkParentHit {
    pub memory_id: MemoryId,
    pub score: f32,
    pub best_chunk: Option<Chunk>,
}

/// Helper to parse JSON from database, converting errors to rusqlite errors
fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> std::result::Result<T, rusqlite::Error> {
    serde_json::from_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

/// Helper to parse UUID from database, converting errors to rusqlite errors
fn parse_uuid(s: &str) -> std::result::Result<uuid::Uuid, rusqlite::Error> {
    uuid::Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

/// Helper to parse datetime from database, converting errors to rusqlite errors
fn parse_datetime(s: &str) -> std::result::Result<chrono::DateTime<chrono::Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

/// Decode a little-endian `f32` blob (as stored in the `embeddings` table) back
/// into a vector. Returns an error if the byte length is not a multiple of 4,
/// rather than silently zeroing a partial chunk (which would corrupt the vector).
fn bytes_to_f32_vec(bytes: &[u8]) -> Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(HippoError::Other(format!(
            "corrupt embedding blob: {} bytes is not a multiple of 4",
            bytes.len()
        )));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// Escape SQL `LIKE` wildcards (`%` and `_`, plus the escape char itself) so
/// that user-supplied text is matched literally. Must be paired with an
/// `ESCAPE '\'` clause in the SQL statement.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[allow(dead_code)]
pub struct Storage {
    db: Mutex<Connection>,
    qdrant: Arc<QdrantStorage>,
    qdrant_url: String,
    /// Whether chunk-level indexing/retrieval is active (mirrors `HippoConfig`).
    chunking_enabled: bool,
    /// How chunk scores roll up to a parent memory's semantic score.
    chunk_rollup: ChunkRollup,
}

// TODO(perf, follow-up): this layer serializes ALL database I/O through a
// single `Mutex<Connection>`, and the synchronous rusqlite calls run directly
// on tokio worker threads (blocking-in-async). Under concurrency this both
// serializes unrelated queries and can starve the async runtime. The correct
// fix is a connection pool (e.g. r2d2/deadpool) with blocking calls moved onto
// `spawn_blocking`. That is a large, cross-cutting refactor that would change
// method internals (not signatures), so it is intentionally deferred to keep
// the public API stable for parallel work. Do NOT attempt it piecemeal here.
impl Storage {
    /// Safely acquire the database lock, recovering from poisoning if necessary.
    ///
    /// Mutex poisoning happens when a thread panics while holding the lock. Now
    /// that every `query_map`/`query_row` closure uses the `?`-returning parse
    /// helpers (`parse_json`/`parse_uuid`/`parse_datetime`) instead of
    /// `.unwrap()`, a corrupt row returns an error instead of panicking — so a
    /// genuine poison here signals a real bug, not merely bad data. We still
    /// recover (the underlying SQLite connection remains valid), but log loudly
    /// at ERROR level so the condition is never silent.
    fn get_db(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        match self.db.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                tracing::error!(
                    "Database mutex was poisoned (a thread panicked while holding the lock); \
                     recovering the connection. This indicates a bug — please report it."
                );
                // The SQLite connection itself is still usable, so recover it.
                Ok(poisoned.into_inner())
            }
        }
    }
}

impl Storage {
    pub async fn new(config: &HippoConfig) -> Result<Self> {
        let db_path = config.data_dir.join("hippo.db");
        let conn = Connection::open(&db_path)?;

        // Enable WAL mode and aggressive performance optimizations
        // These settings provide 5-10x improvement for concurrent operations
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -262144;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 1073741824;
            PRAGMA page_size = 4096;
            PRAGMA wal_autocheckpoint = 1000;
            PRAGMA busy_timeout = 5000;
            PRAGMA foreign_keys = OFF;
            PRAGMA auto_vacuum = INCREMENTAL;
            PRAGMA locking_mode = NORMAL;
            "#,
        )?;

        // Run ANALYZE on startup for query optimization (if tables exist)
        let _ = conn.execute_batch("ANALYZE");

        info!("SQLite optimized: 1GB cache, 1GB mmap, WAL mode");

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
            chunking_enabled: config.chunking_enabled,
            chunk_rollup: config.chunk_rollup,
        })
    }

    /// Whether chunk-level indexing/retrieval is enabled.
    pub fn chunking_enabled(&self) -> bool {
        self.chunking_enabled
    }

    /// The configured chunk-score roll-up strategy.
    pub fn chunk_rollup(&self) -> ChunkRollup {
        self.chunk_rollup
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
                count INTEGER NOT NULL DEFAULT 0,
                color TEXT
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

        // Migration: Add color column to tags table
        let _ = conn.execute("ALTER TABLE tags ADD COLUMN color TEXT", []);

        // Migration: Add text_preview column for FTS content search
        let _ = conn.execute("ALTER TABLE memories ADD COLUMN text_preview TEXT", []);

        // Create indexes for faster queries (5-50x improvement for filtered queries)
        // Core indexes
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

        // Search optimization indexes
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_title ON memories(title)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_filename ON memories(filename)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_tags_text ON memories(tags_text)",
            [],
        );

        // Composite indexes for common filter combinations
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_kind_created ON memories(kind_name, created_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_kind_favorite ON memories(kind_name, is_favorite)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_modified_desc ON memories(modified_at DESC)",
            [],
        );

        // Additional compound indexes for common query patterns
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_favorite_modified ON memories(is_favorite, modified_at DESC)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_kind_modified ON memories(kind_name, modified_at DESC)",
            [],
        );

        // Check if FTS table needs rebuild (to add text_preview column)
        let needs_fts_rebuild: bool = conn
            .prepare("SELECT * FROM memories_fts LIMIT 0")
            .map(|stmt| stmt.column_count() < 5)
            .unwrap_or(true);

        if needs_fts_rebuild {
            let _ = conn.execute("DROP TRIGGER IF EXISTS memories_fts_insert", []);
            let _ = conn.execute("DROP TRIGGER IF EXISTS memories_fts_delete", []);
            let _ = conn.execute("DROP TRIGGER IF EXISTS memories_fts_update", []);
            let _ = conn.execute("DROP TABLE IF EXISTS memories_fts", []);
        }

        // FTS5 with text_preview, porter stemmer, and columnsize for bm25
        let _ = conn.execute(
            r#"CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                id, title, filename, tags_text, text_preview,
                content='memories', content_rowid='rowid',
                tokenize='porter unicode61', columnsize=1
            )"#,
            [],
        );

        // Triggers updated to include text_preview
        let _ = conn.execute(
            r#"CREATE TRIGGER IF NOT EXISTS memories_fts_insert AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, id, title, filename, tags_text, text_preview)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.filename, NEW.tags_text, NEW.text_preview);
            END"#,
            [],
        );
        let _ = conn.execute(
            r#"CREATE TRIGGER IF NOT EXISTS memories_fts_delete AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, id, title, filename, tags_text, text_preview)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.filename, OLD.tags_text, OLD.text_preview);
            END"#,
            [],
        );
        let _ = conn.execute(
            r#"CREATE TRIGGER IF NOT EXISTS memories_fts_update AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, id, title, filename, tags_text, text_preview)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.filename, OLD.tags_text, OLD.text_preview);
                INSERT INTO memories_fts(rowid, id, title, filename, tags_text, text_preview)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.filename, NEW.tags_text, NEW.text_preview);
            END"#,
            [],
        );

        // Populate FTS index from existing data
        let _ = conn.execute(
            r#"INSERT OR IGNORE INTO memories_fts(rowid, id, title, filename, tags_text, text_preview)
               SELECT rowid, id, title, filename, tags_text, text_preview FROM memories
               WHERE title IS NOT NULL OR filename IS NOT NULL"#,
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

        // Populate text_preview from metadata_json for existing data
        let _ = conn.execute(
            r#"UPDATE memories SET text_preview = json_extract(metadata_json, '$.text_preview')
               WHERE text_preview IS NULL AND json_extract(metadata_json, '$.text_preview') IS NOT NULL"#,
            [],
        );

        // === Session 14: Saved Searches & Search History tables ===

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS saved_searches (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                query TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                filters_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                last_used_at TEXT,
                use_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                result_count INTEGER NOT NULL DEFAULT 0,
                searched_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_search_history_searched_at
                ON search_history(searched_at DESC);
            "#,
        )?;

        // === Chunk-level retrieval tables (additive; existing DBs keep working) ===
        //
        // `chunks` holds the passage rows for each parent memory. `chunk_embeddings`
        // is the durable SQLite copy of each chunk's vector (SQLite is the source
        // of truth; Qdrant `hippo_text` is a write-through accelerator). Both are
        // created with IF NOT EXISTS so upgrading an existing database is a no-op
        // migration that never drops data.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS chunks (
                chunk_id TEXT PRIMARY KEY,
                parent_memory_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                text TEXT NOT NULL,
                char_start INTEGER NOT NULL,
                char_end INTEGER NOT NULL,
                embedding_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_parent ON chunks(parent_memory_id);

            CREATE TABLE IF NOT EXISTS chunk_embeddings (
                chunk_id TEXT PRIMARY KEY,
                parent_memory_id TEXT NOT NULL,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_chunk_embeddings_parent
                ON chunk_embeddings(parent_memory_id);
            "#,
        )?;

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
        let db = self.get_db()?;
        Self::upsert_memory_in(&db, memory)
    }

    /// Insert-or-replace a batch of memories inside a SINGLE transaction.
    ///
    /// `upsert_memory` runs each write as its own implicit WAL transaction, so
    /// indexing a batch of N files cost N commits (+ per-row fsync/checkpoint
    /// pressure). Wrapping the whole batch in one `BEGIN … COMMIT` turns that
    /// into a single commit — a large indexing throughput win — and makes the
    /// batch atomic (a failure rolls the whole batch back rather than leaving a
    /// partially-written index).
    pub async fn upsert_memories_batch(&self, memories: &[Memory]) -> Result<()> {
        if memories.is_empty() {
            return Ok(());
        }
        let mut db = self.get_db()?;
        let tx = db.transaction()?;
        for memory in memories {
            Self::upsert_memory_in(&tx, memory)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Shared insert-or-replace body, parameterised over the connection so it
    /// can run standalone (`upsert_memory`) or inside a transaction
    /// (`upsert_memories_batch`). `Transaction` derefs to `Connection`.
    fn upsert_memory_in(db: &Connection, memory: &Memory) -> Result<()> {
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
        let text_preview = memory.metadata.text_preview.clone();

        db.execute(
            r#"INSERT OR REPLACE INTO memories
               (id, path, source_json, kind_json, metadata_json, tags_json,
                embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at,
                title, filename, extension, kind_name, tags_text, text_preview)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"#,
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
                text_preview,
            ],
        )?;

        // Ensure a `tags` row exists for each tag so that colors can be stored
        // against it (see `set_tag_color`). We deliberately do NOT maintain a
        // `count` counter here: the previous `count = count + 1` ran on every
        // upsert (and every add_tag/remove_tag/reindex re-ran it, while
        // delete/remove never decremented), so counts inflated without bound.
        // Tag counts are now DERIVED from the actual memory→tag relationships at
        // read time (see `list_tags` / `list_tags_with_colors` / `get_stats`),
        // which cannot drift.
        for tag in &memory.tags {
            db.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag.name],
            )?;
        }

        Ok(())
    }

    pub async fn get_memory(&self, id: MemoryId) -> Result<Option<Memory>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(Memory {
                id,
                path: PathBuf::from(row.get::<_, String>(0)?),
                source: parse_json(&row.get::<_, String>(1)?)?,
                kind: parse_json(&row.get::<_, String>(2)?)?,
                metadata: parse_json(&row.get::<_, String>(3)?)?,
                tags: parse_json(&row.get::<_, String>(4)?)?,
                embedding_id: row.get(5)?,
                connections: parse_json(&row.get::<_, String>(6)?)?,
                is_favorite: row.get::<_, i32>(7).unwrap_or(0) == 1,
                created_at: parse_datetime(&row.get::<_, String>(8)?)?,
                modified_at: parse_datetime(&row.get::<_, String>(9)?)?,
                indexed_at: parse_datetime(&row.get::<_, String>(10)?)?,
            })
        });

        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn toggle_favorite(&self, id: MemoryId) -> Result<bool> {
        let db = self.get_db()?;

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
        let db = self.get_db()?;
        let source_json = serde_json::to_string(source)?;
        db.execute(
            "DELETE FROM memories WHERE source_json = ?1",
            params![source_json],
        )?;
        Self::sweep_orphan_chunks(&db)?;
        Ok(())
    }

    /// Delete chunk rows + chunk embeddings whose parent memory no longer exists.
    /// Used by the bulk-delete paths (which operate by path/source rather than by
    /// id) to keep the chunk tables free of orphans without a second round-trip.
    fn sweep_orphan_chunks(db: &Connection) -> Result<()> {
        db.execute(
            "DELETE FROM chunks WHERE parent_memory_id NOT IN (SELECT id FROM memories)",
            [],
        )?;
        db.execute(
            "DELETE FROM chunk_embeddings WHERE parent_memory_id NOT IN (SELECT id FROM memories)",
            [],
        )?;
        Ok(())
    }

    /// Remove a memory by its ID
    pub async fn delete_memory(&self, id: MemoryId) -> Result<()> {
        {
            let db = self.get_db()?;
            db.execute(
                "DELETE FROM memories WHERE id = ?1",
                params![id.to_string()],
            )?;
        }
        // Tear down the memory's chunk rows + chunk vectors (SQLite + Qdrant).
        self.delete_chunks(id).await?;
        Ok(())
    }

    /// Remove a memory by its file path
    pub async fn remove_memory_by_path(&self, path: &std::path::Path) -> Result<()> {
        // Resolve the id first so we can clean up its chunks after deletion.
        let id = self.get_memory_by_path(path).await?.map(|m| m.id);
        {
            let db = self.get_db()?;
            let path_str = path.to_string_lossy();
            db.execute(
                "DELETE FROM memories WHERE path = ?1",
                params![path_str.as_ref()],
            )?;
        }
        if let Some(id) = id {
            self.delete_chunks(id).await?;
        }
        Ok(())
    }

    /// Get a memory by its file path
    pub async fn get_memory_by_path(&self, path: &std::path::Path) -> Result<Option<Memory>> {
        let db = self.get_db()?;
        let path_str = path.to_string_lossy();

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE path = ?1",
        )?;

        let result = stmt.query_row(params![path_str.as_ref()], |row| {
            Ok(Memory {
                id: parse_uuid(&row.get::<_, String>(0)?)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                source: parse_json(&row.get::<_, String>(2)?)?,
                kind: parse_json(&row.get::<_, String>(3)?)?,
                metadata: parse_json(&row.get::<_, String>(4)?)?,
                tags: parse_json(&row.get::<_, String>(5)?)?,
                embedding_id: row.get(6)?,
                connections: parse_json(&row.get::<_, String>(7)?)?,
                is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                created_at: parse_datetime(&row.get::<_, String>(9)?)?,
                modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
                indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
            })
        });

        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn find_by_path_prefix(&self, prefix: &str) -> Result<Vec<Memory>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE path LIKE ?1",
        )?;

        let pattern = format!("{}%", prefix);
        let memories = stmt
            .query_map(params![pattern], |row| {
                Ok(Memory {
                    id: parse_uuid(&row.get::<_, String>(0)?)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    source: parse_json(&row.get::<_, String>(2)?)?,
                    kind: parse_json(&row.get::<_, String>(3)?)?,
                    metadata: parse_json(&row.get::<_, String>(4)?)?,
                    tags: parse_json(&row.get::<_, String>(5)?)?,
                    embedding_id: row.get(6)?,
                    connections: parse_json(&row.get::<_, String>(7)?)?,
                    is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
                    created_at: parse_datetime(&row.get::<_, String>(9)?)?,
                    modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
                    indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
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
        let db = self.get_db()?;

        // Counts are derived from the real memory→tag relationships (the tag
        // arrays stored in each memory's `tags_json`), so they always reflect
        // reality and never inflate. `json_each` expands each tag array row.
        let mut stmt = db.prepare(
            "SELECT json_extract(j.value, '$.name') AS name, COUNT(*) AS cnt
             FROM memories, json_each(memories.tags_json) AS j
             WHERE name IS NOT NULL
             GROUP BY name
             ORDER BY cnt DESC",
        )?;
        let tags = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tags)
    }

    /// List all tags with their colors.
    ///
    /// Counts are derived from the real memory→tag relationships (see
    /// `list_tags`); the `tags` table is consulted only for the stored color.
    pub async fn list_tags_with_colors(&self) -> Result<Vec<(String, u64, Option<String>)>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT json_extract(j.value, '$.name') AS name, COUNT(*) AS cnt, MAX(t.color) AS color
             FROM memories, json_each(memories.tags_json) AS j
             LEFT JOIN tags t ON t.name = json_extract(j.value, '$.name')
             WHERE name IS NOT NULL
             GROUP BY name
             ORDER BY cnt DESC",
        )?;
        let tags = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tags)
    }

    /// Set color for a tag
    pub async fn set_tag_color(&self, tag_name: &str, color: Option<&str>) -> Result<()> {
        let db = self.get_db()?;

        db.execute(
            "UPDATE tags SET color = ?1 WHERE name = ?2",
            params![color, tag_name],
        )?;

        Ok(())
    }

    // === Source Operations ===

    pub async fn add_source(&self, source: Source) -> Result<()> {
        let db = self.get_db()?;
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
        let db = self.get_db()?;
        let source_json = serde_json::to_string(source)?;
        db.execute(
            "DELETE FROM sources WHERE config_json LIKE ?1",
            params![format!("%{}%", source_json)],
        )?;
        Ok(())
    }

    pub async fn remove_memories_by_path_prefix(&self, path_prefix: &str) -> Result<usize> {
        let db = self.get_db()?;
        let count = db.execute(
            "DELETE FROM memories WHERE path LIKE ?1",
            params![format!("{}%", path_prefix)],
        )?;
        Self::sweep_orphan_chunks(&db)?;
        Ok(count)
    }

    pub async fn clear_all(&self) -> Result<()> {
        let db = self.get_db()?;
        db.execute_batch(
            r#"
            DELETE FROM memories;
            DELETE FROM sources;
            DELETE FROM clusters;
            DELETE FROM tags;
            DELETE FROM embeddings;
            DELETE FROM chunks;
            DELETE FROM chunk_embeddings;
        "#,
        )?;
        Ok(())
    }

    /// Vacuum and optimize the database to reclaim space
    ///
    /// This should be called after bulk deletions or periodically to:
    /// - Reclaim disk space from deleted records
    /// - Rebuild indexes for better performance
    /// - Analyze tables for query optimization
    pub async fn vacuum(&self) -> Result<VacuumStats> {
        let db = self.get_db()?;

        // Get size before vacuum
        let size_before: i64 = db
            .query_row(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Run VACUUM to reclaim space and defragment
        db.execute_batch("VACUUM")?;

        // Analyze for query optimization
        db.execute_batch("ANALYZE")?;

        // Get size after vacuum
        let size_after: i64 = db
            .query_row(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let bytes_reclaimed = (size_before - size_after).max(0) as u64;
        info!(
            "Database vacuum complete: reclaimed {} bytes ({:.2} MB)",
            bytes_reclaimed,
            bytes_reclaimed as f64 / 1024.0 / 1024.0
        );

        Ok(VacuumStats {
            size_before: size_before as u64,
            size_after: size_after as u64,
            bytes_reclaimed,
        })
    }

    /// Clean up orphaned embeddings (embeddings without matching memories)
    pub async fn cleanup_orphaned_embeddings(&self) -> Result<usize> {
        let db = self.get_db()?;

        let deleted = db.execute(
            "DELETE FROM embeddings WHERE memory_id NOT IN (SELECT id FROM memories)",
            [],
        )?;

        info!("Cleaned up {} orphaned embeddings", deleted);
        Ok(deleted)
    }

    pub async fn get_stats(&self) -> Result<StorageStats> {
        let db = self.get_db()?;

        let memory_count: i64 =
            db.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        let source_count: i64 =
            db.query_row("SELECT COUNT(*) FROM sources", [], |row| row.get(0))?;
        // Derive the distinct tag count from the real memory→tag relationships
        // rather than counting (possibly stale) rows in the `tags` table.
        let tag_count: i64 = db.query_row(
            "SELECT COUNT(*) FROM (
                 SELECT DISTINCT json_extract(j.value, '$.name') AS name
                 FROM memories, json_each(memories.tags_json) AS j
                 WHERE name IS NOT NULL
             )",
            [],
            |row| row.get(0),
        )?;
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
        let db = self.get_db()?;

        let mut stmt = db.prepare("SELECT config_json FROM sources")?;
        let sources = stmt
            .query_map([], |row| {
                parse_json::<SourceConfig>(&row.get::<_, String>(0)?)
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sources)
    }

    /// Update the last_sync timestamp for a source
    pub async fn update_source_last_sync(&self, source: &Source) -> Result<()> {
        let db = self.get_db()?;
        let now = chrono::Utc::now();
        // Read all sources, update matching one, write back
        let mut stmt = db.prepare("SELECT id, config_json FROM sources")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (id, config_json) in rows {
            if let Ok(mut config) = serde_json::from_str::<SourceConfig>(&config_json) {
                if std::mem::discriminant(&config.source) == std::mem::discriminant(source) {
                    let source_path = match source {
                        Source::Local { root_path } => root_path.to_string_lossy().to_string(),
                        _ => continue,
                    };
                    let config_path = match &config.source {
                        Source::Local { root_path } => root_path.to_string_lossy().to_string(),
                        _ => continue,
                    };
                    if source_path == config_path {
                        config.last_sync = Some(now);
                        let new_json = serde_json::to_string(&config)?;
                        db.execute(
                            "UPDATE sources SET config_json = ?1, last_sync = ?2 WHERE id = ?3",
                            params![new_json, now.to_rfc3339(), id],
                        )?;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Update source sync interval
    pub async fn set_source_sync_interval(
        &self,
        source: &Source,
        interval_secs: u64,
    ) -> Result<()> {
        let db = self.get_db()?;
        let mut stmt = db.prepare("SELECT id, config_json FROM sources")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (id, config_json) in rows {
            if let Ok(mut config) = serde_json::from_str::<SourceConfig>(&config_json) {
                let matches = match (source, &config.source) {
                    (Source::Local { root_path: a }, Source::Local { root_path: b }) => a == b,
                    _ => false,
                };
                if matches {
                    config.sync_interval_secs = interval_secs;
                    let new_json = serde_json::to_string(&config)?;
                    db.execute(
                        "UPDATE sources SET config_json = ?1 WHERE id = ?2",
                        params![new_json, id],
                    )?;
                    break;
                }
            }
        }
        Ok(())
    }

    // === Cluster Operations ===

    pub async fn list_clusters(&self) -> Result<Vec<Cluster>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT id, name, kind, memory_ids_json, cover_id, auto_generated, created_at, metadata_json
             FROM clusters"
        )?;

        let clusters = stmt
            .query_map([], |row| {
                Ok(Cluster {
                    id: parse_uuid(&row.get::<_, String>(0)?)?,
                    name: row.get(1)?,
                    kind: parse_json(&row.get::<_, String>(2)?)?,
                    memory_ids: parse_json(&row.get::<_, String>(3)?)?,
                    cover_memory_id: row
                        .get::<_, Option<String>>(4)?
                        .map(|s| parse_uuid(&s))
                        .transpose()?,
                    auto_generated: row.get::<_, i32>(5)? != 0,
                    created_at: parse_datetime(&row.get::<_, String>(6)?)?,
                    metadata: parse_json(&row.get::<_, String>(7)?)?,
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

        let db = self.get_db()?;
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
        let db = self.get_db()?;

        let mut stmt = db.prepare("SELECT memory_ids_json FROM clusters WHERE id = ?1")?;
        let current: Vec<MemoryId> = stmt.query_row(params![cluster_id.to_string()], |row| {
            parse_json(&row.get::<_, String>(0)?)
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
        // Get the memory to know its kind
        let memory = match self.get_memory(memory_id).await? {
            Some(m) => m,
            None => return Ok(vec![]),
        };

        // Try Qdrant first (preferred - embeddings stored there)
        if self.qdrant.is_available().await {
            // Get embedding from Qdrant (primary storage)
            if let Some(embedding) = self.qdrant.get_embedding(memory_id, &memory.kind).await? {
                return self
                    .qdrant
                    .find_similar(memory_id, embedding, &memory.kind, limit)
                    .await;
            }
        }

        // Fallback: Try SQLite (for legacy data or offline mode)
        if let Some(embedding) = self.get_embedding(memory_id).await? {
            if self.qdrant.is_available().await {
                return self
                    .qdrant
                    .find_similar(memory_id, embedding, &memory.kind, limit)
                    .await;
            }
            // Final fallback: SQLite-based similarity search
            return self.find_similar_sqlite(memory_id, limit).await;
        }

        Ok(vec![])
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

    /// Store an embedding durably, with a write-through to Qdrant.
    ///
    /// Contract (fixed): **SQLite is the durable source of truth** — the
    /// embedding is ALWAYS written to SQLite. When Qdrant is available we
    /// additionally write it through to Qdrant for fast vector search.
    ///
    /// The previous implementation stored the embedding in *exactly one* place
    /// (Qdrant when up, SQLite when down). If Qdrant availability flapped
    /// between writes, embeddings written while it was up became unreachable
    /// once it went down (and vice-versa), silently losing them. Always writing
    /// to SQLite guarantees durability regardless of Qdrant state; `find_similar`
    /// still prefers Qdrant and falls back to SQLite.
    ///
    /// A Qdrant write-through failure is logged but does NOT fail the call,
    /// because the durable copy already succeeded.
    pub async fn store_embedding_with_qdrant(
        &self,
        memory_id: MemoryId,
        embedding: &[f32],
        model: &str,
        kind: &MemoryKind,
    ) -> Result<()> {
        // Durable write: always persist to SQLite first.
        self.store_embedding(memory_id, embedding, model).await?;

        // Write-through to Qdrant when available (accelerator, not source of truth).
        if self.qdrant.is_available().await {
            if let Err(e) = self
                .qdrant
                .upsert(memory_id, embedding.to_vec(), kind)
                .await
            {
                warn!(
                    "Failed to write embedding through to Qdrant for {} (kept durably in SQLite): {}",
                    memory_id, e
                );
            }
        }

        Ok(())
    }

    // === Chunk Operations ===

    /// Decode a `chunks` row into a [`Chunk`].
    fn row_to_chunk(row: &rusqlite::Row) -> std::result::Result<Chunk, rusqlite::Error> {
        Ok(Chunk {
            chunk_id: row.get(0)?,
            parent_memory_id: parse_uuid(&row.get::<_, String>(1)?)?,
            ordinal: row.get::<_, i64>(2)? as usize,
            text: row.get(3)?,
            char_start: row.get::<_, i64>(4)? as usize,
            char_end: row.get::<_, i64>(5)? as usize,
            embedding_id: row.get(6)?,
        })
    }

    /// Replace all chunk rows for a parent memory in a single transaction.
    ///
    /// Any previously stored chunks (and their SQLite embeddings) for this parent
    /// are deleted first, so a re-index produces no orphaned or duplicated chunks.
    /// Chunk vectors are persisted separately via
    /// [`store_chunk_embedding_with_qdrant`](Self::store_chunk_embedding_with_qdrant).
    pub async fn upsert_chunks(&self, parent_id: MemoryId, chunks: &[Chunk]) -> Result<()> {
        let mut db = self.get_db()?;
        let tx = db.transaction()?;
        let pid = parent_id.to_string();
        tx.execute(
            "DELETE FROM chunks WHERE parent_memory_id = ?1",
            params![pid],
        )?;
        tx.execute(
            "DELETE FROM chunk_embeddings WHERE parent_memory_id = ?1",
            params![pid],
        )?;
        for c in chunks {
            tx.execute(
                r#"INSERT OR REPLACE INTO chunks
                   (chunk_id, parent_memory_id, ordinal, text, char_start, char_end, embedding_id)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                params![
                    c.chunk_id,
                    c.parent_memory_id.to_string(),
                    c.ordinal as i64,
                    c.text,
                    c.char_start as i64,
                    c.char_end as i64,
                    c.embedding_id,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Fetch all chunks for a parent memory, ordered by ordinal.
    pub async fn get_chunks(&self, parent_id: MemoryId) -> Result<Vec<Chunk>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(
            "SELECT chunk_id, parent_memory_id, ordinal, text, char_start, char_end, embedding_id
             FROM chunks WHERE parent_memory_id = ?1 ORDER BY ordinal",
        )?;
        let chunks = stmt
            .query_map(params![parent_id.to_string()], Self::row_to_chunk)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(chunks)
    }

    /// Delete every chunk (rows, SQLite embeddings, and Qdrant points) for a
    /// parent memory. Called when a memory is removed or re-indexed.
    pub async fn delete_chunks(&self, parent_id: MemoryId) -> Result<()> {
        let pid = parent_id.to_string();

        // Collect chunk ids for Qdrant cleanup, then delete the SQLite rows. The
        // guard is scoped so it is dropped before the `await` on Qdrant below.
        let chunk_ids: Vec<String> = {
            let db = self.get_db()?;
            let mut stmt = db.prepare("SELECT chunk_id FROM chunks WHERE parent_memory_id = ?1")?;
            let ids: Vec<String> = stmt
                .query_map(params![pid], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);
            db.execute(
                "DELETE FROM chunks WHERE parent_memory_id = ?1",
                params![pid],
            )?;
            db.execute(
                "DELETE FROM chunk_embeddings WHERE parent_memory_id = ?1",
                params![pid],
            )?;
            ids
        };

        if !chunk_ids.is_empty() && self.qdrant.is_available().await {
            if let Err(e) = self.qdrant.delete_text_points(&chunk_ids).await {
                warn!(
                    "Failed to delete {} chunk vectors from Qdrant for parent {} (SQLite cleaned): {}",
                    chunk_ids.len(),
                    parent_id,
                    e
                );
            }
        }
        Ok(())
    }

    /// Durably store a chunk's embedding (SQLite source of truth) and write it
    /// through to Qdrant `hippo_text` when available, mirroring
    /// [`store_embedding_with_qdrant`](Self::store_embedding_with_qdrant). A
    /// Qdrant failure is logged but does not fail the call.
    pub async fn store_chunk_embedding_with_qdrant(
        &self,
        chunk: &Chunk,
        embedding: &[f32],
    ) -> Result<()> {
        {
            let db = self.get_db()?;
            let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
            db.execute(
                r#"INSERT OR REPLACE INTO chunk_embeddings
                   (chunk_id, parent_memory_id, embedding, model, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5)"#,
                params![
                    chunk.chunk_id,
                    chunk.parent_memory_id.to_string(),
                    embedding_bytes,
                    "chunk_embedding",
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
        }

        if self.qdrant.is_available().await {
            if let Err(e) = self.qdrant.upsert_chunk(chunk, embedding.to_vec()).await {
                warn!(
                    "Failed to write chunk embedding through to Qdrant for {} (kept durably in SQLite): {}",
                    chunk.chunk_id, e
                );
            }
        }
        Ok(())
    }

    /// Chunk-level semantic retrieval with parent roll-up.
    ///
    /// Matches at chunk granularity, groups hits by `parent_memory_id`, and scores
    /// each parent by the configured [`ChunkRollup`] strategy. Memories that have
    /// no chunks (older / non-text) remain retrievable via their whole-document
    /// embedding — the two sources are merged by taking the higher score per
    /// parent, so every memory appears exactly once. Prefers Qdrant when
    /// available and falls back to the durable SQLite copy otherwise.
    pub async fn semantic_rollup(
        &self,
        query: &[f32],
        limit: usize,
        rollup: ChunkRollup,
    ) -> Result<Vec<ChunkParentHit>> {
        if self.qdrant.is_available().await {
            return self.semantic_rollup_qdrant(query, limit, rollup).await;
        }
        self.semantic_rollup_sqlite(query, limit, rollup)
    }

    /// Reduce a parent's sorted-descending chunk scores to a single score.
    fn apply_rollup(sorted_desc: &[f32], rollup: ChunkRollup) -> f32 {
        match rollup {
            ChunkRollup::Max => sorted_desc.first().copied().unwrap_or(0.0),
            ChunkRollup::TopNMean(n) => {
                let take = n.max(1).min(sorted_desc.len());
                if take == 0 {
                    0.0
                } else {
                    sorted_desc[..take].iter().sum::<f32>() / take as f32
                }
            }
        }
    }

    /// SQLite (offline) implementation of [`semantic_rollup`](Self::semantic_rollup).
    fn semantic_rollup_sqlite(
        &self,
        query: &[f32],
        limit: usize,
        rollup: ChunkRollup,
    ) -> Result<Vec<ChunkParentHit>> {
        use std::collections::HashMap;
        let db = self.get_db()?;

        // 1. Score every chunk, grouping scores + best chunk id per parent.
        let mut per_parent: HashMap<MemoryId, Vec<(f32, String)>> = HashMap::new();
        {
            let mut stmt =
                db.prepare("SELECT chunk_id, parent_memory_id, embedding FROM chunk_embeddings")?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok());
            for (chunk_id, parent, bytes) in rows {
                let parent_id = match uuid::Uuid::parse_str(&parent) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let emb = match bytes_to_f32_vec(&bytes) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let score = cosine_similarity(query, &emb);
                per_parent
                    .entry(parent_id)
                    .or_default()
                    .push((score, chunk_id));
            }
        }

        // parent -> (rolled-up score, best chunk id)
        let mut best: HashMap<MemoryId, (f32, Option<String>)> = HashMap::new();
        for (parent, mut scores) in per_parent {
            scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let best_chunk_id = scores.first().map(|(_, id)| id.clone());
            let only_scores: Vec<f32> = scores.iter().map(|(s, _)| *s).collect();
            best.insert(
                parent,
                (Self::apply_rollup(&only_scores, rollup), best_chunk_id),
            );
        }

        // 2. Whole-document embeddings: a memory with no chunk wins keeps its
        //    whole-doc score (fallback); an existing chunk score is only raised.
        {
            let mut stmt = db.prepare("SELECT memory_id, embedding FROM embeddings")?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                })?
                .filter_map(|r| r.ok());
            for (mid, bytes) in rows {
                let id = match uuid::Uuid::parse_str(&mid) {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let emb = match bytes_to_f32_vec(&bytes) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let score = cosine_similarity(query, &emb);
                best.entry(id)
                    .and_modify(|(s, _)| {
                        if score > *s {
                            *s = score;
                        }
                    })
                    .or_insert((score, None));
            }
        }

        // 3. Rank, truncate, then attach the best chunk row for snippet offsets.
        let mut ranked: Vec<(MemoryId, f32, Option<String>)> =
            best.into_iter().map(|(id, (s, c))| (id, s, c)).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);

        let mut out = Vec::with_capacity(ranked.len());
        for (id, score, chunk_id) in ranked {
            let best_chunk = match chunk_id {
                Some(cid) => Self::get_chunk_by_id_in(&db, &cid)?,
                None => None,
            };
            out.push(ChunkParentHit {
                memory_id: id,
                score,
                best_chunk,
            });
        }
        Ok(out)
    }

    /// Fetch a single chunk row by its id using an already-held connection.
    fn get_chunk_by_id_in(db: &Connection, chunk_id: &str) -> Result<Option<Chunk>> {
        let mut stmt = db.prepare(
            "SELECT chunk_id, parent_memory_id, ordinal, text, char_start, char_end, embedding_id
             FROM chunks WHERE chunk_id = ?1",
        )?;
        match stmt.query_row(params![chunk_id], Self::row_to_chunk) {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Qdrant-backed implementation of [`semantic_rollup`](Self::semantic_rollup).
    ///
    /// `hippo_text` holds both chunk points (payload `parent_memory_id`) and the
    /// legacy whole-document points (no parent payload, id == memory id). Both are
    /// grouped under their parent memory id, so chunked and non-chunked memories
    /// roll up uniformly and each memory appears once.
    async fn semantic_rollup_qdrant(
        &self,
        query: &[f32],
        limit: usize,
        rollup: ChunkRollup,
    ) -> Result<Vec<ChunkParentHit>> {
        use std::collections::HashMap;

        // Over-fetch so grouping by parent still yields `limit` distinct parents.
        let fetch = (limit * 6).max(limit + 10);
        let raw = self
            .qdrant
            .search_text_with_parents(query.to_vec(), fetch)
            .await?;

        let mut per_parent: HashMap<MemoryId, Vec<(f32, Option<String>)>> = HashMap::new();
        for (parent, chunk_id, score) in raw {
            per_parent
                .entry(parent)
                .or_default()
                .push((score, chunk_id));
        }

        let mut ranked: Vec<(MemoryId, f32, Option<String>)> = per_parent
            .into_iter()
            .map(|(parent, mut scores)| {
                scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                let best_chunk_id = scores.first().and_then(|(_, c)| c.clone());
                let only_scores: Vec<f32> = scores.iter().map(|(s, _)| *s).collect();
                (
                    parent,
                    Self::apply_rollup(&only_scores, rollup),
                    best_chunk_id,
                )
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);

        let db = self.get_db()?;
        let mut out = Vec::with_capacity(ranked.len());
        for (id, score, chunk_id) in ranked {
            let best_chunk = match chunk_id {
                Some(cid) => Self::get_chunk_by_id_in(&db, &cid)?,
                None => None,
            };
            out.push(ChunkParentHit {
                memory_id: id,
                score,
                best_chunk,
            });
        }
        Ok(out)
    }

    // === Fast SQL Search ===

    /// Search memories using FTS5 full-text search (5-10x faster than LIKE)
    /// Falls back to LIKE search if FTS table doesn't exist
    pub async fn search_fts(
        &self,
        query: &str,
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        // Prepare query for FTS5 - escape special characters and add wildcards
        let fts_query = query
            .split_whitespace()
            .map(|word| format!("\"{}\"*", word.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");

        let sql = if kind.is_some() {
            r#"SELECT m.id, m.path, m.source_json, m.kind_json, m.metadata_json, m.tags_json,
                      m.embedding_id, m.connections_json, m.is_favorite, m.created_at, m.modified_at, m.indexed_at
               FROM memories m
               JOIN memories_fts fts ON m.id = fts.id
               WHERE memories_fts MATCH ?1 AND m.kind_name = ?2
               ORDER BY rank
               LIMIT ?3 OFFSET ?4"#
        } else {
            r#"SELECT m.id, m.path, m.source_json, m.kind_json, m.metadata_json, m.tags_json,
                      m.embedding_id, m.connections_json, m.is_favorite, m.created_at, m.modified_at, m.indexed_at
               FROM memories m
               JOIN memories_fts fts ON m.id = fts.id
               WHERE memories_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2 OFFSET ?3"#
        };

        // IMPORTANT: scope the MutexGuard to this block so it is DROPPED before
        // the `self.search_like(...).await` fallback below. `get_db()` locks a
        // non-reentrant `std::sync::Mutex`; if the guard were still held when
        // `search_like` called `get_db()` again on the same thread, it would
        // deadlock permanently — which happened on every FTS query returning
        // zero rows.
        let result: std::result::Result<Vec<Memory>, rusqlite::Error> = {
            let db = self.get_db()?;
            (|| {
                let mut stmt = db.prepare(sql)?;
                let memories: Vec<Memory> = if let Some(k) = kind {
                    stmt.query_map(
                        params![fts_query, k, limit as i64, offset as i64],
                        Self::row_to_memory,
                    )?
                    .filter_map(|r| r.ok())
                    .collect()
                } else {
                    stmt.query_map(
                        params![fts_query, limit as i64, offset as i64],
                        Self::row_to_memory,
                    )?
                    .filter_map(|r| r.ok())
                    .collect()
                };
                Ok(memories)
            })()
        };

        // Fallback to LIKE search if FTS fails or matches nothing.
        match result {
            Ok(memories) if !memories.is_empty() => Ok(memories),
            _ => self.search_like(query, kind, limit, offset).await,
        }
    }

    /// Search using FTS5 with BM25 ranking and snippet generation
    /// Returns results with highlighted snippets and relevance scores
    pub async fn search_fts5_with_snippets(
        &self,
        fts_query: &str,
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Fts5SearchResult>> {
        let db = self.get_db()?;

        // BM25 weights: id=0, title=10, filename=8, tags=7, text_preview=5
        let sql = if kind.is_some() {
            r#"SELECT m.id, m.path, m.source_json, m.kind_json, m.metadata_json, m.tags_json,
                      m.embedding_id, m.connections_json, m.is_favorite, m.created_at, m.modified_at, m.indexed_at,
                      bm25(memories_fts, 0.0, 10.0, 8.0, 7.0, 5.0) as rank,
                      snippet(memories_fts, 1, '<mark>', '</mark>', '...', 30) as title_snip,
                      snippet(memories_fts, 2, '<mark>', '</mark>', '...', 30) as filename_snip,
                      snippet(memories_fts, 3, '<mark>', '</mark>', '...', 30) as tags_snip,
                      snippet(memories_fts, 4, '<mark>', '</mark>', '...', 40) as content_snip
               FROM memories m
               JOIN memories_fts fts ON m.id = fts.id
               WHERE memories_fts MATCH ?1 AND m.kind_name = ?2
               ORDER BY rank
               LIMIT ?3 OFFSET ?4"#
        } else {
            r#"SELECT m.id, m.path, m.source_json, m.kind_json, m.metadata_json, m.tags_json,
                      m.embedding_id, m.connections_json, m.is_favorite, m.created_at, m.modified_at, m.indexed_at,
                      bm25(memories_fts, 0.0, 10.0, 8.0, 7.0, 5.0) as rank,
                      snippet(memories_fts, 1, '<mark>', '</mark>', '...', 30) as title_snip,
                      snippet(memories_fts, 2, '<mark>', '</mark>', '...', 30) as filename_snip,
                      snippet(memories_fts, 3, '<mark>', '</mark>', '...', 30) as tags_snip,
                      snippet(memories_fts, 4, '<mark>', '</mark>', '...', 40) as content_snip
               FROM memories m
               JOIN memories_fts fts ON m.id = fts.id
               WHERE memories_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2 OFFSET ?3"#
        };

        let result: std::result::Result<Vec<Fts5SearchResult>, rusqlite::Error> = (|| {
            let mut stmt = db.prepare(sql)?;
            let results: Vec<Fts5SearchResult> = if let Some(k) = kind {
                stmt.query_map(params![fts_query, k, limit as i64, offset as i64], |row| {
                    Ok(Fts5SearchResult {
                        memory: Self::row_to_memory(row)?,
                        fts5_rank: row.get(12)?,
                        title_snippet: row.get(13)?,
                        filename_snippet: row.get(14)?,
                        tags_snippet: row.get(15)?,
                        content_snippet: row.get(16)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect()
            } else {
                stmt.query_map(params![fts_query, limit as i64, offset as i64], |row| {
                    Ok(Fts5SearchResult {
                        memory: Self::row_to_memory(row)?,
                        fts5_rank: row.get(12)?,
                        title_snippet: row.get(13)?,
                        filename_snippet: row.get(14)?,
                        tags_snippet: row.get(15)?,
                        content_snippet: row.get(16)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect()
            };
            Ok(results)
        })();

        // Distinguish "no results" from a genuine failure: propagate (and log)
        // errors instead of masquerading them as an empty result set.
        match result {
            Ok(results) => Ok(results),
            Err(e) => {
                warn!("FTS5 snippet search failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Count FTS5 search results for pagination
    pub async fn count_fts5_results(&self, fts_query: &str, kind: Option<&str>) -> Result<usize> {
        let db = self.get_db()?;

        let result: std::result::Result<usize, rusqlite::Error> = if let Some(k) = kind {
            db.query_row(
                r#"SELECT COUNT(*) FROM memories m
                   JOIN memories_fts fts ON m.id = fts.id
                   WHERE memories_fts MATCH ?1 AND m.kind_name = ?2"#,
                params![fts_query, k],
                |row| row.get(0),
            )
        } else {
            db.query_row(
                r#"SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH ?1"#,
                params![fts_query],
                |row| row.get(0),
            )
        };

        // Propagate real failures instead of reporting them as a zero count,
        // which would silently misrepresent pagination totals.
        match result {
            Ok(n) => Ok(n),
            Err(e) => {
                warn!("FTS5 result count failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Rebuild the FTS5 index from scratch
    pub async fn rebuild_fts_index(&self) -> Result<usize> {
        let db = self.get_db()?;

        // Delete all FTS entries
        let _ = db.execute(
            "INSERT INTO memories_fts(memories_fts) VALUES('delete-all')",
            [],
        );

        // Repopulate from memories table
        let count: usize = db
            .execute(
                r#"INSERT INTO memories_fts(rowid, id, title, filename, tags_text, text_preview)
               SELECT rowid, id, title, filename, tags_text, text_preview FROM memories
               WHERE title IS NOT NULL OR filename IS NOT NULL"#,
                [],
            )
            .unwrap_or(0);

        info!("Rebuilt FTS5 index with {} entries", count);
        Ok(count)
    }

    /// Search memories using LIKE pattern matching (fallback when FTS fails)
    pub async fn search_like(
        &self,
        query: &str,
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        let db = self.get_db()?;

        // Escape LIKE wildcards so a query such as "50%" or "foo_bar" matches
        // literally instead of being treated as a pattern.
        let pattern = format!("%{}%", escape_like(&query.to_lowercase()));

        let sql = if kind.is_some() {
            r#"SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                      embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
               FROM memories
               WHERE kind_name = ?1 AND (
                   LOWER(filename) LIKE ?2 ESCAPE '\' OR
                   LOWER(title) LIKE ?2 ESCAPE '\' OR
                   LOWER(path) LIKE ?2 ESCAPE '\' OR
                   LOWER(tags_text) LIKE ?2 ESCAPE '\'
               )
               ORDER BY modified_at DESC
               LIMIT ?3 OFFSET ?4"#
        } else {
            r#"SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                      embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
               FROM memories
               WHERE LOWER(filename) LIKE ?1 ESCAPE '\' OR
                     LOWER(title) LIKE ?1 ESCAPE '\' OR
                     LOWER(path) LIKE ?1 ESCAPE '\' OR
                     LOWER(tags_text) LIKE ?1 ESCAPE '\'
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

    /// Helper to convert a row to `Memory`.
    ///
    /// Columns MUST be selected in this order: id, path, source_json,
    /// kind_json, metadata_json, tags_json, embedding_id, connections_json,
    /// is_favorite, created_at, modified_at, indexed_at. A corrupt row yields a
    /// rusqlite error (via the parse helpers) instead of panicking and
    /// poisoning the DB mutex.
    fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
        Ok(Memory {
            id: parse_uuid(&row.get::<_, String>(0)?)?,
            path: PathBuf::from(row.get::<_, String>(1)?),
            source: parse_json(&row.get::<_, String>(2)?)?,
            kind: parse_json(&row.get::<_, String>(3)?)?,
            metadata: parse_json(&row.get::<_, String>(4)?)?,
            tags: parse_json(&row.get::<_, String>(5)?)?,
            embedding_id: row.get(6)?,
            connections: parse_json(&row.get::<_, String>(7)?)?,
            is_favorite: row.get::<_, i32>(8).unwrap_or(0) == 1,
            created_at: parse_datetime(&row.get::<_, String>(9)?)?,
            modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
            indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
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
        let db = self.get_db()?;

        // Build dynamic SQL based on filters
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Text search condition (LIKE wildcards in user input are escaped)
        if let Some(q) = query {
            if !q.trim().is_empty() {
                let pattern = format!("%{}%", escape_like(&q.to_lowercase()));
                conditions.push("(LOWER(filename) LIKE ? ESCAPE '\\' OR LOWER(title) LIKE ? ESCAPE '\\' OR LOWER(path) LIKE ? ESCAPE '\\' OR LOWER(tags_text) LIKE ? ESCAPE '\\')".to_string());
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }
        }

        // Tag filters
        for tag in tags {
            conditions.push("LOWER(tags_text) LIKE ? ESCAPE '\\'".to_string());
            params_vec.push(Box::new(format!("%{}%", escape_like(&tag.to_lowercase()))));
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
            .query_map(params_refs.as_slice(), Self::row_to_memory)?
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
        let db = self.get_db()?;

        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(q) = query {
            if !q.trim().is_empty() {
                let pattern = format!("%{}%", escape_like(&q.to_lowercase()));
                conditions.push("(LOWER(filename) LIKE ? ESCAPE '\\' OR LOWER(title) LIKE ? ESCAPE '\\' OR LOWER(path) LIKE ? ESCAPE '\\' OR LOWER(tags_text) LIKE ? ESCAPE '\\')".to_string());
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }
        }

        for tag in tags {
            conditions.push("LOWER(tags_text) LIKE ? ESCAPE '\\'".to_string());
            params_vec.push(Box::new(format!("%{}%", escape_like(&tag.to_lowercase()))));
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
        let db = self.get_db()?;

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
        let db = self.get_db()?;

        let mut stmt = db.prepare("SELECT embedding FROM embeddings WHERE memory_id = ?1")?;
        let result = stmt.query_row(params![memory_id.to_string()], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        });

        match result {
            // Decode the blob, returning an error on a corrupt (non-multiple-of-4)
            // length rather than silently zeroing a partial chunk.
            Ok(bytes) => Ok(Some(bytes_to_f32_vec(&bytes)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all embeddings for similarity search
    pub async fn get_all_embeddings(&self) -> Result<Vec<(MemoryId, Vec<f32>)>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare("SELECT memory_id, embedding FROM embeddings")?;
        let rows: Vec<(String, Vec<u8>)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Decode each row, skipping (with a warning) any with an invalid UUID or
        // corrupt blob rather than panicking or silently zeroing bytes.
        let mut embeddings = Vec::with_capacity(rows.len());
        for (id_str, bytes) in rows {
            let id = match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => id,
                Err(e) => {
                    warn!(
                        "Skipping embedding with invalid memory id {}: {}",
                        id_str, e
                    );
                    continue;
                }
            };
            match bytes_to_f32_vec(&bytes) {
                Ok(floats) => embeddings.push((id, floats)),
                Err(e) => warn!("Skipping corrupt embedding for {}: {}", id, e),
            }
        }

        Ok(embeddings)
    }

    // === Stats ===

    /// Return EVERY memory in the index.
    ///
    /// This path previously capped at `LIMIT 5000`, which silently presented a
    /// truncated view as complete — losing data for callers like `export_index`
    /// and `find_duplicates` past 5000 files. The cap is removed so the
    /// full-table iteration is actually complete. For bounded/UI access, use
    /// `get_all_memories_paginated`.
    pub async fn get_all_memories(&self) -> Result<Vec<Memory>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories ORDER BY modified_at DESC",
        )?;

        let memories = stmt
            .query_map([], Self::row_to_memory)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Return a page of memories ordered by most-recently-modified.
    ///
    /// Use this instead of `get_all_memories` when a caller only needs a window
    /// (e.g. UI lists); it avoids loading the entire table into memory.
    pub async fn get_all_memories_paginated(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>> {
        let db = self.get_db()?;

        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories ORDER BY modified_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let memories = stmt
            .query_map(params![limit as i64, offset as i64], Self::row_to_memory)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    pub async fn stats(&self) -> Result<IndexStats> {
        let db = self.get_db()?;

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
            let db = self.get_db()?;

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

    // === Saved Searches ===

    pub async fn save_search(
        &self,
        id: &str,
        name: &str,
        query: &str,
        tags: &[String],
        filters: &serde_json::Value,
    ) -> Result<()> {
        let db = self.get_db()?;
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "INSERT OR REPLACE INTO saved_searches (id, name, query, tags_json, filters_json, created_at, last_used_at, use_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 0)",
            params![
                id,
                name,
                query,
                serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(filters).unwrap_or_else(|_| "{}".to_string()),
                now
            ],
        )?;
        Ok(())
    }

    pub async fn list_saved_searches(&self) -> Result<Vec<serde_json::Value>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(
            "SELECT id, name, query, tags_json, filters_json, created_at, last_used_at, use_count
             FROM saved_searches ORDER BY use_count DESC, last_used_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_str: String = row.get(3)?;
            let filters_str: String = row.get(4)?;
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "query": row.get::<_, String>(2)?,
                "tags": serde_json::from_str::<serde_json::Value>(&tags_str).unwrap_or(serde_json::json!([])),
                "filters": serde_json::from_str::<serde_json::Value>(&filters_str).unwrap_or(serde_json::json!({})),
                "created_at": row.get::<_, String>(5)?,
                "last_used_at": row.get::<_, Option<String>>(6)?,
                "use_count": row.get::<_, i64>(7)?,
            }))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub async fn delete_saved_search(&self, id: &str) -> Result<()> {
        let db = self.get_db()?;
        db.execute("DELETE FROM saved_searches WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub async fn use_saved_search(&self, id: &str) -> Result<()> {
        let db = self.get_db()?;
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "UPDATE saved_searches SET use_count = use_count + 1, last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // === Search History ===

    pub async fn add_search_history(&self, query: &str, result_count: usize) -> Result<()> {
        let db = self.get_db()?;
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO search_history (query, result_count, searched_at) VALUES (?1, ?2, ?3)",
            params![query, result_count as i64, now],
        )?;
        // Keep max 50 entries
        db.execute(
            "DELETE FROM search_history WHERE id NOT IN (SELECT id FROM search_history ORDER BY searched_at DESC LIMIT 50)",
            [],
        )?;
        Ok(())
    }

    pub async fn get_search_history(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(
            "SELECT id, query, result_count, searched_at FROM search_history ORDER BY searched_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "query": row.get::<_, String>(1)?,
                "result_count": row.get::<_, i64>(2)?,
                "searched_at": row.get::<_, String>(3)?,
            }))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub async fn clear_search_history(&self) -> Result<()> {
        let db = self.get_db()?;
        db.execute("DELETE FROM search_history", [])?;
        Ok(())
    }

    pub async fn delete_search_history_entry(&self, id: i64) -> Result<()> {
        if id <= 0 {
            return Err(crate::error::HippoError::Other(
                "Invalid history entry ID".to_string(),
            ));
        }
        let db = self.get_db()?;
        db.execute("DELETE FROM search_history WHERE id = ?1", params![id])?;
        Ok(())
    }

    // === Recent Files ===

    pub async fn get_recent_files(&self, limit: usize, days: usize) -> Result<Vec<Memory>> {
        let db = self.get_db()?;
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE indexed_at >= ?1 ORDER BY indexed_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![cutoff, limit as i64], |row| {
            Ok(Memory {
                id: parse_uuid(&row.get::<_, String>(0)?)?,
                path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                source: parse_json(&row.get::<_, String>(2)?)?,
                kind: parse_json(&row.get::<_, String>(3)?)?,
                metadata: parse_json(&row.get::<_, String>(4)?)?,
                tags: parse_json(&row.get::<_, String>(5)?)?,
                embedding_id: row.get(6)?,
                connections: parse_json(&row.get::<_, String>(7)?)?,
                is_favorite: row.get::<_, i32>(8)? != 0,
                created_at: parse_datetime(&row.get::<_, String>(9)?)?,
                modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
                indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
            })
        })?;
        Ok(rows.flatten().collect())
    }

    pub async fn get_recently_modified(&self, limit: usize, days: usize) -> Result<Vec<Memory>> {
        let db = self.get_db()?;
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = db.prepare(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories WHERE modified_at >= ?1 ORDER BY modified_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![cutoff, limit as i64], |row| {
            Ok(Memory {
                id: parse_uuid(&row.get::<_, String>(0)?)?,
                path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                source: parse_json(&row.get::<_, String>(2)?)?,
                kind: parse_json(&row.get::<_, String>(3)?)?,
                metadata: parse_json(&row.get::<_, String>(4)?)?,
                tags: parse_json(&row.get::<_, String>(5)?)?,
                embedding_id: row.get(6)?,
                connections: parse_json(&row.get::<_, String>(7)?)?,
                is_favorite: row.get::<_, i32>(8)? != 0,
                created_at: parse_datetime(&row.get::<_, String>(9)?)?,
                modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
                indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
            })
        })?;
        Ok(rows.flatten().collect())
    }

    // === Batch Rename ===

    pub async fn rename_memory(&self, memory_id: &str, new_path: &std::path::Path) -> Result<()> {
        let db = self.get_db()?;
        let new_filename = new_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_extension = new_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_title = new_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        db.execute(
            "UPDATE memories SET path = ?1, filename = ?2, extension = ?3, title = ?4 WHERE id = ?5",
            params![
                new_path.to_string_lossy(),
                new_filename,
                new_extension,
                new_title,
                memory_id
            ],
        )?;
        Ok(())
    }

    // === Paginated Search ===

    pub async fn search_paginated(
        &self,
        query: &str,
        tags: &[String],
        kind: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Memory>, usize)> {
        let db = self.get_db()?;

        // Build WHERE clause
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !query.is_empty() {
            conditions.push(
                "(title LIKE ?1 ESCAPE '\\' OR filename LIKE ?1 ESCAPE '\\' OR tags_text LIKE ?1 ESCAPE '\\' OR path LIKE ?1 ESCAPE '\\')".to_string(),
            );
            param_values.push(Box::new(format!("%{}%", escape_like(query))));
        }

        for (i, tag) in tags.iter().enumerate() {
            let param_idx = param_values.len() + 1;
            conditions.push(format!("tags_text LIKE ?{} ESCAPE '\\'", param_idx));
            param_values.push(Box::new(format!("%{}%", escape_like(tag))));
            let _ = i; // silence unused warning
        }

        if let Some(k) = kind {
            let param_idx = param_values.len() + 1;
            conditions.push(format!("kind_name = ?{}", param_idx));
            param_values.push(Box::new(k.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count
        let count_sql = format!("SELECT COUNT(*) FROM memories {}", where_clause);
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let total: usize = db.query_row(&count_sql, params_ref.as_slice(), |r| r.get(0))?;

        // Get paginated results
        let limit_idx = param_values.len() + 1;
        let offset_idx = param_values.len() + 2;
        let select_sql = format!(
            "SELECT id, path, source_json, kind_json, metadata_json, tags_json,
                    embedding_id, connections_json, is_favorite, created_at, modified_at, indexed_at
             FROM memories {} ORDER BY modified_at DESC LIMIT ?{} OFFSET ?{}",
            where_clause, limit_idx, offset_idx
        );
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));
        let params_ref2: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = db.prepare(&select_sql)?;
        let rows = stmt.query_map(params_ref2.as_slice(), |row| {
            Ok(Memory {
                id: parse_uuid(&row.get::<_, String>(0)?)?,
                path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                source: parse_json(&row.get::<_, String>(2)?)?,
                kind: parse_json(&row.get::<_, String>(3)?)?,
                metadata: parse_json(&row.get::<_, String>(4)?)?,
                tags: parse_json(&row.get::<_, String>(5)?)?,
                embedding_id: row.get(6)?,
                connections: parse_json(&row.get::<_, String>(7)?)?,
                is_favorite: row.get::<_, i32>(8)? != 0,
                created_at: parse_datetime(&row.get::<_, String>(9)?)?,
                modified_at: parse_datetime(&row.get::<_, String>(10)?)?,
                indexed_at: parse_datetime(&row.get::<_, String>(11)?)?,
            })
        })?;
        let results: Vec<Memory> = rows.flatten().collect();
        Ok((results, total))
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
    fn test_bytes_to_f32_vec_roundtrip() {
        let original = vec![1.0f32, -2.5, 3.25, 0.0];
        let bytes: Vec<u8> = original.iter().flat_map(|f| f.to_le_bytes()).collect();
        let decoded = bytes_to_f32_vec(&bytes).expect("valid blob should decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_bytes_to_f32_vec_corrupt_length_errors() {
        // 3 bytes is not a multiple of 4 -> must be an error, not a zeroed chunk.
        let bytes = vec![1u8, 2, 3];
        assert!(bytes_to_f32_vec(&bytes).is_err());
        // 5 bytes (one full f32 + a dangling byte) is also corrupt.
        assert!(bytes_to_f32_vec(&[0u8, 0, 0, 0, 7]).is_err());
    }

    #[test]
    fn test_escape_like() {
        assert_eq!(escape_like("plain"), "plain");
        assert_eq!(escape_like("50%"), "50\\%");
        assert_eq!(escape_like("foo_bar"), "foo\\_bar");
        assert_eq!(escape_like("a\\b"), "a\\\\b");
    }

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
