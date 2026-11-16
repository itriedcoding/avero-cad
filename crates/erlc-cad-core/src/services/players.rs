//! Player lookup: live roster, local records, Roblox identity and avatars.

use super::{snapshot, AppContext};
use crate::db::audit::AuditEntry;
use crate::error::{AppError, AppResult};
use crate::models::{
    PlayerProfile, PlayerView, RobloxUserView, SearchResult, StaffNoteView,
};
use crate::permissions::{Access, Permission};
use crate::roblox::AvatarKind;
use crate::util::{is_valid_roblox_username, require_text};

fn live_players(ctx: &AppContext) -> Vec<PlayerView> {
    ctx.cache
        .lock()
        .ok()
        .and_then(|cache| {
            cache
                .current
                .as_ref()
                .map(|cached| cached.snapshot.players.clone())
        })
        .unwrap_or_default()
}

pub fn search(ctx: &AppContext, access: &Access, query: &str) -> AppResult<Vec<SearchResult>> {
    access.require(Permission::PlayersSearch)?;
    let trimmed = query.trim();
    if trimmed.chars().count() < 2 {
        return Err(AppError::validation(
            "Enter at least 2 characters to search players.",
        ));
    }
    let needle = trimmed.to_ascii_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();

    for player in live_players(ctx) {
        let id_matches = player
            .roblox_user_id
            .map(|id| id.to_string() == trimmed)
            .unwrap_or(false);
        let callsign_matches = player
            .callsign
            .as_deref()
            .map(|c| c.to_ascii_lowercase().contains(&needle))
            .unwrap_or(false);
        if player.username.to_ascii_lowercase().contains(&needle) || id_matches || callsign_matches {
            results.push(SearchResult {
                roblox_user_id: player.roblox_user_id,
                username: player.username.clone(),
                online: true,
                team: Some(player.team.clone()),
                callsign: player.callsign.clone(),
                permission: Some(player.permission.clone()),
                source: "live".into(),
            });
        }
    }

    for row in ctx.db.search_players(trimmed, 40)? {
        if results
            .iter()
            .any(|existing| existing.roblox_user_id == Some(row.roblox_user_id))
        {
            continue;
        }
        results.push(SearchResult {
            roblox_user_id: Some(row.roblox_user_id),
            username: row.username,
            online: false,
            team: row.last_team,
            callsign: row.last_callsign,
            permission: None,
            source: "database".into(),
        });
    }

    results.sort_by(|a, b| {
        b.online
            .cmp(&a.online)
            .then_with(|| a.username.to_ascii_lowercase().cmp(&b.username.to_ascii_lowercase()))
    });
    results.truncate(50);
    Ok(results)
}

/// Resolve an identifier against the live roster, the local database and Roblox.
async fn resolve_identity(
    ctx: &AppContext,
    roblox_user_id: Option<u64>,
    username: Option<&str>,
) -> AppResult<(Option<u64>, String)> {
    if let Some(id) = roblox_user_id {
        let live = live_players(ctx)
            .into_iter()
            .find(|p| p.roblox_user_id == Some(id));
        let name = match live {
            Some(player) => player.username,
            None => match ctx.db.player_by_roblox_id(id)? {
                Some(record) => record.username,
                None => ctx.roblox.user_by_id(id).await?.name,
            },
        };
        return Ok((Some(id), name));
    }

    let username = username.unwrap_or("").trim().to_string();
    if username.is_empty() {
        return Err(AppError::validation("Enter a Roblox username or user ID."));
    }
    if let Ok(id) = username.parse::<u64>() {
        let live = live_players(ctx)
            .into_iter()
            .find(|player| player.roblox_user_id == Some(id));
        let name = match live {
            Some(player) => player.username,
            None => match ctx.db.player_by_roblox_id(id)? {
                Some(record) => record.username,
                None => ctx.roblox.user_by_id(id).await?.name,
            },
        };
        return Ok((Some(id), name));
    }
    if !is_valid_roblox_username(&username) {
        return Err(AppError::validation(
            "Enter a valid Roblox username (3-20 letters, numbers or underscores) or a numeric user ID.",
        ));
    }

    if let Some(player) = live_players(ctx)
        .into_iter()
        .find(|p| p.username.eq_ignore_ascii_case(&username))
    {
        return Ok((player.roblox_user_id, player.username));
    }
    if let Some(record) = ctx.db.player_by_username(&username)? {
        return Ok((Some(record.roblox_user_id), record.username));
    }
    let resolved = ctx.roblox.resolve(&username).await?;
    Ok((Some(resolved.id), resolved.name))
}

/// Players stored in the local database (most recently seen first).
pub fn known(
    ctx: &AppContext,
    access: &Access,
    limit: usize,
) -> AppResult<Vec<crate::models::SearchResult>> {
    access.require(Permission::PlayersView)?;
    let online: std::collections::HashMap<u64, PlayerView> = live_players(ctx)
        .into_iter()
        .filter_map(|player| player.roblox_user_id.map(|id| (id, player)))
        .collect();
    let mut results = Vec::new();
    for row in ctx.db.list_players(limit.clamp(1, 500))? {
        let live = online.get(&row.roblox_user_id);
        results.push(SearchResult {
            roblox_user_id: Some(row.roblox_user_id),
            username: row.username,
            online: live.is_some(),
            team: live.map(|p| p.team.clone()).or(row.last_team),
            callsign: live.and_then(|p| p.callsign.clone()).or(row.last_callsign),
            permission: live.map(|p| p.permission.clone()),
            source: "database".into(),
        });
    }
    Ok(results)
}

