//! Live server snapshot service.
//!
//! A single `/v2/server` request fetches every optional block the API supports
//! (players, staff, vehicles, logs, queue, emergency calls). Results are cached
//! for the configured refresh interval so multiple pages polling the dashboard
//! never multiply API traffic.

use super::AppContext;
use crate::api::ServerQuery;
use crate::db::audit::AuditEntry;
use crate::error::AppResult;
use crate::models::{
    service_bucket, split_player, AppNotification, ConnectionView, EmergencyCallView, JoinLogView,
    KillLogView, MetricPoint, PlayerLocationView, PlayerView, QueueEntryView, RosterCounts,
    ServerInfo, ServerSummary, Snapshot, StaffMember, StaffRoster, VehicleView,
};
use crate::util::now_ms;

const MAX_NOTIFICATIONS: usize = 60;
const MAX_SEEN: usize = 400;
const METRIC_INTERVAL_MS: i64 = 60_000;

#[derive(Debug, Clone)]
pub struct CachedSnapshot {
    pub snapshot: Snapshot,
    pub player_count: i64,
    pub max_players: i64,
    pub staff_count: i64,
}

fn build_staff_roster(staff: Option<crate::models::ApiStaff>) -> (StaffRoster, usize) {
    let mut roster = StaffRoster::default();
    let Some(staff) = staff else {
        return (roster, 0);
    };
    let push = |role: &str, map: std::collections::BTreeMap<String, String>| {
        map.into_iter()
            .filter_map(|(id, username)| {
                id.parse::<u64>().ok().map(|roblox_user_id| StaffMember {
                    roblox_user_id,
                    username,
                    role: role.to_string(),
                })
            })
            .collect::<Vec<_>>()
    };
    roster.admins = push("Administrator", staff.admins);
    roster.mods = push("Moderator", staff.mods);
    roster.helpers = push("Helper", staff.helpers);
    let count = roster.admins.len() + roster.mods.len() + roster.helpers.len();
    (roster, count)
}

fn build_players(ctx: &AppContext, info: &ServerInfo) -> Vec<PlayerView> {
    let Some(players) = info.players.as_ref() else {
        return Vec::new();
    };
    let now = now_ms();
    let mut views = Vec::with_capacity(players.len());
    for player in players {
        let (username, roblox_user_id) = split_player(&player.player);
        let permission = player.permission.clone().unwrap_or_else(|| "Normal".into());
        let staff_role = match permission.as_str() {
            "Server Owner" | "Server Administrator" | "Server Moderator" => Some(permission.clone()),
            _ => None,
        };
        if let Some(id) = roblox_user_id {
            let _ = ctx.db.upsert_player_presence(
                id,
                &username,
                Some(&player.team),
                player.callsign.as_deref(),
                now,
            );
        }
        views.push(PlayerView {
            roblox_user_id,
            username,
            team: player.team.clone(),
            permission,
            callsign: player.callsign.clone().filter(|c| !c.trim().is_empty()),
            location: player.location.as_ref().map(|l| PlayerLocationView {
                x: l.location_x,
                z: l.location_z,
                postal: l.postal_code.clone(),
                street: l.street_name.clone(),
                building: l.building_number.clone(),
            }),
            wanted_stars: player.wanted_stars.unwrap_or(0),
            service: service_bucket(&player.team).to_string(),
            staff_role,
        });
    }
    views.sort_by(|a, b| {
        a.username
            .to_ascii_lowercase()
            .cmp(&b.username.to_ascii_lowercase())
    });
    views
}

