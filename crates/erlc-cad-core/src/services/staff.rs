//! Staff account, role and permission management.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::db::users::NewUser;
use crate::error::{AppError, AppResult};
use crate::models::{RoleView, StaffUserView};
use crate::permissions::{Access, Permission, PermissionInfo, ROLE_OWNER};
use crate::security::hash_password;
use crate::util::require_text;

pub fn list_staff(ctx: &AppContext, access: &Access) -> AppResult<Vec<StaffUserView>> {
    access.require(Permission::StaffView)?;
    ctx.db.list_staff_users()
}

pub fn permission_catalogue(_ctx: &AppContext, access: &Access) -> AppResult<Vec<PermissionInfo>> {
    access.require(Permission::StaffView)?;
    Ok(crate::permissions::catalogue())
}

pub fn create_staff(
    ctx: &AppContext,
    access: &Access,
    username: &str,
    display_name: &str,
    password: &str,
    role_key: &str,
    roblox_user_id: Option<u64>,
) -> AppResult<StaffUserView> {
    access.require(Permission::StaffManage)?;
    if role_key == ROLE_OWNER && !access.is_owner() {
        return Err(AppError::permission(
            "Only a Server Owner can create another Server Owner account.",
        ));
    }
    let display_name = require_text(display_name, "Display name", 2, 64)?;
    let hash = hash_password(password)?;
    let id = ctx.db.create_user(NewUser {
        username: username.trim(),
        display_name: &display_name,
        password_hash: &hash,
        role_key,
        roblox_user_id,
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "staff.created",
        target: username.trim(),
        details: &format!("role: {role_key}"),
    })?;
    ctx.db
        .list_staff_users()?
        .into_iter()
        .find(|user| user.id == id)
        .ok_or_else(|| AppError::internal("Staff account could not be reloaded."))
}

fn ensure_not_last_owner(ctx: &AppContext, user_id: i64) -> AppResult<()> {
    let user = ctx
        .db
        .user_by_id(user_id)?
        .ok_or_else(|| AppError::not_found("Staff account not found."))?;
    if user.role_key == ROLE_OWNER && user.is_active && ctx.db.count_active_owners()? <= 1 {
        return Err(AppError::conflict(
            "This is the only active Server Owner. Promote another owner before changing this account.",
        ));
    }
    Ok(())
}

pub fn update_role(
    ctx: &AppContext,
    access: &Access,
    user_id: i64,
    role_key: &str,
) -> AppResult<()> {
    access.require(Permission::StaffManage)?;
    if role_key == ROLE_OWNER && !access.is_owner() {
        return Err(AppError::permission(
            "Only a Server Owner can grant the Server Owner role.",
        ));
    }
    ensure_not_last_owner(ctx, user_id)?;
    ctx.db.update_user_role(user_id, role_key)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "staff.role_changed",
        target: &format!("user #{user_id}"),
        details: role_key,
    })?;
    Ok(())
}

pub fn set_active(ctx: &AppContext, access: &Access, user_id: i64, active: bool) -> AppResult<()> {
    access.require(Permission::StaffManage)?;
    if access.user_id == user_id && !active {
        return Err(AppError::conflict(
            "You cannot deactivate your own account.",
        ));
    }
    if !active {
        ensure_not_last_owner(ctx, user_id)?;
    }
    ctx.db.set_user_active(user_id, active)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: if active {
            "staff.activated"
        } else {
            "staff.deactivated"
        },
        target: &format!("user #{user_id}"),
        details: "",
    })?;
    Ok(())
}

pub fn reset_password(
    ctx: &AppContext,
    access: &Access,
    user_id: i64,
    new_password: &str,
) -> AppResult<()> {
    access.require(Permission::StaffManage)?;
    let target = ctx
        .db
        .user_by_id(user_id)?
        .ok_or_else(|| AppError::not_found("Staff account not found."))?;
    if target.role_key == ROLE_OWNER && !access.is_owner() {
        return Err(AppError::permission(
            "Only a Server Owner can reset another owner's password.",
        ));
    }
    let hash = hash_password(new_password)?;
    ctx.db.set_user_password_hash(user_id, &hash)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "staff.password_reset",
        target: &target.username,
        details: "",
    })?;
    Ok(())
}

pub fn set_override(
    ctx: &AppContext,
    access: &Access,
    user_id: i64,
    permission_key: &str,
    allowed: Option<bool>,
) -> AppResult<()> {
    access.require(Permission::RolesManage)?;
    if Permission::from_key(permission_key).is_none() {
        return Err(AppError::validation("Unknown permission."));
    }
    let target = ctx
        .db
        .user_by_id(user_id)?
        .ok_or_else(|| AppError::not_found("Staff account not found."))?;
    if target.role_key == ROLE_OWNER && allowed == Some(false) {
        return Err(AppError::permission(
            "The Server Owner role already holds every permission.",
        ));
    }
    ctx.db
        .set_user_permission_override(user_id, permission_key, allowed)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "staff.permission_override",
        target: &target.username,
        details: &format!(
            "{permission_key}: {}",
            match allowed {
                Some(true) => "granted",
                Some(false) => "denied",
                None => "cleared",
            }
        ),
    })?;
    Ok(())
}

pub fn roles(ctx: &AppContext, access: &Access) -> AppResult<Vec<RoleView>> {
    access.require(Permission::StaffView)?;
    ctx.db.list_roles()
}

pub fn create_role(
    ctx: &AppContext,
    access: &Access,
    key: &str,
    name: &str,
    description: &str,
    permissions: Vec<String>,
) -> AppResult<i64> {
    access.require(Permission::RolesManage)?;
    let id = ctx.db.create_role(key, name, description, &permissions)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "role.created",
        target: key,
        details: &format!("{} permission(s)", permissions.len()),
    })?;
    Ok(id)
}

pub fn update_role_definition(
    ctx: &AppContext,
    access: &Access,
    role_id: i64,
    name: &str,
    description: &str,
    permissions: Vec<String>,
) -> AppResult<()> {
    access.require(Permission::RolesManage)?;
    ctx.db.update_role(role_id, name, description)?;
    ctx.db.set_role_permissions(role_id, &permissions)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "role.permissions_changed",
        target: &format!("role #{role_id}"),
        details: &format!("{} permission(s)", permissions.len()),
    })?;
    Ok(())
}

pub fn delete_role(ctx: &AppContext, access: &Access, role_id: i64) -> AppResult<()> {
    access.require(Permission::RolesManage)?;
    ctx.db.delete_role(role_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "staff",
        action: "role.deleted",
        target: &format!("role #{role_id}"),
        details: "",
    })?;
    Ok(())
}

pub fn audit_logs(
    ctx: &AppContext,
    access: &Access,
    category: Option<&str>,
    query: Option<&str>,
    limit: usize,
) -> AppResult<Vec<crate::models::AuditLogView>> {
    access.require(Permission::AuditView)?;
    ctx.db.list_audit_logs(category, query, limit.clamp(1, 500))
}

pub fn audit_categories(ctx: &AppContext, access: &Access) -> AppResult<Vec<String>> {
    access.require(Permission::AuditView)?;
    ctx.db.list_audit_categories()
}
