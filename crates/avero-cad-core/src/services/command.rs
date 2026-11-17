//! Remote ER:LC command console.
//!
//! Every command passes through this service, which enforces:
//! 1. structural validation (must be an in-game `:` command, single line, bounded),
//! 2. an explicit, owner-managed policy that names the minimum permission,
//! 3. an explicit confirmation for destructive commands,
//! 4. a single API attempt — commands are never retried automatically,
//! 5. history and audit records for successes *and* rejections.
//!
//! Windows shell commands can never reach this path: anything not starting with
//! `:` and matching a stored policy is rejected before any request is made, and
//! the application never spawns operating-system processes.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::db::commands::NewCommandExecution;
use crate::error::{AppError, AppResult};
use crate::models::{CommandHistoryView, CommandPolicyView, CommandResultView};
use crate::permissions::{Access, Permission};
use crate::util::{now_ms, require_text};
use std::time::Instant;

pub struct ValidatedCommand {
    pub name: String,
    pub raw: String,
    pub arguments: String,
}

/// Validate the structure of an ER:LC command without touching the API.
pub fn validate(raw: &str) -> AppResult<ValidatedCommand> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Enter a command."));
    }
    if !trimmed.starts_with(':') {
        return Err(AppError::validation(
            "ER:LC commands start with \":\" (for example :h Hello). Windows commands are not accepted.",
        ));
    }
    if trimmed.chars().count() > 300 {
        return Err(AppError::validation(
            "Commands are limited to 300 characters.",
        ));
    }
    if trimmed
        .chars()
        .any(|c| c == '\n' || c == '\r' || c == '\0' || (c.is_control() && c != '\t'))
    {
        return Err(AppError::validation(
            "Commands must be a single line without control characters.",
        ));
    }
    let without_colon = &trimmed[1..];
    let name_end = without_colon
        .find(char::is_whitespace)
        .unwrap_or(without_colon.len());
    let name = without_colon[..name_end].to_ascii_lowercase();
    if name.is_empty() || name.len() > 24 || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AppError::validation(
            "Command names may only contain letters and numbers after the colon.",
        ));
    }
    let arguments = without_colon[name_end..].trim().to_string();
    Ok(ValidatedCommand {
        name: format!(":{name}"),
        raw: trimmed.to_string(),
        arguments,
    })
}

fn record_rejection(
    ctx: &AppContext,
    access: &Access,
    raw: &str,
    message: &str,
) -> AppResult<CommandResultView> {
    ctx.db.insert_command_history(&NewCommandExecution {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        command: raw,
        status: "rejected",
        response: message,
        http_status: None,
        command_id: None,
        duration_ms: 0,
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "command",
        action: "command.rejected",
        target: raw,
        details: message,
    })?;
    Err(AppError::permission(message.to_string()))
}

