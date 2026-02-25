//! extension auto-update: checks and applies updates automatically.
//!
//! runs on app startup to update catalog extensions that have
//! `auto_update = true` and a newer version available.

use tracing::{info, warn};

use hagitori_config::ExtensionRegistry;
use hagitori_core::entities::catalog::ExtensionSyncStatus;
use hagitori_core::error::Result;

use crate::catalog::CatalogFetcher;
use crate::installer::ExtensionInstaller;
use crate::updater::UpdateChecker;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateResult {
    pub updated: Vec<AutoUpdatedEntry>,
    pub failed: Vec<AutoUpdateFailure>,
    pub skipped: u32,
    pub up_to_date: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdatedEntry {
    pub id: String,
    pub name: String,
    pub from_version: u32,
    pub to_version: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateFailure {
    pub id: String,
    pub name: String,
    pub error: String,
}

/// runs auto-update for all catalog extensions with `auto_update = true`.
///
/// # Flow
/// 1. fetches the remote catalog
/// 2. compares with installed extensions
/// 3. for each extension with `UpdateAvailable` and `auto_update = true`:
///    - downloads and installs the new version
///    - updates the registry
/// 4. returns result with updated, failed, and skipped extensions
pub async fn run_auto_update(
    fetcher: &CatalogFetcher,
    installer: &ExtensionInstaller,
    registry: &ExtensionRegistry,
) -> Result<AutoUpdateResult> {
    let mut result = AutoUpdateResult::default();

    info!("starting extension auto-update...");

    // fetch catalog
    let catalog = match fetcher.fetch().await {
        Ok(c) => c,
        Err(e) => {
            warn!("auto-update: failed to fetch catalog: {e}");
            return Err(e);
        }
    };

    // compare with local registry
    let checker = UpdateChecker::new_with_ref(fetcher);
    let updates = checker.compare(&catalog, registry)?;

    let raw_base_url = fetcher.raw_base_url();

    // process each extension
    for update_info in &updates {
        match update_info.status {
            ExtensionSyncStatus::UpdateAvailable => {
                // check if auto-update is enabled for this extension
                let installed = match registry.get(&update_info.id)? {
                    Some(inst) => inst,
                    None => {
                        warn!(
                            "auto-update: '{}' marked as UpdateAvailable but not found in registry   skipping",
                            update_info.id
                        );
                        continue;
                    }
                };

                if !installed.auto_update {
                    info!(
                        "auto-update skipped for '{}' (auto_update disabled)",
                        update_info.name
                    );
                    result.skipped += 1;
                    continue;
                }

                // find entry in catalog
                let entry = match catalog.extensions.iter().find(|e| e.id == update_info.id) {
                    Some(e) => e,
                    None => {
                        warn!(
                            "auto-update: '{}' not found in catalog despite UpdateAvailable status   skipping",
                            update_info.id
                        );
                        continue;
                    }
                };

                info!(
                    "auto-update: updating '{}' v{} -> v{}",
                    entry.name, installed.version_id, entry.version_id
                );

                // download and install
                match installer.install(entry, &catalog, &raw_base_url).await {
                    Ok(_final_dir) => {
                        // update registry   store resolved relative path
                        let resolved_path = entry.relative_path();
                        if let Err(e) = registry.register_catalog(
                            &entry.id,
                            &entry.name,
                            entry.version_id,
                            &entry.lang,
                            &catalog.repo,
                            &catalog.branch,
                            &resolved_path,
                        ) {
                            warn!(
                                "auto-update: extension '{}' installed but failed to register: {e}",
                                entry.id
                            );
                            result.failed.push(AutoUpdateFailure {
                                id: entry.id.clone(),
                                name: entry.name.clone(),
                                error: e.to_string(),
                            });
                            continue;
                        }

                        result.updated.push(AutoUpdatedEntry {
                            id: entry.id.clone(),
                            name: entry.name.clone(),
                            from_version: installed.version_id,
                            to_version: entry.version_id,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "auto-update: failed to update '{}': {e}",
                            entry.id
                        );
                        result.failed.push(AutoUpdateFailure {
                            id: entry.id.clone(),
                            name: entry.name.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
            ExtensionSyncStatus::UpToDate => {
                result.up_to_date += 1;
            }
            _ => {}
        }
    }

    info!(
        "auto-update complete: {} updated, {} failed, {} skipped, {} already up-to-date",
        result.updated.len(),
        result.failed.len(),
        result.skipped,
        result.up_to_date
    );

    Ok(result)
}
