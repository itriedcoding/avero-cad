//! BOLO (be on the lookout) reports.
//!
//! A BOLO is purely a CAD record: it never changes the ER:LC server. Active
//! reports are matched against the latest live snapshot by the service layer.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::BoloView;
use crate::util::{clamp_text, now_ms};
use rusqlite::{params, OptionalExtension};

pub const BOLO_KINDS: &[&str] = &["person", "vehicle"];
pub const BOLO_PRIORITIES: &[&str] = &["low", "medium", "high", "critical"];
pub const BOLO_STATUSES: &[&str] = &["active", "cleared"];

pub struct NewBolo<'a> {
    pub kind: &'a str,
    pub subject_name: &'a str,
    pub plate: &'a str,
    pub description: &'a str,
    pub last_seen: &'a str,
    pub priority: &'a str,
}

fn map_bolo(row: &rusqlite::Row<'_>) -> rusqlite::Result<BoloView> {
    Ok(BoloView {
        id: row.get("id")?,
        kind: row.get("kind")?,
        subject_name: row.get("subject_name")?,
        plate: row.get("plate")?,
        description: row.get("description")?,
        last_seen: row.get("last_seen")?,
        priority: row.get("priority")?,
        status: row.get("status")?,
        created_by_name: row.get("created_by_name")?,
        created_at: row.get("created_at")?,
        cleared_by_name: row.get("cleared_by_name")?,
        cleared_at: row.get("cleared_at")?,
        clearance_note: row.get("clearance_note")?,
        live_match: None,
    })
}

const SELECT_BOLO: &str = "SELECT id, kind, subject_name, plate, description, last_seen, priority,
                                  status, created_by_name, created_at, cleared_by_name, cleared_at,
                                  clearance_note
                           FROM bolo_reports";

impl Database {
    pub fn create_bolo(
        &self,
        new: &NewBolo<'_>,
        created_by: Option<i64>,
        created_by_name: &str,
    ) -> AppResult<BoloView> {
        if !BOLO_KINDS.contains(&new.kind) {
            return Err(AppError::validation(
                "Choose whether the BOLO is for a person or a vehicle.",
            ));
        }
        if !BOLO_PRIORITIES.contains(&new.priority) {
            return Err(AppError::validation("Choose a valid BOLO priority."));
        }
        let subject_name = clamp_text(new.subject_name, 80);
        if subject_name.is_empty() {
            return Err(AppError::validation(
                "A subject name or description is required.",
            ));
        }
        let plate = clamp_text(new.plate, 12).to_uppercase();
        if new.kind == "vehicle" && plate.chars().filter(|c| c.is_ascii_alphanumeric()).count() < 2
        {
            return Err(AppError::validation(
                "A vehicle BOLO needs a plate of at least two characters.",
            ));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO bolo_reports
             (kind, subject_name, plate, description, last_seen, priority, status,
              created_by_user_id, created_by_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9)",
            params![
                new.kind,
                subject_name,
                plate,
                clamp_text(new.description, 2000),
                clamp_text(new.last_seen, 200),
                new.priority,
                created_by,
                created_by_name,
                now_ms()
            ],
        )?;
        let id = conn.last_insert_rowid();
        let bolo = conn.query_row(
            &format!("{SELECT_BOLO} WHERE id = ?1"),
            params![id],
            map_bolo,
        )?;
        Ok(bolo)
    }

    pub fn list_bolos(&self, status: Option<&str>, limit: usize) -> AppResult<Vec<BoloView>> {
        if let Some(status) = status {
            if !BOLO_STATUSES.contains(&status) {
                return Err(AppError::validation("Unknown BOLO status."));
            }
        }
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&format!(
            "{SELECT_BOLO}
             WHERE (?1 IS NULL OR status = ?1)
             ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END,
                      CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1
                                    WHEN 'medium' THEN 2 ELSE 3 END,
                      created_at DESC
             LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![status, limit as i64], map_bolo)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn bolo_by_id(&self, id: i64) -> AppResult<BoloView> {
        let conn = self.lock()?;
        conn.query_row(
            &format!("{SELECT_BOLO} WHERE id = ?1"),
            params![id],
            map_bolo,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("BOLO report not found."))
    }

    pub fn clear_bolo(&self, id: i64, cleared_by_name: &str, note: &str) -> AppResult<BoloView> {
        let note = clamp_text(note, 500);
        if note.trim().is_empty() {
            return Err(AppError::validation(
                "Record how the BOLO was resolved before clearing it.",
            ));
        }
        let conn = self.lock()?;
        let status: Option<String> = conn
            .query_row(
                "SELECT status FROM bolo_reports WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        let status = status.ok_or_else(|| AppError::not_found("BOLO report not found."))?;
        if status != "active" {
            return Err(AppError::conflict("That BOLO is already cleared."));
        }
        conn.execute(
            "UPDATE bolo_reports SET status = 'cleared', cleared_by_name = ?1, cleared_at = ?2,
                    clearance_note = ?3 WHERE id = ?4",
            params![cleared_by_name, now_ms(), note, id],
        )?;
        drop(conn);
        self.bolo_by_id(id)
    }

    pub fn reopen_bolo(&self, id: i64) -> AppResult<BoloView> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE bolo_reports SET status = 'active', cleared_by_name = NULL, cleared_at = NULL,
                    clearance_note = NULL WHERE id = ?1 AND status = 'cleared'",
            params![id],
        )?;
        if changed == 0 {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM bolo_reports WHERE id = ?1",
                    params![id],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            if !exists {
                return Err(AppError::not_found("BOLO report not found."));
            }
            return Err(AppError::conflict("That BOLO is already active."));
        }
        drop(conn);
        self.bolo_by_id(id)
    }

