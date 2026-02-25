use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use hagitori_core::error::{HagitoriError, Result};
use hagitori_http::client::RequestOptions;
use hagitori_http::HttpClient;

use crate::image::{extract_extension, resolve_output_filename, save_image};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn download_page_http_retry(
    http_client: &HttpClient,
    url: &str,
    output_dir: &Path,
    page_number: usize,
    max_retries: u32,
    options: Option<&RequestOptions>,
    cancel_token: &CancellationToken,
    target_format: &str,
) -> Result<PathBuf> {
    let source_ext = extract_extension(url);
    let (filename, file_ext) = resolve_output_filename(page_number, source_ext, target_format);
    let output_path = output_dir.join(&filename);

    if output_path.exists() {
        let metadata = tokio::fs::metadata(&output_path).await.ok();
        if metadata.is_some_and(|m| m.len() > 0) {
            tracing::debug!("page {} already exists, skipping: {}", page_number, url);
            return Ok(output_path);
        }
    }

    let mut last_error = None;

    for attempt in 1..=max_retries {
        if cancel_token.is_cancelled() {
            return Err(HagitoriError::download("download cancelled"));
        }

        tracing::debug!(
            "downloading page {} (attempt {}/{}): {}",
            page_number,
            attempt,
            max_retries,
            url
        );

        match http_client.get(url, options.cloned()).await {
            Ok(response) => {
                let status = response.status();

                if status == wreq::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(60);

                    tracing::warn!(
                        "429 rate-limited on page {}   waiting {}s (attempt {}/{})",
                        page_number,
                        retry_after,
                        attempt,
                        max_retries
                    );

                    last_error = Some(HagitoriError::download(format!(
                        "429 Too Many Requests for page {page_number}: {url}"
                    )));

                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                    continue;
                }

                if !status.is_success() {
                    last_error = Some(HagitoriError::download(format!(
                        "page {page_number} returned status {status}: {url}"
                    )));

                    if attempt < max_retries {
                        let delay =
                            std::time::Duration::from_millis(1000 * (1u64 << (attempt - 1)));
                        tokio::time::sleep(delay).await;
                    }
                    continue;
                }

                let bytes = response.bytes().await.map_err(|e| {
                    HagitoriError::download(format!(
                        "failed to read bytes for page {page_number}: {e}"
                    ))
                })?;

                if bytes.is_empty() {
                    last_error = Some(HagitoriError::download(format!(
                        "empty response for page {page_number}: {url}"
                    )));
                    continue;
                }

                save_image(&bytes, &output_path, &file_ext, page_number).await?;

                return Ok(output_path);
            }
            Err(e) => {
                if attempt == max_retries {
                    tracing::warn!(
                        "page {} failed after {} attempts: {e}",
                        page_number,
                        max_retries,
                    );
                }
                last_error = Some(e);

                if attempt < max_retries {
                    let delay = std::time::Duration::from_millis(1000 * (1u64 << (attempt - 1)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        HagitoriError::download(format!(
            "failed to download page {page_number} after {max_retries} attempts: {url}"
        ))
    }))
}
