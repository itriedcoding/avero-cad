//! Command execution history and the authorization policy table.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{CommandHistoryView, CommandPolicyView};
use crate::permissions::Permission;
use crate::util::{clamp_text, now_ms};
use rusqlite::{params, OptionalExtension};

pub struct NewCommandExecution<'a> {
    pub actor_user_id: Option<i64>,
    pub actor_name: &'a str,
    pub command: &'a str,
    pub status: &'a str,
    pub response: &'a str,
    pub http_status: Option<u16>,
    pub command_id: Option<&'a str>,
    pub duration_ms: i64,
}

fn map_policy(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommandPolicyView> {
    Ok(CommandPolicyView {
        command: row.get("command")?,
        min_permission: row.get("min_permission")?,
        destructive: row.get::<_, i64>("destructive")? == 1,
        enabled: row.get::<_, i64>("enabled")? == 1,
        description: row.get("description")?,
    })
}

impl Database {
    pub fn insert_command_history(&self, execution: &NewCommandExecution<'_>) -> AppResult<i64> {
        if !["success", "failed", "rejected"].contains(&execution.status) {
            return Err(AppError::validation("Unknown command status."));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO command_history
             (actor_user_id, actor_name, command, status, response, http_status, command_id, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                execution.actor_user_id,
                execution.actor_name,
                clamp_text(execution.command, 500),
                execution.status,
                clamp_text(execution.response, 1000),
                execution.http_status.map(|s| s as i64),
                execution.command_id,
                execution.duration_ms,
                now_ms()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_command_history(&self, limit: usize) -> AppResult<Vec<CommandHistoryView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, actor_name, command, status, response, http_status, command_id, duration_ms, created_at
             FROM command_history ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(CommandHistoryView {
                    id: row.get(0)?,
                    actor_name: row.get(1)?,
                    command: row.get(2)?,
                    status: row.get(3)?,
                    response: row.get(4)?,
                    http_status: row.get::<_, Option<i64>>(5)?.map(|v| v as u16),
                    command_id: row.get(6)?,
                    duration_ms: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count_command_history(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row("SELECT COUNT(*) FROM command_history", [], |row| row.get(0))?)
    }

    pub fn command_policy(&self, command: &str) -> AppResult<Option<CommandPolicyView>> {
        let conn = self.lock()?;
        let policy = conn
            .query_row(
                "SELECT command, min_permission, destructive, enabled, description
                 FROM command_policies WHERE command = ?1",
                params![command.trim().to_ascii_lowercase()],
                map_policy,
            )
            .optional()?;
        Ok(policy)
    }

    pub fn command_policies(&self) -> AppResult<Vec<CommandPolicyView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT command, min_permission, destructive, enabled, description
             FROM command_policies ORDER BY command",
        )?;
        let rows = stmt
            .query_map([], map_policy)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_command_policy(
        &self,
        command: &str,
        min_permission: &str,
        destructive: bool,
        enabled: bool,
        description: &str,
    ) -> AppResult<()> {
        let normalized = command.trim().to_ascii_lowercase();
        if !normalized.starts_with(':') || normalized.len() < 2 || normalized.len() > 40 {
            return Err(AppError::validation(
                "Command policies must start with \":\" (for example \":kick\").",
            ));
        }
        if normalized[1..]
            .chars()
            .any(|c| !c.is_ascii_alphanumeric())
        {
            return Err(AppError::validation(
                "Command names may only contain letters and numbers after the colon.",
            ));
        }
        if Permission::from_key(min_permission).is_none() {
            return Err(AppError::validation("Unknown permission for this command."));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO command_policies (command, min_permission, destructive, enabled, description, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(command) DO UPDATE SET min_permission = excluded.min_permission,
                destructive = excluded.destructive, enabled = excluded.enabled,
                description = excluded.description, updated_at = excluded.updated_at",
            params![
                normalized,
                min_permission,
                if destructive { 1 } else { 0 },
                if enabled { 1 } else { 0 },
                clamp_text(description, 300),
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub fn delete_command_policy(&self, command: &str) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "DELETE FROM command_policies WHERE command = ?1",
            params![command.trim().to_ascii_lowercase()],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Command policy not found."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_documented_command_policies() {
        let db = Database::open_in_memory().unwrap();
        let policies = db.command_policies().unwrap();
        assert!(policies.iter().any(|p| p.command == ":h"));
        assert!(policies.iter().any(|p| p.command == ":ban" && p.destructive));
    }

    #[test]
    fn upserts_and_deletes_policies() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_command_policy(":respawn", "commands.execute", true, true, "Respawn vehicles")
            .unwrap();
        let policy = db.command_policy(":respawn").unwrap().unwrap();
        assert!(policy.destructive);
        db.upsert_command_policy(":respawn", "commands.execute", false, false, "")
            .unwrap();
        assert!(!db.command_policy(":respawn").unwrap().unwrap().enabled);
        db.delete_command_policy(":respawn").unwrap();
        assert!(db.command_policy(":respawn").unwrap().is_none());
    }

    #[test]
    fn validates_policy_input() {
        let db = Database::open_in_memory().unwrap();
        assert!(db
            .upsert_command_policy("kick", "commands.execute", false, true, "")
            .is_err());
        assert!(db
            .upsert_command_policy(":bad name", "commands.execute", false, true, "")
            .is_err());
        assert!(db
            .upsert_command_policy(":ok", "not.a.permission", false, true, "")
            .is_err());
    }

    #[test]
    fn records_execution_history() {
        let db = Database::open_in_memory().unwrap();
        db.insert_command_history(&NewCommandExecution {
            actor_user_id: None,
            actor_name: "Owner",
            command: ":h hello",
            status: "success",
            response: "Success",
            http_status: Some(200),
            command_id: None,
            duration_ms: 120,
        })
        .unwrap();
        let history = db.list_command_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, "success");
        assert_eq!(history[0].http_status, Some(200));
    }
}
