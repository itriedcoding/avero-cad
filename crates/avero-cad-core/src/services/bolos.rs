//! BOLO service: permission checks, audit trail and live matching.
//!
//! Matching is deliberately strict and honest: a person BOLO only matches when
//! the exact username is present in the latest live snapshot, and a vehicle
//! BOLO only matches when a spawned vehicle reports the same plate. When no
//! snapshot is available no match is claimed.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::db::bolos::NewBolo;
use crate::error::AppResult;
use crate::models::{BoloView, Snapshot};
use crate::permissions::{Access, Permission};
use crate::util::require_text;

const LIST_LIMIT: usize = 200;

pub fn list(ctx: &AppContext, access: &Access, status: Option<&str>) -> AppResult<Vec<BoloView>> {
    access.require(Permission::BolosView)?;
    let mut bolos = ctx.db.list_bolos(status, LIST_LIMIT)?;
    if let Some(snapshot) = live_snapshot(ctx) {
        for bolo in &mut bolos {
            bolo.live_match = match_in_snapshot(bolo, &snapshot);
        }
    }
    Ok(bolos)
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    ctx: &AppContext,
    access: &Access,
    kind: &str,
    subject_name: &str,
    plate: &str,
    description: &str,
    last_seen: &str,
    priority: &str,
) -> AppResult<BoloView> {
    access.require(Permission::BolosManage)?;
    let subject = require_text(subject_name, "Subject", 2, 80)?;
    let bolo = ctx.db.create_bolo(
        &NewBolo {
            kind,
            subject_name: &subject,
            plate,
            description,
            last_seen,
            priority,
        },
        Some(access.user_id),
        &access.display_name,
    )?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "bolo",
        action: "bolo.created",
        target: &bolo.subject_name,
        details: &format!("{} · {} priority", bolo.kind, bolo.priority),
    })?;
    Ok(bolo)
}

pub fn clear(ctx: &AppContext, access: &Access, bolo_id: i64, note: &str) -> AppResult<BoloView> {
    access.require(Permission::BolosManage)?;
    let note = require_text(note, "Resolution note", 3, 500)?;
    let bolo = ctx.db.clear_bolo(bolo_id, &access.display_name, &note)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "bolo",
        action: "bolo.cleared",
        target: &bolo.subject_name,
        details: &note,
    })?;
    Ok(bolo)
}

pub fn reopen(ctx: &AppContext, access: &Access, bolo_id: i64) -> AppResult<BoloView> {
    access.require(Permission::BolosManage)?;
    let bolo = ctx.db.reopen_bolo(bolo_id)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "bolo",
        action: "bolo.reopened",
        target: &bolo.subject_name,
        details: "",
    })?;
    Ok(bolo)
}

fn live_snapshot(ctx: &AppContext) -> Option<Snapshot> {
    ctx.cache
        .lock()
        .ok()
        .and_then(|cache| cache.current.as_ref().map(|cached| cached.snapshot.clone()))
}

/// Strip separators and casing so "abc-123" and "ABC 123" compare equal.
pub fn normalize_plate(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub fn match_in_snapshot(bolo: &BoloView, snapshot: &Snapshot) -> Option<String> {
    if bolo.status != "active" {
        return None;
    }
    match bolo.kind.as_str() {
        "person" => {
            let wanted = bolo.subject_name.trim();
            snapshot
                .players
                .iter()
                .find(|player| player.username.eq_ignore_ascii_case(wanted))
                .map(|player| {
                    let mut detail = format!("In server · {}", player.team);
                    if let Some(callsign) = player.callsign.as_ref().filter(|c| !c.is_empty()) {
                        detail.push_str(" · ");
                        detail.push_str(callsign);
                    }
                    if player.wanted_stars > 0 {
                        detail.push_str(&format!(" · {} wanted stars", player.wanted_stars));
                    }
                    detail
                })
        }
        "vehicle" => {
            let plate = normalize_plate(&bolo.plate);
            if plate.is_empty() {
                return None;
            }
            snapshot
                .vehicles
                .iter()
                .find(|vehicle| normalize_plate(&vehicle.plate) == plate)
                .map(|vehicle| {
                    let owner = if vehicle.owner.is_empty() {
                        "owner unknown".to_string()
                    } else {
                        vehicle.owner.clone()
                    };
                    format!("Plate seen · {} · {}", vehicle.name, owner)
                })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ConnectionView, PlayerView, RosterCounts, Snapshot, StaffRoster, VehicleView,
    };

    fn bolo(kind: &str, subject: &str, plate: &str, status: &str) -> BoloView {
        BoloView {
            id: 1,
            kind: kind.into(),
            subject_name: subject.into(),
            plate: plate.into(),
            description: String::new(),
            last_seen: String::new(),
            priority: "high".into(),
            status: status.into(),
            created_by_name: "Owner".into(),
            created_at: 0,
            cleared_by_name: None,
            cleared_at: None,
            clearance_note: None,
            live_match: None,
        }
    }

    fn player(username: &str) -> PlayerView {
        PlayerView {
            roblox_user_id: Some(1),
            username: username.into(),
            team: "Police".into(),
            permission: "Normal".into(),
            callsign: Some("1A-1".into()),
            location: None,
            wanted_stars: 2,
            service: "law_enforcement".into(),
            staff_role: None,
        }
    }

    fn snapshot(players: Vec<PlayerView>, vehicles: Vec<VehicleView>) -> Snapshot {
        Snapshot {
            server: None,
            players,
            staff: StaffRoster::default(),
            vehicles,
            emergency_calls: vec![],
            join_logs: vec![],
            kill_logs: vec![],
            command_logs: vec![],
            mod_calls: vec![],
            queue: vec![],
            counts: RosterCounts::default(),
            connection: ConnectionView {
                state: "connected".into(),
                detail: None,
                api_url: "https://api.erlc.gg".into(),
                latency_ms: Some(20),
                rate_limit: None,
            },
            notifications: vec![],
            history: vec![],
            fetched_at: 0,
        }
    }

    #[test]
    fn matches_a_live_person_by_username() {
        let live = snapshot(vec![player("Troll")], vec![]);
        let match_detail = match_in_snapshot(&bolo("person", "troll", "", "active"), &live);
        assert_eq!(
            match_detail.as_deref(),
            Some("In server · Police · 1A-1 · 2 wanted stars")
        );
    }

    #[test]
    fn does_not_match_absent_or_cleared_subjects() {
        let unrelated = snapshot(vec![player("SomeoneElse")], vec![]);
        assert!(match_in_snapshot(&bolo("person", "Troll", "", "active"), &unrelated).is_none());
        let with_troll = snapshot(vec![player("Troll")], vec![]);
        assert!(match_in_snapshot(&bolo("person", "Troll", "", "cleared"), &with_troll).is_none());
    }

    #[test]
    fn normalizes_plates_before_matching() {
        assert_eq!(normalize_plate("abc-123"), "ABC123");
        assert_eq!(normalize_plate(" ABC 123 "), "ABC123");
        let vehicles = vec![VehicleView {
            name: "Falcon".into(),
            owner: "Builderman".into(),
            plate: "ABC-123".into(),
            texture: None,
            color_hex: "#ffffff".into(),
            color_name: "White".into(),
        }];
        let live = snapshot(vec![], vehicles);
        assert_eq!(
            match_in_snapshot(&bolo("vehicle", "Falcon", "abc 123", "active"), &live).as_deref(),
            Some("Plate seen · Falcon · Builderman")
        );
        assert!(match_in_snapshot(&bolo("vehicle", "Falcon", "", "active"), &live).is_none());
    }
}
