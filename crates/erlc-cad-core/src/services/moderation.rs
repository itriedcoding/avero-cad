//! Moderation workflows.
//!
//! ER:LC exposes no ban-write endpoint: bans, unbans and kicks are executed as
//! in-game commands through `POST /v2/server/command`, and the result is stored
//! locally. Internal CAD bans are database-only records and are labelled as such
//! everywhere in the UI and in the audit trail.

use super::{command, AppContext};
use crate::db::audit::AuditEntry;
use crate::db::moderation::NewModerationRecord;
use crate::error::{AppError, AppResult};
use crate::models::{CommandResultView, InternalBanView, LiveBanView, ModerationRecordView};
use crate::permissions::{Access, Permission};
use crate::util::{is_valid_roblox_username, require_text};

fn validate_target(target: &str) -> AppResult<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(
            "Enter a Roblox username or numeric user ID.",
        ));
    }
    if trimmed.parse::<u64>().is_ok() {
        return Ok(trimmed.to_string());
    }
    if !is_valid_roblox_username(trimmed) {
        return Err(AppError::validation(
            "Enter a valid Roblox username (3-20 letters, numbers or underscores) or a numeric user ID.",
        ));
    }
    Ok(trimmed.to_string())
}

#[allow(clippy::too_many_arguments)]
fn record_outcome(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    target: &str,
    action: &str,
    reason: &str,
    outcome: &Result<CommandResultView, AppError>,
) -> AppResult<()> {
    let (success, message, command_id) = match outcome {
        Ok(result) => (true, Some(result.message.clone()), result.command_id.clone()),
        Err(err) => (false, Some(err.message()), err.command_id()),
    };
    ctx.db.insert_moderation_record(&NewModerationRecord {
        roblox_user_id,
        username: target,
        action,
        source: "erlc_api",
        reason,
        moderator_user_id: Some(access.user_id),
        moderator_name: &access.display_name,
        success,
        command_id: command_id.as_deref(),
        api_message: message.as_deref(),
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "moderation",
        action: &format!("player.{action}"),
        target,
        details: &format!(
            "{} | reason: {} | success: {}",
            message.unwrap_or_default(),
            reason,
            success
        ),
    })?;
    Ok(())
}

async fn run_action(
    ctx: &AppContext,
    access: &Access,
    permission: Permission,
    command_name: &str,
    target: &str,
    reason: &str,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> AppResult<CommandResultView> {
    access.require(permission)?;
    let target = validate_target(target)?;
    let reason = require_text(reason, "Reason", 3, 500)?;
    let raw = format!("{command_name} {target}");
    let outcome = command::execute_moderation_command(ctx, access, &raw, confirmed).await;
    record_outcome(
        ctx,
        access,
        roblox_user_id,
        &target,
        command_name.trim_start_matches(':'),
        &reason,
        &outcome,
    )?;
    outcome
}

pub async fn kick(
    ctx: &AppContext,
    access: &Access,
    target: &str,
    reason: &str,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> AppResult<CommandResultView> {
    run_action(
        ctx,
        access,
        Permission::ModerationKick,
        ":kick",
        target,
        reason,
        confirmed,
        roblox_user_id,
    )
    .await
}

pub async fn ban(
    ctx: &AppContext,
    access: &Access,
    target: &str,
    reason: &str,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> AppResult<CommandResultView> {
    run_action(
        ctx,
        access,
        Permission::ModerationBan,
        ":ban",
        target,
        reason,
        confirmed,
        roblox_user_id,
    )
    .await
}

pub async fn unban(
    ctx: &AppContext,
    access: &Access,
    target: &str,
    reason: &str,
    confirmed: bool,
    roblox_user_id: Option<u64>,
) -> AppResult<CommandResultView> {
    run_action(
        ctx,
        access,
        Permission::ModerationUnban,
        ":unban",
        target,
        reason,
        confirmed,
        roblox_user_id,
    )
    .await
}

/// CAD-only ban. This never alters the ER:LC server.
pub fn create_internal_ban(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    username: &str,
    reason: &str,
) -> AppResult<InternalBanView> {
    access.require(Permission::ModerationInternal)?;
    let username = require_text(username, "Username", 1, 20)?;
    let reason = require_text(reason, "Reason", 3, 500)?;
    let id = ctx.db.create_internal_ban(
        roblox_user_id,
        &username,
        &reason,
        Some(access.user_id),
        &access.display_name,
    )?;
    ctx.db.insert_moderation_record(&NewModerationRecord {
        roblox_user_id,
        username: &username,
        action: "internal_ban",
        source: "internal_cad",
        reason: &reason,
        moderator_user_id: Some(access.user_id),
        moderator_name: &access.display_name,
        success: true,
        command_id: None,
        api_message: Some(
            "Internal CAD-only ban. This does not ban the player in ER:LC.",
        ),
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "moderation",
        action: "player.internal_ban",
        target: &username,
        details: &reason,
    })?;
    ctx.db
        .list_internal_bans(false, 500)?
        .into_iter()
        .find(|ban| ban.id == id)
        .ok_or_else(|| AppError::internal("Internal ban could not be reloaded."))
}

pub fn lift_internal_ban(
    ctx: &AppContext,
    access: &Access,
    ban_id: i64,
    reason: &str,
) -> AppResult<()> {
    access.require(Permission::ModerationInternal)?;
    let reason = require_text(reason, "Reason", 3, 500)?;
    ctx.db
        .lift_internal_ban(ban_id, &access.display_name, &reason)?;
    ctx.db.insert_moderation_record(&NewModerationRecord {
        roblox_user_id: None,
        username: &format!("internal-ban-{ban_id}"),
        action: "internal_unban",
        source: "internal_cad",
        reason: &reason,
        moderator_user_id: Some(access.user_id),
        moderator_name: &access.display_name,
        success: true,
        command_id: None,
        api_message: Some("Internal CAD ban lifted."),
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "moderation",
        action: "player.internal_ban_lifted",
        target: &format!("ban #{ban_id}"),
        details: &reason,
    })?;
    Ok(())
}

pub fn history_for_player(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    username: &str,
    limit: usize,
) -> AppResult<Vec<ModerationRecordView>> {
    access.require(Permission::ModerationView)?;
    ctx.db
        .moderation_for_player(roblox_user_id, username, limit.clamp(1, 200))
}

pub fn recent(
    ctx: &AppContext,
    access: &Access,
    limit: usize,
) -> AppResult<Vec<ModerationRecordView>> {
    access.require(Permission::ModerationView)?;
    ctx.db.recent_moderation(limit.clamp(1, 200))
}

pub fn internal_bans(
    ctx: &AppContext,
    access: &Access,
    active_only: bool,
    limit: usize,
) -> AppResult<Vec<InternalBanView>> {
    access.require(Permission::ModerationView)?;
    ctx.db.list_internal_bans(active_only, limit.clamp(1, 500))
}

/// Live ban list from `GET /v1/server/bans`.
pub async fn live_bans(ctx: &AppContext, access: &Access) -> AppResult<Vec<LiveBanView>> {
    access.require(Permission::ModerationView)?;
    let Some(client) = ctx.api_client()? else {
        return Err(AppError::validation(
            "Configure your ER:LC server key to read the server ban list.",
        ));
    };
    let (bans, _meta) = client.fetch_bans().await?;
    Ok(bans)
}
