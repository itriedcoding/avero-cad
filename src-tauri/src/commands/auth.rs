use avero_cad_core::models::{BootstrapView, SessionView};
use avero_cad_core::services::{auth, AppContext};
use avero_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub fn app_bootstrap(state: State<'_, AppContext>) -> Result<BootstrapView, AppError> {
    auth::bootstrap(&state)
}

#[tauri::command]
pub fn setup_owner(
    state: State<'_, AppContext>,
    username: String,
    display_name: String,
    password: String,
) -> Result<SessionView, AppError> {
    auth::create_initial_owner(&state, &username, &display_name, &password)
}

#[tauri::command]
pub fn login(
    state: State<'_, AppContext>,
    username: String,
    password: String,
) -> Result<SessionView, AppError> {
    auth::login(&state, &username, &password)
}

#[tauri::command]
pub fn logout(state: State<'_, AppContext>, token: String) -> Result<(), AppError> {
    auth::logout(&state, &token)
}

#[tauri::command]
pub fn session_info(state: State<'_, AppContext>, token: String) -> Result<SessionView, AppError> {
    auth::session_info(&state, &token)
}

#[tauri::command]
pub fn change_password(
    state: State<'_, AppContext>,
    token: String,
    current_password: String,
    new_password: String,
) -> Result<(), AppError> {
    auth::change_password(&state, &token, &current_password, &new_password)
}
