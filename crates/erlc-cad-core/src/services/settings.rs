//! Settings, API configuration and database maintenance.

use super::{snapshot, AppContext};
use crate::api::ApiClient;
use crate::db::audit::AuditEntry;
use crate::error::{AppError, AppResult};
use crate::models::{
    ApiConfigView, BackupView, ConnectionTestView, DatabaseStatsView, SettingsUpdate, SettingsView,
};
use crate::permissions::{Access, Permission};
use crate::security::{fingerprint, mask_key};
use crate::util::{now_ms, require_text};
use std::path::PathBuf;

pub fn read_settings(ctx: &AppContext, access: &Access) -> AppResult<SettingsView> {
    // Any signed-in staff member may read display preferences; nothing here is
    // sensitive (the API key is never included).
    access.require(Permission::DashboardView)?;
    ctx.db.settings_view()
}

pub fn update_settings(
    ctx: &AppContext,
    access: &Access,
    update: SettingsUpdate,
) -> AppResult<SettingsView> {
    access.require(Permission::SettingsManage)?;
    let mut values: Vec<(String, String)> = Vec::new();

    if let Some(interval) = update.refresh_interval_seconds {
        if !(5..=300).contains(&interval) {
            return Err(AppError::validation(
                "The refresh interval must be between 5 and 300 seconds.",
            ));
        }
        values.push(("refresh_interval_seconds".into(), interval.to_string()));
    }
    if let Some(page) = update.startup_page {
        let allowed = [
            "dashboard",
            "dispatch",
            "map",
            "players",
            "moderation",
            "commands",
            "logs",
            "settings",
        ];
        if !allowed.contains(&page.as_str()) {
            return Err(AppError::validation("Unknown startup page."));
        }
        values.push(("startup_page".into(), page));
    }
    if let Some(value) = update.auto_connect {
        values.push(("auto_connect".into(), value.to_string()));
    }
    if let Some(value) = update.confirm_destructive {
        values.push(("confirm_destructive".into(), value.to_string()));
    }
    if let Some(value) = update.compact_tables {
        values.push(("compact_tables".into(), value.to_string()));
    }
    if let Some(image) = update.map_image {
        let trimmed = image.trim().to_string();
        if !trimmed.starts_with("https://api.erlc.gg/maps/") || !trimmed.ends_with(".png") {
            return Err(AppError::validation(
                "Map images must be one of the official ER:LC map images from api.erlc.gg/maps.",
            ));
        }
        values.push(("map_image".into(), trimmed));
    }
    if let Some(scale) = update.map_scale {
        if !(0.2..=20.0).contains(&scale) || !scale.is_finite() {
            return Err(AppError::validation(
                "The map scale must be between 0.2 and 20 pixels per world unit.",
            ));
        }
        values.push(("map_scale".into(), format!("{scale:.3}")));
    }
    if let Some(minutes) = update.session_idle_minutes {
        if !(5..=480).contains(&minutes) {
            return Err(AppError::validation(
                "The session timeout must be between 5 and 480 minutes.",
            ));
        }
        values.push(("session_idle_minutes".into(), minutes.to_string()));
    }
    if let Some(directory) = update.backup_directory {
        let trimmed = directory.trim().to_string();
        if !trimmed.is_empty() {
            let path = PathBuf::from(&trimmed);
            if !path.is_absolute() {
                return Err(AppError::validation(
                    "The backup folder must be an absolute path.",
                ));
            }
        }
        values.push(("backup_directory".into(), trimmed));
    }
    if let Some(hours) = update.auto_backup_hours {
        if hours > 720 {
            return Err(AppError::validation(
                "Automatic backups can run at most every 720 hours.",
            ));
        }
        values.push(("auto_backup_hours".into(), hours.to_string()));
    }
    if let Some(value) = update.notify_emergency_calls {
        values.push(("notify_emergency_calls".into(), value.to_string()));
    }
    if let Some(value) = update.notify_mod_calls {
        values.push(("notify_mod_calls".into(), value.to_string()));
    }
    if let Some(value) = update.notify_connection {
        values.push(("notify_connection".into(), value.to_string()));
    }

    if values.is_empty() {
        return ctx.db.settings_view();
    }
    ctx.db.set_settings(&values)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "settings",
        action: "settings.updated",
        target: "application settings",
        details: &values
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>()
            .join(", "),
    })?;
    ctx.db.settings_view()
}

