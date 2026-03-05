use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Manager, State};

use hagitori_core::entities::{Chapter, ExtensionMeta, Manga, MangaDetails};
use hagitori_http::HttpClient;

use crate::AppState;
use crate::utils::CommandResult;

/// downloads an image from url and saves it to {covers_dir}/{manga_id}.{ext}.
/// returns the absolute path on success.
async fn cache_cover_to_disk(
    http_client: &Arc<HttpClient>,
    covers_dir: &PathBuf,
    manga_id: &str,
    url: &str,
) -> Result<String, String> {
    tokio::fs::create_dir_all(covers_dir)
        .await
        .map_err(|e| format!("failed to create covers directory: {e}"))?;

    let sanitized_id: String = manga_id
        .chars()
        .map(|c| if r#"/\:*?"<>|"#.contains(c) { '_' } else { c })
        .collect();

    let ext = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('.')
        .next()
        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_alphanumeric()))
        .unwrap_or("jpg");

    let response = http_client
        .get(url, None)
        .await
        .map_err(|e| format!("cache_cover GET failed: {e}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("cache_cover failed to read bytes: {e}"))?;
    if bytes.is_empty() {
        return Err("cache_cover: empty response".to_string());
    }

    let file_name = format!("{sanitized_id}.{ext}");
    let file_path = covers_dir.join(&file_name);
    tokio::fs::write(&file_path, &bytes)
        .await
        .map_err(|e| format!("cache_cover failed to write: {e}"))?;

    file_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "cache_cover: invalid UTF-8 path".to_string())
}

#[tauri::command]
pub async fn get_manga(
    url: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Manga, String> {
    let provider = {
        let registry = state.registry.read().await;
        registry.find_provider_by_url(&url).cmd()?
    };
    let provider_id = provider.meta().id.clone();

    let mut manga = provider.get_manga(&url).await.cmd()?;

    if manga.url.is_none() {
        manga.url = Some(url);
    }

    // cache cover image to disk if it's a remote URL
    if let Some(cover_url) = &manga.cover
        && cover_url.starts_with("http")
    {
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(|e| format!("failed to get app_cache_dir: {e}"))?;
        let covers_dir = cache_dir.join("covers");
        match cache_cover_to_disk(&state.http_client, &covers_dir, &manga.id, cover_url).await
        {
            Ok(path) => manga.cover = Some(path),
            Err(e) => tracing::warn!("failed to cache cover for {}: {e}", manga.id),
        }
    }

    let manga_id = manga.id.clone();
    state
        .manga_cache
        .write()
        .await
        .put(manga_id.clone(), manga.clone());
    state
        .provider_cache
        .write()
        .await
        .put(manga_id, provider_id);

    Ok(manga)
}

#[tauri::command]
pub async fn get_chapters(
    manga_id: String,
    source: String,
    state: State<'_, AppState>,
) -> Result<Vec<Chapter>, String> {
    let provider_id = state
        .provider_cache
        .read()
        .await
        .peek(&manga_id)
        .cloned()
        .unwrap_or_else(|| source.clone());

    {
        let mut cache = state.provider_cache.write().await;
        if cache.peek(&manga_id).is_none() {
            cache.put(manga_id.clone(), source);
        }
    }

    let registry = state.registry.read().await;
    let provider = registry.get_provider(&provider_id).cmd()?;
    drop(registry);
    provider
        .get_chapters(&manga_id)
        .await
        .cmd()
}

#[tauri::command]
pub async fn get_details(
    manga_id: String,
    source: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<MangaDetails, String> {
    let provider_id = state
        .provider_cache
        .write()
        .await
        .get(&manga_id)
        .cloned()
        .unwrap_or(source);

    let provider = {
        let registry = state.registry.read().await;
        registry.get_provider(&provider_id).cmd()?
    };
    let mut details = provider
        .get_details(&manga_id)
        .await
        .cmd()?;

    // cache cover image to disk if it's a remote URL
    if let Some(cover_url) = &details.cover
        && cover_url.starts_with("http")
    {
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(|e| format!("failed to get app_cache_dir: {e}"))?;
        let covers_dir = cache_dir.join("covers");
        match cache_cover_to_disk(&state.http_client, &covers_dir, &manga_id, cover_url).await
        {
            Ok(path) => details.cover = Some(path),
            Err(e) => tracing::warn!("failed to cache details cover for {manga_id}: {e}"),
        }
    }

    Ok(details)
}

#[tauri::command]
pub async fn set_extension_lang(
    extension_id: String,
    lang: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let registry = state.registry.read().await;
    registry
        .set_extension_lang(&extension_id, &lang)
        .cmd()
}

#[tauri::command]
pub async fn list_extensions(state: State<'_, AppState>) -> Result<Vec<ExtensionMeta>, String> {
    let registry = state.registry.read().await;
    Ok(registry.list())
}
