use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use hagitori_extensions::{ExtensionLoader, JsRuntime};
use hagitori_http::HttpClient;

fn create_test_runtime() -> Arc<JsRuntime> {
    let http_client = Arc::new(HttpClient::new().unwrap());
    Arc::new(JsRuntime::new(http_client))
}

/// Helper that creates package.json + index.js files in the extension directory.
fn write_extension(ext_dir: &std::path::Path, _name: &str, id: &str, lang: &str, version: u32, domains: &[&str], script: &str) {
    fs::create_dir_all(ext_dir).unwrap();

    let domains_json: Vec<String> = domains.iter().map(|d| format!("\"{}\"", d)).collect();
    let pkg = format!(r#"{{
        "name": "hagitori.{lang}.{id}",
        "version": {version},
        "main": "index.js",
        "hagitori": {{
            "apiVersion": 1,
            "type": "source",
            "lang": "{lang}",
            "domains": [{}]
        }}
    }}"#, domains_json.join(", "));

    fs::write(ext_dir.join("package.json"), pkg).unwrap();
    fs::write(ext_dir.join("index.js"), script).unwrap();
}

#[test]
fn load_all_returns_empty_for_nonexistent_dir() {
    let loader = ExtensionLoader::new(
        PathBuf::from("/tmp/hagitori_test_nonexistent"),
        create_test_runtime(),
    );

    let (result, _errors) = loader.load_all();
    assert!(result.is_empty());
}

#[test]
fn load_extension_with_valid_files() {
    let dir = tempfile::tempdir().unwrap();
    let ext_dir = dir.path().join("pt_br").join("test_site");

    write_extension(
        &ext_dir,
        "TestSite",
        "test_site",
        "pt-br",
        1,
        &["testsite.com"],
        r#"
function getManga(url) {
    return Manga({ url: url, title: "Test Manga" });
}
function getChapters(mangaId) {
    return [Chapter({ id: mangaId + "/1", number: "1", mangaTitle: "Test Manga" })];
}
function getPages(chapter) {
    return Pages({ chapterId: chapter.id, chapterNumber: chapter.number, mangaTitle: chapter.mangaTitle, pages: [] });
}
        "#,
    );

    let loader = ExtensionLoader::new(dir.path().to_path_buf(), create_test_runtime());
    let (extensions, _errors) = loader.load_all();
    assert_eq!(extensions.len(), 1);

    use hagitori_core::provider::MangaProvider;
    assert_eq!(extensions[0].meta().id, "hagitori.pt-br.test_site");
}

#[test]
fn load_extension_fails_without_package_json() {
    let dir = tempfile::tempdir().unwrap();
    let ext_dir = dir.path().join("pt_br").join("test_ext");
    fs::create_dir_all(&ext_dir).unwrap();

    // Directory exists but no package.json
    fs::write(ext_dir.join("readme.txt"), "nothing here").unwrap();

    let loader = ExtensionLoader::new(dir.path().to_path_buf(), create_test_runtime());
    let result = loader.load_extension(&ext_dir);
    assert!(result.is_err());
}

#[test]
fn load_extension_with_custom_entry_point() {
    let dir = tempfile::tempdir().unwrap();
    let ext_dir = dir.path().join("en").join("custom");
    fs::create_dir_all(&ext_dir).unwrap();

    // package.json aponta para src/main.js
    let pkg = r#"{
        "name": "hagitori.en.custom",
        "version": 1,
        "main": "src/main.js",
        "hagitori": {
            "apiVersion": 1,
            "type": "source",
            "lang": "en",
            "domains": ["custom.com"]
        }
    }"#;

    fs::write(ext_dir.join("package.json"), pkg).unwrap();
    fs::create_dir_all(ext_dir.join("src")).unwrap();
    fs::write(
        ext_dir.join("src").join("main.js"),
        r#"
function getManga(url) {
    return Manga({ url: url, title: "Custom" });
}
function getChapters(mangaId) {
    return [Chapter({ id: mangaId + "/1", number: "1", mangaTitle: "Custom" })];
}
function getPages(chapter) {
    return Pages({ chapterId: chapter.id, chapterNumber: chapter.number, mangaTitle: chapter.mangaTitle, pages: [] });
}
"#,
    ).unwrap();

    let loader = ExtensionLoader::new(dir.path().to_path_buf(), create_test_runtime());

    let (extensions, _errors) = loader.load_all();
    assert_eq!(extensions.len(), 1);

    use hagitori_core::provider::MangaProvider;
    assert_eq!(extensions[0].meta().id, "hagitori.en.custom");
}
