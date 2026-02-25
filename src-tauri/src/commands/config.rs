use std::collections::HashMap;

use serde::Deserialize;
use tauri::State;

use crate::AppState;
use crate::utils::CommandResult;

/// a key-value pair received from the frontend to update a config setting.
#[derive(Debug, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    state.config.get_all().cmd()
}

#[tauri::command]
pub async fn set_config(entry: ConfigEntry, state: State<'_, AppState>) -> Result<(), String> {
    state
        .config
        .set(&entry.key, &entry.value)
        .cmd()
}

#[tauri::command]
pub async fn get_download_path(state: State<'_, AppState>) -> Result<String, String> {
    state.config.download_dir().cmd()
}