pub async fn execute(
    ctx: &AppContext,
    access: &Access,
    raw: &str,
    confirmed: bool,
) -> AppResult<CommandResultView> {
    access.require(Permission::CommandsView)?;
    let command = validate(raw)?;

    let Some(policy) = ctx.db.command_policy(&command.name)? else {
        return record_rejection(
            ctx,
            access,
            &command.raw,
            &format!(
                "\"{}\" is not an authorized command. A Server Owner must add it in Command Policies first.",
                command.name
            ),
        );
    };
    if !policy.enabled {
        return record_rejection(
            ctx,
            access,
            &command.raw,
            &format!(
                "\"{}\" is currently disabled by the Server Owner.",
                command.name
            ),
        );
    }
    let Some(required) = Permission::from_key(&policy.min_permission) else {
        return record_rejection(
            ctx,
            access,
            &command.raw,
            "This command policy references an unknown permission.",
        );
    };
    if !access.can(required) {
        return record_rejection(
            ctx,
            access,
            &command.raw,
            &format!(
                "\"{}\" requires the \"{}\" permission.",
                command.name,
                required.label()
            ),
        );
    }
    if policy.destructive && command.arguments.is_empty() {
        return Err(AppError::validation(format!(
            "{} needs a target (a Roblox username or user ID).",
            command.name
        )));
    }
    if policy.destructive && !confirmed {
        return record_rejection(
            ctx,
            access,
            &command.raw,
            &format!(
                "{} is a destructive command and must be confirmed before it is sent.",
                command.name
            ),
        );
    }

    let Some(client) = ctx.api_client()? else {
        return Err(AppError::validation(
            "Configure your ER:LC server key in API Configuration before running commands.",
        ));
    };

    let started = Instant::now();
    match client.execute_command(&command.raw).await {
        Ok((message, command_id, meta)) => {
            let duration_ms = started.elapsed().as_millis() as i64;
            ctx.db.insert_command_history(&NewCommandExecution {
                actor_user_id: Some(access.user_id),
                actor_name: &access.display_name,
                command: &command.raw,
                status: "success",
                response: &message,
                http_status: Some(meta.status),
                command_id: command_id.as_deref(),
                duration_ms,
            })?;
            ctx.db.record_audit(&AuditEntry {
                actor_user_id: Some(access.user_id),
                actor_name: &access.display_name,
                category: "command",
                action: "command.executed",
                target: &command.raw,
                details: &format!("HTTP {} — {}", meta.status, message),
            })?;
            Ok(CommandResultView {
                status: "success".into(),
                message,
                command_id,
                http_status: Some(meta.status),
                duration_ms: duration_ms as u64,
                executed_at: now_ms(),
            })
        }
        Err(err) => {
            let duration_ms = started.elapsed().as_millis() as i64;
            let message = err.message();
            ctx.db.insert_command_history(&NewCommandExecution {
                actor_user_id: Some(access.user_id),
                actor_name: &access.display_name,
                command: &command.raw,
                status: "failed",
                response: &message,
                http_status: err.http_status(),
                command_id: err.command_id().as_deref(),
                duration_ms,
            })?;
            ctx.db.record_audit(&AuditEntry {
                actor_user_id: Some(access.user_id),
                actor_name: &access.display_name,
                category: "command",
                action: "command.failed",
                target: &command.raw,
                details: &message,
            })?;
            Err(err)
        }
    }
}

/// Shared helper used by moderation actions so they follow the same policy and
/// history rules as console commands.
pub async fn execute_moderation_command(
    ctx: &AppContext,
    access: &Access,
    raw: &str,
    confirmed: bool,
) -> AppResult<CommandResultView> {
    execute(ctx, access, raw, confirmed).await
}

pub fn history(
    ctx: &AppContext,
    access: &Access,
    limit: usize,
) -> AppResult<Vec<CommandHistoryView>> {
    access.require(Permission::CommandsView)?;
    ctx.db.list_command_history(limit.clamp(1, 500))
}

pub fn policies(ctx: &AppContext, access: &Access) -> AppResult<Vec<CommandPolicyView>> {
    access.require(Permission::CommandsView)?;
    ctx.db.command_policies()
}

pub fn upsert_policy(
    ctx: &AppContext,
    access: &Access,
    command: &str,
    min_permission: &str,
    destructive: bool,
    enabled: bool,
    description: &str,
) -> AppResult<()> {
    access.require(Permission::CommandsPoliciesManage)?;
    let description = require_text(description, "Description", 0, 300).unwrap_or_default();
    ctx.db
        .upsert_command_policy(command, min_permission, destructive, enabled, &description)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "command",
        action: "command.policy_updated",
        target: command,
        details: &format!(
            "minimum permission: {min_permission}; destructive: {destructive}; enabled: {enabled}"
        ),
    })?;
    Ok(())
}

pub fn delete_policy(ctx: &AppContext, access: &Access, command: &str) -> AppResult<()> {
    access.require(Permission::CommandsPoliciesManage)?;
    ctx.db.delete_command_policy(command)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "command",
        action: "command.policy_deleted",
        target: command,
        details: "",
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_commands() {
        let parsed = validate(":h Welcome to the server").unwrap();
        assert_eq!(parsed.name, ":h");
        assert_eq!(parsed.arguments, "Welcome to the server");
        let parsed = validate(":KICK Builderman").unwrap();
        assert_eq!(parsed.name, ":kick");
        assert_eq!(parsed.arguments, "Builderman");
    }

    #[test]
    fn rejects_windows_shell_and_unknown_shapes() {
        assert!(validate("cmd.exe /c whoami").is_err());
        assert!(validate("del C:\\Windows").is_err());
        assert!(validate("").is_err());
        assert!(validate(":").is_err());
        assert!(validate(":h a\nb").is_err());
        assert!(validate(":ban; shutdown").is_err());
        assert!(validate(":ban x".repeat(100).as_str()).is_err());
    }

    #[test]
    fn rejects_non_alphanumeric_command_names() {
        assert!(validate(":../../etc/passwd").is_err());
        assert!(validate(":ba$h").is_err());
    }
}
