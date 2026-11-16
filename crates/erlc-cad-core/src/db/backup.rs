//! Database backup and restore.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{BackupView, DatabaseStatsView};
use crate::util::now_ms;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;

const REQUIRED_TABLES: &[&str] = &[
    "app_users",
    "roles",
    "role_permissions",
    "permissions",
    "players",
    "moderation_records",
    "internal_bans",
    "staff_notes",
    "dispatch_calls",
    "dispatch_units",
    "audit_logs",
    "settings",
    "api_config",
    "command_history",
];

impl Database {
    pub fn backup_to(&self, directory: &Path) -> AppResult<BackupView> {
        std::fs::create_dir_all(directory)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let path = unique_path(directory, &format!("erlc-cad-backup-{stamp}.sqlite"));
        let conn = self.lock()?;
        let escaped = path.to_string_lossy().replace('\'', "''");
        conn.execute(&format!("VACUUM INTO '{escaped}'"), [])?;
        drop(conn);
        let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size_bytes == 0 {
            return Err(AppError::internal(
                "The backup file was created but is empty. Check free disk space.",
            ));
        }
        let created_at = now_ms();
        self.set_last_backup_at(created_at)?;
        Ok(BackupView {
            path: path.to_string_lossy().to_string(),
            size_bytes,
            created_at,
        })
    }

    /// Restore from a backup file, validating it is a compatible CAD database.
    pub fn restore_from(&self, source: &Path) -> AppResult<()> {
        if !source.exists() {
            return Err(AppError::not_found("Backup file not found."));
        }
        let src = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|err| {
            AppError::validation(format!("That file is not a SQLite database: {err}"))
        })?;

        let integrity: String = src
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|err| AppError::validation(format!("Backup failed integrity check: {err}")))?;
        if integrity != "ok" {
            return Err(AppError::validation(format!(
                "Backup integrity check failed: {integrity}"
            )));
        }
        for table in REQUIRED_TABLES {
            let exists: i64 = src.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(AppError::validation(format!(
                    "That backup is missing the \"{table}\" table and is not an ER:LC CAD database."
                )));
            }
        }

        {
            let mut dest = self.lock()?;
            let backup = rusqlite::backup::Backup::new(&src, &mut dest)?;
            backup
                .run_to_completion(128, Duration::from_millis(10), None)
                .map_err(|err| AppError::database(format!("Restore failed: {err}")))?;
        }
        drop(src);
        self.migrate()?;
        Ok(())
    }

    pub fn stats(&self) -> AppResult<DatabaseStatsView> {
        let conn = self.lock()?;
        let scalar = |sql: &str| -> AppResult<i64> {
            Ok(conn.query_row(sql, [], |row| row.get::<_, i64>(0))?)
        };
        let stats = DatabaseStatsView {
            path: self.path().to_string_lossy().to_string(),
            size_bytes: self.size_bytes(),
            players: scalar("SELECT COUNT(*) FROM players")?,
            moderation_records: scalar("SELECT COUNT(*) FROM moderation_records")?,
            internal_bans: scalar("SELECT COUNT(*) FROM internal_bans")?,
            calls: scalar("SELECT COUNT(*) FROM dispatch_calls")?,
            units: scalar("SELECT COUNT(*) FROM dispatch_units")?,
            users: scalar("SELECT COUNT(*) FROM app_users")?,
            audit_logs: scalar("SELECT COUNT(*) FROM audit_logs")?,
            commands: scalar("SELECT COUNT(*) FROM command_history")?,
        };
        Ok(stats)
    }
}

fn unique_path(directory: &Path, file_name: &str) -> PathBuf {
    let mut path = directory.join(file_name);
    let mut counter = 1;
    while path.exists() {
        let stem = Path::new(file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "backup".into());
        path = directory.join(format!("{stem}-{counter}.sqlite"));
        counter += 1;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::users::NewUser;
    use crate::permissions::ROLE_OWNER;

    fn temp_dir(name: &str) -> PathBuf {
        crate::util::unique_temp_dir(&format!("backup-{name}"))
    }

    #[test]
    fn backs_up_and_restores_a_database() {
        let dir = temp_dir("roundtrip");
        let db_path = dir.join("app.sqlite");
        let source_db = Database::open(&db_path).unwrap();
        source_db
            .create_user(NewUser {
                username: "owner",
                display_name: "Owner",
                password_hash: "$argon2id$fake",
                role_key: ROLE_OWNER,
                roblox_user_id: None,
            })
            .unwrap();

        let backup = source_db.backup_to(&dir).unwrap();
        assert!(backup.size_bytes > 0);
        assert!(Path::new(&backup.path).exists());
        assert_eq!(source_db.stats().unwrap().users, 1);

        source_db
            .create_user(NewUser {
                username: "second",
                display_name: "Second",
                password_hash: "$argon2id$fake",
                role_key: ROLE_OWNER,
                roblox_user_id: None,
            })
            .unwrap();
        assert_eq!(source_db.stats().unwrap().users, 2);

        source_db.restore_from(Path::new(&backup.path)).unwrap();
        assert_eq!(source_db.stats().unwrap().users, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_files_that_are_not_cad_databases() {
        let dir = temp_dir("invalid");
        let db = Database::open(dir.join("app.sqlite")).unwrap();
        let junk = dir.join("junk.sqlite");
        std::fs::write(&junk, b"this is not a database").unwrap();
        assert!(db.restore_from(&junk).is_err());

        let empty = Connection::open(dir.join("empty.sqlite")).unwrap();
        empty
            .execute_batch("CREATE TABLE unrelated (id INTEGER);")
            .unwrap();
        drop(empty);
        let err = db.restore_from(&dir.join("empty.sqlite")).unwrap_err();
        assert!(err.message().contains("missing"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
