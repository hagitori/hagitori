use std::collections::HashMap;

use hagitori_core::entities::catalog::CatalogEntry;
use hagitori_core::prelude::*;

#[test]
fn manga_serde_roundtrip() {
    let manga = Manga {
        id: "https://site.com/manga/123".to_string(),
        name: "Bleach".to_string(),
        cover: Some("https://cdn.com/bleach.jpg".to_string()),
        source: "pt_br.manga_site".to_string(),
        url: None,
    };

    let json = serde_json::to_string(&manga).unwrap();
    let deser: Manga = serde_json::from_str(&json).unwrap();
    assert_eq!(manga, deser);
}

#[test]
fn chapter_serde_roundtrip() {
    let chapter = Chapter {
        id: "https://site.com/ch/42".to_string(),
        number: "42".to_string(),
        name: "Dragon Ball".to_string(),
        title: Some("O Torneio".to_string()),
        date: Some("2024-06-01".to_string()),
        scanlator: None,
    };

    let json = serde_json::to_string(&chapter).unwrap();
    let deser: Chapter = serde_json::from_str(&json).unwrap();
    assert_eq!(chapter, deser);
}

#[test]
fn pages_serde_roundtrip() {
    let mut headers = HashMap::new();
    headers.insert("Referer".to_string(), "https://site.com".to_string());

    let pages = Pages {
        chapter_id: "ch-42".to_string(),
        chapter_number: "42".to_string(),
        manga_name: "Test Manga".to_string(),
        pages: vec!["https://cdn.com/p1.jpg".to_string()],
        headers: Some(headers),
        use_browser: false,
        scanlator: None,
    };

    let json = serde_json::to_string(&pages).unwrap();
    let deser: Pages = serde_json::from_str(&json).unwrap();
    assert_eq!(pages, deser);
}

#[test]
fn download_status_state_checks() {
    // is_finished
    assert!(!DownloadStatus::Queued.is_finished());
    assert!(!DownloadStatus::Downloading.is_finished());
    assert!(!DownloadStatus::Processing.is_finished());
    assert!(DownloadStatus::Completed.is_finished());
    assert!(DownloadStatus::Failed("erro".into()).is_finished());

    // is_active
    assert!(!DownloadStatus::Queued.is_active());
    assert!(DownloadStatus::Downloading.is_active());
    assert!(DownloadStatus::Processing.is_active());
    assert!(!DownloadStatus::Completed.is_active());
    assert!(!DownloadStatus::Failed("erro".into()).is_active());
}

#[test]
fn download_status_serde_roundtrip() {
    let statuses = vec![
        DownloadStatus::Queued,
        DownloadStatus::Downloading,
        DownloadStatus::Processing,
        DownloadStatus::Completed,
        DownloadStatus::Failed("connection timeout".to_string()),
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deser: DownloadStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deser);
    }
}

#[test]
fn extension_meta_features_and_serde() {
    let meta = ExtensionMeta::new(
        "en.protected_site",
        "ProtectedSite",
        "en",
        "2.0.0",
        vec!["protected.com".to_string()],
    )
    .with_features(vec!["browser".to_string(), "crypto".to_string()]);

    assert!(meta.requires_browser());
    assert!(meta.requires_crypto());

    let json = serde_json::to_string(&meta).unwrap();
    let deser: ExtensionMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(meta, deser);
}

#[test]
fn error_types_and_conversions() {
    // display messages
    assert_eq!(HagitoriError::http("refused").to_string(), "HTTP error: refused");
    assert_eq!(HagitoriError::browser("not found").to_string(), "browser error: not found");
    assert_eq!(HagitoriError::download("disk full").to_string(), "download error: disk full");

    // From<io::Error>
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    assert!(matches!(HagitoriError::from(io_err), HagitoriError::Io(_)));

    // From<serde_json::Error>
    let json_err = serde_json::from_str::<Manga>("invalid").unwrap_err();
    assert!(matches!(HagitoriError::from(json_err), HagitoriError::Json(_)));
}

#[test]
fn json_contract_chapters_array() {
    let json = r#"[
        {"id": "ch-1", "number": "1", "name": "One Piece", "title": "Romance Dawn", "date": "1997-07-22"},
        {"id": "ch-2", "number": "2", "name": "One Piece", "title": null, "date": null}
    ]"#;

    let chapters: Vec<Chapter> = serde_json::from_str(json).unwrap();
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].title, Some("Romance Dawn".to_string()));
    assert!(chapters[1].title.is_none());
}

#[test]
fn json_contract_download_status_tagged() {
    let status: DownloadStatus = serde_json::from_str(r#""queued""#).unwrap();
    assert_eq!(status, DownloadStatus::Queued);

    let status: DownloadStatus = serde_json::from_str(r#"{"failed":"timeout"}"#).unwrap();
    assert_eq!(status, DownloadStatus::Failed("timeout".to_string()));
}

// ---------------------------------------------------------------------------
// CatalogEntry::relative_path
// ---------------------------------------------------------------------------

fn make_entry(path: &str) -> CatalogEntry {
    CatalogEntry {
        id: "test.ext".to_string(),
        name: "Test".to_string(),
        lang: "en".to_string(),
        version_id: 1,
        path: path.to_string(),
        entry: "index.js".to_string(),
        requires: vec![],
        icon: None,
        domains: vec![],
        features: vec![],
        supports_details: false,
        languages: vec![],
        files: Default::default(),
        min_app_version: None,
    }
}

#[test]
fn relative_path_strips_builds_prefix() {
    let entry = make_entry("builds/pt-br/sakuramangas");
    assert_eq!(entry.relative_path(), "pt-br/sakuramangas");
}

#[test]
fn relative_path_preserves_two_component() {
    let entry = make_entry("pt-br/sakuramangas");
    assert_eq!(entry.relative_path(), "pt-br/sakuramangas");
}

#[test]
fn relative_path_single_component() {
    let entry = make_entry("index.js");
    assert_eq!(entry.relative_path(), "index.js");
}

#[test]
fn relative_path_deep_prefix() {
    let entry = make_entry("some/deep/prefix/en/test_ext");
    assert_eq!(
        entry.relative_path(),
        "en/test_ext",
        "should always take only the last 2 segments"
    );
}