    pub fn count_bolos(&self, status: &str) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM bolo_reports WHERE status = ?1",
            params![status],
            |row| row.get(0),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_bolo<'a>(kind: &'a str, subject: &'a str) -> NewBolo<'a> {
        NewBolo {
            kind,
            subject_name: subject,
            plate: "",
            description: "",
            last_seen: "",
            priority: "medium",
        }
    }

    #[test]
    fn creates_and_lists_active_bolos_first() {
        let db = Database::open_in_memory().unwrap();
        let person = db
            .create_bolo(&new_bolo("person", "Troll"), None, "Owner")
            .unwrap();
        let vehicle = db
            .create_bolo(
                &NewBolo {
                    kind: "vehicle",
                    subject_name: "Falcon",
                    plate: "abc 123",
                    description: "Spotted fleeing",
                    last_seen: "Route 1",
                    priority: "high",
                },
                None,
                "Owner",
            )
            .unwrap();
        assert_eq!(vehicle.plate, "ABC 123");
        db.clear_bolo(person.id, "Owner", "Detained").unwrap();
        let active = db.list_bolos(Some("active"), 50).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, vehicle.id);
        assert_eq!(db.count_bolos("cleared").unwrap(), 1);
    }

    #[test]
    fn vehicle_bolos_need_a_plate() {
        let db = Database::open_in_memory().unwrap();
        let err = db
            .create_bolo(&new_bolo("vehicle", "Falcon"), None, "Owner")
            .unwrap_err();
        assert_eq!(err.code(), "validation");
    }

    #[test]
    fn clearing_requires_a_note_and_only_happens_once() {
        let db = Database::open_in_memory().unwrap();
        let bolo = db
            .create_bolo(&new_bolo("person", "Suspect"), None, "Owner")
            .unwrap();
        assert!(db.clear_bolo(bolo.id, "Owner", "   ").is_err());
        let cleared = db.clear_bolo(bolo.id, "Owner", "Arrested").unwrap();
        assert_eq!(cleared.status, "cleared");
        assert_eq!(cleared.clearance_note.as_deref(), Some("Arrested"));
        assert!(cleared.cleared_at.is_some());
        assert_eq!(
            db.clear_bolo(bolo.id, "Owner", "again").unwrap_err().code(),
            "conflict"
        );
        let reopened = db.reopen_bolo(bolo.id).unwrap();
        assert_eq!(reopened.status, "active");
        assert!(reopened.cleared_at.is_none());
    }

    #[test]
    fn rejects_unknown_kinds_and_priorities() {
        let db = Database::open_in_memory().unwrap();
        assert!(db
            .create_bolo(&new_bolo("animal", "Dog"), None, "Owner")
            .is_err());
        let mut priority = new_bolo("person", "Someone");
        priority.priority = "urgent";
        assert!(db.create_bolo(&priority, None, "Owner").is_err());
    }

    #[test]
    fn filters_by_status() {
        let db = Database::open_in_memory().unwrap();
        let bolo = db
            .create_bolo(&new_bolo("person", "A"), None, "Owner")
            .unwrap();
        db.clear_bolo(bolo.id, "Owner", "Resolved").unwrap();
        assert_eq!(db.list_bolos(None, 50).unwrap().len(), 1);
        assert!(db.list_bolos(Some("active"), 50).unwrap().is_empty());
        assert_eq!(db.list_bolos(Some("cleared"), 50).unwrap().len(), 1);
        assert!(db.list_bolos(Some("nonsense"), 50).is_err());
    }
}