pub fn api_config(ctx: &AppContext, access: &Access) -> AppResult<ApiConfigView> {
    access.require(Permission::ApiConfigure)?;
    let config = ctx.db.api_config()?;
    let key = ctx.server_key()?;
    Ok(ApiConfigView {
        base_url: config.base_url,
        configured: key.is_some(),
        key_last4: key.as_deref().map(mask_key),
        key_fingerprint: key.as_deref().map(fingerprint),
        key_storage: ctx.secrets.storage_kind().to_string(),
        last_verified_at: config.last_verified_at,
        server_name: config.server_name,
        server_id: config.server_id,
    })
}

/// Store a server key in DPAPI-protected storage. The key is never echoed back.
pub fn save_api_key(
    ctx: &AppContext,
    access: &Access,
    base_url: &str,
    key: &str,
) -> AppResult<ApiConfigView> {
    access.require(Permission::ApiConfigure)?;
    let base = require_text(base_url, "API base URL", 8, 200)?;
    if !base.starts_with("https://") {
        return Err(AppError::validation("The API base URL must use HTTPS."));
    }
    let trimmed_key = register_key(ctx, key)?;
    ctx.db.update_api_config(
        &base,
        ctx.secrets.storage_kind(),
        Some(&fingerprint(&trimmed_key)),
        Some(&mask_key(&trimmed_key)),
    )?;
    ctx.invalidate_api_client();
    snapshot::clear(ctx);
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "api",
        action: "api.key_saved",
        target: &base,
        details: &format!("key fingerprint {}", fingerprint(&trimmed_key)),
    })?;
    api_config(ctx, access)
}

fn register_key(ctx: &AppContext, key: &str) -> AppResult<String> {
    let trimmed = key.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::validation("Paste your ER:LC server key."));
    }
    ctx.secrets.store(&trimmed)?;
    Ok(trimmed)
}

pub fn clear_api_key(ctx: &AppContext, access: &Access) -> AppResult<()> {
    access.require(Permission::ApiConfigure)?;
    ctx.secrets.clear()?;
    ctx.invalidate_api_client();
    snapshot::clear(ctx);
    let config = ctx.db.api_config()?;
    ctx.db
        .update_api_config(&config.base_url, ctx.secrets.storage_kind(), None, None)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "api",
        action: "api.key_cleared",
        target: &config.base_url,
        details: "",
    })?;
    Ok(())
}

/// Test the stored key, or a candidate key before saving it.
pub async fn test_connection(
    ctx: &AppContext,
    access: &Access,
    candidate_key: Option<String>,
) -> AppResult<ConnectionTestView> {
    access.require(Permission::ApiConfigure)?;
    let config = ctx.db.api_config()?;
    let candidate = candidate_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string);
    let using_stored = candidate.is_none();
    let key = match candidate {
        Some(key) => key,
        None => ctx.server_key()?.ok_or_else(|| {
            AppError::validation("No server key is stored yet. Paste one to test the connection.")
        })?,
    };
    let client = match ApiClient::new(&config.base_url, &key, Some(&ctx.paths.log_path)) {
        Ok(client) => client,
        Err(err) => {
            return Ok(ConnectionTestView {
                ok: false,
                message: err.message(),
                latency_ms: 0,
                server_name: None,
                current_players: None,
                max_players: None,
                rate_limit: None,
            })
        }
    };
    match client.fetch_summary().await {
        Ok((summary, meta)) => {
            if using_stored {
                let _ = ctx
                    .db
                    .mark_api_verified(&summary.name, Some(summary.owner_id as i64));
            }
            ctx.db.record_audit(&AuditEntry {
                actor_user_id: Some(access.user_id),
                actor_name: &access.display_name,
                category: "api",
                action: "api.connection_verified",
                target: &summary.name,
                details: &format!("HTTP {} in {} ms", meta.status, meta.latency_ms),
            })?;
            Ok(ConnectionTestView {
                ok: true,
                message: format!(
                    "Connected to \"{}\" — {} of {} players online.",
                    summary.name, summary.current_players, summary.max_players
                ),
                latency_ms: meta.latency_ms,
                server_name: Some(summary.name),
                current_players: Some(summary.current_players),
                max_players: Some(summary.max_players),
                rate_limit: Some(meta.rate_limit),
            })
        }
        Err(err) => Ok(ConnectionTestView {
            ok: false,
            message: err.message(),
            latency_ms: 0,
            server_name: None,
            current_players: None,
            max_players: None,
            rate_limit: None,
        }),
    }
}

