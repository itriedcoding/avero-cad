//! SQLite persistence layer.

pub mod audit;
pub mod backup;
pub mod commands;
pub mod dispatch;
pub mod metrics;
pub mod migrations;
pub mod moderation;
pub mod players;
pub mod settings;
pub mod users;

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// Thread-safe handle around a single SQLite connection.
///
/// WAL mode plus a busy timeout keeps concurrent reads responsive while the
/// application performs short writes from the UI thread pool.
pub struct Database {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        Self::configure(&conn, true)?;
        let db = Self {
            path,
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn, false)?;
        let db = Self {
            path: PathBuf::from(":memory:"),
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn configure(conn: &Connection, on_disk: bool) -> AppResult<()> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        if on_disk {
            let _ = conn.pragma_update(None, "journal_mode", "WAL");
            let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        }
        Ok(())
    }

    pub fn migrate(&self) -> AppResult<()> {
        let conn = self.lock()?;
        migrations::apply(&conn)
    }

    pub fn lock(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size_bytes(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// Run a closure inside a single SQLite transaction.
    pub fn with_tx<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_seeds_roles_and_permissions() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.lock().unwrap();
        let roles: i64 = conn
            .query_row("SELECT COUNT(*) FROM roles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(roles, 3);
        let permissions: i64 = conn
            .query_row("SELECT COUNT(*) FROM permissions", [], |row| row.get(0))
            .unwrap();
        assert!(permissions >= 30);
        let owner_perms: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM role_permissions rp JOIN roles r ON r.id = rp.role_id WHERE r.key = 'owner'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(owner_perms, permissions);
    }

    #[test]
    fn migration_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();
        let conn = db.lock().unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.lock().unwrap();
        let result = conn.execute(
            "INSERT INTO role_permissions (role_id, permission_key) VALUES (999, 'players.view')",
            [],
        );
        assert!(result.is_err());
    }
}
