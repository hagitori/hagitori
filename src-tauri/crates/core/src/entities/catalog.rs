use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// remote catalog (catalog.json hosted on GitHub)
// ---------------------------------------------------------------------------

/// full `catalog.json` file from the extensions repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionCatalog {
    /// catalog schema version (currently 1).
    pub version: u32,
    /// ISO 8601 timestamp of the last catalog update.
    pub updated_at: String,
    /// canonical GitHub repository (e.g. "owner/hagitori-extensions").
    pub repo: String,
    /// reference branch (e.g. "main").
    pub branch: String,
    pub extensions: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// must match the `@id` in the JS script header.
    pub id: String,
    pub name: String,
    /// primary language (`pt-br`, `en`, `es`, `multi`).
    pub lang: String,
    /// incremental numeric version   MUST match the `@versionId` header.
    pub version_id: u32,
    /// relative path from the repository root (e.g. `builds/en/mangadex`).
    pub path: String,
    /// main file name (e.g. `index.js`).
    pub entry: String,
    /// additional files declared via `@require`.
    #[serde(default)]
    pub requires: Vec<String>,
    /// icon file name (e.g. `icon.png`), relative to `path`.
    pub icon: Option<String>,
    pub domains: Vec<String>,
    /// required features (`browser`, `crypto`, etc.).
    #[serde(default)]
    pub features: Vec<String>,
    /// whether the extension implements `getDetails()`.
    #[serde(default)]
    pub supports_details: bool,
    /// available languages (for `multi` extensions).
    #[serde(default)]
    pub languages: Vec<String>,
    /// `filename -> sha256_hex` map for integrity validation.
    #[serde(default)]
    pub files: HashMap<String, String>,
    pub min_app_version: Option<String>,
}

impl CatalogEntry {
    /// extracts the `<lang>/<ext_name>` portion from `self.path`, stripping
    /// any repo-level prefix (e.g., `builds/`).
    ///
    /// - `"builds/en/mangadex"` -> `"en/mangadex"`
    /// - `"en/mangadex"` -> `"en/mangadex"` (unchanged)
    pub fn relative_path(&self) -> String {
        let parts: Vec<&str> = self.path.split('/').collect();
        if parts.len() > 2 {
            parts[parts.len() - 2..].join("/")
        } else {
            self.path.clone()
        }
    }
}

// ---------------------------------------------------------------------------
// sync status
// ---------------------------------------------------------------------------

/// result of comparing local version vs catalog version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSyncStatus {
    NotInstalled,
    UpToDate,
    UpdateAvailable,
    /// local version is newer than remote (e.g. catalog rolled back).
    LocalNewer,
    /// installed locally but no longer present in the remote catalog.
    Orphaned,
}

// ---------------------------------------------------------------------------
// installed extension record (persisted in SQLite)
// ---------------------------------------------------------------------------

/// installed extension data, stored in the `installed_extensions` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledExtension {
    pub extension_id: String,
    pub name: String,
    pub version_id: u32,
    pub lang: String,
    /// source repository (e.g. `owner/hagitori-extensions`).
    pub source_repo: Option<String>,
    pub source_branch: Option<String>,
    /// resolved relative path used for local storage.
    /// derived from [`CatalogEntry::relative_path()`] at install time.
    pub source_path: Option<String>,
    /// ISO 8601 install timestamp.
    pub installed_at: String,
    /// ISO 8601 last update timestamp.
    pub updated_at: Option<String>,
    pub auto_update: bool,
}

// ---------------------------------------------------------------------------
// update info (sent to the frontend)
// ---------------------------------------------------------------------------

/// DTO sent to the frontend with sync info for an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionUpdateInfo {
    pub id: String,
    pub name: String,
    pub lang: String,
    /// `None` if the extension is not installed.
    pub local_version_id: Option<u32>,
    pub remote_version_id: u32,
    pub status: ExtensionSyncStatus,
    pub domains: Vec<String>,
    pub features: Vec<String>,
    pub icon_url: Option<String>,
}
