//! Moderation history and internal (CAD-only) bans.
//!
//! Internal bans are deliberately stored separately from ER:LC server bans:
//! the ER:LC API has no ban-write endpoint, so a CAD ban never changes anything
//! inside the Roblox server.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{InternalBanView, ModerationRecordView};
use crate::util::{clamp_text, now_ms};
use rusqlite::{params, OptionalExtension};

pub struct NewModerationRecord<'a> {
    pub roblox_user_id: Option<u64>,
    pub username: &'a str,
    pub action: &'a str,
    pub source: &'a str,
    pub reason: &'a str,
    pub moderator_user_id: Option<i64>,
    pub moderator_name: &'a str,
    pub success: bool,
    pub command_id: Option<&'a str>,
    pub api_message: Option<&'a str>,
}

fn map_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModerationRecordView> {
    Ok(ModerationRecordView {
        id: row.get("id")?,
        roblox_user_id: row.get::<_, Option<i64>>("roblox_user_id")?.map(|v| v as u64),
        username: row.get("username")?,
        action: row.get("action")?,
        source: row.get("source")?,
        reason: row.get("reason")?,
        moderator_name: row.get("moderator_name")?,
        success: row.get::<_, i64>("success")? == 1,
        api_message: row.get("api_message")?,
        command_id: row.get("command_id")?,
        created_at: row.get("created_at")?,
    })
}

fn map_ban(row: &rusqlite::Row<'_>) -> rusqlite::Result<InternalBanView> {
    Ok(InternalBanView {
        id: row.get("id")?,
        roblox_user_id: row.get::<_, Option<i64>>("roblox_user_id")?.map(|v| v as u64),
        username: row.get("username")?,
        reason: row.get("reason")?,
        created_by_name: row.get("created_by_name")?,
        created_at: row.get("created_at")?,
        active: row.get::<_, i64>("active")? == 1,
        lifted_at: row.get("lifted_at")?,
        lifted_by_name: row.get("lifted_by_name")?,
        lift_reason: row.get("lift_reason")?,
    })
}

impl Database {
    pub fn insert_moderation_record(&self, record: &NewModerationRecord<'_>) -> AppResult<i64> {
        if !["kick", "ban", "unban", "internal_ban", "internal_unban", "note"]
            .contains(&record.action)
        {
            return Err(AppError::validation("Unknown moderation action."));
        }
        if !["erlc_api", "internal_cad"].contains(&record.source) {
            return Err(AppError::validation("Unknown moderation source."));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO moderation_records
             (roblox_user_id, username, action, source, reason, moderator_user_id, moderator_name,
              success, command_id, api_message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.roblox_user_id.map(|id| id as i64),
                record.username.trim(),
                record.action,
                record.source,
                clamp_text(record.reason, 1000),
                record.moderator_user_id,
                record.moderator_name,
                if record.success { 1 } else { 0 },
                record.command_id,
                record.api_message.map(|m| clamp_text(m, 500)),
                now_ms()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn moderation_for_player(
        &self,
        roblox_user_id: Option<u64>,
        username: &str,
        limit: usize,
    ) -> AppResult<Vec<ModerationRecordView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, roblox_user_id, username, action, source, reason, moderator_name,
                    success, api_message, command_id, created_at
             FROM moderation_records
             WHERE (?1 IS NOT NULL AND roblox_user_id = ?1) OR username = ?2 COLLATE NOCASE
             ORDER BY created_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![
                    roblox_user_id.map(|id| id as i64),
                    username.trim(),
                    limit as i64
                ],
                map_record,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn recent_moderation(&self, limit: usize) -> AppResult<Vec<ModerationRecordView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, roblox_user_id, username, action, source, reason, moderator_name,
                    success, api_message, command_id, created_at
             FROM moderation_records ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], map_record)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_moderation_records(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM moderation_records", [], |row| row.get(0))?)
    }

    pub fn create_internal_ban(
        &self,
        roblox_user_id: Option<u64>,
        username: &str,
        reason: &str,
        created_by_user_id: Option<i64>,
        created_by_name: &str,
    ) -> AppResult<i64> {
        let username = clamp_text(username, 20);
        if username.is_empty() {
            return Err(AppError::validation("A username is required."));
        }
        if clamp_text(reason, 1000).is_empty() {
            return Err(AppError::validation(
                "A reason is required for an internal ban.",
            ));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO internal_bans (roblox_user_id, username, reason, created_by_user_id, created_by_name, created_at, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            params![
                roblox_user_id.map(|id| id as i64),
                username,
                clamp_text(reason, 1000),
                created_by_user_id,
                created_by_name,
                now_ms()
            ],
        )
        .map_err(|err| match err {
            rusqlite::Error::SqliteFailure(e, _)
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                AppError::conflict("That player already has an active internal ban.")
            }
            other => other.into(),
        })?;
        Ok(conn.last_insert_rowid())
    }

    pub fn lift_internal_ban(
        &self,
        ban_id: i64,
        lifted_by_name: &str,
        lift_reason: &str,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE internal_bans SET active = 0, lifted_at = ?1, lifted_by_name = ?2, lift_reason = ?3
             WHERE id = ?4 AND active = 1",
            params![
                now_ms(),
                lifted_by_name,
                clamp_text(lift_reason, 1000),
                ban_id
            ],
        )?;
        if changed == 0 {
            return Err(AppError::not_found(
                "No active internal ban with that ID.",
            ));
        }
        Ok(())
    }

