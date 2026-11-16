//! Schema migrations.
//!
//! Migrations are embedded at compile time and applied inside a transaction,
//! with the applied version tracked in `PRAGMA user_version`.

use crate::error::AppResult;
use rusqlite::Connection;

pub struct Migration {
    pub version: i64,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/0001_initial.sql"
    )),
}];

pub fn apply(conn: &Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_migration_sets_user_version() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        apply(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let highest = MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0);
        assert_eq!(version, highest);
    }
}
