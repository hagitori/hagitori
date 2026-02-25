use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use hagitori_browser::BrowserManager;
use hagitori_core::error::{HagitoriError, Result};

use crate::image::{extract_extension, resolve_output_filename, save_image};

pub(crate) async fn download_page_browser_retry(
    page: &hagitori_browser::Page,
    url: &str,
    output_dir: &Path,
    page_number: usize,
    max_retries: u32,
    cancel_token: &CancellationToken,
    target_format: &str,
) -> Result<PathBuf> {
    let source_ext = extract_extension(url);
    let (filename, file_ext) = resolve_output_filename(page_number, source_ext, target_format);
    let output_path = output_dir.join(&filename);

    if output_path.exists() {
        let metadata = tokio::fs::metadata(&output_path).await.ok();
        if metadata.is_some_and(|m| m.len() > 0) {
            return Ok(output_path);
        }
    }

    for attempt in 1..=max_retries {
        if cancel_token.is_cancelled() {
            return Err(HagitoriError::download("download cancelled"));
        }

        tracing::debug!(
            "downloading page {} via browser (attempt {}/{}): {}",
            page_number,
            attempt,
            max_retries,
            url
        );

        match hagitori_browser::download_image_with_page(page, url, 15).await {
            Ok(bytes) if !bytes.is_empty() => {
                save_image(&bytes, &output_path, &file_ext, page_number).await?;
                tracing::debug!(
                    "browser download OK for page {}: {} bytes",
                    page_number,
                    bytes.len()
                );
                return Ok(output_path);
            }
            Ok(_) => {
                tracing::warn!(
                    "empty response for page {} via browser (attempt {}/{})",
                    page_number,
                    attempt,
                    max_retries
                );
            }
            Err(e) => {
                tracing::warn!(
                    "browser download failed for page {} (attempt {}/{}): {:?}",
                    page_number,
                    attempt,
                    max_retries,
                    e
                );
            }
        }

        if attempt < max_retries {
            let delay = std::time::Duration::from_millis(1000 * (1u64 << (attempt - 1)));
            tokio::time::sleep(delay).await;
        }
    }

    Err(HagitoriError::download(format!(
        "failed to download page {page_number} via browser after {max_retries} attempts: {url}"
    )))
}

pub(crate) async fn get_or_launch_browser(
    browser_manager: &Arc<Mutex<Option<Arc<BrowserManager>>>>,
) -> Result<Arc<BrowserManager>> {
    let mut guard = browser_manager.lock().await;
    if let Some(bm) = guard.as_ref() {
        return Ok(bm.clone());
    }

    tracing::info!("launching browser for download (no active browser)");
    let new_browser = BrowserManager::launch()
        .await
        .map_err(|e| HagitoriError::download(format!("failed to launch browser: {e}")))?;

    let bm = Arc::new(new_browser);
    *guard = Some(bm.clone());
    Ok(bm)
}
