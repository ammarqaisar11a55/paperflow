//! Settings and recent activity.

use tauri::State;

use crate::error::AppResult;
use crate::services::store::{ActivityEntry, Settings};
use crate::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.store.settings()
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    state.store.save_settings(settings)
}

#[tauri::command]
pub fn get_activity(state: State<'_, AppState>) -> Vec<ActivityEntry> {
    state.store.activity()
}

#[tauri::command]
pub fn remove_activity(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.store.remove_activity(&id)
}

#[tauri::command]
pub fn clear_activity(state: State<'_, AppState>) -> AppResult<()> {
    state.store.clear_activity()
}
