//! opens SQLite databases and applies pending schema migrations.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tracing::info;

use hagitori_core::error::{HagitoriError, Result};

// ---------------------------------------------------------------------------
// Schema version tracking
// ---------------------------------------------------------------------------

const SCHEMA_VERSIONS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS schema_versions (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// A migration is a (version, sql) pair. Versions must be sequential.
type Migration = (i64, &'static str);

// ---------------------------------------------------------------------------
// per-database migrations
// ---------------------------------------------------------------------------

const CONFIG_MIGRATIONS: &[Migration] = &[(
    1,
    "
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
",
)];

const SESSIONS_MIGRATIONS: &[Migration] = &[(
    1,
    "
CREATE TABLE IF NOT EXISTS sessions (
    domain TEXT PRIMARY KEY,
    cookies TEXT,
    headers TEXT,
    user_agent TEXT,
    updated_at TEXT
);
",
)];

const HISTORY_MIGRATIONS: &[Migration] = &[(
    1,
    "
CREATE TABLE IF NOT EXISTS downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_name TEXT NOT NULL,
    chapter_number TEXT NOT NULL,
    extension_id TEXT NOT NULL,
    source_url TEXT,
    download_path TEXT,
    status TEXT NOT NULL DEFAULT 'completed',
    downloaded_at TEXT NOT NULL
);
",
)];

const EXTENSIONS_MIGRATIONS: &[Migration] = &[(
    1,
    "
CREATE TABLE IF NOT EXISTS installed_extensions (
    extension_id    TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    version_id      INTEGER NOT NULL,
    lang            TEXT NOT NULL,
    install_source  TEXT NOT NULL DEFAULT 'local',
    source_repo     TEXT,
    source_branch   TEXT,
    source_path     TEXT,
    installed_at    TEXT NOT NULL,
    updated_at      TEXT,
    auto_update     INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS extension_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
",
)];

// ---------------------------------------------------------------------------
// Migration runner
// ---------------------------------------------------------------------------

fn apply_migrations(conn: &Connection, migrations: &[Migration]) -> Result<()> {
    conn.execute_batch(SCHEMA_VERSIONS_TABLE)
        .map_err(|e| HagitoriError::config(format!("failed to create schema_versions: {e}")))?;

    let max_applied: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
            [],
            |row| row.get(0),
        )
        .map_err(|e| HagitoriError::config(format!("failed to read schema_versions: {e}")))?;

    for &(version, sql) in migrations {
        if version <= max_applied {
            continue;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| HagitoriError::config(format!("failed to start transaction: {e}")))?;

        tx.execute_batch(sql).map_err(|e| {
            HagitoriError::config(format!("failed to apply migration v{version}: {e}"))
        })?;

        tx.execute(
            "INSERT INTO schema_versions (version) VALUES (?1)",
            [version],
        )
        .map_err(|e| {
            HagitoriError::config(format!("failed to register migration v{version}: {e}"))
        })?;

        tx.commit().map_err(|e| {
            HagitoriError::config(format!("failed to commit migration v{version}: {e}"))
        })?;

        info!("migration v{} applied successfully", version);
    }

    Ok(())
}

/// returns the hagitori data directory, creating it if needed.
pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| HagitoriError::config("unable to determine config directory"))?
        .join("hagitori");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| HagitoriError::config(format!("failed to create directory {dir:?}: {e}")))?;
    }

    Ok(dir)
}

/// opens (or creates) a SQLite database at the given path and applies pending migrations.
pub fn open_database(path: &Path, migrations: &[Migration]) -> Result<Connection> {
    let conn = Connection::open(path)
        .map_err(|e| HagitoriError::config(format!("failed to open database {path:?}: {e}")))?;

    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| HagitoriError::config(format!("failed to enable WAL mode: {e}")))?;

    apply_migrations(&conn, migrations)?;

    info!("database opened: {:?}", path);
    Ok(conn)
}

pub fn open_config_db(base_dir: &Path) -> Result<Connection> {
    open_database(&base_dir.join("config.db"), CONFIG_MIGRATIONS)
}

pub fn open_sessions_db(base_dir: &Path) -> Result<Connection> {
    open_database(&base_dir.join("sessions.db"), SESSIONS_MIGRATIONS)
}

pub fn open_history_db(base_dir: &Path) -> Result<Connection> {
    open_database(&base_dir.join("history.db"), HISTORY_MIGRATIONS)
}

pub fn open_extensions_db(base_dir: &Path) -> Result<Connection> {
    open_database(&base_dir.join("extensions.db"), EXTENSIONS_MIGRATIONS)
}
