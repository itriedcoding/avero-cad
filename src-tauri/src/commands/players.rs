use erlc_cad_core::models::{
    AvatarView, PlayerProfile, RobloxUserView, SearchResult, Snapshot, StaffNoteView,
};
use erlc_cad_core::permissions::{Access, Permission};
use erlc_cad_core::services::{players, snapshot, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub async fn get_server_snapshot(
    state: State<'_, AppContext>,
    token: String,
    force: bool,
) -> Result<Snapshot, AppError> {
    let access: Access = state.authenticate(&token)?;
    access.require(Permission::DashboardView)?;
    snapshot::load(&state, force).await
}

#[tauri::command]
pub fn search_players(
    state: State<'_, AppContext>,
    token: String,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    let access = state.authenticate(&token)?;
    players::search(&state, &access, &query)
}

#[tauri::command]
pub fn list_known_players(
    state: State<'_, AppContext>,
    token: String,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let access = state.authenticate(&token)?;
    players::known(&state, &access, limit)
}

#[tauri::command]
pub async fn get_player_profile(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: Option<u64>,
    username: Option<String>,
) -> Result<PlayerProfile, AppError> {
    let access = state.authenticate(&token)?;
    players::profile(&state, &access, roblox_user_id, username).await
}

#[tauri::command]
pub async fn get_player_avatar(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: u64,
    full_body: bool,
) -> Result<AvatarView, AppError> {
    let access = state.authenticate(&token)?;
    players::avatar(&state, &access, roblox_user_id, full_body).await
}

#[tauri::command]
pub async fn lookup_roblox_user(
    state: State<'_, AppContext>,
    token: String,
    identifier: String,
) -> Result<RobloxUserView, AppError> {
    let access = state.authenticate(&token)?;
    players::resolve_roblox(&state, &access, &identifier).await
}

#[tauri::command]
pub fn add_staff_note(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: Option<u64>,
    username: String,
    body: String,
) -> Result<StaffNoteView, AppError> {
    let access = state.authenticate(&token)?;
    players::add_note(&state, &access, roblox_user_id, &username, &body)
}

#[tauri::command]
pub fn list_staff_notes(
    state: State<'_, AppContext>,
    token: String,
    roblox_user_id: Option<u64>,
    username: String,
) -> Result<Vec<StaffNoteView>, AppError> {
    let access = state.authenticate(&token)?;
    players::notes(&state, &access, roblox_user_id, &username)
}

#[tauri::command]
pub fn delete_staff_note(
    state: State<'_, AppContext>,
    token: String,
    note_id: i64,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    players::delete_note(&state, &access, note_id)
}
