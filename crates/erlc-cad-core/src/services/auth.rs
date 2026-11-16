//! Local staff authentication.
//!
//! Accounts are local to the installation (ER:LC does not expose an
//! authentication API). Passwords are Argon2id hashes, sessions live only in
//! memory and every action is attributed to the signed-in staff member.

use super::{AppContext, APP_VERSION};
use crate::db::audit::AuditEntry;
use crate::db::users::NewUser;
use crate::error::{AppError, AppResult};
use crate::models::{BootstrapView, SessionView};
use crate::permissions::ROLE_OWNER;
use crate::security::{hash_password, mask_key, verify_password};
use crate::util::{is_valid_username, now_ms, require_text};

const MAX_FAILED_ATTEMPTS: i64 = 5;
const LOCK_MINUTES: i64 = 5;

pub fn bootstrap(ctx: &AppContext) -> AppResult<BootstrapView> {
    let has_users = ctx.db.has_users()?;
    let key = ctx.secrets.load()?;
    let config = ctx.db.api_config()?;
    Ok(BootstrapView {
        needs_setup: !has_users,
        app_version: APP_VERSION.to_string(),
        api_configured: key.is_some(),
        api_key_last4: key.as_deref().map(mask_key),
        api_base_url: config.base_url,
        platform: std::env::consts::OS.to_string(),
        database_path: ctx.db.path().to_string_lossy().to_string(),
        log_path: ctx.paths.log_path.to_string_lossy().to_string(),
    })
}

/// Create the first Server Owner account. Only possible on a fresh install.
pub fn create_initial_owner(
    ctx: &AppContext,
    username: &str,
    display_name: &str,
    password: &str,
) -> AppResult<SessionView> {
    if ctx.db.has_users()? {
        return Err(AppError::conflict(
            "This installation already has staff accounts. Sign in instead.",
        ));
    }
    if !is_valid_username(username.trim()) {
        return Err(AppError::validation(
            "Usernames must be 3-32 characters using letters, numbers, dots, dashes or underscores.",
        ));
    }
    let display = require_text(display_name, "Display name", 2, 64)?;
    let hash = hash_password(password)?;
    let user_id = ctx.db.create_user(NewUser {
        username: username.trim(),
        display_name: &display,
        password_hash: &hash,
        role_key: ROLE_OWNER,
        roblox_user_id: None,
    })?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(user_id),
        actor_name: &display,
        category: "auth",
        action: "staff.owner_created",
        target: username.trim(),
        details: "Initial Server Owner account created during first-run setup.",
    })?;
    let access = ctx
        .db
        .load_access(user_id)?
        .ok_or_else(|| AppError::internal("Account creation failed."))?;
    ctx.db.record_login_success(user_id)?;
    ctx.create_session(&access)
}

pub fn login(ctx: &AppContext, username: &str, password: &str) -> AppResult<SessionView> {
    let username = username.trim();
    if username.is_empty() || password.is_empty() {
        return Err(AppError::unauthorized("Enter your username and password."));
    }
    let Some(user) = ctx.db.user_by_username(username)? else {
        ctx.db.record_audit(&AuditEntry {
            actor_user_id: None,
            actor_name: username,
            category: "auth",
            action: "auth.failed",
            target: username,
            details: "Unknown username.",
        })?;
        return Err(AppError::unauthorized("Invalid username or password."));
    };
    if !user.is_active {
        return Err(AppError::unauthorized(
            "This staff account has been deactivated. Ask the server owner to reactivate it.",
        ));
    }
    if let Some(locked_until) = user.locked_until {
        if locked_until > now_ms() {
            let minutes = ((locked_until - now_ms()) as f64 / 60_000.0).ceil() as i64;
            return Err(AppError::unauthorized(format!(
                "Too many failed attempts. Try again in {minutes} minute(s)."
            )));
        }
    }
    if !verify_password(&user.password_hash, password) {
        let (attempts, locked) =
            ctx.db
                .record_login_failure(user.id, MAX_FAILED_ATTEMPTS, LOCK_MINUTES)?;
        ctx.db.record_audit(&AuditEntry {
            actor_user_id: Some(user.id),
            actor_name: &user.username,
            category: "auth",
            action: "auth.failed",
            target: &user.username,
            details: &format!("Incorrect password (attempt {attempts})."),
        })?;
        if locked.is_some() {
            return Err(AppError::unauthorized(format!(
                "Too many failed attempts. This account is locked for {LOCK_MINUTES} minutes."
            )));
        }
        let remaining = (MAX_FAILED_ATTEMPTS - attempts).max(0);
        return Err(AppError::unauthorized(format!(
            "Invalid username or password. {remaining} attempt(s) remaining before a temporary lock."
        )));
    }

    ctx.db.clear_login_failures(user.id)?;
    ctx.db.record_login_success(user.id)?;
    let access = ctx
        .db
        .load_access(user.id)?
        .ok_or_else(|| AppError::internal("Could not load permissions."))?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(user.id),
        actor_name: &user.display_name,
        category: "auth",
        action: "auth.login",
        target: &user.username,
        details: &format!("Signed in as {}.", user.role_name),
    })?;
    ctx.create_session(&access)
}

pub fn session_info(ctx: &AppContext, token: &str) -> AppResult<SessionView> {
    let access = ctx.authenticate(token)?;
    let user = ctx
        .db
        .list_staff_users()?
        .into_iter()
        .find(|u| u.id == access.user_id)
        .ok_or_else(|| AppError::not_found("Staff account not found."))?;
    Ok(SessionView {
        token: token.to_string(),
        expires_at: ctx.session_expiry(token).unwrap_or(now_ms()),
        is_owner: access.is_owner(),
        permissions: access.effective_permissions(),
        user,
    })
}

pub fn logout(ctx: &AppContext, token: &str) -> AppResult<()> {
    if let Ok(access) = ctx.authenticate(token) {
        ctx.db.record_audit(&AuditEntry {
            actor_user_id: Some(access.user_id),
            actor_name: &access.display_name,
            category: "auth",
            action: "auth.logout",
            target: &access.username,
            details: "",
        })?;
    }
    ctx.logout(token);
    Ok(())
}

pub fn change_password(
    ctx: &AppContext,
    token: &str,
    current_password: &str,
    new_password: &str,
) -> AppResult<()> {
    let access = ctx.authenticate(token)?;
    let user = ctx
        .db
        .user_by_id(access.user_id)?
        .ok_or_else(|| AppError::not_found("Staff account not found."))?;
    if !verify_password(&user.password_hash, current_password) {
        return Err(AppError::unauthorized(
            "Your current password is incorrect.",
        ));
    }
    if verify_password(&user.password_hash, new_password) {
        return Err(AppError::validation(
            "Choose a password different from your current one.",
        ));
    }
    let hash = hash_password(new_password)?;
    ctx.db.set_user_password_hash(user.id, &hash)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "auth",
        action: "auth.password_changed",
        target: &access.username,
        details: "",
    })?;
    Ok(())
}
