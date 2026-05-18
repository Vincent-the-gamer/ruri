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
//! - `users`                     — User accounts for WebUI authentication
//! - `sessions`                  — User login sessions
//! - `shell_command_blacklist`   — Shell command blacklist patterns

use anyhow::{Context, Result};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::path::PathBuf;
use tracing::info;

/// Name of the unified database file.
pub const DB_FILENAME: &str = "ruri.db";

/// Default shell command blacklist patterns used to seed the database
/// when the table is empty on first initialization.
pub fn default_shell_blacklist_patterns() -> Vec<String> {
    vec![
        // ── Linux / macOS ──
        "sudo ".to_string(),
        "rm -rf".to_string(),
        "dd if=".to_string(),
        "mkfs.".to_string(),
        ":(){ :|:& };:".to_string(),
        "chmod 777".to_string(),
        "chown -R".to_string(),
        "> /dev/sda".to_string(),
        "mv /* ".to_string(),
        "| sh".to_string(),
        "| bash".to_string(),
        "fdisk".to_string(),
        "parted".to_string(),
        "shutdown".to_string(),
        "reboot".to_string(),
        "halt".to_string(),
        "poweroff".to_string(),
        "init 0".to_string(),
        "init 6".to_string(),
        "kill -9".to_string(),
        "pkill".to_string(),
        "killall".to_string(),
        "iptables -F".to_string(),
        "ufw disable".to_string(),
        "systemctl disable".to_string(),
        "modprobe -r".to_string(),
        "rmmod".to_string(),
        "diskutil eraseDisk".to_string(),
        "diskutil unmount".to_string(),
        "hdiutil".to_string(),
        "launchctl unload".to_string(),
        "csrutil disable".to_string(),
        "fdesetup".to_string(),
        "softwareupdate".to_string(),
        // ── Windows ──
        "format ".to_string(),
        "del /f /s".to_string(),
        "rmdir /s".to_string(),
        "diskpart".to_string(),
        "reg delete".to_string(),
        "reg add".to_string(),
        "bcdedit".to_string(),
        "icacls ".to_string(),
        "takeown".to_string(),
        "cipher /w".to_string(),
        "sc delete".to_string(),
        "sc stop".to_string(),
        "net stop".to_string(),
        "Remove-Item -Force -Recurse".to_string(),
        "Set-ExecutionPolicy".to_string(),
        "Stop-Process -Force".to_string(),
        "Clear-RecycleBin".to_string(),
        "Disable-WindowsOptionalFeature".to_string(),
        "Reset-ComputerMachinePassword".to_string(),
    ]
}

/// Returns the path to the unified database file: `<config_dir>/ruri.db`
pub fn database_path() -> PathBuf {
    let config_dir = dirs::home_dir()
        .map(|h| h.join(".ruri"))
        .unwrap_or_else(|| PathBuf::from(".ruri"));
    config_dir.join(DB_FILENAME)
}

/// Hash a password using Argon2id with a randomly generated salt.
/// Returns the full PHC string which includes the salt, so we don't need to store it separately.
pub fn hash_password(password: &str) -> Result<String> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    Ok(hash.to_string())
}

