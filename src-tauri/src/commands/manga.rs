use tauri::{Manager, State};

use hagitori_core::entities::{Chapter, ExtensionMeta, Manga, MangaDetails};

use crate::AppState;
use crate::utils::CommandResult;

#[tauri::command]
pub async fn get_manga(url: String, state: State<'_, AppState>) -> Result<Manga, String> {
    let provider = {
        let registry = state.registry.read().await;
        registry.find_provider_by_url(&url).cmd()?
    };
    let provider_id = provider.meta().id.clone();

    let mut manga = provider.get_manga(&url).await.cmd()?;

    if manga.url.is_none() {
        manga.url = Some(url);
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
    provider
        .get_details(&manga_id)
        .await
        .cmd()
}

#[tauri::command]
pub async fn proxy_image(url: String, state: State<'_, AppState>) -> Result<String, String> {
    use base64::Engine as _;

    let response = state
        .http_client
        .get(&url, None)
        .await
        .map_err(|e| format!("proxy_image GET failed: {e}"))?;

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| {
            v.to_str()
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        url = url.as_str(),
                        "content-type header contains non-ASCII characters   defaulting to image/jpeg"
                    );
                    "image/jpeg"
                })
        })
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("proxy_image failed to read bytes: {e}"))?;

    if bytes.is_empty() {
        return Err("proxy_image: empty response".to_string());
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{content_type};base64,{b64}"))
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


#[tauri::command]
pub async fn cache_cover_image(
    manga_id: String,
    url: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // resolve cache directory
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("failed to get app_cache_dir: {e}"))?;
    let covers_dir = cache_dir.join("covers");
    tokio::fs::create_dir_all(&covers_dir)
        .await
        .map_err(|e| format!("failed to create covers directory: {e}"))?;

    // sanitize manga_id for filesystem safety
    let sanitized_id: String = manga_id
        .chars()
        .map(|c| if r#"/\:*?"<>|"#.contains(c) { '_' } else { c })
        .collect();

    // extract extension from URL (default to jpg)
    let ext = url
        .split('?').next().unwrap_or(&url)
        .rsplit('.').next()
        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_alphanumeric()))
        .unwrap_or("jpg");

    // download the image bytes using the existing http_client
    let response = state
        .http_client
        .get(&url, None)
        .await
        .map_err(|e| format!("cache_cover_image GET failed: {e}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("cache_cover_image failed to read bytes: {e}"))?;
    if bytes.is_empty() {
        return Err("cache_cover_image: empty response".to_string());
    }

    // write to disk
    let file_name = format!("{sanitized_id}.{ext}");
    let file_path = covers_dir.join(&file_name);
    tokio::fs::write(&file_path, &bytes)
        .await
        .map_err(|e| format!("cache_cover_image failed to write: {e}"))?;

    // return the absolute path as a string
    file_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "cache_cover_image: invalid UTF-8 path".to_string())
}
