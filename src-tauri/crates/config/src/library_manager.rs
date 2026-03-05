use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::params;
use tracing::debug;

use hagitori_core::entities::{Chapter, Manga, MangaDetails};
use hagitori_core::error::{HagitoriError, Result};

use crate::database;

/// library entry combining manga, chapters, and optional details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub manga: Manga,
    pub chapters: Vec<Chapter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<MangaDetails>,
    pub updated_at: i64,
}

/// source display metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMeta {
    pub display_name: Option<String>,
    pub supports_details: bool,
}

pub struct LibraryManager {
    conn: Mutex<rusqlite::Connection>,
}

impl LibraryManager {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let conn = database::open_library_db(base_dir)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ─── manga CRUD ─────────────────────────────────────

    pub fn add_manga(&self, manga: &Manga, chapters: &[Chapter]) -> Result<()> {
        let conn = self.lock()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| HagitoriError::config(format!("transaction failed: {e}")))?;

        tx.execute(
            "INSERT OR REPLACE INTO manga (id, name, cover, source, url, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![manga.id, manga.name, manga.cover, manga.source, manga.url],
        )
        .map_err(|e| HagitoriError::config(format!("failed to insert manga: {e}")))?;

        // replace chapters
        tx.execute("DELETE FROM chapters WHERE manga_id = ?1", params![manga.id])
            .map_err(|e| HagitoriError::config(format!("failed to clear chapters: {e}")))?;

        let mut stmt = tx
            .prepare(
                "INSERT INTO chapters (id, manga_id, number, name, title, date, scanlator)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| HagitoriError::config(format!("failed to prepare chapter insert: {e}")))?;

        for ch in chapters {
            stmt.execute(params![
                ch.id,
                manga.id,
                ch.number,
                ch.name,
                ch.title,
                ch.date,
                ch.scanlator
            ])
            .map_err(|e| HagitoriError::config(format!("failed to insert chapter: {e}")))?;
        }
        drop(stmt);

        tx.commit()
            .map_err(|e| HagitoriError::config(format!("commit failed: {e}")))?;

        debug!(manga_id = manga.id.as_str(), chapters = chapters.len(), "library: manga added");
        Ok(())
    }

