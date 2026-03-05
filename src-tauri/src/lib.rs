//! # hagitori
//!
//! tauri app entry point, command handlers, and global state.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use tauri::Manager;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use hagitori_browser::{set_default_profile_dir, BrowserManager};
use hagitori_config::{ConfigManager, DownloadHistory, ExtensionRegistry, LibraryManager, SessionStore};
use hagitori_core::entities::Manga;
use hagitori_http::HttpClient;
use hagitori_providers::ProviderRegistry;

mod commands;
mod sync_commands;
pub mod utils;

// ---------------------------------------------------------------------------
// shared app state
// ---------------------------------------------------------------------------

const CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(256).unwrap();

pub struct AppState {
    pub(crate) registry: RwLock<ProviderRegistry>,
    pub(crate) http_client: Arc<HttpClient>,
    pub(crate) config: Arc<ConfigManager>,
    pub(crate) ext_registry: Arc<ExtensionRegistry>,
    pub(crate) download_history: Arc<DownloadHistory>,
    pub(crate) session_store: Arc<SessionStore>,
    pub(crate) library: Arc<LibraryManager>,
    pub(crate) manga_cache: RwLock<lru::LruCache<String, Manga>>,
    pub(crate) provider_cache: RwLock<lru::LruCache<String, String>>,
    pub(crate) cancel_token: Mutex<CancellationToken>,
    pub(crate) browser_manager: Arc<tokio::sync::Mutex<Option<Arc<BrowserManager>>>>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    fn new(
        registry: ProviderRegistry,
        config: ConfigManager,
        http_client: Arc<HttpClient>,
        ext_registry: Arc<ExtensionRegistry>,
        download_history: Arc<DownloadHistory>,
        session_store: Arc<SessionStore>,
        library: LibraryManager,
        browser_manager: Arc<tokio::sync::Mutex<Option<Arc<BrowserManager>>>>,
    ) -> Self {
        Self {
            registry: RwLock::new(registry),
            http_client,
            config: Arc::new(config),
            ext_registry,
            download_history,
            session_store,
            library: Arc::new(library),
            manga_cache: RwLock::new(lru::LruCache::new(CACHE_SIZE)),
            provider_cache: RwLock::new(lru::LruCache::new(CACHE_SIZE)),
            cancel_token: Mutex::new(CancellationToken::new()),
            browser_manager,
        }
    }
}

