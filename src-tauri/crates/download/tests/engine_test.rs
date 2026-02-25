use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use hagitori_core::entities::{DownloadProgress, DownloadStatus, Pages};
use hagitori_download::{DownloadEngine, DownloadEngineConfig};
use hagitori_http::HttpClient;

fn create_test_engine(dir: &std::path::Path) -> DownloadEngine {
    let http_client = Arc::new(HttpClient::new().unwrap());
    let config = DownloadEngineConfig {
        max_retries: 2,
        download_dir: dir.to_path_buf(),
        max_concurrent_pages: 5,
        image_format: "original".to_string(),
    };
    DownloadEngine::with_browser(http_client, config, Arc::new(Mutex::new(None)))
}

fn create_test_pages(urls: Vec<String>) -> Pages {
    Pages::new("ch-1", "1", "Test Manga", urls)
}

#[tokio::test]
async fn download_empty_pages_completes_successfully() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_test_engine(dir.path());
    let pages = create_test_pages(vec![]);
    let (tx, mut rx) = mpsc::channel::<DownloadProgress>(32);
    let cancel = CancellationToken::new();

    let result = engine.download_chapter(&pages, &tx, &cancel).await;
    assert!(result.is_ok());

    drop(tx);
    let mut progress_msgs = Vec::new();
    while let Some(msg) = rx.recv().await {
        progress_msgs.push(msg);
    }

    assert!(progress_msgs.len() >= 2);
    assert!(matches!(
        progress_msgs.last().unwrap().status,
        DownloadStatus::Completed
    ));
}

#[tokio::test]
async fn download_invalid_urls_fails_with_retry() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_test_engine(dir.path());
    let pages = create_test_pages(vec!["http://invalid.test.localhost/page1.jpg".into()]);
    let (tx, _rx) = mpsc::channel::<DownloadProgress>(32);
    let cancel = CancellationToken::new();

    let result = engine.download_chapter(&pages, &tx, &cancel).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cancel_token_stops_download() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_test_engine(dir.path());
    let pages = create_test_pages(vec![
        "http://invalid.test.localhost/page1.jpg".into(),
        "http://invalid.test.localhost/page2.jpg".into(),
    ]);
    let (tx, _rx) = mpsc::channel::<DownloadProgress>(32);
    let cancel = CancellationToken::new();

    // cancel immediately
    cancel.cancel();

    let result = engine.download_chapter(&pages, &tx, &cancel).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cancelled"), "error: {err_msg}");
}

#[tokio::test]
async fn progress_channel_receives_updates() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_test_engine(dir.path());
    let pages = create_test_pages(vec![]);
    let (tx, mut rx) = mpsc::channel::<DownloadProgress>(32);
    let cancel = CancellationToken::new();

    engine.download_chapter(&pages, &tx, &cancel).await.unwrap();
    drop(tx);

    let mut received = Vec::new();
    while let Some(msg) = rx.recv().await {
        received.push(msg);
    }

    assert!(!received.is_empty());

    // first event should be Downloading
    assert!(matches!(received[0].status, DownloadStatus::Downloading));
    assert_eq!(received[0].manga_name, "Test Manga");
    assert_eq!(received[0].chapter_number, "1");
}
