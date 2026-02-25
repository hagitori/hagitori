use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::debug;

use hagitori_core::error::{HagitoriError, Result};

use crate::database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: Option<i64>,
    pub manga_name: String,
    pub chapter_number: String,
    pub extension_id: String,
    pub source_url: Option<String>,
    pub download_path: Option<String>,
    pub status: String,
    pub downloaded_at: String,
}

impl DownloadRecord {
    pub fn completed(
        manga_name: impl Into<String>,
        chapter_number: impl Into<String>,
        extension_id: impl Into<String>,
        download_path: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            manga_name: manga_name.into(),
            chapter_number: chapter_number.into(),
            extension_id: extension_id.into(),
            source_url: None,
            download_path: Some(download_path.into()),
            status: "completed".to_string(),
            downloaded_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn failed(
        manga_name: impl Into<String>,
        chapter_number: impl Into<String>,
        extension_id: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            manga_name: manga_name.into(),
            chapter_number: chapter_number.into(),
            extension_id: extension_id.into(),
            source_url: None,
            download_path: None,
            status: "failed".to_string(),
            downloaded_at: Utc::now().to_rfc3339(),
        }
    }

}

pub struct DownloadHistory {
    conn: Mutex<rusqlite::Connection>,
}

impl DownloadHistory {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let conn = database::open_history_db(base_dir)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn add(&self, record: &DownloadRecord) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        conn.execute(
            "INSERT INTO downloads (manga_name, chapter_number, extension_id, source_url, download_path, status, downloaded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.manga_name,
                record.chapter_number,
                record.extension_id,
                record.source_url,
                record.download_path,
                record.status,
                record.downloaded_at,
            ],
        )
        .map_err(|e| HagitoriError::config(format!("failed to insert into history: {e}")))?;

        debug!(
            manga = record.manga_name.as_str(),
            chapter = record.chapter_number.as_str(),
            "DownloadHistory::add"
        );
        Ok(())
    }
}
