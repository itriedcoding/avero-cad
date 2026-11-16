use erlc_cad_core::models::{CallView, UnitView};
use erlc_cad_core::services::{dispatch, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub fn list_calls(
    state: State<'_, AppContext>,
    token: String,
    status: Option<String>,
    limit: usize,
) -> Result<Vec<CallView>, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::list_calls(&state, &access, status.as_deref(), limit)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_call(
    state: State<'_, AppContext>,
    token: String,
    title: String,
    description: String,
    location: String,
    postal: String,
    priority: String,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::create_call(
        &state,
        &access,
        &title,
        &description,
        &location,
        &postal,
        &priority,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_call(
    state: State<'_, AppContext>,
    token: String,
    call_id: i64,
    title: String,
    description: String,
    location: String,
    postal: String,
    priority: String,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::update_call(
        &state,
        &access,
        call_id,
        &title,
        &description,
        &location,
        &postal,
        &priority,
    )
}

#[tauri::command]
pub fn set_call_status(
    state: State<'_, AppContext>,
    token: String,
    call_id: i64,
    status: String,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::set_call_status(&state, &access, call_id, &status)
}

#[tauri::command]
pub fn attach_player_to_call(
    state: State<'_, AppContext>,
    token: String,
    call_id: i64,
    roblox_user_id: u64,
    username: String,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::attach_player(&state, &access, call_id, roblox_user_id, &username)
}

#[tauri::command]
pub fn detach_player_from_call(
    state: State<'_, AppContext>,
    token: String,
    call_id: i64,
    roblox_user_id: u64,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::detach_player(&state, &access, call_id, roblox_user_id)
}

#[tauri::command]
pub fn list_units(state: State<'_, AppContext>, token: String) -> Result<Vec<UnitView>, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::units(&state, &access)
}

#[tauri::command]
pub fn create_unit(
    state: State<'_, AppContext>,
    token: String,
    callsign: String,
    label: String,
    team: String,
    status: String,
) -> Result<UnitView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::create_unit(&state, &access, &callsign, &label, &team, &status)
}

#[tauri::command]
pub fn update_unit(
    state: State<'_, AppContext>,
    token: String,
    unit_id: i64,
    label: String,
    team: String,
    status: String,
) -> Result<UnitView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::update_unit(&state, &access, unit_id, &label, &team, &status)
}

#[tauri::command]
pub fn assign_unit_to_call(
    state: State<'_, AppContext>,
    token: String,
    call_id: i64,
    unit_id: i64,
) -> Result<CallView, AppError> {
    let access = state.authenticate(&token)?;
    dispatch::assign_unit(&state, &access, call_id, unit_id)
}

#[tauri::command]
pub fn unassign_unit(
    state: State<'_, AppContext>,
    token: String,
    unit_id: i64,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    dispatch::unassign_unit(&state, &access, unit_id)
}

#[tauri::command]
pub fn delete_unit(
    state: State<'_, AppContext>,
    token: String,
    unit_id: i64,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    dispatch::delete_unit(&state, &access, unit_id)
}