    pub fn active_internal_ban(
        &self,
        roblox_user_id: Option<u64>,
        username: &str,
    ) -> AppResult<Option<InternalBanView>> {
        let conn = self.lock()?;
        let ban = conn
            .query_row(
                "SELECT id, roblox_user_id, username, reason, created_by_name, created_at,
                        active, lifted_at, lifted_by_name, lift_reason
                 FROM internal_bans
                 WHERE active = 1 AND ((?1 IS NOT NULL AND roblox_user_id = ?1) OR username = ?2 COLLATE NOCASE)
                 ORDER BY created_at DESC LIMIT 1",
                params![
                    roblox_user_id.map(|id| id as i64),
                    username.trim()
                ],
                map_ban,
            )
            .optional()?;
        Ok(ban)
    }

    pub fn list_internal_bans(&self, active_only: bool, limit: usize) -> AppResult<Vec<InternalBanView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, roblox_user_id, username, reason, created_by_name, created_at,
                    active, lifted_at, lifted_by_name, lift_reason
             FROM internal_bans
             WHERE (?1 = 0 OR active = 1)
             ORDER BY active DESC, created_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![if active_only { 1 } else { 0 }, limit as i64], map_ban)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_internal_bans(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM internal_bans", [], |row| row.get(0))?)
    }

    pub fn internal_ban_count_for(&self, roblox_user_id: Option<u64>, username: &str) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM internal_bans
             WHERE (?1 IS NOT NULL AND roblox_user_id = ?1) OR username = ?2 COLLATE NOCASE",
            params![roblox_user_id.map(|id| id as i64), username.trim()],
            |row| row.get(0),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_moderation_history() {
        let db = Database::open_in_memory().unwrap();
        db.insert_moderation_record(&NewModerationRecord {
            roblox_user_id: Some(1),
            username: "Builderman",
            action: "kick",
            source: "erlc_api",
            reason: "AFK",
            moderator_user_id: None,
            moderator_name: "Owner",
            success: true,
            command_id: None,
            api_message: Some("Success"),
        })
        .unwrap();
        let history = db.moderation_for_player(Some(1), "Builderman", 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].action, "kick");
        assert!(history[0].success);
        assert_eq!(db.recent_moderation(10).unwrap().len(), 1);
    }

    #[test]
    fn rejects_unknown_actions_and_sources() {
        let db = Database::open_in_memory().unwrap();
        let err = db
            .insert_moderation_record(&NewModerationRecord {
                roblox_user_id: Some(1),
                username: "x",
                action: "exterminate",
                source: "erlc_api",
                reason: "",
                moderator_user_id: None,
                moderator_name: "Owner",
                success: true,
                command_id: None,
                api_message: None,
            })
            .unwrap_err();
        assert_eq!(err.code(), "validation");
    }

    #[test]
    fn internal_bans_are_unique_while_active_and_can_be_lifted() {
        let db = Database::open_in_memory().unwrap();
        let id = db
            .create_internal_ban(Some(7), "Troll", "Repeated RDM", None, "Owner")
            .unwrap();
        let err = db
            .create_internal_ban(Some(7), "Troll", "Again", None, "Owner")
            .unwrap_err();
        assert_eq!(err.code(), "conflict");
        assert!(db.active_internal_ban(Some(7), "Troll").unwrap().is_some());
        db.lift_internal_ban(id, "Owner", "Appeal accepted").unwrap();
        assert!(db.active_internal_ban(Some(7), "Troll").unwrap().is_none());
        assert_eq!(db.internal_ban_count_for(Some(7), "Troll").unwrap(), 1);
    }

    #[test]
    fn internal_ban_requires_a_reason() {
        let db = Database::open_in_memory().unwrap();
        assert!(db
            .create_internal_ban(Some(7), "Troll", "   ", None, "Owner")
            .is_err());
    }
}
