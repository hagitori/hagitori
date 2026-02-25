//! tauri commands for syncing extensions with the remote catalog.

use std::sync::Arc;

use tauri::State;

use hagitori_core::entities::catalog::{
    ExtensionCatalog, ExtensionUpdateInfo, InstalledExtension,
};
use hagitori_core::provider::MangaProvider;
use hagitori_extensions::{ExtensionLoader, JsRuntime};
use hagitori_sync::catalog::CatalogFetcher;
use hagitori_sync::installer::ExtensionInstaller;
use hagitori_sync::updater::UpdateChecker;

use crate::AppState;
use crate::utils::CommandResult;

// ---------------------------------------------------------------------------
// configuration constants
// ---------------------------------------------------------------------------

const CONFIG_CATALOG_URL: &str = "extensions_catalog_url";
const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/hagitori/hagitori-extensions/main/catalog.min.json";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

async fn resolve_extensions_dir(_app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let data_dir = hagitori_config::data_dir()
        .map_err(|e| format!("failed to resolve data directory: {e}"))?;
    let dir = data_dir.join("extensions");
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir).await
            .map_err(|e| format!("failed to create extensions directory: {e}"))?;
    }
    Ok(dir)
}

fn get_catalog_url(state: &AppState) -> Result<String, String> {
    let url = state
        .config
        .get(CONFIG_CATALOG_URL)
        .map_err(|e| format!("failed to read catalog URL from config: {e}"))?
        .unwrap_or_else(|| DEFAULT_CATALOG_URL.to_string());
    Ok(url)
}

// ---------------------------------------------------------------------------
// commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn fetch_catalog(state: State<'_, AppState>) -> Result<ExtensionCatalog, String> {
    let catalog_url = get_catalog_url(&state)?;

    tracing::info!("fetching catalog from {}", catalog_url);

    let fetcher = CatalogFetcher::new(state.http_client.clone(), &catalog_url);
    let catalog = fetcher.fetch().await.cmd()?;

    tracing::info!(
        "catalog fetched: {} extensions available",
        catalog.extensions.len()
    );

    Ok(catalog)
}

#[tauri::command]
pub async fn check_extension_updates(
    state: State<'_, AppState>,
    catalog: Option<ExtensionCatalog>,
) -> Result<Vec<ExtensionUpdateInfo>, String> {
    let catalog_url = get_catalog_url(&state)?;
    let fetcher = CatalogFetcher::new(state.http_client.clone(), &catalog_url);
    let checker = UpdateChecker::new(fetcher);

    let updates = match catalog {
        Some(c) => checker.compare(&c, &state.ext_registry).cmd()?,
        None => checker.check_updates(&state.ext_registry).await.cmd()?,
    };

    let updates_available = updates
        .iter()
        .filter(|u| {
            u.status == hagitori_core::entities::catalog::ExtensionSyncStatus::UpdateAvailable
        })
        .count();

    tracing::info!(
        "update check: {} extension(s), {} with updates available",
        updates.len(),
        updates_available
    );

    Ok(updates)
}

// ---------------------------------------------------------------------------
// install or update extension
// ---------------------------------------------------------------------------

