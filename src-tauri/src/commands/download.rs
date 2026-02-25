use std::path::PathBuf;

use tauri::{Emitter, State};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use hagitori_config::DownloadRecord;
use hagitori_core::entities::{Chapter, DownloadProgress, DownloadStatus};
use hagitori_download::{DownloadEngine, DownloadEngineConfig};
use hagitori_grouper::{create_archive, GroupFormat};

use crate::utils::{build_comic_info, infer_iso639_1, CommandResult};
use crate::AppState;

#[expect(clippy::too_many_arguments, reason = "helper needs all context to emit + record")]
fn record_failure(
    app: &tauri::AppHandle,
    history: &hagitori_config::DownloadHistory,
    manga_name: &str,
    chapter_number: &str,
    source: &str,
    completed: u32,
    total: u32,
    err: String,
) {
    tracing::error!("{err}");
    let _ = app.emit(
        "download-progress",
        &DownloadProgress::new(manga_name, chapter_number, completed, total, DownloadStatus::Failed(err)),
    );
    if let Err(e) = history.add(&DownloadRecord::failed(manga_name, chapter_number, source)) {
        tracing::warn!("failed to record failed download in history: {e}");
    }
}

#[tauri::command]
pub async fn download_chapters(
    chapters: Vec<Chapter>,
    source: String,
    manga_name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let download_dir = state.config.download_dir().cmd()?;
    let group_format_str = state.config.group_format().cmd()?;
    let max_concurrent = state.config.max_concurrent_pages().cmd()?;
    let image_format = state.config.image_format().cmd()?;

    let group_format = match group_format_str.as_str() {
        "zip" => Some(GroupFormat::Zip),
        "folder" => None,
        _ => Some(GroupFormat::Cbz),
    };

    let cancel_token = CancellationToken::new();
    *state.cancel_token.lock().await = cancel_token.clone();

    let engine_config = DownloadEngineConfig {
        max_retries: 3,
        download_dir: PathBuf::from(&download_dir),
        max_concurrent_pages: max_concurrent,
        image_format,
    };

    let engine = DownloadEngine::with_browser(
        state.http_client.clone(),
        engine_config,
        state.browser_manager.clone(),
    );

    let metadata_context = {
        let maybe_manga_id = {
            let manga_cache = state.manga_cache.read().await;
            let provider_cache = state.provider_cache.read().await;

            manga_cache.iter().find_map(|(manga_id, manga)| {
                if manga.name != manga_name {
                    return None;
                }

                let provider_matches = provider_cache
                    .peek(manga_id)
                    .map(|provider_id| provider_id == &source)
                    .unwrap_or(true);

                if provider_matches {
                    Some(manga_id.clone())
                } else {
                    None
                }
            })
        };

        if let Some(manga_id) = maybe_manga_id {
            let provider = {
                let registry = state.registry.read().await;
                registry.get_provider(&source).cmd()?
            };
            match provider.get_details(&manga_id).await {
                Ok(details) => {
                    let web = state
                        .manga_cache
                        .read()
                        .await
                        .peek(&manga_id)
                        .and_then(|m| m.url.clone());
                    Some((details, web, infer_iso639_1(&source)))
                }
                Err(e) => {
                    tracing::warn!(
                        "could not get details for ComicInfo metadata ({}): {}",
                        manga_id,
                        e
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                "could not determine manga_id to generate ComicInfo.xml for '{}'.",
                manga_name
            );
            None
        }
    };

    // pre-fetched pages for the next chapter (populated during the previous iteration).
    let mut prefetched_pages: Option<hagitori_core::error::Result<hagitori_core::entities::Pages>> = None;
    let mut processed_up_to: usize = 0;

    for (i, chapter) in chapters.iter().enumerate() {
        if cancel_token.is_cancelled() {
            break;
        }
        processed_up_to = i + 1;

        // use pre-fetched pages when available, otherwise fetch fresh.
        let pages_result = match prefetched_pages.take() {
            Some(result) => result,
            None => {
                let provider = {
                    let registry = state.registry.read().await;
                    registry.get_provider(&source).cmd()?
                };
                provider.get_pages(chapter).await
            }
        };
        let pages = match pages_result {
            Ok(mut p) => {
                p.manga_name = manga_name.clone();
                p.scanlator = chapter.scanlator.clone();
                p
            }
            Err(e) => {
                record_failure(
                    &app, &state.download_history,
                    &manga_name, &chapter.number, &source,
                    0, 0,
                    format!("get_pages failed for Ch. {}: {e}", chapter.number),
                );
                continue;
            }
        };

        let total_pages = pages.pages.len() as u32;

        tracing::info!(
            "starting download: Ch. {} ({} pages)",
            pages.chapter_number,
            total_pages,
        );

        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);

        let need_grouping = group_format.is_some();
        let app_progress = app.clone();
        let progress_task = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                if need_grouping && matches!(progress.status, DownloadStatus::Completed) {
                    continue;
                }
                let _ = app_progress.emit("download-progress", &progress);
            }
        });

        // run download and pre-fetch next chapter's page URLs concurrently.
        // get_pages uses a RwLock read guard, so concurrent readers are safe.
        let download_result = if let Some(next_ch) = chapters.get(i + 1) {
            let next_provider = {
                let registry = state.registry.read().await;
                registry.get_provider(&source).cmd()?
            };
            let (dl_result, pf_result) = tokio::join!(
                engine.download_chapter(&pages, &progress_tx, &cancel_token),
                next_provider.get_pages(next_ch)
            );
            prefetched_pages = Some(pf_result);
            dl_result
        } else {
            engine
                .download_chapter(&pages, &progress_tx, &cancel_token)
                .await
        };

        drop(progress_tx);
        let _ = progress_task.await;

        let chapter_dir = match download_result {
            Ok(dir) => dir,
            Err(e) => {
                record_failure(
                    &app, &state.download_history,
                    &manga_name, &chapter.number, &source,
                    0, total_pages,
                    format!("download failed for Ch. {}: {e}", pages.chapter_number),
                );
                continue;
            }
        };

        // track the final save path for the completed event
        let mut save_path = chapter_dir.to_string_lossy().to_string();

        if let Some(format) = group_format {
            let _ = app.emit(
                "download-progress",
                &DownloadProgress::new(
                    &pages.manga_name,
                    &pages.chapter_number,
                    total_pages,
                    total_pages,
                    DownloadStatus::Processing,
                ),
            );

            let ext = match format {
                GroupFormat::Cbz => "cbz",
                GroupFormat::Zip => "zip",
            };

            let manga_dir = chapter_dir.parent().unwrap_or(&chapter_dir);
            let output_path = manga_dir.join(format!("Cap. {}.{}", pages.chapter_number, ext));
            save_path = output_path.to_string_lossy().to_string();

            let chapter_metadata = metadata_context
                .as_ref()
                .map(|(details, web, iso639_1)| {
                    build_comic_info(details, chapter, web.clone(), iso639_1.clone())
                });

            // zip crate is synchronous   use spawn_blocking to avoid blocking the runtime
            let cbz_chapter_dir = chapter_dir.clone();
            let cbz_output_path = output_path.clone();
            let cbz_metadata = chapter_metadata.clone();
            let cbz_result = tokio::task::spawn_blocking(move || {
                let result = create_archive(
                    &cbz_chapter_dir,
                    Some(&cbz_output_path),
                    format,
                    cbz_metadata.as_ref(),
                );
                if result.is_ok()
                    && let Err(e) = hagitori_grouper::cleanup_chapter(&cbz_chapter_dir) {
                        tracing::warn!(
                            "cleanup failed for {}: {}",
                            cbz_chapter_dir.display(),
                            e
                        );
                    }
                result
            })
            .await;

            match cbz_result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    record_failure(
                        &app, &state.download_history,
                        &manga_name, &chapter.number, &source,
                        total_pages, total_pages,
                        format!("grouping failed for Ch. {}: {e}", pages.chapter_number),
                    );
                    continue;
                }
                Err(join_err) => {
                    record_failure(
                        &app, &state.download_history,
                        &manga_name, &chapter.number, &source,
                        total_pages, total_pages,
                        format!("grouping panicked for Ch. {}: {join_err}", pages.chapter_number),
                    );
                    continue;
                }
            }
        }

        let _ = app.emit(
            "download-progress",
            &DownloadProgress::completed_with_path(
                &pages.manga_name,
                &pages.chapter_number,
                total_pages,
                &save_path,
            ),
        );

        // record successful download in history
        let download_path = chapter_dir.to_string_lossy().to_string();
        if let Err(e) = state.download_history.add(
            &DownloadRecord::completed(&manga_name, &chapter.number, &source, &download_path),
        ) {
            tracing::warn!("failed to record download in history: {e}");
        }
    }

    if cancel_token.is_cancelled() {
        for chapter in chapters.iter().skip(processed_up_to) {
            let _ = app.emit(
                "download-progress",
                &DownloadProgress::new(
                    &manga_name,
                    &chapter.number,
                    0,
                    0,
                    DownloadStatus::Failed("cancelled by user".to_string()),
                ),
            );
        }
    }

    {
        let mut guard = state.browser_manager.lock().await;
        if guard.is_some() {
            tracing::info!("closing browser after downloads completed");
            *guard = None;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, AppState>) -> Result<(), String> {
    let token = state.cancel_token.lock().await;
    token.cancel();
    Ok(())
}
