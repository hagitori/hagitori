use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::params;
use tracing::debug;

use hagitori_core::error::{HagitoriError, Result};

use crate::database;

const DEFAULTS: &[(&str, &str)] = &[
    ("download_dir", ""),
    ("image_format", "original"),
    ("group_format", "cbz"),
    ("language", "en"),
];

pub struct ConfigManager {
    conn: Mutex<rusqlite::Connection>,
}

impl ConfigManager {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let conn = database::open_config_db(base_dir)?;
        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.ensure_defaults()?;
        Ok(manager)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|e| HagitoriError::config(format!("failed to prepare query: {e}")))?;

        let result = match stmt.query_row(params![key], |row| row.get::<_, String>(0)) {
            Ok(value) => Some(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(HagitoriError::config(format!("failed to read config '{key}': {e}"))),
        };

        debug!(key = key, found = result.is_some(), "ConfigManager::get");
        Ok(result)
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| HagitoriError::config(format!("failed to save config '{key}': {e}")))?;

        debug!(key = key, value = value, "ConfigManager::set");
        Ok(())
    }

    pub fn get_all(&self) -> Result<HashMap<String, String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| HagitoriError::config(format!("failed to prepare query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| HagitoriError::config(format!("failed to read configs: {e}")))?;

        rows.collect::<std::result::Result<HashMap<_, _>, _>>()
            .map_err(|e| HagitoriError::config(format!("failed to read row: {e}")))
    }

    // ─── typed accessors ──────────────────────────────────


    /// returns the download directory, falling back to `data_dir()/Outputs`.
    pub fn download_dir(&self) -> Result<String> {
        if let Some(dir) = self.get("download_dir")?
            && !dir.is_empty() {
                return Ok(dir);
            }

        // Default: APP_DATA/Outputs  (e.g. ~/.config/hagitori/Outputs)
        let default = crate::database::data_dir()?.join("Outputs");

        Ok(default.to_string_lossy().to_string())
    }

    pub fn group_format(&self) -> Result<String> {
        Ok(self
            .get("group_format")?
            .unwrap_or_else(|| "cbz".to_string()))
    }

    pub fn image_format(&self) -> Result<String> {
        Ok(self
            .get("image_format")?
            .unwrap_or_else(|| "original".to_string()))
    }

    /// max concurrent page downloads (capped at 5).
    pub fn max_concurrent_pages(&self) -> Result<usize> {
        let value = match self.get("max_concurrent_pages")? {
            Some(v) => v
                .parse::<usize>()
                .map_err(|e| HagitoriError::config(format!("invalid max_concurrent_pages '{v}': {e}")))?,
            None => 3,
        };
        Ok(value.min(5))
    }

    // ─── internals ──────────────────────────────────────────

    fn ensure_defaults(&self) -> Result<()> {
        for (key, value) in DEFAULTS {
            if self.get(key)?.is_none() {
                self.set(key, value)?;
            }
        }
        Ok(())
    }
}
