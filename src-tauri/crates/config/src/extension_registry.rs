//! installed extensions registry (SQLite).

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::params;
use tracing::debug;

use hagitori_core::entities::catalog::InstalledExtension;
use hagitori_core::error::{HagitoriError, Result};

use crate::database;

pub struct ExtensionRegistry {
    conn: Mutex<rusqlite::Connection>,
}

impl ExtensionRegistry {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let conn = database::open_extensions_db(base_dir)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ─── CRUD ─────────────────────────────────────────────────

    pub fn upsert(&self, ext: &InstalledExtension) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        conn.execute(
            "INSERT INTO installed_extensions
                (extension_id, name, version_id, lang, install_source,
                 source_repo, source_branch, source_path,
                 installed_at, updated_at, auto_update)
             VALUES (?1, ?2, ?3, ?4, 'catalog', ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(extension_id) DO UPDATE SET
                name           = excluded.name,
                version_id     = excluded.version_id,
                lang           = excluded.lang,
                install_source = 'catalog',
                source_repo    = excluded.source_repo,
                source_branch  = excluded.source_branch,
                source_path    = excluded.source_path,
                updated_at     = excluded.updated_at,
                auto_update    = COALESCE(installed_extensions.auto_update, excluded.auto_update)",
            params![
                ext.extension_id,
                ext.name,
                ext.version_id,
                ext.lang,
                ext.source_repo,
                ext.source_branch,
                ext.source_path,
                ext.installed_at,
                ext.updated_at,
                i32::from(ext.auto_update),
            ],
        )
        .map_err(|e| {
            HagitoriError::config(format!(
                "failed to upsert extension '{}': {e}",
                ext.extension_id
            ))
        })?;

        debug!(
            extension_id = ext.extension_id.as_str(),
            "ExtensionRegistry::upsert"
        );
        Ok(())
    }

    pub fn get(&self, extension_id: &str) -> Result<Option<InstalledExtension>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT extension_id, name, version_id, lang,
                        source_repo, source_branch, source_path,
                        installed_at, updated_at, auto_update
                 FROM installed_extensions
                 WHERE extension_id = ?1",
            )
            .map_err(|e| HagitoriError::config(format!("failed to prepare query: {e}")))?;

        let result = match stmt.query_row(params![extension_id], |row| {
            row_to_installed_extension(row)
        }) {
            Ok(ext) => Some(ext),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                return Err(HagitoriError::config(format!(
                    "failed to read extension '{extension_id}': {e}"
                )));
            }
        };

        Ok(result)
    }

    pub fn list_all(&self) -> Result<Vec<InstalledExtension>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT extension_id, name, version_id, lang,
                        source_repo, source_branch, source_path,
                        installed_at, updated_at, auto_update
                 FROM installed_extensions
                 ORDER BY name ASC",
            )
            .map_err(|e| HagitoriError::config(format!("failed to prepare query: {e}")))?;

        let rows = stmt
            .query_map([], row_to_installed_extension)
            .map_err(|e| {
                HagitoriError::config(format!("failed to read installed extensions: {e}"))
            })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| HagitoriError::config(format!("failed to read row: {e}")))
    }

    pub fn remove(&self, extension_id: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        let affected = conn
            .execute(
                "DELETE FROM installed_extensions WHERE extension_id = ?1",
                params![extension_id],
            )
            .map_err(|e| {
                HagitoriError::config(format!("failed to remove extension '{}': {e}", extension_id))
            })?;

        debug!(
            extension_id = extension_id,
            removed = affected > 0,
            "ExtensionRegistry::remove"
        );
        Ok(affected > 0)
    }

    // ─── partial updates ──────────────────────────────────────

    pub fn set_auto_update(&self, extension_id: &str, enabled: bool) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| HagitoriError::config(format!("Mutex poisoned: {e}")))?;
        conn.execute(
            "UPDATE installed_extensions SET auto_update = ?2 WHERE extension_id = ?1",
            params![extension_id, i32::from(enabled)],
        )
        .map_err(|e| {
            HagitoriError::config(format!(
                "failed to set auto_update for '{}': {e}",
                extension_id
            ))
        })?;
        Ok(())
    }

    #[expect(clippy::too_many_arguments, reason = "parameters mirror SQL table columns")]
    pub fn register_catalog(
        &self,
        extension_id: &str,
        name: &str,
        version_id: u32,
        lang: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.upsert(&InstalledExtension {
            extension_id: extension_id.to_string(),
            name: name.to_string(),
            version_id,
            lang: lang.to_string(),
            source_repo: Some(repo.to_string()),
            source_branch: Some(branch.to_string()),
            source_path: Some(path.to_string()),
            installed_at: now.clone(),
            updated_at: Some(now),
            auto_update: true,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_installed_extension(row: &rusqlite::Row<'_>) -> rusqlite::Result<InstalledExtension> {
    let raw_version: i64 = row.get(2)?;
    let version_id = u32::try_from(raw_version).map_err(|_| {
        rusqlite::Error::IntegralValueOutOfRange(2, raw_version)
    })?;

    Ok(InstalledExtension {
        extension_id: row.get(0)?,
        name: row.get(1)?,
        version_id,
        lang: row.get(3)?,
        source_repo: row.get(4)?,
        source_branch: row.get(5)?,
        source_path: row.get(6)?,
        installed_at: row.get(7)?,
        updated_at: row.get(8)?,
        auto_update: row.get::<_, Option<i32>>(9)?.unwrap_or(1) != 0,
    })
}
