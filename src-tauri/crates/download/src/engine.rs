//! concurrent chapter page download engine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use hagitori_browser::BrowserManager;
use hagitori_core::entities::{DownloadProgress, DownloadStatus, Pages};
use hagitori_core::error::{HagitoriError, Result};
use hagitori_http::client::RequestOptions;
use hagitori_http::HttpClient;

use crate::browser::{download_page_browser_retry, get_or_launch_browser};
use crate::http::download_page_http_retry;

const DEFAULT_MAX_CONCURRENT_PAGES: usize = 3;
const MAX_CONCURRENT_PAGES: usize = 5;

#[derive(Debug, Clone)]
pub struct DownloadEngineConfig {
    pub max_retries: u32,
    pub download_dir: PathBuf,
    pub max_concurrent_pages: usize,
    pub image_format: String,
}

impl Default for DownloadEngineConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            download_dir: PathBuf::from("downloads"),
            max_concurrent_pages: DEFAULT_MAX_CONCURRENT_PAGES,
            image_format: "original".to_string(),
        }
    }
}

pub struct DownloadEngine {
    http_client: Arc<HttpClient>,
    browser_manager: Arc<Mutex<Option<Arc<BrowserManager>>>>,
    config: DownloadEngineConfig,
    page_semaphore: Arc<Semaphore>,
}

impl DownloadEngine {
    pub fn with_browser(
        http_client: Arc<HttpClient>,
        config: DownloadEngineConfig,
        browser_manager: Arc<Mutex<Option<Arc<BrowserManager>>>>,
    ) -> Self {
        let max_pages = config.max_concurrent_pages.min(MAX_CONCURRENT_PAGES);
        Self {
            http_client,
            browser_manager,
            config,
            page_semaphore: Arc::new(Semaphore::new(max_pages)),
        }
    }

