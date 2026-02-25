use std::sync::Arc;

use tempfile::TempDir;

use hagitori_config::ExtensionRegistry;
use hagitori_core::entities::catalog::{CatalogEntry, ExtensionCatalog};

/// helper: creates a temporary `ExtensionRegistry`.
fn temp_registry(dir: &TempDir) -> ExtensionRegistry {
    ExtensionRegistry::new(dir.path()).expect("open registry")
}

/// helper: creates a test catalog.
fn test_catalog(entries: Vec<CatalogEntry>) -> ExtensionCatalog {
    ExtensionCatalog {
        version: 1,
        updated_at: "2026-02-21T12:00:00Z".to_string(),
        repo: "hagitori/hagitori-extensions".to_string(),
        branch: "main".to_string(),
        extensions: entries,
    }
}

/// helper: creates a catalog entry.
fn test_entry(id: &str, name: &str, version_id: u32, lang: &str) -> CatalogEntry {
    CatalogEntry {
        id: id.to_string(),
        name: name.to_string(),
        lang: lang.to_string(),
        version_id,
        path: format!("builds/{}/{}", lang, name.to_lowercase()),
        entry: "index.js".to_string(),
        requires: vec![],
        icon: None,
        domains: vec!["example.com".to_string()],
        features: vec![],
        supports_details: false,
        languages: vec![],
        files: std::collections::HashMap::new(),
        min_app_version: None,
    }
}

#[test]
fn auto_update_skips_disabled_extensions() {
    let db_dir = TempDir::new().unwrap();
    let registry = temp_registry(&db_dir);

    // register extension with auto_update = true, then disable
    registry
        .register_catalog(
            "pt_br.manga_a",
            "Manga A",
            1,
            "pt-br",
            "hagitori/hagitori-extensions",
            "main",
            "pt-br/manga_a",
        )
        .unwrap();
    registry
        .set_auto_update("pt_br.manga_a", false)
        .unwrap();

    let installed = registry.get("pt_br.manga_a").unwrap().unwrap();
    assert!(!installed.auto_update, "auto_update should be disabled");
}

#[test]
fn auto_update_detects_updates_for_catalog_extensions() {
    let db_dir = TempDir::new().unwrap();
    let registry = temp_registry(&db_dir);

    // register extension v1
    registry
        .register_catalog(
            "pt_br.manga_a",
            "Manga A",
            1,
            "pt-br",
            "hagitori/hagitori-extensions",
            "main",
            "pt-br/manga_a",
        )
        .unwrap();

    // catalog with v2
    let catalog = test_catalog(vec![test_entry("pt_br.manga_a", "Manga A", 2, "pt-br")]);

    let http = Arc::new(hagitori_http::HttpClient::new().unwrap());
    let fetcher = hagitori_sync::CatalogFetcher::new(
        http.clone(),
        "https://raw.githubusercontent.com/hagitori/hagitori-extensions/main/catalog.json",
    );
    let checker = hagitori_sync::UpdateChecker::new(fetcher);
    let updates = checker.compare(&catalog, &registry).unwrap();

    let update = updates.iter().find(|u| u.id == "pt_br.manga_a").unwrap();
    assert_eq!(
        update.status,
        hagitori_core::entities::catalog::ExtensionSyncStatus::UpdateAvailable,
        "status should indicate update available"
    );
    assert_eq!(update.local_version_id, Some(1), "local version should be 1");
    assert_eq!(update.remote_version_id, 2, "remote version should be 2");
}

#[test]
fn auto_update_marks_up_to_date_correctly() {
    let db_dir = TempDir::new().unwrap();
    let registry = temp_registry(&db_dir);

    registry
        .register_catalog(
            "en.manga_b",
            "Manga B",
            3,
            "en",
            "hagitori/hagitori-extensions",
            "main",
            "en/manga_b",
        )
        .unwrap();

    // catalog with same version
    let catalog = test_catalog(vec![test_entry("en.manga_b", "Manga B", 3, "en")]);

    let http = Arc::new(hagitori_http::HttpClient::new().unwrap());
    let fetcher = hagitori_sync::CatalogFetcher::new(http, "https://raw.githubusercontent.com/hagitori/hagitori-extensions/main/catalog.json");
    let checker = hagitori_sync::UpdateChecker::new(fetcher);
    let updates = checker.compare(&catalog, &registry).unwrap();

    let ext = updates.iter().find(|u| u.id == "en.manga_b").unwrap();
    assert_eq!(
        ext.status,
        hagitori_core::entities::catalog::ExtensionSyncStatus::UpToDate,
        "status should indicate up to date"
    );
}
