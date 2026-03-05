use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;

use hagitori_core::entities::{Chapter, ExtensionMeta, Manga, Pages};
use hagitori_core::error::Result;
use hagitori_core::provider::MangaProvider;
use hagitori_http::HttpClient;
use hagitori_providers::ProviderRegistry;

/// mock provider for unit tests.
struct MockProvider {
    meta: ExtensionMeta,
}

impl MockProvider {
    fn new(id: &str, name: &str, domains: Vec<String>) -> Self {
        Self {
            meta: ExtensionMeta::new(id, name, "en", "1.0.0", domains),
        }
    }
}

#[async_trait]
impl MangaProvider for MockProvider {
    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn get_manga(&self, url: &str) -> Result<Manga> {
        Ok(Manga::new("mock-manga", format!("Mock Manga from {}", url), "mock"))
    }

    async fn get_chapters(&self, _manga_id: &str) -> Result<Vec<Chapter>> {
        Ok(vec![Chapter::new("ch-1", "1", "Test Chapter")])
    }

    async fn get_pages(&self, chapter: &Chapter) -> Result<Pages> {
        Ok(Pages::new(
            &chapter.id,
            &chapter.number,
            "Mock Manga",
            vec!["https://example.com/page1.jpg".into()],
        ))
    }
}

#[test]
fn register_and_get_provider() {
    let mut registry = ProviderRegistry::new();
    let provider = MockProvider::new("en.test", "TestProvider", vec!["test.com".into()]);

    registry.register(Box::new(provider));

    let found = registry.find_provider_by_url("https://test.com/manga/1");
    assert!(found.is_ok(), "registered provider should be found by URL");
    assert_eq!(found.unwrap().meta().name, "TestProvider", "provider name should match");
}

#[test]
fn find_provider_by_url_matches_domain() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(MockProvider::new(
        "en.manga",
        "MangaSite",
        vec!["mangasite.com".into(), "api.mangasite.com".into()],
    )));

    let provider = registry.find_provider_by_url("https://mangasite.com/manga/123").unwrap();
    assert_eq!(provider.meta().id, "en.manga", "should find provider by domain");

    let provider = registry.find_provider_by_url("https://api.mangasite.com/v1/manga/123").unwrap();
    assert_eq!(provider.meta().id, "en.manga", "should find provider by subdomain");
}

#[test]
fn find_provider_by_url_normalizes_www() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(MockProvider::new(
        "en.site",
        "Site",
        vec!["example.com".into()],
    )));

    let provider = registry.find_provider_by_url("https://www.example.com/manga/1").unwrap();
    assert_eq!(provider.meta().id, "en.site", "should normalize www in domain");
}

#[test]
fn find_provider_by_url_returns_error_for_unknown_domain() {
    let registry = ProviderRegistry::new();
    let result = registry.find_provider_by_url("https://unknown.com/page");
    assert!(result.is_err(), "unknown domain should return error");
}

#[test]
fn find_provider_by_url_returns_error_for_invalid_url() {
    let registry = ProviderRegistry::new();
    let result = registry.find_provider_by_url("not a valid url");
    assert!(result.is_err(), "invalid URL should return error");
}

#[test]
fn list_returns_all_metas() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(MockProvider::new(
        "en.a",
        "ProviderA",
        vec!["a.com".into()],
    )));
    registry.register(Box::new(MockProvider::new(
        "en.b",
        "ProviderB",
        vec!["b.com".into()],
    )));

    let metas = registry.list();
    assert_eq!(metas.len(), 2, "should list all providers");

    let ids: Vec<&str> = metas.iter().map(|m| m.id.as_str()).collect();
    assert!(ids.contains(&"en.a"), "should contain provider en.a");
    assert!(ids.contains(&"en.b"), "should contain provider en.b");
}

#[test]
fn remove_provider() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(MockProvider::new(
        "en.rem",
        "Removable",
        vec!["removable.com".into()],
    )));

    let found_before = registry.find_provider_by_url("https://removable.com/manga/1");
    assert!(found_before.is_ok(), "provider should be found before removal");

    let removed = registry.remove("en.rem");
    assert!(removed.is_some(), "removal should return the provider");

    let found_after = registry.find_provider_by_url("https://removable.com/manga/1");
    assert!(found_after.is_err(), "provider should not be found after removal");
}

#[test]
fn load_extensions_from_directory() {
    let dir = tempfile::tempdir().unwrap();
    let ext_dir = dir.path().join("en").join("test_ext");
    fs::create_dir_all(&ext_dir).unwrap();

    fs::write(
        ext_dir.join("package.json"),
        r#"{
            "name": "hagitori.en.test_ext",
            "version": 1,
            "main": "index.js",
            "hagitori": {
                "apiVersion": 1,
                "type": "source",
                "lang": "en",
                "displayName": "TestExt",
                "domains": ["testext.com"]
            }
        }"#,
    )
    .unwrap();

    fs::write(
        ext_dir.join("index.js"),
        r#"
function getManga(url) {
    return Manga({ id: url, name: "Test Manga" });
}
function getChapters(mangaId) {
    return [Chapter({ id: mangaId + "/1", number: "1", name: "Cap 1" })];
}
function getPages(chapter) {
    return Pages({ id: chapter.id, number: chapter.number, name: chapter.name, urls: [] });
}
        "#,
    )
    .unwrap();

    let mut registry = ProviderRegistry::new();
    let http_client = Arc::new(HttpClient::new().unwrap());

    let count = registry.load_extensions(dir.path(), http_client, Arc::new(Mutex::new(None))).unwrap();
    assert_eq!(count, 1, "should load one extension");

    let metas = registry.list();
    assert_eq!(metas.len(), 1, "registry should have one extension");

    let found = registry.find_provider_by_url("https://testext.com/manga/1");
    assert!(found.is_ok(), "extension domain should be registered");
}