fn build_emergency_calls(info: &ServerInfo) -> Vec<EmergencyCallView> {
    info.emergency_calls
        .as_ref()
        .map(|calls| {
            calls
                .iter()
                .map(|call| EmergencyCallView {
                    call_number: call.call_number,
                    team: call.team.clone(),
                    caller_id: call.caller,
                    player_ids: call.players.clone(),
                    description: call.description.clone(),
                    position_descriptor: call.position_descriptor.clone(),
                    x: call.position.first().copied(),
                    z: call.position.get(1).copied(),
                    started_at: call.started_at,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_snapshot(
    ctx: &AppContext,
    info: &ServerInfo,
    latency_ms: u64,
    rate_limit: crate::models::RateLimitView,
    api_url: &str,
    notifications: Vec<AppNotification>,
) -> CachedSnapshot {
    let players = build_players(ctx, info);
    let (staff, staff_count) = build_staff_roster(info.staff.clone());

    let counts = RosterCounts {
        total: players.len(),
        law_enforcement: players.iter().filter(|p| p.service == "law_enforcement").count(),
        fire_ems: players.iter().filter(|p| p.service == "fire_ems").count(),
        civilian: players.iter().filter(|p| p.service == "civilian").count(),
        staff: staff_count,
        wanted: players.iter().filter(|p| p.wanted_stars > 0).count(),
    };

    let vehicles: Vec<VehicleView> = info
        .vehicles
        .as_ref()
        .map(|vehicles| {
            vehicles
                .iter()
                .map(|v| VehicleView {
                    name: v.name.clone(),
                    owner: v.owner.clone(),
                    plate: v.plate.clone(),
                    texture: v.texture.clone(),
                    color_hex: v.color_hex.clone(),
                    color_name: v.color_name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let join_logs: Vec<JoinLogView> = info
        .join_logs
        .as_ref()
        .map(|logs| {
            let mut entries: Vec<JoinLogView> = logs
                .iter()
                .map(|log| {
                    let (username, roblox_user_id) = split_player(&log.player);
                    JoinLogView {
                        roblox_user_id,
                        username,
                        joined: log.join,
                        timestamp: log.timestamp,
                    }
                })
                .collect();
            entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp));
            entries
        })
        .unwrap_or_default();

    let kill_logs: Vec<KillLogView> = info
        .kill_logs
        .as_ref()
        .map(|logs| {
            let mut entries: Vec<KillLogView> = logs
                .iter()
                .map(|log| KillLogView {
                    killer: split_player(&log.killer).0,
                    killed: split_player(&log.killed).0,
                    timestamp: log.timestamp,
                })
                .collect();
            entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp));
            entries
        })
        .unwrap_or_default();

    let command_logs: Vec<crate::models::CommandLogView> = info
        .command_logs
        .as_ref()
        .map(|logs| {
            let mut entries: Vec<crate::models::CommandLogView> = logs
                .iter()
                .map(|log| crate::models::CommandLogView {
                    player: split_player(&log.player).0,
                    command: log.command.clone(),
                    timestamp: log.timestamp,
                })
                .collect();
            entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp));
            entries
        })
        .unwrap_or_default();

    let mod_calls: Vec<crate::models::ModCallView> = info
        .mod_calls
        .as_ref()
        .map(|calls| {
            let mut entries: Vec<crate::models::ModCallView> = calls
                .iter()
                .map(|call| crate::models::ModCallView {
                    caller: split_player(&call.caller).0,
                    moderator: call
                        .moderator
                        .as_deref()
                        .filter(|m| !m.trim().is_empty())
                        .map(|m| split_player(m).0),
                    timestamp: call.timestamp,
                })
                .collect();
            entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp));
            entries
        })
        .unwrap_or_default();

    let queue: Vec<QueueEntryView> = info
        .queue
        .as_ref()
        .map(|ids| {
            ids.iter()
                .enumerate()
                .map(|(index, id)| QueueEntryView {
                    roblox_user_id: *id,
                    position: index + 1,
                })
                .collect()
        })
        .unwrap_or_default();

    let history: Vec<MetricPoint> = ctx.db.recent_metrics(60).unwrap_or_default();

    let connection = ConnectionView {
        state: "connected".into(),
        detail: None,
        api_url: api_url.to_string(),
        latency_ms: Some(latency_ms),
        rate_limit: Some(rate_limit),
    };

    CachedSnapshot {
        snapshot: Snapshot {
            server: Some(ServerSummary::from(info)),
            players,
            staff,
            vehicles,
            emergency_calls: build_emergency_calls(info),
            join_logs,
            kill_logs,
            command_logs,
            mod_calls,
            queue,
            counts,
            connection,
            notifications,
            history,
            fetched_at: now_ms(),
        },
        player_count: info.current_players,
        max_players: info.max_players,
        staff_count: staff_count as i64,
    }
}

fn empty_snapshot(
    ctx: &AppContext,
    state: &str,
    detail: Option<String>,
    api_url: &str,
) -> Snapshot {
    Snapshot {
        server: None,
        players: Vec::new(),
        staff: StaffRoster::default(),
        vehicles: Vec::new(),
        emergency_calls: Vec::new(),
        join_logs: Vec::new(),
        kill_logs: Vec::new(),
        command_logs: Vec::new(),
        mod_calls: Vec::new(),
        queue: Vec::new(),
        counts: RosterCounts::default(),
        connection: ConnectionView {
            state: state.to_string(),
            detail,
            api_url: api_url.to_string(),
            latency_ms: None,
            rate_limit: None,
        },
        notifications: Vec::new(),
        history: ctx.db.recent_metrics(60).unwrap_or_default(),
        fetched_at: 0,
    }
}

/// Load the live snapshot, using a fresh cache when possible.
pub async fn load(ctx: &AppContext, force: bool) -> AppResult<Snapshot> {
    let settings = ctx.db.settings_view()?;
    let ttl_ms = settings.refresh_interval_seconds as i64 * 1000;

    if !force {
        if let Ok(cache) = ctx.cache.lock() {
            if let Some(cached) = cache.current.as_ref() {
                if now_ms() - cached.snapshot.fetched_at < ttl_ms {
                    return Ok(cached.snapshot.clone());
                }
            }
        }
    }

    let config = ctx.db.api_config()?;
    let Some(client) = ctx.api_client()? else {
        ctx.db.record_audit(&AuditEntry {
            actor_user_id: None,
            actor_name: "system",
            category: "api",
            action: "api.missing_key",
            target: &config.base_url,
            details: "Snapshot requested without a configured server key.",
        })?;
        return Ok(empty_snapshot(
            ctx,
            "not_configured",
            Some(
                "Add your ER:LC server key in API Configuration to load live server data."
                    .to_string(),
            ),
            &config.base_url,
        ));
    };

    let fetched = client.fetch_server(ServerQuery::everything()).await;

    match fetched {
        Ok((info, meta)) => {
            let mut cache = ctx
                .cache
                .lock()
                .map_err(|_| crate::error::AppError::internal("Snapshot cache unavailable"))?;
            cache.last_attempt_at = now_ms();
            cache.last_error = None;

            // Notifications for new events, derived by comparing against the
            // previous poll. Only real API events are ever surfaced.
            let mut new_notifications = Vec::new();
            for call in info.emergency_calls.as_ref().map(|c| c.as_slice()).unwrap_or(&[]) {
                if !cache.seen_emergency.contains(&call.call_number) {
                    new_notifications.push(AppNotification {
                        id: format!("emergency-{}-{}", call.call_number, call.started_at),
                        kind: "emergency".into(),
                        title: format!("Emergency call #{}", call.call_number),
                        detail: format!("{} — {}", call.description, call.position_descriptor),
                        at: call.started_at.saturating_mul(1000),
                    });
                    cache.seen_emergency.push(call.call_number);
                }
            }
            for call in info.mod_calls.as_ref().map(|c| c.as_slice()).unwrap_or(&[]) {
                if !cache.seen_mod_calls.contains(&call.timestamp) {
                    new_notifications.push(AppNotification {
                        id: format!("modcall-{}", call.timestamp),
                        kind: "modcall".into(),
                        title: "Moderator call".into(),
                        detail: format!("{} requested a moderator.", split_player(&call.caller).0),
                        at: call.timestamp.saturating_mul(1000),
                    });
                    cache.seen_mod_calls.push(call.timestamp);
                }
            }
            if cache.seen_emergency.len() > MAX_SEEN {
                let drain = cache.seen_emergency.len() - MAX_SEEN;
                cache.seen_emergency.drain(0..drain);
            }
            if cache.seen_mod_calls.len() > MAX_SEEN {
                let drain = cache.seen_mod_calls.len() - MAX_SEEN;
                cache.seen_mod_calls.drain(0..drain);
            }
            new_notifications.truncate(MAX_NOTIFICATIONS);

            if !new_notifications.is_empty() {
                cached_append_notifications(&mut cache, &new_notifications);
            }

            // Mirror live ER:LC emergency calls into CAD dispatch once each.
            for call in info.emergency_calls.as_ref().map(|c| c.as_slice()).unwrap_or(&[]) {
                if !cache.mirrored_calls.contains(&call.call_number) {
                    let location = if call.position_descriptor.trim().is_empty() {
                        "Location unavailable".to_string()
                    } else {
                        call.position_descriptor.clone()
                    };
                    if ctx
                        .db
                        .mirror_emergency_call(
                            call.call_number,
                            &format!("{} call", call.team),
                            &call.description,
                            &location,
                        )
                        .unwrap_or(false)
                    {
                        cache.mirrored_calls.push(call.call_number);
                    } else {
                        cache.mirrored_calls.push(call.call_number);
                    }
                }
            }

            if now_ms() - cache.last_metric_at >= METRIC_INTERVAL_MS {
                let _ = ctx.db.insert_metric(
                    now_ms(),
                    info.current_players,
                    info.max_players,
                    build_staff_roster(info.staff.clone()).1 as i64,
                    Some(meta.latency_ms as i64),
                );
                let _ = ctx.db.prune_metrics(720);
                cache.last_metric_at = now_ms();
            }

            let notifications = cache
                .current
                .as_ref()
                .map(|c| c.snapshot.notifications.clone())
                .unwrap_or_default();
            let built = build_snapshot(
                ctx,
                &info,
                meta.latency_ms,
                meta.rate_limit,
                &config.base_url,
                notifications,
            );
            cache.current = Some(built.clone());
            drop(cache);

            // Keep API verification metadata current without extra requests.
            let _ = ctx.db.mark_api_verified(&info.name, Some(info.owner_id as i64));
            Ok(built.snapshot)
        }
        Err(err) => {
            let message = err.message();
            let mut cache = ctx
                .cache
                .lock()
                .map_err(|_| crate::error::AppError::internal("Snapshot cache unavailable"))?;
            cache.last_attempt_at = now_ms();
            cache.last_error = Some(message.clone());
            let mut snapshot = match cache.current.as_ref() {
                Some(cached) => cached.snapshot.clone(),
                None => empty_snapshot(ctx, "error", Some(message.clone()), &config.base_url),
            };
            snapshot.connection.state = "error".into();
            snapshot.connection.detail = Some(message);
            Ok(snapshot)
        }
    }
}

fn cached_append_notifications(cache: &mut super::SnapshotCache, entries: &[AppNotification]) {
    let existing = cache
        .current
        .as_ref()
        .map(|c| c.snapshot.notifications.clone())
        .unwrap_or_default();
    let mut merged = existing;
    merged.extend_from_slice(entries);
    // Newest first, bounded.
    merged.sort_by_key(|entry| std::cmp::Reverse(entry.at));
    merged.truncate(MAX_NOTIFICATIONS);
    cache.notifications = merged;
}

/// Reset cached live data (for example after restoring a database or changing keys).
pub fn clear(ctx: &AppContext) {
    if let Ok(mut cache) = ctx.cache.lock() {
        *cache = super::SnapshotCache::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tests::test_context;

    #[tokio::test]
    async fn reports_not_configured_without_a_key() {
        let (ctx, _guard) = test_context();
        let snapshot = load(&ctx, true).await.unwrap();
        assert_eq!(snapshot.connection.state, "not_configured");
        assert!(snapshot.server.is_none());
        assert!(snapshot.players.is_empty());
    }

    #[tokio::test]
    async fn caches_snapshots_between_polls() {
        let (ctx, _guard) = test_context();
        let first = load(&ctx, false).await.unwrap();
        let second = load(&ctx, false).await.unwrap();
        assert_eq!(first.fetched_at, second.fetched_at);
    }
}