/// Verify a password against a stored Argon2 hash string.
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let argon2 = Argon2::default();
    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
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

    // Seed default user if table is empty
    seed_default_user(&pool).await?;

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
            tags TEXT,
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

    // ─── KB Documents: tags migration ─────────────────────────────
    // Add tags column if it doesn't exist yet
    {
        let columns: Vec<String> =
            sqlx::query_as("SELECT name FROM pragma_table_info('kb_documents')")
                .fetch_all(pool)
                .await
                .context("Failed to check kb_documents table columns")?
                .into_iter()
                .map(|row: (String,)| row.0)
                .collect();

        if !columns.iter().any(|c| c == "tags") {
            sqlx::query("ALTER TABLE kb_documents ADD COLUMN tags TEXT")
                .execute(pool)
                .await
                .context("Failed to add tags column to kb_documents table")?;
            info!("Added tags column to kb_documents table");
        }
    }

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

    // ─── Users ────────────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            must_change_password INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create users table")?;

    // ─── Users: avatar_url migration ────────────────────────────
    // Add avatar_url column if it doesn't exist yet
    {
        let columns: Vec<String> = sqlx::query_as("SELECT name FROM pragma_table_info('users')")
            .fetch_all(pool)
            .await
            .context("Failed to check users table columns")?
            .into_iter()
            .map(|row: (String,)| row.0)
            .collect();

        if !columns.iter().any(|c| c == "avatar_url") {
            sqlx::query("ALTER TABLE users ADD COLUMN avatar_url TEXT")
                .execute(pool)
                .await
                .context("Failed to add avatar_url column to users table")?;
            info!("Added avatar_url column to users table");
        }
    }

    // ─── Sessions ───────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create sessions table")?;

    // ─── Shell Command Blacklist ────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS shell_command_blacklist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create shell_command_blacklist table")?;

    Ok(())
}

/// Seed the default user if the users table is empty
async fn seed_default_user(pool: &SqlitePool) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .context("Failed to count users")?;

    if count == 0 {
        let default_id = uuid::Uuid::new_v4().to_string();
        let default_username = "ruri";
        let default_password = "ruri";
        let now = chrono::Utc::now().to_rfc3339();

        let password_hash =
            hash_password(default_password).context("Failed to hash default password")?;

        sqlx::query(
            r#"
            INSERT INTO users (id, username, password_hash, must_change_password, created_at, updated_at)
            VALUES (?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&default_id)
        .bind(default_username)
        .bind(&password_hash)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .context("Failed to seed default user")?;

        info!("Default user 'ruri' created with password 'ruri' (must change on first login)");
    }

    Ok(())
}

/// Seed the default shell command blacklist if the table is empty.
/// Returns the current list of blacklist patterns (either seeded defaults or existing ones).
pub async fn seed_shell_blacklist(pool: &SqlitePool) -> Result<Vec<String>> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shell_command_blacklist")
        .fetch_one(pool)
        .await
        .context("Failed to count shell_command_blacklist")?;

    if count == 0 {
        let now = chrono::Utc::now().to_rfc3339();
        let defaults = default_shell_blacklist_patterns();

        for pattern in &defaults {
            sqlx::query("INSERT INTO shell_command_blacklist (pattern, created_at) VALUES (?, ?)")
                .bind(pattern)
                .bind(&now)
                .execute(pool)
                .await
                .context("Failed to seed shell command blacklist pattern")?;
        }

        info!(
            "Seeded shell command blacklist with {} default patterns",
            defaults.len()
        );
        Ok(defaults)
    } else {
        // Load existing patterns from DB
        get_all_blacklist_patterns(pool).await
    }
}

/// Get all shell command blacklist patterns from the database.
pub async fn get_all_blacklist_patterns(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT pattern FROM shell_command_blacklist ORDER BY id")
            .fetch_all(pool)
            .await
            .context("Failed to fetch shell command blacklist")?;

    Ok(rows.into_iter().map(|(p,)| p).collect())
}

/// Replace the entire shell command blacklist in the database.
/// This clears all existing entries and inserts the new ones in a transaction.
pub async fn replace_blacklist_patterns(
    pool: &SqlitePool,
    patterns: &[String],
) -> Result<Vec<String>> {
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin transaction for blacklist update")?;

    sqlx::query("DELETE FROM shell_command_blacklist")
        .execute(&mut *tx)
        .await
        .context("Failed to clear shell command blacklist")?;

    for pattern in patterns {
        sqlx::query("INSERT INTO shell_command_blacklist (pattern, created_at) VALUES (?, ?)")
            .bind(pattern)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .context("Failed to insert blacklist pattern")?;
    }

    tx.commit()
        .await
        .context("Failed to commit blacklist transaction")?;

    info!(
        "Updated shell command blacklist with {} patterns",
        patterns.len()
    );

    Ok(patterns.to_vec())
}