pub fn backup_database(
    ctx: &AppContext,
    access: &Access,
    directory: Option<String>,
) -> AppResult<BackupView> {
    access.require(Permission::DatabaseBackup)?;
    let settings = ctx.db.settings_view()?;
    let target = directory
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .or_else(|| {
            if settings.backup_directory.trim().is_empty() {
                None
            } else {
                Some(settings.backup_directory.clone())
            }
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.paths.data_dir.join("backups"));
    let backup = ctx.db.backup_to(&target)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "database",
        action: "database.backup_created",
        target: &backup.path,
        details: &crate::util::human_bytes(backup.size_bytes),
    })?;
    Ok(backup)
}

pub fn restore_database(
    ctx: &AppContext,
    access: &Access,
    source: &str,
    confirmed: bool,
) -> AppResult<BackupView> {
    access.require(Permission::DatabaseRestore)?;
    if !confirmed {
        return Err(AppError::validation(
            "Restoring a database replaces all current records and must be confirmed.",
        ));
    }
    let source_path = PathBuf::from(source.trim());
    if !source_path.exists() {
        return Err(AppError::not_found("That backup file does not exist."));
    }
    // Always keep a safety copy of the current data first.
    let safety = ctx.db.backup_to(&ctx.paths.data_dir.join("backups"))?;
    ctx.db.restore_from(&source_path)?;
    ctx.invalidate_api_client();
    snapshot::clear(ctx);
    ctx.clear_sessions();
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: None,
        actor_name: &access.display_name,
        category: "database",
        action: "database.restored",
        target: &source_path.to_string_lossy(),
        details: &format!("Pre-restore safety backup: {}", safety.path),
    })?;
    Ok(BackupView {
        path: safety.path,
        size_bytes: safety.size_bytes,
        created_at: safety.created_at,
    })
}

pub fn database_stats(ctx: &AppContext, access: &Access) -> AppResult<DatabaseStatsView> {
    access.require(Permission::SettingsManage)?;
    ctx.db.stats()
}

/// Run an automatic backup when one is due. Called on application start.
pub fn auto_backup_if_due(ctx: &AppContext) -> AppResult<Option<BackupView>> {
    let settings = ctx.db.settings_view()?;
    if settings.auto_backup_hours == 0 {
        return Ok(None);
    }
    if settings.backup_directory.trim().is_empty() {
        return Ok(None);
    }
    let due = match settings.last_backup_at {
        Some(at) => now_ms() - at >= settings.auto_backup_hours as i64 * 3_600_000,
        None => true,
    };
    if !due {
        return Ok(None);
    }
    let directory = PathBuf::from(&settings.backup_directory);
    let backup = ctx.db.backup_to(&directory)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: None,
        actor_name: "system",
        category: "database",
        action: "database.auto_backup",
        target: &backup.path,
        details: &crate::util::human_bytes(backup.size_bytes),
    })?;
    Ok(Some(backup))
}
