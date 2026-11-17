use avero_cad_core::models::{BoloView, DutyShiftView, DutyStatusView, StatsOverviewView};
use avero_cad_core::services::{bolos, shifts, stats, AppContext};
use avero_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub fn list_bolos(
    state: State<'_, AppContext>,
    token: String,
    status: Option<String>,
) -> Result<Vec<BoloView>, AppError> {
    let access = state.authenticate(&token)?;
    bolos::list(&state, &access, status.as_deref())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_bolo(
    state: State<'_, AppContext>,
    token: String,
    kind: String,
    subject_name: String,
    plate: String,
    description: String,
    last_seen: String,
    priority: String,
) -> Result<BoloView, AppError> {
    let access = state.authenticate(&token)?;
    bolos::create(
        &state,
        &access,
        &kind,
        &subject_name,
        &plate,
        &description,
        &last_seen,
        &priority,
    )
}

#[tauri::command]
pub fn clear_bolo(
    state: State<'_, AppContext>,
    token: String,
    bolo_id: i64,
    note: String,
) -> Result<BoloView, AppError> {
    let access = state.authenticate(&token)?;
    bolos::clear(&state, &access, bolo_id, &note)
}

#[tauri::command]
pub fn reopen_bolo(
    state: State<'_, AppContext>,
    token: String,
    bolo_id: i64,
) -> Result<BoloView, AppError> {
    let access = state.authenticate(&token)?;
    bolos::reopen(&state, &access, bolo_id)
}

#[tauri::command]
pub fn duty_status(
    state: State<'_, AppContext>,
    token: String,
) -> Result<DutyStatusView, AppError> {
    let access = state.authenticate(&token)?;
    shifts::status(&state, &access)
}

#[tauri::command]
pub fn clock_in(state: State<'_, AppContext>, token: String) -> Result<DutyStatusView, AppError> {
    let access = state.authenticate(&token)?;
    shifts::clock_in(&state, &access)
}

#[tauri::command]
pub fn clock_out(state: State<'_, AppContext>, token: String) -> Result<DutyStatusView, AppError> {
    let access = state.authenticate(&token)?;
    shifts::clock_out(&state, &access)
}

#[tauri::command]
pub fn list_shifts(
    state: State<'_, AppContext>,
    token: String,
    mine_only: bool,
) -> Result<Vec<DutyShiftView>, AppError> {
    let access = state.authenticate(&token)?;
    shifts::history(&state, &access, mine_only)
}

#[tauri::command]
pub fn end_shift(
    state: State<'_, AppContext>,
    token: String,
    user_id: i64,
    reason: String,
) -> Result<DutyShiftView, AppError> {
    let access = state.authenticate(&token)?;
    shifts::end_shift(&state, &access, user_id, &reason)
}

#[tauri::command]
pub fn stats_overview(
    state: State<'_, AppContext>,
    token: String,
    days: i64,
) -> Result<StatsOverviewView, AppError> {
    let access = state.authenticate(&token)?;
    stats::overview(&state, &access, days)
}
