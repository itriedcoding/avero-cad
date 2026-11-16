//! CAD dispatch service.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::db::dispatch::{NewCall, CALL_PRIORITIES};
use crate::error::{AppError, AppResult};
use crate::models::{CallView, UnitView};
use crate::permissions::{Access, Permission};
use crate::util::require_text;

pub fn list_calls(
    ctx: &AppContext,
    access: &Access,
    status: Option<&str>,
    limit: usize,
) -> AppResult<Vec<CallView>> {
    access.require(Permission::DispatchView)?;
    ctx.db.list_calls(status, limit.clamp(1, 300))
}

pub fn create_call(
    ctx: &AppContext,
    access: &Access,
    title: &str,
    description: &str,
    location: &str,
    postal: &str,
    priority: &str,
) -> AppResult<CallView> {
    access.require(Permission::DispatchCreate)?;
    let title = require_text(title, "Call title", 3, 120)?;
    if !CALL_PRIORITIES.contains(&priority) {
        return Err(AppError::validation("Choose a valid call priority."));
    }
    let call = ctx.db.create_call(
        &NewCall {
            title: &title,
            description,
            location,
            postal,
            priority,
        },
        Some(access.user_id),
        &access.display_name,
    )?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "call.created",
        target: &format!("#{}", call.call_number),
        details: &title,
    })?;
    Ok(call)
}

pub fn update_call(
    ctx: &AppContext,
    access: &Access,
    call_id: i64,
    title: &str,
    description: &str,
    location: &str,
    postal: &str,
    priority: &str,
) -> AppResult<CallView> {
    access.require(Permission::DispatchUpdate)?;
    let title = require_text(title, "Call title", 3, 120)?;
    let call = ctx.db.update_call_details(
        call_id,
        &title,
        description,
        location,
        postal,
        priority,
    )?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "call.updated",
        target: &format!("#{}", call.call_number),
        details: &title,
    })?;
    Ok(call)
}

pub fn set_call_status(
    ctx: &AppContext,
    access: &Access,
    call_id: i64,
    status: &str,
) -> AppResult<CallView> {
    if status == "closed" {
        access.require(Permission::DispatchClose)?;
    } else {
        access.require(Permission::DispatchUpdate)?;
    }
    let call = ctx
        .db
        .set_call_status(call_id, status, Some(&access.display_name))?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: if status == "closed" {
            "call.closed"
        } else {
            "call.status_changed"
        },
        target: &format!("#{}", call.call_number),
        details: status,
    })?;
    Ok(call)
}

pub fn attach_player(
    ctx: &AppContext,
    access: &Access,
    call_id: i64,
    roblox_user_id: u64,
    username: &str,
) -> AppResult<CallView> {
    access.require(Permission::DispatchUpdate)?;
    ctx.db.add_call_player(call_id, roblox_user_id, username)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "call.player_attached",
        target: &format!("call #{call_id}"),
        details: username,
    })?;
    ctx.db.call_by_id(call_id)
}

pub fn detach_player(
    ctx: &AppContext,
    access: &Access,
    call_id: i64,
    roblox_user_id: u64,
) -> AppResult<CallView> {
    access.require(Permission::DispatchUpdate)?;
    ctx.db.remove_call_player(call_id, roblox_user_id)?;
    ctx.db.call_by_id(call_id)
}

pub fn units(ctx: &AppContext, access: &Access) -> AppResult<Vec<UnitView>> {
    access.require(Permission::DispatchView)?;
    ctx.db.list_units()
}

pub fn create_unit(
    ctx: &AppContext,
    access: &Access,
    callsign: &str,
    label: &str,
    team: &str,
    status: &str,
) -> AppResult<UnitView> {
    access.require(Permission::DispatchUnits)?;
    let unit = ctx.db.create_unit(
        callsign,
        label,
        team,
        status,
        None,
        Some(access.user_id),
    )?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "unit.created",
        target: &unit.callsign,
        details: &format!("{} / {}", unit.team, unit.status),
    })?;
    Ok(unit)
}

pub fn update_unit(
    ctx: &AppContext,
    access: &Access,
    unit_id: i64,
    label: &str,
    team: &str,
    status: &str,
) -> AppResult<UnitView> {
    access.require(Permission::DispatchUnits)?;
    let unit = ctx.db.update_unit(unit_id, label, team, status)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "unit.updated",
        target: &unit.callsign,
        details: &format!("{} / {}", unit.team, unit.status),
    })?;
    Ok(unit)
}

pub fn assign_unit(
    ctx: &AppContext,
    access: &Access,
    call_id: i64,
    unit_id: i64,
) -> AppResult<CallView> {
    access.require(Permission::DispatchUpdate)?;
    ctx.db.assign_unit(call_id, unit_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "unit.assigned",
        target: &format!("call #{call_id}"),
        details: &format!("unit #{unit_id}"),
    })?;
    ctx.db.call_by_id(call_id)
}

pub fn unassign_unit(ctx: &AppContext, access: &Access, unit_id: i64) -> AppResult<()> {
    access.require(Permission::DispatchUpdate)?;
    ctx.db.unassign_unit(unit_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "unit.unassigned",
        target: &format!("unit #{unit_id}"),
        details: "",
    })?;
    Ok(())
}

pub fn delete_unit(ctx: &AppContext, access: &Access, unit_id: i64) -> AppResult<()> {
    access.require(Permission::DispatchUnits)?;
    ctx.db.delete_unit(unit_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "dispatch",
        action: "unit.deleted",
        target: &format!("unit #{unit_id}"),
        details: "",
    })?;
    Ok(())
}

pub fn require_map_access(access: &Access) -> AppResult<()> {
    if !access.can(Permission::MapView) && !access.can(Permission::DispatchView) {
        return Err(AppError::permission(
            "Your role does not include live map access.",
        ));
    }
    Ok(())
}
