use std::sync::Arc;

use hagitori_http::HttpClient;
use hagitori_sync::catalog::CatalogFetcher;

#[test]
fn validate_catalog_path_accepts_valid_rejects_invalid() {
    use hagitori_sync::catalog::validate_catalog_path;

    // valid paths
    assert!(validate_catalog_path("builds/pt-br/sakuramangas").is_ok());
    assert!(validate_catalog_path("pt-br/sakuramangas").is_ok());
    assert!(validate_catalog_path("index.js").is_ok());
    assert!(validate_catalog_path("lib/utils.js").is_ok());
    assert!(validate_catalog_path("icon.png").is_ok());

    // invalid: traversal, absolute, empty, null bytes
    assert!(validate_catalog_path("../etc/passwd").is_err());
    assert!(validate_catalog_path("foo/../bar").is_err());
    assert!(validate_catalog_path("/etc/passwd").is_err());
    assert!(validate_catalog_path("C:\\Windows").is_err());
    assert!(validate_catalog_path("\\\\server\\share").is_err());
    assert!(validate_catalog_path("").is_err());
    assert!(validate_catalog_path("foo\0bar").is_err());
}

#[test]
fn raw_base_url_construction() {
    let http = Arc::new(HttpClient::new().unwrap());

    // strips catalog.json
    let fetcher = CatalogFetcher::new(
        http.clone(),
        "https://raw.githubusercontent.com/hagitori/hagitori-extensions/main/catalog.json",
    );
    assert_eq!(
        fetcher.raw_base_url(),
        "https://raw.githubusercontent.com/hagitori/hagitori-extensions/main",
    );

    // preserves subpath
    let fetcher = CatalogFetcher::new(
        http.clone(),
        "https://raw.githubusercontent.com/user/repo/main/path/to/extensions/catalog.json",
    );
    assert_eq!(
        fetcher.raw_base_url(),
        "https://raw.githubusercontent.com/user/repo/main/path/to/extensions",
    );

    // no catalog suffix   unchanged
    let fetcher = CatalogFetcher::new(http, "https://example.com/extensions");
    assert_eq!(fetcher.raw_base_url(), "https://example.com/extensions");
}
