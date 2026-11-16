use erlc_cad_core::models::{AuditLogView, RoleView, StaffUserView};
use erlc_cad_core::permissions::PermissionInfo;
use erlc_cad_core::services::{staff, AppContext};
use erlc_cad_core::AppError;
use tauri::State;

#[tauri::command]
pub fn list_staff(
    state: State<'_, AppContext>,
    token: String,
) -> Result<Vec<StaffUserView>, AppError> {
    let access = state.authenticate(&token)?;
    staff::list_staff(&state, &access)
}

#[tauri::command]
pub fn list_roles(state: State<'_, AppContext>, token: String) -> Result<Vec<RoleView>, AppError> {
    let access = state.authenticate(&token)?;
    staff::roles(&state, &access)
}

#[tauri::command]
pub fn list_permission_catalogue(
    state: State<'_, AppContext>,
    token: String,
) -> Result<Vec<PermissionInfo>, AppError> {
    let access = state.authenticate(&token)?;
    staff::permission_catalogue(&state, &access)
}

#[tauri::command]
pub fn create_staff(
    state: State<'_, AppContext>,
    token: String,
    username: String,
    display_name: String,
    password: String,
    role_key: String,
    roblox_user_id: Option<u64>,
) -> Result<StaffUserView, AppError> {
    let access = state.authenticate(&token)?;
    staff::create_staff(
        &state,
        &access,
        &username,
        &display_name,
        &password,
        &role_key,
        roblox_user_id,
    )
}

#[tauri::command]
pub fn update_staff_role(
    state: State<'_, AppContext>,
    token: String,
    user_id: i64,
    role_key: String,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::update_role(&state, &access, user_id, &role_key)
}

#[tauri::command]
pub fn set_staff_active(
    state: State<'_, AppContext>,
    token: String,
    user_id: i64,
    active: bool,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::set_active(&state, &access, user_id, active)
}

#[tauri::command]
pub fn reset_staff_password(
    state: State<'_, AppContext>,
    token: String,
    user_id: i64,
    new_password: String,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::reset_password(&state, &access, user_id, &new_password)
}

#[tauri::command]
pub fn set_permission_override(
    state: State<'_, AppContext>,
    token: String,
    user_id: i64,
    permission_key: String,
    allowed: Option<bool>,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::set_override(&state, &access, user_id, &permission_key, allowed)
}

#[tauri::command]
pub fn create_role(
    state: State<'_, AppContext>,
    token: String,
    key: String,
    name: String,
    description: String,
    permissions: Vec<String>,
) -> Result<i64, AppError> {
    let access = state.authenticate(&token)?;
    staff::create_role(&state, &access, &key, &name, &description, permissions)
}

#[tauri::command]
pub fn update_role_definition(
    state: State<'_, AppContext>,
    token: String,
    role_id: i64,
    name: String,
    description: String,
    permissions: Vec<String>,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::update_role_definition(&state, &access, role_id, &name, &description, permissions)
}

#[tauri::command]
pub fn delete_role(
    state: State<'_, AppContext>,
    token: String,
    role_id: i64,
) -> Result<(), AppError> {
    let access = state.authenticate(&token)?;
    staff::delete_role(&state, &access, role_id)
}

#[tauri::command]
pub fn list_audit_logs(
    state: State<'_, AppContext>,
    token: String,
    category: Option<String>,
    query: Option<String>,
    limit: usize,
) -> Result<Vec<AuditLogView>, AppError> {
    let access = state.authenticate(&token)?;
    staff::audit_logs(&state, &access, category.as_deref(), query.as_deref(), limit)
}

#[tauri::command]
pub fn list_audit_categories(
    state: State<'_, AppContext>,
    token: String,
) -> Result<Vec<String>, AppError> {
    let access = state.authenticate(&token)?;
    staff::audit_categories(&state, &access)
}
