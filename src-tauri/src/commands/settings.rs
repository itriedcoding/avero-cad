use erlc_cad_core::models::{
    ApiConfigView, BackupView, ConnectionTestView, DatabaseStatsView, SettingsUpdate, SettingsView,
};
use erlc_cad_core::services::{settings, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub fn get_settings(
    state: State<'_, AppContext>,
    token: String,
) -> Result<SettingsView, AppError> {
    let access = state.authenticate(&token)?;
    settings::read_settings(&state, &access)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppContext>,
    token: String,
    update: SettingsUpdate,
) -> Result<SettingsView, AppError> {
    let access = state.authenticate(&token)?;
    settings::update_settings(&state, &access, update)
}

#[tauri::command]
pub fn get_api_config(
    state: State<'_, AppContext>,
    token: String,
) -> Result<ApiConfigView, AppError> {
    let access = state.authenticate(&token)?;
    settings::api_config(&state, &access)
}

#[tauri::command]
pub fn save_api_key(
    state: State<'_, AppContext>,
    token: String,
    base_url: String,
    api_key: String,
) -> Result<ApiConfigView, AppError> {
    let access = state.authenticate(&token)?;
    settings::save_api_key(&state, &access, &base_url, &api_key)
}

#[tauri::command]
pub fn clear_api_key(state: State<'_, AppContext>, token: String) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    settings::clear_api_key(&state, &access)
}

#[tauri::command]
pub async fn test_api_connection(
    state: State<'_, AppContext>,
    token: String,
    candidate_key: Option<String>,
) -> Result<ConnectionTestView, AppError> {
    let access = state.authenticate(&token)?;
    settings::test_connection(&state, &access, candidate_key).await
}

#[tauri::command]
pub fn backup_database(
    state: State<'_, AppContext>,
    token: String,
    directory: Option<String>,
) -> Result<BackupView, AppError> {
    let access = state.authenticate(&token)?;
    settings::backup_database(&state, &access, directory)
}

#[tauri::command]
pub fn restore_database(
    state: State<'_, AppContext>,
    token: String,
    source: String,
    confirmed: bool,
) -> Result<BackupView, AppError> {
    let access = state.authenticate(&token)?;
    settings::restore_database(&state, &access, &source, confirmed)
}

#[tauri::command]
pub fn database_stats(
    state: State<'_, AppContext>,
    token: String,
) -> Result<DatabaseStatsView, AppError> {
    let access = state.authenticate(&token)?;
    settings::database_stats(&state, &access)
}
