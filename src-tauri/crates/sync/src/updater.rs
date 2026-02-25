//! update checking by comparing installed extensions against the remote catalog.


use hagitori_config::ExtensionRegistry;
use hagitori_core::entities::catalog::{
    CatalogEntry, ExtensionCatalog, ExtensionSyncStatus, ExtensionUpdateInfo,
    InstalledExtension,
};
use hagitori_core::error::Result;

use crate::catalog::CatalogFetcher;

pub struct UpdateChecker {
    fetcher: CatalogFetcher,
}

impl UpdateChecker {
    pub fn new(fetcher: CatalogFetcher) -> Self {
        Self { fetcher }
    }

    /// clones the HttpClient Arc and the fetcher URL internally.
    pub fn new_with_ref(fetcher: &CatalogFetcher) -> Self {
        Self {
            fetcher: CatalogFetcher::new(
                fetcher.http().clone(),
                fetcher.catalog_url(),
            ),
        }
    }

    pub async fn check_updates(
        &self,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<ExtensionUpdateInfo>> {
        let catalog = self.fetcher.fetch().await?;
        self.compare(&catalog, registry)
    }

    /// compares catalog with local registry without I/O.
    pub fn compare(
        &self,
        catalog: &ExtensionCatalog,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<ExtensionUpdateInfo>> {
        let installed = registry.list_all()?;

        // index installed extensions by ID for fast lookup
        let installed_map: std::collections::HashMap<&str, &InstalledExtension> = installed
            .iter()
            .map(|e| (e.extension_id.as_str(), e))
            .collect();

        // track which installed IDs were seen in the catalog
        let mut seen_ids = std::collections::HashSet::new();

        let mut results: Vec<ExtensionUpdateInfo> = Vec::new();

        // compute status for each catalog extension
        for entry in &catalog.extensions {
            seen_ids.insert(entry.id.as_str());

            let (status, local_version_id) =
                match installed_map.get(entry.id.as_str()) {
                    Some(inst) => {
                        let status = compute_sync_status(inst, entry);
                        (status, Some(inst.version_id))
                    }
                    None => (ExtensionSyncStatus::NotInstalled, None),
                };

            let icon_url = build_icon_url(&self.fetcher.raw_base_url(), entry);

            results.push(ExtensionUpdateInfo {
                id: entry.id.clone(),
                name: entry.name.clone(),
                lang: entry.lang.clone(),
                local_version_id,
                remote_version_id: entry.version_id,
                status,
                domains: entry.domains.clone(),
                features: entry.features.clone(),
                icon_url,
            });
        }

        // report installed extensions that are no longer in the catalog.
        for inst in &installed {
            if !seen_ids.contains(inst.extension_id.as_str()) {
                tracing::warn!(
                    "extension '{}' installed locally but absent from catalog (orphaned)",
                    inst.extension_id
                );
                results.push(ExtensionUpdateInfo {
                    id: inst.extension_id.clone(),
                    name: inst.name.clone(),
                    lang: inst.lang.clone(),
                    local_version_id: Some(inst.version_id),
                    remote_version_id: 0,
                    status: ExtensionSyncStatus::Orphaned,
                    domains: Vec::new(),
                    features: Vec::new(),
                    icon_url: None,
                });
            }
        }

        tracing::debug!(
            "UpdateChecker: {} entries ({} update available, {} not installed, {} up-to-date, {} orphaned)",
            results.len(),
            results.iter().filter(|r| r.status == ExtensionSyncStatus::UpdateAvailable).count(),
            results.iter().filter(|r| r.status == ExtensionSyncStatus::NotInstalled).count(),
            results.iter().filter(|r| r.status == ExtensionSyncStatus::UpToDate).count(),
            results.iter().filter(|r| r.status == ExtensionSyncStatus::Orphaned).count(),
        );

        Ok(results)
    }

    pub fn fetcher(&self) -> &CatalogFetcher {
        &self.fetcher
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compute_sync_status(installed: &InstalledExtension, remote: &CatalogEntry) -> ExtensionSyncStatus {
    if remote.version_id > installed.version_id {
        ExtensionSyncStatus::UpdateAvailable
    } else if installed.version_id > remote.version_id {
        // local is newer catalog may have rolled back.
        tracing::warn!(
            "extension '{}' local version {} > remote {}, possible catalog rollback",
            remote.id,
            installed.version_id,
            remote.version_id
        );
        ExtensionSyncStatus::LocalNewer
    } else {
        ExtensionSyncStatus::UpToDate
    }
}

fn build_icon_url(raw_base_url: &str, entry: &CatalogEntry) -> Option<String> {
    entry.icon.as_ref().map(|icon_name| {
        format!(
            "{}/{}/{}",
            raw_base_url.trim_end_matches('/'), entry.path, icon_name
        )
    })
}