pub async fn profile(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    username: Option<String>,
) -> AppResult<PlayerProfile> {
    access.require(Permission::PlayersView)?;
    let (resolved_id, resolved_name) = resolve_identity(ctx, roblox_user_id, username.as_deref()).await?;

    let live = live_players(ctx).into_iter().find(|player| {
        player.roblox_user_id == resolved_id
            || player.username.eq_ignore_ascii_case(&resolved_name)
    });

    let record = match resolved_id {
        Some(id) => ctx.db.player_by_roblox_id(id)?,
        None => ctx.db.player_by_username(&resolved_name)?,
    };

    let roblox = match resolved_id {
        Some(id) => ctx.roblox.user_by_id(id).await.ok(),
        None => None,
    };

    let moderation = ctx.db.moderation_for_player(
        resolved_id,
        &resolved_name,
        100,
    )?;
    let notes = if access.can(Permission::PlayersNotesView) {
        ctx.db.list_staff_notes(resolved_id, &resolved_name, 100)?
    } else {
        Vec::new()
    };
    let internal_ban = ctx.db.active_internal_ban(resolved_id, &resolved_name)?;
    let internal_ban_count = ctx.db.internal_ban_count_for(resolved_id, &resolved_name)?;

    let (erlc_ban, erlc_ban_detail) = if access.can(Permission::ModerationView) {
        match ctx.api_client()? {
            Some(client) => match client.fetch_bans().await {
                Ok((bans, _)) => {
                    let banned = bans.iter().any(|ban| {
                        ban.roblox_user_id == resolved_id
                            || ban
                                .username
                                .as_deref()
                                .map(|name| name.eq_ignore_ascii_case(&resolved_name))
                                .unwrap_or(false)
                    });
                    if banned {
                        (true, Some("Listed in the live ER:LC server ban list.".to_string()))
                    } else {
                        (false, Some("Not present in the live ER:LC ban list.".to_string()))
                    }
                }
                Err(err) => (false, Some(format!("Ban list unavailable: {}", err.message()))),
            },
            None => (false, Some("ER:LC server key not configured.".to_string())),
        }
    } else {
        (false, None)
    };

    Ok(PlayerProfile {
        username: resolved_name,
        roblox_user_id: resolved_id,
        roblox,
        live,
        first_seen_at: record.as_ref().map(|r| r.first_seen_at),
        last_seen_at: record.as_ref().map(|r| r.last_seen_at),
        moderation,
        notes,
        internal_ban,
        erlc_ban,
        erlc_ban_detail,
        internal_ban_count,
    })
}

pub async fn avatar(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: u64,
    full_body: bool,
) -> AppResult<crate::models::AvatarView> {
    access.require(Permission::PlayersView)?;
    if roblox_user_id == 0 {
        return Err(AppError::validation("Invalid Roblox user ID."));
    }
    let kind = if full_body {
        AvatarKind::FullBody
    } else {
        AvatarKind::Headshot
    };
    ctx.roblox.avatar(roblox_user_id, kind, false).await
}

/// Resolve an arbitrary username to a Roblox profile without touching CAD data.
pub async fn resolve_roblox(
    ctx: &AppContext,
    access: &Access,
    identifier: &str,
) -> AppResult<RobloxUserView> {
    access.require(Permission::PlayersView)?;
    let trimmed = identifier.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Enter a Roblox username or user ID."));
    }
    ctx.roblox.resolve(trimmed).await
}

pub fn add_note(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    username: &str,
    body: &str,
) -> AppResult<StaffNoteView> {
    access.require(Permission::PlayersNotesManage)?;
    let username = require_text(username, "Username", 1, 20)?;
    let body = require_text(body, "Note", 3, 4000)?;
    let id = ctx.db.add_staff_note(
        roblox_user_id,
        &username,
        Some(access.user_id),
        &access.display_name,
        &body,
    )?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "player",
        action: "player.note_added",
        target: &username,
        details: &body.chars().take(240).collect::<String>(),
    })?;
    ctx.db
        .list_staff_notes(roblox_user_id, &username, 500)?
        .into_iter()
        .find(|note| note.id == id)
        .ok_or_else(|| AppError::internal("The note could not be reloaded."))
}

pub fn notes(
    ctx: &AppContext,
    access: &Access,
    roblox_user_id: Option<u64>,
    username: &str,
) -> AppResult<Vec<StaffNoteView>> {
    access.require(Permission::PlayersNotesView)?;
    ctx.db.list_staff_notes(roblox_user_id, username, 200)
}

pub fn delete_note(ctx: &AppContext, access: &Access, note_id: i64) -> AppResult<()> {
    access.require(Permission::PlayersNotesManage)?;
    ctx.db.delete_staff_note(note_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "player",
        action: "player.note_deleted",
        target: &format!("note #{note_id}"),
        details: "",
    })?;
    Ok(())
}

/// Force a refresh of the live snapshot (used by the Players page).
pub async fn refresh_live(ctx: &AppContext, access: &Access) -> AppResult<()> {
    access.require(Permission::PlayersSearch)?;
    snapshot::load(ctx, true).await?;
    Ok(())
}
