//! Application audit trail.

use crate::db::Database;
use crate::error::AppResult;
use crate::models::AuditLogView;
use crate::util::{clamp_text, now_ms};
use rusqlite::params;

pub struct AuditEntry<'a> {
    pub actor_user_id: Option<i64>,
    pub actor_name: &'a str,
    pub category: &'a str,
    pub action: &'a str,
    pub target: &'a str,
    pub details: &'a str,
}

impl Database {
    pub fn record_audit(&self, entry: &AuditEntry<'_>) -> AppResult<i64> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO audit_logs (actor_user_id, actor_name, category, action, target, details, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.actor_user_id,
                entry.actor_name,
                entry.category,
                entry.action,
                clamp_text(entry.target, 200),
                clamp_text(entry.details, 2000),
                now_ms()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_audit_logs(
        &self,
        category: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> AppResult<Vec<AuditLogView>> {
        let conn = self.lock()?;
        let pattern = query
            .map(|q| format!("%{}%", q.trim().replace('%', "")))
            .filter(|q| q.len() > 2);
        let mut stmt = conn.prepare(
            "SELECT id, actor_name, category, action, target, details, created_at
             FROM audit_logs
             WHERE (?1 IS NULL OR category = ?1)
               AND (?2 IS NULL OR actor_name LIKE ?2 COLLATE NOCASE OR action LIKE ?2 COLLATE NOCASE
                    OR target LIKE ?2 COLLATE NOCASE)
             ORDER BY created_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![category, pattern, limit as i64], |row| {
                Ok(AuditLogView {
                    id: row.get(0)?,
                    actor_name: row.get(1)?,
                    category: row.get(2)?,
                    action: row.get(3)?,
                    target: row.get(4)?,
                    details: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_audit_logs(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |row| row.get(0))?)
    }

    pub fn list_audit_categories(&self) -> AppResult<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt =
            conn.prepare("SELECT DISTINCT category FROM audit_logs ORDER BY category")?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_filters_audit_entries() {
        let db = Database::open_in_memory().unwrap();
        db.record_audit(&AuditEntry {
            actor_user_id: None,
            actor_name: "Owner",
            category: "command",
            action: "command.executed",
            target: ":h hello",
            details: "Success",
        })
        .unwrap();
        db.record_audit(&AuditEntry {
            actor_user_id: None,
            actor_name: "Mod",
            category: "moderation",
            action: "player.kicked",
            target: "Troll",
            details: "",
        })
        .unwrap();
        assert_eq!(db.list_audit_logs(None, None, 10).unwrap().len(), 2);
        assert_eq!(
            db.list_audit_logs(Some("command"), None, 10).unwrap().len(),
            1
        );
        assert_eq!(
            db.list_audit_logs(None, Some("Troll"), 10).unwrap().len(),
            1
        );
        assert_eq!(db.list_audit_categories().unwrap().len(), 2);
    }
}
