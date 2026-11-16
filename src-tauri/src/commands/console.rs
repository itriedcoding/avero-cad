use erlc_cad_core::models::{CommandHistoryView, CommandPolicyView, CommandResultView};
use erlc_cad_core::services::{command, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub async fn execute_server_command(
    state: State<'_, AppContext>,
    token: String,
    command_text: String,
    confirmed: bool,
) -> Result<CommandResultView, AppError> {
    let access = state.authenticate(&token)?;
    command::execute(&state, &access, &command_text, confirmed).await
}

#[tauri::command]
pub fn list_command_history(
    state: State<'_, AppContext>,
    token: String,
    limit: usize,
) -> Result<Vec<CommandHistoryView>, AppError> {
    let access = state.authenticate(&token)?;
    command::history(&state, &access, limit)
}

#[tauri::command]
pub fn list_command_policies(
    state: State<'_, AppContext>,
    token: String,
) -> Result<Vec<CommandPolicyView>, AppError> {
    let access = state.authenticate(&token)?;
    command::policies(&state, &access)
}

#[tauri::command]
pub fn upsert_command_policy(
    state: State<'_, AppContext>,
    token: String,
    command_text: String,
    min_permission: String,
    destructive: bool,
    enabled: bool,
    description: String,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    command::upsert_policy(
        &state,
        &access,
        &command_text,
        &min_permission,
        destructive,
        enabled,
        &description,
    )
}

#[tauri::command]
pub fn delete_command_policy(
    state: State<'_, AppContext>,
    token: String,
    command_text: String,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    command::delete_policy(&state, &access, &command_text)
}