    pub fn remove_manga(&self, manga_id: &str) -> Result<()> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM manga WHERE id = ?1", params![manga_id])
            .map_err(|e| HagitoriError::config(format!("failed to remove manga: {e}")))?;
        debug!(manga_id, "library: manga removed");
        Ok(())
    }

    pub fn get_manga(&self, manga_id: &str) -> Result<Option<LibraryEntry>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, cover, source, url,
                        CAST(strftime('%s', updated_at) AS INTEGER)
                 FROM manga WHERE id = ?1",
            )
            .map_err(|e| HagitoriError::config(format!("prepare failed: {e}")))?;

        let manga = match stmt.query_row(params![manga_id], |row| {
            Ok(Manga {
                id: row.get(0)?,
                name: row.get(1)?,
                cover: row.get(2)?,
                source: row.get(3)?,
                url: row.get(4)?,
            })
        }) {
            Ok(m) => m,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(HagitoriError::config(format!("failed to get manga: {e}"))),
        };

        let updated_at: i64 = stmt
            .query_row(params![manga_id], |row| row.get(5))
            .unwrap_or(0);
        drop(stmt);

        let chapters = self.read_chapters(&conn, manga_id)?;
        let details = self.read_details(&conn, manga_id)?;

        Ok(Some(LibraryEntry {
            manga,
            chapters,
            details,
            updated_at: updated_at * 1000, // seconds -> milliseconds
        }))
    }

    pub fn list_manga(&self) -> Result<Vec<LibraryEntry>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, cover, source, url,
                        CAST(strftime('%s', updated_at) AS INTEGER)
                 FROM manga ORDER BY updated_at DESC",
            )
            .map_err(|e| HagitoriError::config(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    Manga {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        cover: row.get(2)?,
                        source: row.get(3)?,
                        url: row.get(4)?,
                    },
                    row.get::<_, i64>(5).unwrap_or(0),
                ))
            })
            .map_err(|e| HagitoriError::config(format!("query failed: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            let (manga, updated_at) =
                row.map_err(|e| HagitoriError::config(format!("row read failed: {e}")))?;
            let manga_id = manga.id.clone();
            let chapters = self.read_chapters(&conn, &manga_id)?;
            let details = self.read_details(&conn, &manga_id)?;
            entries.push(LibraryEntry {
                manga,
                chapters,
                details,
                updated_at: updated_at * 1000,
            });
        }

        Ok(entries)
    }

    pub fn update_cover(&self, manga_id: &str, cover: &str) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE manga SET cover = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![cover, manga_id],
        )
        .map_err(|e| HagitoriError::config(format!("failed to update cover: {e}")))?;
        Ok(())
    }

    // ─── chapters ───────────────────────────────────────

    pub fn update_chapters(&self, manga_id: &str, chapters: &[Chapter]) -> Result<()> {
        let conn = self.lock()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| HagitoriError::config(format!("transaction failed: {e}")))?;

        tx.execute("DELETE FROM chapters WHERE manga_id = ?1", params![manga_id])
            .map_err(|e| HagitoriError::config(format!("failed to clear chapters: {e}")))?;

        let mut stmt = tx
            .prepare(
                "INSERT INTO chapters (id, manga_id, number, name, title, date, scanlator)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| HagitoriError::config(format!("prepare failed: {e}")))?;

        for ch in chapters {
            stmt.execute(params![
                ch.id, manga_id, ch.number, ch.name, ch.title, ch.date, ch.scanlator
            ])
            .map_err(|e| HagitoriError::config(format!("failed to insert chapter: {e}")))?;
        }
        drop(stmt);

        tx.execute(
            "UPDATE manga SET updated_at = datetime('now') WHERE id = ?1",
            params![manga_id],
        )
        .map_err(|e| HagitoriError::config(format!("failed to touch manga: {e}")))?;

        tx.commit()
            .map_err(|e| HagitoriError::config(format!("commit failed: {e}")))?;

        debug!(manga_id, count = chapters.len(), "library: chapters updated");
        Ok(())
    }

    // ─── details ────────────────────────────────────────

    pub fn update_details(&self, manga_id: &str, details: &MangaDetails) -> Result<()> {
        let conn = self.lock()?;
        let alt_titles = serde_json::to_string(&details.alt_titles).unwrap_or_default();
        let tags = serde_json::to_string(&details.tags).unwrap_or_default();

        conn.execute(
            "INSERT OR REPLACE INTO manga_details
             (manga_id, cover, synopsis, author, artist, alt_titles, tags, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                manga_id,
                details.cover,
                details.synopsis,
                details.author,
                details.artist,
                alt_titles,
                tags,
                details.status
            ],
        )
        .map_err(|e| HagitoriError::config(format!("failed to upsert details: {e}")))?;

        debug!(manga_id, "library: details updated");
        Ok(())
    }

    // ─── source metadata ────────────────────────────────

    pub fn set_source_name(&self, source_id: &str, name: &str) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO source_meta (source_id, display_name) VALUES (?1, ?2)
             ON CONFLICT(source_id) DO UPDATE SET display_name = ?2",
            params![source_id, name],
        )
        .map_err(|e| HagitoriError::config(format!("failed to set source name: {e}")))?;
        Ok(())
    }

    pub fn set_source_supports_details(&self, source_id: &str, supports: bool) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO source_meta (source_id, supports_details) VALUES (?1, ?2)
             ON CONFLICT(source_id) DO UPDATE SET supports_details = ?2",
            params![source_id, supports as i32],
        )
        .map_err(|e| {
            HagitoriError::config(format!("failed to set source_supports_details: {e}"))
        })?;
        Ok(())
    }

    pub fn get_source_meta(&self) -> Result<HashMap<String, SourceMeta>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT source_id, display_name, supports_details FROM source_meta")
            .map_err(|e| HagitoriError::config(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    SourceMeta {
                        display_name: row.get(1)?,
                        supports_details: row.get::<_, i32>(2)? != 0,
                    },
                ))
            })
            .map_err(|e| HagitoriError::config(format!("query failed: {e}")))?;

        rows.collect::<std::result::Result<HashMap<_, _>, _>>()
            .map_err(|e| HagitoriError::config(format!("row read failed: {e}")))
    }

    // ─── extension langs ────────────────────────────────

    pub fn set_extension_lang(&self, ext_id: &str, lang: &str) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO extension_langs (extension_id, lang) VALUES (?1, ?2)",
            params![ext_id, lang],
        )
        .map_err(|e| HagitoriError::config(format!("failed to set extension lang: {e}")))?;
        Ok(())
    }

    pub fn get_extension_langs(&self) -> Result<HashMap<String, String>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT extension_id, lang FROM extension_langs")
            .map_err(|e| HagitoriError::config(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| HagitoriError::config(format!("query failed: {e}")))?;

        rows.collect::<std::result::Result<HashMap<_, _>, _>>()
            .map_err(|e| HagitoriError::config(format!("row read failed: {e}")))
    }

    // ─── helpers ────────────────────────────────────────

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>> {
        self.conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("mutex poisoned: {e}")))
    }

    fn read_chapters(
        &self,
        conn: &rusqlite::Connection,
        manga_id: &str,
    ) -> Result<Vec<Chapter>> {
        let mut stmt = conn
            .prepare(
                "SELECT id, number, name, title, date, scanlator
                 FROM chapters WHERE manga_id = ?1",
            )
            .map_err(|e| HagitoriError::config(format!("prepare chapters failed: {e}")))?;

        let rows = stmt
            .query_map(params![manga_id], |row| {
                Ok(Chapter {
                    id: row.get(0)?,
                    number: row.get(1)?,
                    name: row.get(2)?,
                    title: row.get(3)?,
                    date: row.get(4)?,
                    scanlator: row.get(5)?,
                })
            })
            .map_err(|e| HagitoriError::config(format!("chapters query failed: {e}")))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| HagitoriError::config(format!("chapter row read failed: {e}")))
    }

    fn read_details(
        &self,
        conn: &rusqlite::Connection,
        manga_id: &str,
    ) -> Result<Option<MangaDetails>> {
        let mut stmt = conn
            .prepare(
                "SELECT d.cover, d.synopsis, d.author, d.artist, d.alt_titles, d.tags, d.status,
                        m.name, m.source
                 FROM manga_details d
                 JOIN manga m ON m.id = d.manga_id
                 WHERE d.manga_id = ?1",
            )
            .map_err(|e| HagitoriError::config(format!("prepare details failed: {e}")))?;

        match stmt.query_row(params![manga_id], |row| {
            let alt_titles_json: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
            let tags_json: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();

            Ok(MangaDetails {
                id: manga_id.to_string(),
                name: row.get(7)?,
                cover: row.get(0)?,
                source: row.get(8)?,
                synopsis: row.get(1)?,
                author: row.get(2)?,
                artist: row.get(3)?,
                alt_titles: serde_json::from_str(&alt_titles_json).unwrap_or_default(),
                tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                status: row.get(6)?,
            })
        }) {
            Ok(d) => Ok(Some(d)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(HagitoriError::config(format!(
                "failed to read details: {e}"
            ))),
        }
    }
}