// ---------------------------------------------------------------------------
// entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("hagitori=info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let http_client = Arc::new(
                HttpClient::new().map_err(|e| format!("failed to create HttpClient: {e}"))?,
            );

            let data_dir = hagitori_config::data_dir()
                .map_err(|e| format!("failed to determine data directory: {e}"))?;

            let browser_profile_dir = data_dir.join("browser_profile");
            if !browser_profile_dir.exists() {
                std::fs::create_dir_all(&browser_profile_dir)
                    .map_err(|e| format!("failed to create browser profile directory: {e}"))?;
            }

            set_default_profile_dir(browser_profile_dir.clone());
            tracing::info!(
                "persistent browser profile set at {}",
                browser_profile_dir.display()
            );

            let config = ConfigManager::new(&data_dir)
                .map_err(|e| format!("failed to initialize ConfigManager: {e}"))?;

            let mut registry = ProviderRegistry::new();

            let browser_manager: Arc<tokio::sync::Mutex<Option<Arc<BrowserManager>>>> =
                Arc::new(tokio::sync::Mutex::new(None));

            // load installed extensions from catalog (app_data_dir/extensions)
            let extensions_dir = data_dir.join("extensions");
            if !extensions_dir.exists() {
                std::fs::create_dir_all(&extensions_dir)
                    .map_err(|e| format!("failed to create extensions directory: {e}"))?;
            }

            tracing::info!(
                "extensions directory: {} (exists: {})",
                extensions_dir.display(),
                extensions_dir.exists()
            );

            // list contents for diagnostics
            if let Ok(entries) = std::fs::read_dir(&extensions_dir) {
                for entry in entries.flatten() {
                    tracing::debug!(
                        "  ↳ {} (dir: {})",
                        entry.path().display(),
                        entry.path().is_dir()
                    );
                }
            }

            if extensions_dir.exists() {
                match registry.load_extensions(
                    &extensions_dir,
                    http_client.clone(),
                    browser_manager.clone(),
                ) {
                    Ok(count) => {
                        tracing::info!(
                            "{} extension(s) loaded from {}",
                            count,
                            extensions_dir.display()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "failed to load extensions from {}: {}",
                            extensions_dir.display(),
                            e
                        );
                    }
                }
            }

            let ext_registry = Arc::new(
                ExtensionRegistry::new(&data_dir)
                    .map_err(|e| format!("failed to open ExtensionRegistry: {e}"))?,
            );

            let download_history = Arc::new(
                DownloadHistory::new(&data_dir)
                    .map_err(|e| format!("failed to open DownloadHistory: {e}"))?,
            );

            let session_store = Arc::new(
                SessionStore::new(&data_dir)
                    .map_err(|e| format!("failed to open SessionStore: {e}"))?,
            );

            // load persisted sessions (SQLite) into the HttpClient's in-memory session store
            match session_store.load_all() {
                Ok(persisted) if !persisted.is_empty() => {
                    let domain_sessions: HashMap<String, hagitori_http::DomainSession> = persisted
                        .into_iter()
                        .map(|(domain, data)| {
                            (domain, hagitori_http::DomainSession {
                                cookies: data.cookies,
                                headers: data.headers,
                                user_agent: data.user_agent,
                            })
                        })
                        .collect();
                    let count = domain_sessions.len();
                    http_client.session_store().import_all(domain_sessions);
                    tracing::info!("{count} session(s) restored from database");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("failed to load persisted sessions: {e}"),
            }

            let library = LibraryManager::new(&data_dir)
                .map_err(|e| format!("failed to open LibraryManager: {e}"))?;

            let app_state = AppState::new(
                registry,
                config,
                http_client,
                ext_registry,
                download_history,
                session_store,
                library,
                browser_manager,
            );
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::manga::get_manga,
            commands::manga::get_chapters,
            commands::manga::get_details,
            commands::manga::set_extension_lang,
            commands::manga::list_extensions,
            commands::download::download_chapters,
            commands::download::cancel_download,
            commands::config::get_config,
            commands::config::set_config,
            commands::config::get_download_path,
            commands::library::library_list,
            commands::library::library_get,
            commands::library::library_add,
            commands::library::library_remove,
            commands::library::library_update_chapters,
            commands::library::library_update_details,
            commands::library::library_update_cover,
            commands::library::library_set_source_meta,
            commands::library::library_get_source_meta,
            commands::library::library_set_extension_lang,
            commands::library::library_get_extension_langs,
            sync_commands::fetch_catalog,
            sync_commands::check_extension_updates,
            sync_commands::install_catalog_extension,
            sync_commands::update_catalog_extension,
            sync_commands::remove_catalog_extension,
            sync_commands::list_installed_extensions,
            sync_commands::set_extension_auto_update,
            sync_commands::set_catalog_url,
            sync_commands::auto_update_extensions,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // persist in-memory sessions to SQLite before exit
                let state = app.state::<AppState>();
                let in_memory = state.http_client.session_store().export_all();
                let mut saved = 0usize;
                for (domain, session) in &in_memory {
                    let data = hagitori_config::SessionData {
                        cookies: session.cookies.clone(),
                        headers: session.headers.clone(),
                        user_agent: session.user_agent.clone(),
                    };
                    if let Err(e) = state.session_store.save(domain, &data) {
                        tracing::warn!("failed to persist session for '{domain}': {e}");
                    } else {
                        saved += 1;
                    }
                }
                if saved > 0 {
                    tracing::info!("{saved} session(s) persisted to database");
                }
            }
        });
}
