use tauri::State;

use hagitori_config::{LibraryEntry, SourceMeta};
use hagitori_core::entities::{Chapter, Manga, MangaDetails};

use crate::AppState;
use crate::utils::CommandResult;

use std::collections::HashMap;

#[tauri::command]
pub async fn library_list(state: State<'_, AppState>) -> Result<Vec<LibraryEntry>, String> {
    state.library.list_manga().cmd()
}

#[tauri::command]
pub async fn library_get(
    manga_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LibraryEntry>, String> {
    state.library.get_manga(&manga_id).cmd()
}

#[tauri::command]
pub async fn library_add(
    manga: Manga,
    chapters: Vec<Chapter>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.add_manga(&manga, &chapters).cmd()
}

#[tauri::command]
pub async fn library_remove(
    manga_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.remove_manga(&manga_id).cmd()
}

#[tauri::command]
pub async fn library_update_chapters(
    manga_id: String,
    chapters: Vec<Chapter>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.update_chapters(&manga_id, &chapters).cmd()
}

#[tauri::command]
pub async fn library_update_details(
    manga_id: String,
    details: MangaDetails,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.update_details(&manga_id, &details).cmd()
}

#[tauri::command]
pub async fn library_update_cover(
    manga_id: String,
    cover: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.update_cover(&manga_id, &cover).cmd()
}

#[tauri::command]
pub async fn library_set_source_meta(
    source_id: String,
    name: Option<String>,
    supports_details: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(n) = name {
        state.library.set_source_name(&source_id, &n).cmd()?;
    }
    if let Some(s) = supports_details {
        state
            .library
            .set_source_supports_details(&source_id, s)
            .cmd()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn library_get_source_meta(
    state: State<'_, AppState>,
) -> Result<HashMap<String, SourceMeta>, String> {
    state.library.get_source_meta().cmd()
}

#[tauri::command]
pub async fn library_set_extension_lang(
    extension_id: String,
    lang: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.library.set_extension_lang(&extension_id, &lang).cmd()
}

#[tauri::command]
pub async fn library_get_extension_langs(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    state.library.get_extension_langs().cmd()
}
