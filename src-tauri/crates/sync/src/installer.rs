//! atomic download, validation, and installation of extensions from the remote catalog.

use std::path::{Path, PathBuf};
use std::sync::Arc;


use hagitori_core::entities::catalog::{CatalogEntry, ExtensionCatalog};
use hagitori_core::error::{HagitoriError, Result};
use hagitori_extensions::ExtensionManifest;
use hagitori_http::HttpClient;

use crate::integrity::{sha256_hex, SizeLimits};

/// uses atomic download: files go to a temp directory and are only
/// moved to the final destination after full validation (checksum + manifest).
pub struct ExtensionInstaller {
    http: Arc<HttpClient>,
    extensions_dir: PathBuf,
}

struct TmpGuard {
    path: PathBuf,
    disarmed: bool,
}

impl TmpGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, disarmed: false }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for TmpGuard {
    fn drop(&mut self) {
        if !self.disarmed && self.path.exists() {
            std::fs::remove_dir_all(&self.path).ok();
        }
    }
}

impl ExtensionInstaller {
    pub fn new(http: Arc<HttpClient>, extensions_dir: impl Into<PathBuf>) -> Self {
        Self {
            http,
            extensions_dir: extensions_dir.into(),
        }
    }

    pub fn extensions_dir(&self) -> &Path {
        &self.extensions_dir
    }

