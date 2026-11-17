//! Player records and internal staff notes.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::StaffNoteView;
use crate::util::{clamp_text, now_ms};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub id: i64,
    pub roblox_user_id: u64,
    pub username: String,
    pub display_name: Option<String>,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub last_team: Option<String>,
    pub last_callsign: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlayerSearchRow {
    pub roblox_user_id: u64,
    pub username: String,
    pub last_seen_at: i64,
    pub last_team: Option<String>,
    pub last_callsign: Option<String>,
}

impl Database {
    pub fn upsert_player_presence(
        &self,
        roblox_user_id: u64,
        username: &str,
        team: Option<&str>,
        callsign: Option<&str>,
        at: i64,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO players (roblox_user_id, username, first_seen_at, last_seen_at, last_team, last_callsign)
             VALUES (?1, ?2, ?3, ?3, ?4, ?5)
             ON CONFLICT(roblox_user_id) DO UPDATE SET
                username = excluded.username,
                last_seen_at = excluded.last_seen_at,
                last_team = COALESCE(excluded.last_team, players.last_team),
                last_callsign = COALESCE(excluded.last_callsign, players.last_callsign)",
            params![roblox_user_id as i64, username, at, team, callsign],
        )?;
        Ok(())
    }

    pub fn player_by_roblox_id(&self, roblox_user_id: u64) -> AppResult<Option<PlayerRecord>> {
        let conn = self.lock()?;
        let record = conn
            .query_row(
                "SELECT id, roblox_user_id, username, display_name, first_seen_at, last_seen_at,
                        last_team, last_callsign
                 FROM players WHERE roblox_user_id = ?1",
                params![roblox_user_id as i64],
                |row| {
                    Ok(PlayerRecord {
                        id: row.get(0)?,
                        roblox_user_id: row.get::<_, i64>(1)? as u64,
                        username: row.get(2)?,
                        display_name: row.get(3)?,
                        first_seen_at: row.get(4)?,
                        last_seen_at: row.get(5)?,
                        last_team: row.get(6)?,
                        last_callsign: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn player_by_username(&self, username: &str) -> AppResult<Option<PlayerRecord>> {
        let conn = self.lock()?;
        let record = conn
            .query_row(
                "SELECT id, roblox_user_id, username, display_name, first_seen_at, last_seen_at,
                        last_team, last_callsign
                 FROM players WHERE username = ?1 COLLATE NOCASE
                 ORDER BY last_seen_at DESC LIMIT 1",
                params![username.trim()],
                |row| {
                    Ok(PlayerRecord {
                        id: row.get(0)?,
                        roblox_user_id: row.get::<_, i64>(1)? as u64,
                        username: row.get(2)?,
                        display_name: row.get(3)?,
                        first_seen_at: row.get(4)?,
                        last_seen_at: row.get(5)?,
                        last_team: row.get(6)?,
                        last_callsign: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn search_players(&self, query: &str, limit: usize) -> AppResult<Vec<PlayerSearchRow>> {
        let trimmed = query.trim();
        // Require at least one usable character so wildcard-only queries cannot
        // dump the entire player table.
        if trimmed.is_empty() || !trimmed.chars().any(|c| c.is_ascii_alphanumeric()) {
            return Ok(Vec::new());
        }
        // Escape LIKE wildcards in user input.
        let escaped = trimmed
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT roblox_user_id, username, last_seen_at, last_team, last_callsign
             FROM players
             WHERE username LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR CAST(roblox_user_id AS TEXT) = ?2
                OR CAST(roblox_user_id AS TEXT) LIKE ?1 ESCAPE '\\'
             ORDER BY (username = ?2 COLLATE NOCASE) DESC, last_seen_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![pattern, trimmed, limit as i64], |row| {
                Ok(PlayerSearchRow {
                    roblox_user_id: row.get::<_, i64>(0)? as u64,
                    username: row.get(1)?,
                    last_seen_at: row.get(2)?,
                    last_team: row.get(3)?,
                    last_callsign: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Most recently seen players stored in the local database.
    pub fn list_players(&self, limit: usize) -> AppResult<Vec<PlayerSearchRow>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT roblox_user_id, username, last_seen_at, last_team, last_callsign
             FROM players ORDER BY last_seen_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(PlayerSearchRow {
                    roblox_user_id: row.get::<_, i64>(0)? as u64,
                    username: row.get(1)?,
                    last_seen_at: row.get(2)?,
                    last_team: row.get(3)?,
                    last_callsign: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_players(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))?)
    }

    pub fn add_staff_note(
        &self,
        roblox_user_id: Option<u64>,
        username: &str,
        author_user_id: Option<i64>,
        author_name: &str,
        body: &str,
    ) -> AppResult<i64> {
        let text = clamp_text(body, 4000);
        if text.is_empty() {
            return Err(AppError::validation("A note cannot be empty."));
        }
        let now = now_ms();
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO staff_notes (roblox_user_id, username, author_user_id, author_name, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                roblox_user_id.map(|id| id as i64),
                username.trim(),
                author_user_id,
                author_name,
                text,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_staff_notes(
        &self,
        roblox_user_id: Option<u64>,
        username: &str,
        limit: usize,
    ) -> AppResult<Vec<StaffNoteView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, roblox_user_id, username, author_name, body, created_at, updated_at
             FROM staff_notes
             WHERE (?1 IS NOT NULL AND roblox_user_id = ?1)
                OR username = ?2 COLLATE NOCASE
             ORDER BY created_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![
                    roblox_user_id.map(|id| id as i64),
                    username.trim(),
                    limit as i64
                ],
                |row| {
                    Ok(StaffNoteView {
                        id: row.get(0)?,
                        roblox_user_id: row.get::<_, Option<i64>>(1)?.map(|v| v as u64),
                        username: row.get(2)?,
                        author_name: row.get(3)?,
                        body: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn delete_staff_note(&self, note_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute("DELETE FROM staff_notes WHERE id = ?1", params![note_id])?;
        if changed == 0 {
            return Err(AppError::not_found("Staff note not found."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserts_presence_without_resetting_first_seen() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_player_presence(1, "Builderman", Some("Police"), Some("1A-1"), 1000)
            .unwrap();
        db.upsert_player_presence(1, "Builderman", Some("Fire"), None, 2000)
            .unwrap();
        let record = db.player_by_roblox_id(1).unwrap().unwrap();
        assert_eq!(record.first_seen_at, 1000);
        assert_eq!(record.last_seen_at, 2000);
        assert_eq!(record.last_team.as_deref(), Some("Fire"));
        // Callsign is retained when the newer sample has none.
        assert_eq!(record.last_callsign.as_deref(), Some("1A-1"));
        assert_eq!(db.count_players().unwrap(), 1);
    }

    #[test]
    fn searches_by_username_and_id() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_player_presence(123, "Builderman", None, None, 1)
            .unwrap();
        db.upsert_player_presence(456, "Noob", None, None, 2)
            .unwrap();
        assert_eq!(db.search_players("build", 10).unwrap().len(), 1);
        assert_eq!(
            db.search_players("123", 10).unwrap()[0].username,
            "Builderman"
        );
        assert_eq!(db.search_players("456", 10).unwrap()[0].username, "Noob");
        assert!(db.search_players("%", 10).unwrap().is_empty());
    }

    #[test]
    fn lists_players_most_recent_first() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_player_presence(1, "Older", None, None, 100)
            .unwrap();
        db.upsert_player_presence(2, "Newer", None, None, 200)
            .unwrap();
        let players = db.list_players(10).unwrap();
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].username, "Newer");
    }

    #[test]
    fn staff_notes_are_listed_per_player() {
        let db = Database::open_in_memory().unwrap();
        db.add_staff_note(Some(1), "Builderman", None, "Owner", "Watch this player")
            .unwrap();
        db.add_staff_note(Some(2), "Other", None, "Owner", "Unrelated")
            .unwrap();
        let notes = db.list_staff_notes(Some(1), "Builderman", 50).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "Watch this player");
        assert!(db
            .add_staff_note(Some(1), "Builderman", None, "Owner", "   ")
            .is_err());
    }
}