async fn install_or_update_extension(
    entry: &hagitori_core::entities::catalog::CatalogEntry,
    catalog: &ExtensionCatalog,
    app: &tauri::AppHandle,
    state: &AppState,
    is_update: bool,
) -> Result<(), String> {
    let extensions_dir = resolve_extensions_dir(app).await?;
    let action = if is_update { "updating" } else { "installing" };

    tracing::info!(
        "{} extension '{}' v{} from catalog",
        action,
        entry.id,
        entry.version_id
    );

    // if updating, remove from ProviderRegistry first
    if is_update {
        let mut registry = state.registry.write().await;
        registry.remove(&entry.id);
    }

    // build raw_base_url for download
    let catalog_url = get_catalog_url(state)?;
    let fetcher = CatalogFetcher::new(state.http_client.clone(), &catalog_url);
    let raw_base_url = fetcher.raw_base_url();

    // download, validate and save to disk
    let installer = ExtensionInstaller::new(state.http_client.clone(), &extensions_dir);
    let final_dir = installer
        .install(entry, catalog, &raw_base_url)
        .await
        .cmd()?;

    // register/update in database
    let resolved_path = entry.relative_path();
    state
        .ext_registry
        .register_catalog(
            &entry.id,
            &entry.name,
            entry.version_id,
            &entry.lang,
            &catalog.repo,
            &catalog.branch,
            &resolved_path,
        )
        .cmd()?;

    // load into ProviderRegistry for immediate use
    let runtime = Arc::new(JsRuntime::new(state.http_client.clone()));
    let loader = ExtensionLoader::new(extensions_dir, runtime);

    match loader.load_extension(&final_dir) {
        Ok(extension) => {
            let meta = extension.meta();
            let mut registry = state.registry.write().await;
            registry.register(Box::new(extension));
            let verb = if is_update {
                "updated and reloaded"
            } else {
                "loaded successfully"
            };
            tracing::info!("extension '{}' ({}) {}", meta.name, meta.id, verb);
        }
        Err(e) => {
            let verb = if is_update { "updated" } else { "installed" };
            return Err(format!(
                "extension '{}' {} on disk but failed to load: {e}",
                entry.id, verb
            ));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn install_catalog_extension(
    extension_id: String,
    catalog: ExtensionCatalog,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let entry = catalog
        .extensions
        .iter()
        .find(|e| e.id == extension_id)
        .ok_or_else(|| format!("extension '{}' not found in catalog", extension_id))?;

    install_or_update_extension(entry, &catalog, &app, &state, false).await
}

#[tauri::command]
pub async fn update_catalog_extension(
    extension_id: String,
    catalog: ExtensionCatalog,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let entry = catalog
        .extensions
        .iter()
        .find(|e| e.id == extension_id)
        .ok_or_else(|| format!("extension '{}' not found in catalog", extension_id))?;

    install_or_update_extension(entry, &catalog, &app, &state, true).await
}

#[tauri::command]
pub async fn remove_catalog_extension(
    extension_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // get extension info to determine directory
    let installed = state.ext_registry
        .get(&extension_id)
        .cmd()?
        .ok_or_else(|| format!("extension '{}' not found in registry", extension_id))?;

    // remove from ProviderRegistry
    {
        let mut registry = state.registry.write().await;
        registry.remove(&extension_id);
    }

    // remove from disk if path exists
    if let Some(ref source_path) = installed.source_path {
        let extensions_dir = resolve_extensions_dir(&app).await?;
        let ext_dir = extensions_dir.join(source_path);
        if ext_dir.exists() {
            tokio::fs::remove_dir_all(&ext_dir)
                .await
                .map_err(|e| format!("failed to remove {}: {e}", ext_dir.display()))?;
        }
    }

    // remove from registry
    state.ext_registry
        .remove(&extension_id)
        .cmd()?;

    tracing::info!("extension '{}' fully removed", extension_id);

    Ok(())
}

#[tauri::command]
pub async fn list_installed_extensions(
    state: State<'_, AppState>,
) -> Result<Vec<InstalledExtension>, String> {
    state.ext_registry.list_all().cmd()
}

#[tauri::command]
pub async fn set_extension_auto_update(
    extension_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.ext_registry
        .set_auto_update(&extension_id, enabled)
        .cmd()?;

    tracing::info!(
        "auto-update for '{}': {}",
        extension_id,
        if enabled { "enabled" } else { "disabled" }
    );

    Ok(())
}

#[tauri::command]
pub async fn set_catalog_url(
    url: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .config
        .set(CONFIG_CATALOG_URL, &url)
        .cmd()?;

    tracing::info!("extension catalog URL set: {}", url);

    Ok(())
}

/// auto-updates extensions on startup: checks the remote catalog and
/// updates all extensions with `auto_update = true` and a newer version.
#[tauri::command]
pub async fn auto_update_extensions(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<hagitori_sync::AutoUpdateResult, String> {
    let catalog_url = get_catalog_url(&state)?;
    let extensions_dir = resolve_extensions_dir(&app).await?;

    let fetcher = CatalogFetcher::new(state.http_client.clone(), &catalog_url);
    let installer = ExtensionInstaller::new(state.http_client.clone(), &extensions_dir);

    let result = hagitori_sync::run_auto_update(&fetcher, &installer, &state.ext_registry)
        .await
        .cmd()?;

    // reload updated extensions in ProviderRegistry
    if !result.updated.is_empty() {
        let runtime = Arc::new(JsRuntime::new(state.http_client.clone()));
        let loader = ExtensionLoader::new(extensions_dir.clone(), runtime);

        for entry in &result.updated {
            // remove old version from registry
            {
                let mut reg = state.registry.write().await;
                reg.remove(&entry.id);
            }

            // try to reload   need to find the extension directory
            let installed = state
                .ext_registry
                .get(&entry.id)
                .map_err(|e| format!("failed to read registry for '{}': {e}", entry.id))?;

            let Some(installed) = installed else {
                tracing::warn!(
                    "auto-update: '{}' not found in registry after update   skipping reload",
                    entry.id
                );
                continue;
            };

            let Some(ref source_path) = installed.source_path else {
                tracing::warn!(
                    "auto-update: '{}' has no source_path in registry   skipping reload",
                    entry.id
                );
                continue;
            };

            let ext_dir = extensions_dir.join(source_path);
            match loader.load_extension(&ext_dir) {
                Ok(extension) => {
                    let meta = extension.meta();
                    let mut reg = state.registry.write().await;
                    reg.register(Box::new(extension));
                    tracing::info!(
                        "auto-update: '{}' ({}) reloaded v{}",
                        meta.name,
                        meta.id,
                        entry.to_version
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "auto-update: '{}' updated but failed to reload: {e}",
                        entry.id
                    );
                }
            }
        }
    }

    Ok(result)
}
