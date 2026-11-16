use erlc_cad_core::models::{
    CommandResultView, InternalBanView, LiveBanView, ModerationRecordView,
};
use erlc_cad_core::services::{moderation, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub async fn kick_player(
    state: State<'_, AppContext>,
    token: String,
    target: String,
    reason: String,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> Result<CommandResultView, AppError> {
    let access = state.authenticate(&token)?;
    moderation::kick(&state, &access, &target, &reason, confirmed, roblox_user_id).await
}

#[tauri::command]
pub async fn ban_player(
    state: State<'_, AppContext>,
    token: String,
    target: String,
    reason: String,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> Result<CommandResultView, AppError> {
    let access = state.authenticate(&token)?;
    moderation::ban(&state, &access, &target, &reason, confirmed, roblox_user_id).await
}

#[tauri::command]
pub async fn unban_player(
    state: State<'_, AppContext>,
    token: String,
    target: String,
    reason: String,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> Result<CommandResultView, AppError> {
    let access = state.authenticate(&token)?;
    moderation::unban(&state, &access, &target, &reason, confirmed, roblox_user_id).await
}

#[tauri::command]
pub fn create_internal_ban(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: Option<u64>,
    username: String,
    reason: String,
) -> Result<InternalBanView, AppError> {
    let access = state.authenticate(&token)?;
    moderation::create_internal_ban(&state, &access, roblox_user_id, &username, &reason)
}

#[tauri::command]
pub fn lift_internal_ban(
    state: State<'_, AppContext>,
    token: String,
    ban_id: i64,
    reason: String,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    moderation::lift_internal_ban(&state, &access, ban_id, &reason)
}

#[tauri::command]
pub fn list_moderation(
    state: State<'_, AppContext>,
    token: String,
    limit: usize,
) -> Result<Vec<ModerationRecordView>, AppError> {
    let access = state.authenticate(&token)?;
    moderation::recent(&state, &access, limit)
}

#[tauri::command]
pub fn list_player_moderation(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: Option<u64>,
    username: String,
    limit: usize,
) -> Result<Vec<ModerationRecordView>, AppError> {
    let access = state.authenticate(&token)?;
    moderation::history_for_player(&state, &access, roblox_user_id, &username, limit)
}

#[tauri::command]
pub async fn list_live_bans(
    state: State<'_, AppContext>,
    token: String,
) -> Result<Vec<LiveBanView>, AppError> {
    let access = state.authenticate(&token)?;
    moderation::live_bans(&state, &access).await
}

#[tauri::command]
pub fn list_internal_bans(
    state: State<'_, AppContext>,
    token: String,
    active_only: bool,
    limit: usize,
) -> Result<Vec<InternalBanView>, AppError> {
    let access = state.authenticate(&token)?;
    moderation::internal_bans(&state, &access, active_only, limit)
}