    pub async fn download_chapter(
        &self,
        pages: &Pages,
        progress_tx: &mpsc::Sender<DownloadProgress>,
        cancel_token: &CancellationToken,
    ) -> Result<PathBuf> {
        let chapter_folder = match &pages.scanlator {
            Some(scanlator) if !scanlator.is_empty() => {
                sanitize_filename(&format!("Cap. {} [{}]", pages.chapter_number, scanlator))
            }
            _ => sanitize_filename(&format!("Cap. {}", pages.chapter_number)),
        };

        let chapter_dir = self
            .config
            .download_dir
            .join(sanitize_filename(&pages.manga_name))
            .join(chapter_folder);

        tokio::fs::create_dir_all(&chapter_dir).await.map_err(|e| {
            HagitoriError::download(format!(
                "failed to create directory {}: {e}",
                chapter_dir.display()
            ))
        })?;

        let total_pages = pages.pages.len() as u32;

        let _ = progress_tx
            .send(DownloadProgress::new(
                &pages.manga_name,
                &pages.chapter_number,
                0,
                total_pages,
                DownloadStatus::Downloading,
            ))
            .await;

        let mut handles = Vec::with_capacity(pages.pages.len());
        let completed_count = Arc::new(AtomicU32::new(0));

        if pages.use_browser {
            // browser path: pre-create a pool of stealth pages to avoid
            // creating/destroying a page per image download.
            let browser = get_or_launch_browser(&self.browser_manager).await?;
            let max_pages = self.config.max_concurrent_pages.min(MAX_CONCURRENT_PAGES);

            let (pool_tx, pool_rx) = mpsc::channel(max_pages);
            for i in 0..max_pages {
                let page = browser.new_page(None).await.map_err(|e| {
                    HagitoriError::download(format!(
                        "failed to create browser page {}: {e}",
                        i + 1
                    ))
                })?;
                pool_tx.send(page).await.map_err(|_| {
                    HagitoriError::download("browser page pool channel closed unexpectedly")
                })?;
            }
            let pool_rx = Arc::new(Mutex::new(pool_rx));

            for (index, page_url) in pages.pages.iter().enumerate() {
                let pool_tx = pool_tx.clone();
                let pool_rx = pool_rx.clone();
                let cancel = cancel_token.clone();
                let progress = progress_tx.clone();
                let url = page_url.clone();
                let dir = chapter_dir.clone();
                let manga_name = pages.manga_name.clone();
                let chapter_number = pages.chapter_number.clone();
                let max_retries = self.config.max_retries;
                let image_format = self.config.image_format.clone();
                let completed = completed_count.clone();
                let page_number = index + 1;

                handles.push(tokio::spawn(async move {
                    let browser_page = pool_rx
                        .lock()
                        .await
                        .recv()
                        .await
                        .ok_or_else(|| HagitoriError::download("browser page pool closed"))?;

                    let result = download_page_browser_retry(
                        &browser_page,
                        &url,
                        &dir,
                        page_number,
                        max_retries,
                        &cancel,
                        &image_format,
                    )
                    .await;

                    // return page to pool (even on error)
                    let _ = pool_tx.send(browser_page).await;

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let status = match &result {
                        Ok(_) => DownloadStatus::Downloading,
                        Err(e) => DownloadStatus::Failed(e.to_string()),
                    };
                    let _ = progress
                        .send(DownloadProgress::new(
                            &manga_name,
                            &chapter_number,
                            done,
                            total_pages,
                            status,
                        ))
                        .await;

                    result
                }));
            }
        } else {
            // HTTP path: use semaphore for concurrency control
            let request_options = self.build_request_options(pages.headers.as_ref());

            for (index, page_url) in pages.pages.iter().enumerate() {
                let semaphore = self.page_semaphore.clone();
                let http_client = self.http_client.clone();
                let cancel = cancel_token.clone();
                let progress = progress_tx.clone();
                let url = page_url.clone();
                let dir = chapter_dir.clone();
                let manga_name = pages.manga_name.clone();
                let chapter_number = pages.chapter_number.clone();
                let max_retries = self.config.max_retries;
                let opts = request_options.clone();
                let image_format = self.config.image_format.clone();
                let completed = completed_count.clone();
                let page_number = index + 1;

                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|_| {
                        HagitoriError::download("download semaphore closed")
                    })?;

                    if cancel.is_cancelled() {
                        return Err(HagitoriError::download("download cancelled"));
                    }

                    let result = download_page_http_retry(
                        &http_client,
                        &url,
                        &dir,
                        page_number,
                        max_retries,
                        opts.as_ref(),
                        &cancel,
                        &image_format,
                    )
                    .await;

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let status = match &result {
                        Ok(_) => DownloadStatus::Downloading,
                        Err(e) => DownloadStatus::Failed(e.to_string()),
                    };
                    let _ = progress
                        .send(DownloadProgress::new(
                            &manga_name,
                            &chapter_number,
                            done,
                            total_pages,
                            status,
                        ))
                        .await;

                    result
                }));
            }
        }

        // collect results
        let mut errors = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(HagitoriError::download(format!("task join error: {e}"))),
            }
        }

        if cancel_token.is_cancelled() {
            return Err(HagitoriError::download("download cancelled by user"));
        }

        if !errors.is_empty() {
            let failed = errors.len();

            tracing::warn!(
                "download {}:{} failed   {} of {} pages with errors",
                pages.manga_name,
                pages.chapter_number,
                failed,
                total_pages,
            );

            let _ = progress_tx
                .send(DownloadProgress::new(
                    &pages.manga_name,
                    &pages.chapter_number,
                    total_pages - failed as u32,
                    total_pages,
                    DownloadStatus::Failed(format!("{} page(s) failed", failed)),
                ))
                .await;

            return Err(HagitoriError::download(format!(
                "{failed} of {total_pages} pages failed to download"
            )));
        }

        let _ = progress_tx
            .send(DownloadProgress::new(
                &pages.manga_name,
                &pages.chapter_number,
                total_pages,
                total_pages,
                DownloadStatus::Completed,
            ))
            .await;

        tracing::info!(
            "download completed: {}:{} ({} pages) -> {}",
            pages.manga_name,
            pages.chapter_number,
            total_pages,
            chapter_dir.display()
        );

        Ok(chapter_dir)
    }

    fn build_request_options(
        &self,
        headers: Option<&HashMap<String, String>>,
    ) -> Option<RequestOptions> {
        headers.map(|h| RequestOptions {
            headers: Some(h.clone()),
            timeout: None,
            referer: h.get("Referer").cloned(),
        })
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
