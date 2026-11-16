//! CAD dispatch: calls, units and their associations.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{CallPlayerView, CallView, UnitView};
use crate::util::{clamp_text, now_ms};
use rusqlite::{params, Connection, OptionalExtension};

pub const UNIT_STATUSES: &[&str] = &[
    "available",
    "on_duty",
    "en_route",
    "on_scene",
    "transporting",
    "unavailable",
    "off_duty",
];

pub const CALL_STATUSES: &[&str] = &["open", "assigned", "in_progress", "closed"];
pub const CALL_PRIORITIES: &[&str] = &["low", "medium", "high", "critical"];

pub struct NewCall<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub location: &'a str,
    pub postal: &'a str,
    pub priority: &'a str,
}

fn map_unit(row: &rusqlite::Row<'_>) -> rusqlite::Result<UnitView> {
    Ok(UnitView {
        id: row.get("id")?,
        callsign: row.get("callsign")?,
        label: row.get("label")?,
        team: row.get("team")?,
        status: row.get("status")?,
        roblox_user_id: row
            .get::<_, Option<i64>>("roblox_user_id")?
            .map(|v| v as u64),
        assigned_call_id: row.get("assigned_call_id")?,
        updated_at: row.get("updated_at")?,
    })
}

fn units_for_call(conn: &Connection, call_id: i64) -> AppResult<Vec<UnitView>> {
    let mut stmt = conn.prepare(
        "SELECT id, callsign, label, team, status, roblox_user_id, assigned_call_id, updated_at
         FROM dispatch_units WHERE assigned_call_id = ?1 ORDER BY callsign COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map(params![call_id], map_unit)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn players_for_call(conn: &Connection, call_id: i64) -> AppResult<Vec<CallPlayerView>> {
    let mut stmt = conn.prepare(
        "SELECT roblox_user_id, username FROM call_players WHERE call_id = ?1 ORDER BY username COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map(params![call_id], |row| {
            Ok(CallPlayerView {
                roblox_user_id: row.get::<_, i64>(0)? as u64,
                username: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_call(row: &rusqlite::Row<'_>) -> rusqlite::Result<CallView> {
    Ok(CallView {
        id: row.get("id")?,
        call_number: row.get("call_number")?,
        title: row.get("title")?,
        description: row.get("description")?,
        location: row.get("location")?,
        postal: row.get("postal")?,
        priority: row.get("priority")?,
        status: row.get("status")?,
        source: row.get("source")?,
        erlc_call_number: row.get("erlc_call_number")?,
        created_by_name: row.get("created_by_name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        closed_at: row.get("closed_at")?,
        closed_by_name: row.get("closed_by_name")?,
        units: Vec::new(),
        players: Vec::new(),
    })
}

impl Database {
    pub fn create_call(
        &self,
        new: &NewCall<'_>,
        created_by: Option<i64>,
        created_by_name: &str,
    ) -> AppResult<CallView> {
        let title = clamp_text(new.title, 120);
        if title.is_empty() {
            return Err(AppError::validation("A call title is required."));
        }
        let priority = if CALL_PRIORITIES.contains(&new.priority) {
            new.priority
        } else {
            "medium"
        };
        let description = clamp_text(new.description, 2000);
        let location = clamp_text(new.location, 200);
        let postal = clamp_text(new.postal, 20);
        let now = now_ms();
        let call_id = self.with_tx(|tx| {
            let next: i64 = tx.query_row(
                "SELECT COALESCE(MAX(call_number), 1000) + 1 FROM dispatch_calls",
                [],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO dispatch_calls
                 (call_number, title, description, location, postal, priority, status, source,
                  created_by_user_id, created_by_name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', 'cad', ?7, ?8, ?9, ?9)",
                params![
                    next,
                    title,
                    description,
                    location,
                    postal,
                    priority,
                    created_by,
                    created_by_name,
                    now
                ],
            )?;
            Ok(tx.last_insert_rowid())
        })?;
        self.call_by_id(call_id)
    }

    pub fn call_by_id(&self, id: i64) -> AppResult<CallView> {
        let conn = self.lock()?;
        let mut call = conn
            .query_row(
                "SELECT id, call_number, title, description, location, postal, priority, status,
                        source, erlc_call_number, created_by_name, created_at, updated_at,
                        closed_at, closed_by_name
                 FROM dispatch_calls WHERE id = ?1",
                params![id],
                map_call,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("Call not found."))?;
        call.units = units_for_call(&conn, id)?;
        call.players = players_for_call(&conn, id)?;
        Ok(call)
    }

    pub fn list_calls(&self, status: Option<&str>, limit: usize) -> AppResult<Vec<CallView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, call_number, title, description, location, postal, priority, status,
                    source, erlc_call_number, created_by_name, created_at, updated_at,
                    closed_at, closed_by_name
             FROM dispatch_calls
             WHERE (?1 IS NULL OR status = ?1)
             ORDER BY CASE status WHEN 'open' THEN 0 WHEN 'assigned' THEN 1 WHEN 'in_progress' THEN 2 ELSE 3 END,
                      call_number DESC
             LIMIT ?2",
        )?;
        let mut calls = stmt
            .query_map(params![status, limit as i64], map_call)?
            .collect::<Result<Vec<_>, _>>()?;
        for call in &mut calls {
            call.units = units_for_call(&conn, call.id)?;
            call.players = players_for_call(&conn, call.id)?;
        }
        Ok(calls)
    }

    pub fn update_call_details(
        &self,
        id: i64,
        title: &str,
        description: &str,
        location: &str,
        postal: &str,
        priority: &str,
    ) -> AppResult<CallView> {
        let title = clamp_text(title, 120);
        if title.is_empty() {
            return Err(AppError::validation("A call title is required."));
        }
        let priority = if CALL_PRIORITIES.contains(&priority) {
            priority
        } else {
            "medium"
        };
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE dispatch_calls SET title = ?1, description = ?2, location = ?3, postal = ?4,
                    priority = ?5, updated_at = ?6 WHERE id = ?7",
            params![
                title,
                clamp_text(description, 2000),
                clamp_text(location, 200),
                clamp_text(postal, 20),
                priority,
                now_ms(),
                id
            ],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Call not found."));
        }
        drop(conn);
        self.call_by_id(id)
    }

    pub fn set_call_status(
        &self,
        id: i64,
        status: &str,
        closed_by_name: Option<&str>,
    ) -> AppResult<CallView> {
        if !CALL_STATUSES.contains(&status) {
            return Err(AppError::validation("Unknown call status."));
        }
        let now = now_ms();
        self.with_tx(|tx| {
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM dispatch_calls WHERE id = ?1",
                    params![id],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            if !exists {
                return Err(AppError::not_found("Call not found."));
            }
            if status == "closed" {
                tx.execute(
                    "UPDATE dispatch_calls SET status = 'closed', closed_at = ?1, closed_by_name = ?2,
                            updated_at = ?1 WHERE id = ?3",
                    params![now, closed_by_name.unwrap_or("Unknown"), id],
                )?;
                tx.execute(
                    "UPDATE dispatch_units SET assigned_call_id = NULL,
                            status = CASE WHEN status IN ('en_route', 'on_scene', 'transporting')
                                          THEN 'available' ELSE status END,
                            updated_at = ?1
                     WHERE assigned_call_id = ?2",
                    params![now, id],
                )?;
            } else {
                tx.execute(
                    "UPDATE dispatch_calls SET status = ?1, closed_at = NULL, closed_by_name = NULL,
                            updated_at = ?2 WHERE id = ?3",
                    params![status, now, id],
                )?;
            }
            Ok(())
        })?;
        self.call_by_id(id)
    }

    pub fn add_call_player(
        &self,
        call_id: i64,
        roblox_user_id: u64,
        username: &str,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR IGNORE INTO call_players (call_id, roblox_user_id, username) VALUES (?1, ?2, ?3)",
            params![call_id, roblox_user_id as i64, username.trim()],
        )?;
        Ok(())
    }

    pub fn remove_call_player(&self, call_id: i64, roblox_user_id: u64) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM call_players WHERE call_id = ?1 AND roblox_user_id = ?2",
            params![call_id, roblox_user_id as i64],
        )?;
        Ok(())
    }

    pub fn create_unit(
        &self,
        callsign: &str,
        label: &str,
        team: &str,
        status: &str,
        roblox_user_id: Option<u64>,
        created_by: Option<i64>,
    ) -> AppResult<UnitView> {
        let callsign = callsign.trim().to_uppercase();
        if callsign.is_empty() || callsign.chars().count() > 12 {
            return Err(AppError::validation(
                "A callsign of 1-12 characters is required.",
            ));
        }
        if !UNIT_STATUSES.contains(&status) {
            return Err(AppError::validation("Unknown unit status."));
        }
        let now = now_ms();
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO dispatch_units
             (callsign, label, team, status, roblox_user_id, created_by_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                callsign,
                clamp_text(label, 80),
                clamp_text(team, 60),
                status,
                roblox_user_id.map(|id| id as i64),
                created_by,
                now
            ],
        )
        .map_err(|err| match err {
            rusqlite::Error::SqliteFailure(e, _)
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                AppError::conflict("A unit with that callsign already exists.")
            }
            other => other.into(),
        })?;
        let id = conn.last_insert_rowid();
        let unit = conn.query_row(
            "SELECT id, callsign, label, team, status, roblox_user_id, assigned_call_id, updated_at
             FROM dispatch_units WHERE id = ?1",
            params![id],
            map_unit,
        )?;
        Ok(unit)
    }

    pub fn list_units(&self) -> AppResult<Vec<UnitView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, callsign, label, team, status, roblox_user_id, assigned_call_id, updated_at
             FROM dispatch_units
             ORDER BY CASE status WHEN 'available' THEN 0 WHEN 'on_duty' THEN 1 WHEN 'en_route' THEN 2
                                  WHEN 'on_scene' THEN 3 WHEN 'transporting' THEN 4 ELSE 5 END,
                      callsign COLLATE NOCASE",
        )?;
        let units = stmt
            .query_map([], map_unit)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(units)
    }

    pub fn update_unit(
        &self,
        unit_id: i64,
        label: &str,
        team: &str,
        status: &str,
    ) -> AppResult<UnitView> {
        if !UNIT_STATUSES.contains(&status) {
            return Err(AppError::validation("Unknown unit status."));
        }
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE dispatch_units SET label = ?1, team = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
            params![clamp_text(label, 80), clamp_text(team, 60), status, now_ms(), unit_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Unit not found."));
        }
        let unit = conn.query_row(
            "SELECT id, callsign, label, team, status, roblox_user_id, assigned_call_id, updated_at
             FROM dispatch_units WHERE id = ?1",
            params![unit_id],
            map_unit,
        )?;
        Ok(unit)
    }

    pub fn assign_unit(&self, call_id: i64, unit_id: i64) -> AppResult<()> {
        self.with_tx(|tx| {
            let call_status: String = tx
                .query_row(
                    "SELECT status FROM dispatch_calls WHERE id = ?1",
                    params![call_id],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| AppError::not_found("Call not found."))?;
            if call_status == "closed" {
                return Err(AppError::conflict("Closed calls cannot receive units."));
            }
            let now = now_ms();
            let changed = tx.execute(
                "UPDATE dispatch_units SET assigned_call_id = ?1,
                        status = CASE WHEN status = 'available' THEN 'en_route' ELSE status END,
                        updated_at = ?2
                 WHERE id = ?3",
                params![call_id, now, unit_id],
            )?;
            if changed == 0 {
                return Err(AppError::not_found("Unit not found."));
            }
            tx.execute(
                "INSERT OR IGNORE INTO call_units (call_id, unit_id, assigned_at) VALUES (?1, ?2, ?3)",
                params![call_id, unit_id, now],
            )?;
            if call_status == "open" {
                tx.execute(
                    "UPDATE dispatch_calls SET status = 'assigned', updated_at = ?1 WHERE id = ?2",
                    params![now, call_id],
                )?;
            } else {
                tx.execute(
                    "UPDATE dispatch_calls SET updated_at = ?1 WHERE id = ?2",
                    params![now, call_id],
                )?;
            }
            Ok(())
        })
    }

    pub fn unassign_unit(&self, unit_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE dispatch_units SET assigned_call_id = NULL, updated_at = ?1 WHERE id = ?2",
            params![now_ms(), unit_id],
        )?;
        conn.execute(
            "DELETE FROM call_units WHERE unit_id = ?1",
            params![unit_id],
        )?;
        Ok(())
    }

    pub fn delete_unit(&self, unit_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute("DELETE FROM dispatch_units WHERE id = ?1", params![unit_id])?;
        if changed == 0 {
            return Err(AppError::not_found("Unit not found."));
        }
        Ok(())
    }

    pub fn count_units(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM dispatch_units", [], |row| row.get(0))?)
    }

    pub fn count_calls(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM dispatch_calls", [], |row| row.get(0))?)
    }

    /// Mirror a live ER:LC emergency call into CAD exactly once.
    pub fn mirror_emergency_call(
        &self,
        erlc_call_number: i64,
        title: &str,
        description: &str,
        location: &str,
    ) -> AppResult<bool> {
        let now = now_ms();
        let conn = self.lock()?;
        let next: i64 = conn.query_row(
            "SELECT COALESCE(MAX(call_number), 1000) + 1 FROM dispatch_calls",
            [],
            |row| row.get(0),
        )?;
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO dispatch_calls
             (call_number, title, description, location, postal, priority, status, source,
              erlc_call_number, created_by_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '', 'high', 'open', 'erlc', ?5, 'ER:LC', ?6, ?6)",
            params![next, title, description, location, erlc_call_number, now],
        )?;
        Ok(inserted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_call<'a>(title: &'a str) -> NewCall<'a> {
        NewCall {
            title,
            description: "",
            location: "",
            postal: "",
            priority: "medium",
        }
    }

    #[test]
    fn assigns_sequential_call_numbers_starting_at_1001() {
        let db = Database::open_in_memory().unwrap();
        let first = db
            .create_call(&new_call("Structure fire"), None, "Owner")
            .unwrap();
        let second = db
            .create_call(&new_call("Traffic stop"), None, "Owner")
            .unwrap();
        assert_eq!(first.call_number, 1001);
        assert_eq!(second.call_number, 1002);
        assert_eq!(first.status, "open");
    }

    #[test]
    fn closing_a_call_releases_units() {
        let db = Database::open_in_memory().unwrap();
        let call = db
            .create_call(&new_call("Collision"), None, "Owner")
            .unwrap();
        let unit = db
            .create_unit("1A-1", "Patrol", "Police", "available", None, None)
            .unwrap();
        db.assign_unit(call.id, unit.id).unwrap();
        let updated = db.call_by_id(call.id).unwrap();
        assert_eq!(updated.status, "assigned");
        assert_eq!(updated.units.len(), 1);
        assert_eq!(updated.units[0].status, "en_route");

        let closed = db
            .set_call_status(call.id, "closed", Some("Owner"))
            .unwrap();
        assert_eq!(closed.status, "closed");
        assert!(closed.closed_at.is_some());
        let unit = db.list_units().unwrap().into_iter().next().unwrap();
        assert!(unit.assigned_call_id.is_none());
        assert_eq!(unit.status, "available");
    }

    #[test]
    fn unit_callsigns_are_unique() {
        let db = Database::open_in_memory().unwrap();
        db.create_unit("2B-2", "", "Fire", "available", None, None)
            .unwrap();
        let err = db
            .create_unit("2b-2", "", "Fire", "available", None, None)
            .unwrap_err();
        assert_eq!(err.code(), "conflict");
    }

    #[test]
    fn validates_statuses() {
        let db = Database::open_in_memory().unwrap();
        let call = db.create_call(&new_call("Test"), None, "Owner").unwrap();
        assert!(db.set_call_status(call.id, "banana", None).is_err());
        assert!(db
            .create_unit("9Z-9", "", "Police", "banana", None, None)
            .is_err());
    }

    #[test]
    fn mirrors_erlc_calls_once() {
        let db = Database::open_in_memory().unwrap();
        assert!(db
            .mirror_emergency_call(12, "Robbery", "Store robbery", "302 Park Ave")
            .unwrap());
        assert!(!db
            .mirror_emergency_call(12, "Robbery", "Store robbery", "302 Park Ave")
            .unwrap());
        assert_eq!(db.count_calls().unwrap(), 1);
    }
}
