use std::sync::Arc;

use hagitori_core::entities::catalog::CatalogEntry;
use hagitori_http::HttpClient;
use hagitori_sync::installer::ExtensionInstaller;

#[test]
fn sanitize_dir_name() {
    assert_eq!(
        hagitori_sync::installer::sanitize_dir_name("pt_br.sakura"),
        "pt_br.sakura",
        "valid name should not be changed"
    );
    assert_eq!(
        hagitori_sync::installer::sanitize_dir_name("foo/bar"),
        "foo_bar",
        "slash should be replaced by underscore"
    );
    assert_eq!(
        hagitori_sync::installer::sanitize_dir_name("a b c"),
        "a_b_c",
        "spaces should be replaced by underscore"
    );
}

#[test]
fn resolve_final_dir() {
    let http = Arc::new(HttpClient::new().unwrap());
    let installer = ExtensionInstaller::new(http, "/tmp/extensions");
    let entry = CatalogEntry {
        id: "pt_br.sakura".to_string(),
        name: "Sakura".to_string(),
        lang: "pt-br".to_string(),
        version_id: 1,
        path: "builds/pt-br/sakuramangas".to_string(),
        entry: "index.js".to_string(),
        requires: vec![],
        icon: None,
        domains: vec![],
        features: vec![],
        supports_details: false,
        languages: vec![],
        files: Default::default(),
        min_app_version: None,
    };

    let dir = installer.resolve_final_dir(&entry);
    assert!(
        dir.ends_with("pt-br/sakuramangas"),
        "should strip 'builds/' prefix: got {}",
        dir.display()
    );
}

#[test]
fn resolve_final_dir_short_path() {
    let http = Arc::new(HttpClient::new().unwrap());
    let installer = ExtensionInstaller::new(http, "/tmp/extensions");
    let entry = CatalogEntry {
        id: "en.test".to_string(),
        name: "Test".to_string(),
        lang: "en".to_string(),
        version_id: 1,
        path: "en/test_ext".to_string(),
        entry: "index.js".to_string(),
        requires: vec![],
        icon: None,
        domains: vec![],
        features: vec![],
        supports_details: false,
        languages: vec![],
        files: Default::default(),
        min_app_version: None,
    };

    let dir = installer.resolve_final_dir(&entry);
    assert!(
        dir.ends_with("en/test_ext"),
        "2-component path should remain unchanged: got {}",
        dir.display()
    );
}