    /// downloads and installs an extension from the catalog.
    ///
    /// # Flow
    /// 1. creates temp directory (`.tmp_<id>`)
    /// 2. downloads each file declared in `entry.files`
    /// 3. validates individual size and SHA-256 checksum
    /// 4. validates total extension size
    /// 5. validates that the manifest matches the catalog (`id`, `version`)
    /// 6. atomically moves to the final directory (`<lang>/<ext_name>/`)
    ///
    /// on failure at any step, the temp directory is removed (rollback).
    pub async fn install(
        &self,
        entry: &CatalogEntry,
        catalog: &ExtensionCatalog,
        raw_base_url: &str,
    ) -> Result<PathBuf> {
        let base_url = format!(
            "{}/{}",
            raw_base_url.trim_end_matches('/'), entry.path
        );

        // sanitize ID for use as temp folder name
        let safe_id = sanitize_dir_name(&entry.id);
        let tmp_dir = self.extensions_dir.join(format!(".tmp_{}", safe_id));

        // clean up leftover tmp from a previous failed attempt
        if tmp_dir.exists() {
            tokio::fs::remove_dir_all(&tmp_dir).await.ok();
        }

        tokio::fs::create_dir_all(&tmp_dir).await.map_err(|e| {
            HagitoriError::extension(format!(
                "failed to create temp directory {}: {e}",
                tmp_dir.display()
            ))
        })?;

        let mut guard = TmpGuard::new(tmp_dir.clone());

        tracing::info!(
            "downloading extension '{}' v{} from {}/{}",
            entry.id,
            entry.version_id,
            catalog.repo,
            entry.path
        );

        // download and validate each file
        let mut total_size: usize = 0;

        for (filename, expected_hash) in &entry.files {
            // validate filename even though catalog already checks
            crate::catalog::validate_catalog_path(filename)?;

            let url = format!("{}/{}", base_url, filename);

            tracing::debug!("downloading: {} -> {}", url, filename);

            let bytes = self.http.get_bytes(&url, None).await.map_err(|e| {
                HagitoriError::extension(format!(
                    "failed to download '{}': {e}",
                    filename
                ))
            })?;

            // validate individual file size
            SizeLimits::validate_file(filename, bytes.len())?;

            total_size += bytes.len();

            // validate SHA-256 checksum
            let actual_hash = sha256_hex(&bytes);
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                return Err(HagitoriError::extension(format!(
                    "invalid checksum for '{}' in extension '{}': expected {}, got {}",
                    filename, entry.id, expected_hash, actual_hash
                )));
            }

            // create subdirectories if filename contains path separators
            let file_path = tmp_dir.join(filename);
            if let Some(parent) = file_path.parent()
                && !parent.exists() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        HagitoriError::extension(format!(
                            "failed to create subdirectory for '{}': {e}",
                            filename
                        ))
                    })?;
                }

            tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
                HagitoriError::extension(format!(
                    "failed to save '{}': {e}",
                    filename
                ))
            })?;
        }

        // validate total size
        SizeLimits::validate_total(total_size, &entry.id)?;

        // validate extension package.json
        let manifest = ExtensionManifest::from_dir(&tmp_dir)?;

        // ID and version must match the catalog
        if manifest.id() != entry.id {
            return Err(HagitoriError::extension(format!(
                "extension ID ('{}') does not match catalog ('{}')",
                manifest.id(), entry.id
            )));
        }

        if manifest.version != entry.version_id {
            return Err(HagitoriError::extension(format!(
                "extension version ({}) does not match catalog ({}) for '{}'",
                manifest.version, entry.version_id, entry.id
            )));
        }

        // atomically move to the final directory using backup-and-swap
        // to prevent data loss if rename + copy both fail
        let final_dir = self.resolve_final_dir(entry);
        let backup_dir = self.extensions_dir.join(format!(".backup_{}", safe_id));

        // back up existing version instead of deleting it
        if final_dir.exists() {
            if backup_dir.exists() {
                tokio::fs::remove_dir_all(&backup_dir).await.ok();
            }
            tokio::fs::rename(&final_dir, &backup_dir).await.map_err(|e| {
                HagitoriError::extension(format!(
                    "failed to back up previous version at {}: {e}",
                    final_dir.display()
                ))
            })?;
        }

        // ensure parent directory exists
        if let Some(parent) = final_dir.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                HagitoriError::extension(format!(
                    "failed to create parent directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        let swap_result: Result<()> = async {
            if let Err(rename_err) = tokio::fs::rename(&tmp_dir, &final_dir).await {
                tracing::warn!(
                    "rename failed ({}), falling back to copy + remove",
                    rename_err
                );
                if let Err(copy_err) = copy_dir_recursive(&tmp_dir, &final_dir).await {
                    // clean up partial copy before restoring backup
                    if final_dir.exists() {
                        tokio::fs::remove_dir_all(&final_dir).await.ok();
                    }
                    return Err(HagitoriError::extension(format!(
                        "failed to move extension to {}: rename={rename_err}, copy={copy_err}",
                        final_dir.display()
                    )));
                }
                // guard will clean up tmp_dir on drop
            } else {
                // tmp_dir no longer exists, disarm guard
                guard.disarm();
            }
            Ok(())
        }.await;

        match swap_result {
            Ok(()) => {
                // success   remove backup
                if backup_dir.exists() {
                    tokio::fs::remove_dir_all(&backup_dir).await.ok();
                }
            }
            Err(e) => {
                // failure   restore backup
                if backup_dir.exists() {
                    if let Err(restore_err) = tokio::fs::rename(&backup_dir, &final_dir).await {
                        tracing::error!(
                            "failed to restore backup for '{}': {restore_err}",
                            entry.id
                        );
                    } else {
                        tracing::info!("previous version of '{}' restored from backup", entry.id);
                    }
                }
                return Err(e);
            }
        }

        tracing::info!(
            "extension '{}' v{} installed at {}",
            entry.id,
            entry.version_id,
            final_dir.display()
        );

        Ok(final_dir)
    }

    /// resolves the final directory for an extension: `<extensions_dir>/<lang>/<ext_short_name>/`
    ///
    /// uses [`CatalogEntry::relative_path()`] to strip repo prefixes
    /// (e.g., `builds/pt-br/sakuramangas` -> `pt-br/sakuramangas`).
    pub fn resolve_final_dir(&self, entry: &CatalogEntry) -> PathBuf {
        self.extensions_dir.join(entry.relative_path())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
            c
        } else {
            '_'
        })
        .collect()
}

/// recursively copies a directory (fallback when `fs::rename` fails cross-device).
fn copy_dir_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        tokio::fs::create_dir_all(dst).await.map_err(|e| {
            HagitoriError::extension(format!(
                "failed to create destination directory {}: {e}",
                dst.display()
            ))
        })?;

        let mut entries = tokio::fs::read_dir(src).await.map_err(|e| {
            HagitoriError::extension(format!(
                "failed to read directory {}: {e}",
                src.display()
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            HagitoriError::extension(format!("failed to read directory entry: {e}"))
        })? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path).await?;
            } else {
                tokio::fs::copy(&src_path, &dst_path).await.map_err(|e| {
                    HagitoriError::extension(format!(
                        "failed to copy {} -> {}: {e}",
                        src_path.display(),
                        dst_path.display()
                    ))
                })?;
            }
        }

        Ok(())
    })
}
