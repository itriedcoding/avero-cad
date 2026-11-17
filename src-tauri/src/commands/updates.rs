use avero_cad_core::models::{UpdateCheckView, UpdateInstallView};
use avero_cad_core::services::{updater, AppContext};
use avero_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub async fn check_for_update(
    state: State<'_, AppContext>,
    token: String,
) -> Result<UpdateCheckView, AppError> {
    let access = state.authenticate(&token)?;
    updater::check(&state, &access).await
}

#[tauri::command]
pub async fn install_update(
    state: State<'_, AppContext>,
    token: String,
    confirmed: bool,
) -> Result<UpdateInstallView, AppError> {
    let access = state.authenticate(&token)?;
    updater::install(&state, &access, confirmed).await
}

#[tauri::command]
pub fn dismiss_pending_update(state: State<'_, AppContext>, token: String) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    updater::dismiss_pending(&state, &access)
}
