//! Unified database module for Ruri
//!
//! All features share a single `ruri.db` SQLite database.
//! Different functionalities use different tables within the same database.
//!
//! Tables:
//! - `conversations` / `messages` — Chat conversation history
//! - `mcp_servers`                — MCP server configurations
//! - `knowledge_bases`           — Knowledge base definitions
//! - `kb_documents`              — Documents within knowledge bases
//! - `kb_chunks`                 — Document chunks with embeddings for vector search
//! - Future tables can be added here as needed

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::info;

/// Name of the unified database file.
pub const DB_FILENAME: &str = "ruri.db";

/// Returns the path to the unified database file: `<config_dir>/ruri.db`
pub fn database_path() -> PathBuf {
    let config_dir = dirs::home_dir()
        .map(|h| h.join(".ruri"))
        .unwrap_or_else(|| PathBuf::from(".ruri"));
    config_dir.join(DB_FILENAME)
}

/// Open (or create) the unified `ruri.db` and ensure that every required table exists.
///
/// This is the **single** entry-point for obtaining a database connection pool.
/// All sub-modules (`conversation`, `mcp`, …) receive a clone of the same `SqlitePool`.
pub async fn init(db_path: PathBuf) -> Result<SqlitePool> {
    // Make sure the parent directory exists
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
    }

    let connect_options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5));

    info!("Connecting to database at: {:?}", db_path);

    let pool = SqlitePoolOptions::new()
        .max_connections(10) // shared pool — a bit larger than before
        .connect_with(connect_options)
        .await
        .context("Failed to connect to database")?;

    // Create all tables in one transaction so that the schema is always consistent
    init_schema(&pool).await?;

    info!("Database schema initialized successfully ({:?})", db_path);
    Ok(pool)
}

/// Create every table that the application needs.
///
/// Each feature-area is responsible for declaring its own `CREATE TABLE IF NOT EXISTS`
/// in the `init_schema` call so that the whole schema is managed centrally.
async fn init_schema(pool: &SqlitePool) -> Result<()> {
    // ─── Conversations ───────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            bot_name TEXT NOT NULL,
            chat_type TEXT NOT NULL,
            chat_id TEXT NOT NULL,
            title TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_conversations_bot_name  ON conversations(bot_name);
        CREATE INDEX IF NOT EXISTS idx_conversations_chat_type ON conversations(chat_type);
        CREATE INDEX IF NOT EXISTS idx_conversations_chat_id   ON conversations(chat_id);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create conversations table")?;

    // ─── Messages ────────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create messages table")?;

    // ─── MCP Servers ─────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            transport_type TEXT NOT NULL,
            transport_config TEXT NOT NULL,
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create mcp_servers table")?;

    // ─── Knowledge Bases ──────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS knowledge_bases (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            embedding_provider_config TEXT NOT NULL,
            rerank_provider_config TEXT,
            chunk_size INTEGER NOT NULL DEFAULT 512,
            chunk_overlap INTEGER NOT NULL DEFAULT 64,
            document_count INTEGER NOT NULL DEFAULT 0,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create knowledge_bases table")?;

    // ─── KB Documents ─────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kb_documents (
            id TEXT PRIMARY KEY,
            knowledge_base_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            file_size INTEGER NOT NULL DEFAULT 0,
            file_type TEXT NOT NULL DEFAULT '',
            content_hash TEXT NOT NULL DEFAULT '',
            chunk_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create kb_documents table")?;

    // ─── KB Chunks ────────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kb_chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            knowledge_base_id TEXT NOT NULL,
            content TEXT NOT NULL,
            chunk_index INTEGER NOT NULL DEFAULT 0,
            start_char INTEGER NOT NULL DEFAULT 0,
            end_char INTEGER NOT NULL DEFAULT 0,
            embedding BLOB,
            created_at TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES kb_documents(id) ON DELETE CASCADE,
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_kb_chunks_knowledge_base_id ON kb_chunks(knowledge_base_id);
        CREATE INDEX IF NOT EXISTS idx_kb_chunks_document_id ON kb_chunks(document_id);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create kb_chunks table")?;

    Ok(())
}
