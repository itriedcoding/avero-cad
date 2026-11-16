//! Local staff accounts, roles and permission grants.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{PermissionOverrideView, RoleView, StaffUserView};
use crate::permissions::{resolve_permissions, Access};
use crate::util::{is_valid_username, now_ms};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role_id: i64,
    pub role_key: String,
    pub role_name: String,
    pub roblox_user_id: Option<i64>,
    pub is_active: bool,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub failed_attempts: i64,
    pub locked_until: Option<i64>,
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub display_name: &'a str,
    pub password_hash: &'a str,
    pub role_key: &'a str,
    pub roblox_user_id: Option<u64>,
}

fn map_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRow> {
    Ok(UserRow {
        id: row.get("id")?,
        username: row.get("username")?,
        display_name: row.get("display_name")?,
        password_hash: row.get("password_hash")?,
        role_id: row.get("role_id")?,
        role_key: row.get("role_key")?,
        role_name: row.get("role_name")?,
        roblox_user_id: row.get("roblox_user_id")?,
        is_active: row.get::<_, i64>("is_active")? == 1,
        created_at: row.get("created_at")?,
        last_login_at: row.get("last_login_at")?,
        failed_attempts: row.get("failed_attempts")?,
        locked_until: row.get("locked_until")?,
    })
}

impl Database {
    pub fn has_users(&self) -> AppResult<bool> {
        let conn = self.lock()?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM app_users", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn count_active_owners(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM app_users u JOIN roles r ON r.id = u.role_id
             WHERE r.key = 'owner' AND u.is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn create_user(&self, new: NewUser<'_>) -> AppResult<i64> {
        if !is_valid_username(new.username) {
            return Err(AppError::validation(
                "Usernames must be 3-32 characters using letters, numbers, dots, dashes or underscores.",
            ));
        }
        if new.display_name.trim().is_empty() {
            return Err(AppError::validation("A display name is required."));
        }
        let conn = self.lock()?;
        let role_id: i64 = conn
            .query_row(
                "SELECT id FROM roles WHERE key = ?1",
                params![new.role_key],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::validation(format!("Unknown role \"{}\".", new.role_key)))?;
        let created_at = now_ms();
        let result = conn.execute(
            "INSERT INTO app_users (username, display_name, password_hash, role_id, roblox_user_id, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
            params![
                new.username.trim(),
                new.display_name.trim(),
                new.password_hash,
                role_id,
                new.roblox_user_id.map(|id| id as i64),
                created_at
            ],
        );
        match result {
            Ok(_) => Ok(conn.last_insert_rowid()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(AppError::conflict(
                    "That username is already used by another staff account.",
                ))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn user_by_username(&self, username: &str) -> AppResult<Option<UserRow>> {
        let conn = self.lock()?;
        let user = conn
            .query_row(
                "SELECT u.id, u.username, u.display_name, u.password_hash, u.role_id,
                        u.roblox_user_id, u.is_active, u.created_at, u.last_login_at,
                        u.failed_attempts, u.locked_until,
                        r.key AS role_key, r.name AS role_name
                 FROM app_users u JOIN roles r ON r.id = u.role_id
                 WHERE u.username = ?1 COLLATE NOCASE",
                params![username.trim()],
                map_user_row,
            )
            .optional()?;
        Ok(user)
    }

    pub fn user_by_id(&self, id: i64) -> AppResult<Option<UserRow>> {
        let conn = self.lock()?;
        let user = conn
            .query_row(
                "SELECT u.id, u.username, u.display_name, u.password_hash, u.role_id,
                        u.roblox_user_id, u.is_active, u.created_at, u.last_login_at,
                        u.failed_attempts, u.locked_until,
                        r.key AS role_key, r.name AS role_name
                 FROM app_users u JOIN roles r ON r.id = u.role_id
                 WHERE u.id = ?1",
                params![id],
                map_user_row,
            )
            .optional()?;
        Ok(user)
    }

    pub fn role_permission_keys(&self, role_id: i64) -> AppResult<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT permission_key FROM role_permissions WHERE role_id = ?1 ORDER BY permission_key",
        )?;
        let keys = stmt
            .query_map(params![role_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    pub fn user_overrides(&self, user_id: i64) -> AppResult<Vec<PermissionOverrideView>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT permission_key, allowed FROM user_permissions WHERE user_id = ?1 ORDER BY permission_key",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(PermissionOverrideView {
                    permission_key: row.get(0)?,
                    allowed: row.get::<_, i64>(1)? == 1,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn load_access(&self, user_id: i64) -> AppResult<Option<Access>> {
        let Some(user) = self.user_by_id(user_id)? else {
            return Ok(None);
        };
        // A single connection is used for each query; the pool is a mutex so
        // nested locking would deadlock. Read role grants first, then overrides.
        let role_grants = self.role_permission_keys(user.role_id)?;
        let overrides = self.user_overrides(user_id)?;
        let permissions = resolve_permissions(
            &user.role_key,
            role_grants,
            overrides.into_iter().map(|o| (o.permission_key, o.allowed)),
        );
        Ok(Some(Access {
            user_id: user.id,
            username: user.username,
            display_name: user.display_name,
            role_id: user.role_id,
            role_key: user.role_key,
            role_name: user.role_name,
            permissions,
        }))
    }

    pub fn list_staff_users(&self) -> AppResult<Vec<StaffUserView>> {
        let rows: Vec<(i64, String, String, i64, String, String, Option<i64>, bool, i64, Option<i64>)> = {
            let conn = self.lock()?;
            let mut stmt = conn.prepare(
                "SELECT u.id, u.username, u.display_name, u.role_id, r.key, r.name,
                        u.roblox_user_id, u.is_active, u.created_at, u.last_login_at
                 FROM app_users u JOIN roles r ON r.id = u.role_id
                 ORDER BY u.username COLLATE NOCASE",
            )?;
            let mapped = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get::<_, i64>(7)? == 1,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            mapped
        };

        let mut users = Vec::with_capacity(rows.len());
        for (id, username, display_name, role_id, role_key, role_name, roblox_user_id, is_active, created_at, last_login_at) in rows {
            users.push(StaffUserView {
                id,
                username,
                display_name,
                role_id,
                role_key,
                role_name,
                roblox_user_id: roblox_user_id.map(|v| v as u64),
                is_active,
                created_at,
                last_login_at,
                overrides: self.user_overrides(id)?,
            });
        }
        Ok(users)
    }

    pub fn update_user_role(&self, user_id: i64, role_key: &str) -> AppResult<()> {
        let conn = self.lock()?;
        let role_id: i64 = conn
            .query_row(
                "SELECT id FROM roles WHERE key = ?1",
                params![role_key],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::validation("Unknown role."))?;
        let changed = conn.execute(
            "UPDATE app_users SET role_id = ?1 WHERE id = ?2",
            params![role_id, user_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Staff account not found."));
        }
        Ok(())
    }

    pub fn set_user_active(&self, user_id: i64, active: bool) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE app_users SET is_active = ?1 WHERE id = ?2",
            params![if active { 1 } else { 0 }, user_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Staff account not found."));
        }
        Ok(())
    }

    pub fn set_user_display_name(&self, user_id: i64, display_name: &str) -> AppResult<()> {
        if display_name.trim().is_empty() {
            return Err(AppError::validation("A display name is required."));
        }
        let conn = self.lock()?;
        conn.execute(
            "UPDATE app_users SET display_name = ?1 WHERE id = ?2",
            params![display_name.trim(), user_id],
        )?;
        Ok(())
    }

    pub fn set_user_password_hash(&self, user_id: i64, hash: &str) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE app_users SET password_hash = ?1, failed_attempts = 0, locked_until = NULL WHERE id = ?2",
            params![hash, user_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Staff account not found."));
        }
        Ok(())
    }

    pub fn set_user_permission_override(
        &self,
        user_id: i64,
        permission_key: &str,
        allowed: Option<bool>,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        match allowed {
            Some(allowed) => {
                conn.execute(
                    "INSERT INTO user_permissions (user_id, permission_key, allowed)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(user_id, permission_key) DO UPDATE SET allowed = excluded.allowed",
                    params![user_id, permission_key, if allowed { 1 } else { 0 }],
                )?;
            }
            None => {
                conn.execute(
                    "DELETE FROM user_permissions WHERE user_id = ?1 AND permission_key = ?2",
                    params![user_id, permission_key],
                )?;
            }
        }
        Ok(())
    }

    pub fn record_login_success(&self, user_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE app_users SET last_login_at = ?1, failed_attempts = 0, locked_until = NULL WHERE id = ?2",
            params![now_ms(), user_id],
        )?;
        Ok(())
    }

    pub fn record_login_failure(&self, user_id: i64, lock_after: i64, lock_minutes: i64) -> AppResult<(i64, Option<i64>)> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE app_users SET failed_attempts = failed_attempts + 1 WHERE id = ?1",
            params![user_id],
        )?;
        let attempts: i64 = conn.query_row(
            "SELECT failed_attempts FROM app_users WHERE id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if attempts >= lock_after {
            let locked_until = now_ms() + lock_minutes * 60_000;
            conn.execute(
                "UPDATE app_users SET locked_until = ?1 WHERE id = ?2",
                params![locked_until, user_id],
            )?;
            Ok((attempts, Some(locked_until)))
        } else {
            Ok((attempts, None))
        }
    }

    pub fn clear_login_failures(&self, user_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE app_users SET failed_attempts = 0, locked_until = NULL WHERE id = ?1",
            params![user_id],
        )?;
        Ok(())
    }

    pub fn list_roles(&self) -> AppResult<Vec<RoleView>> {
        let base: Vec<(i64, String, String, String, bool)> = {
            let conn = self.lock()?;
            let mut stmt = conn.prepare(
                "SELECT r.id, r.key, r.name, r.description, r.is_system FROM roles r ORDER BY r.is_system DESC, r.name",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, i64>(4)? == 1,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut roles = Vec::with_capacity(base.len());
        for (id, key, name, description, is_system) in base {
            let permissions = self.role_permission_keys(id)?;
            let user_count = {
                let conn = self.lock()?;
                conn.query_row(
                    "SELECT COUNT(*) FROM app_users WHERE role_id = ?1",
                    params![id],
                    |row| row.get(0),
                )?
            };
            roles.push(RoleView {
                id,
                key,
                name,
                description,
                is_system,
                permissions,
                user_count,
            });
        }
        Ok(roles)
    }

    pub fn create_role(&self, key: &str, name: &str, description: &str, permissions: &[String]) -> AppResult<i64> {
        let key = key.trim().to_ascii_lowercase();
        if key.len() < 3 || key.len() > 32 || !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return Err(AppError::validation(
                "Role keys must be 3-32 lowercase letters, numbers, dashes or underscores.",
            ));
        }
        if name.trim().is_empty() {
            return Err(AppError::validation("A role name is required."));
        }
        let id = self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO roles (key, name, description, is_system, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
                params![key, name.trim(), description.trim(), now_ms()],
            )
            .map_err(|err| match err {
                rusqlite::Error::SqliteFailure(e, _)
                    if e.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    AppError::conflict("A role with that key already exists.")
                }
                other => other.into(),
            })?;
            let role_id = tx.last_insert_rowid();
            for permission in permissions {
                tx.execute(
                    "INSERT OR IGNORE INTO role_permissions (role_id, permission_key) VALUES (?1, ?2)",
                    params![role_id, permission],
                )?;
            }
            Ok(role_id)
        })?;
        Ok(id)
    }

    pub fn update_role(&self, role_id: i64, name: &str, description: &str) -> AppResult<()> {
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE roles SET name = ?1, description = ?2 WHERE id = ?3",
            params![name.trim(), description.trim(), role_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found("Role not found."));
        }
        Ok(())
    }

    pub fn set_role_permissions(&self, role_id: i64, permissions: &[String]) -> AppResult<()> {
        self.with_tx(|tx| {
            let (key, is_system): (String, i64) = tx
                .query_row(
                    "SELECT key, is_system FROM roles WHERE id = ?1",
                    params![role_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::not_found("Role not found."))?;
            if key == "owner" {
                return Err(AppError::permission(
                    "The Server Owner role always holds every permission and cannot be limited.",
                ));
            }
            let _ = is_system;
            tx.execute(
                "DELETE FROM role_permissions WHERE role_id = ?1",
                params![role_id],
            )?;
            for permission in permissions {
                if crate::permissions::Permission::from_key(permission).is_some() {
                    tx.execute(
                        "INSERT OR IGNORE INTO role_permissions (role_id, permission_key) VALUES (?1, ?2)",
                        params![role_id, permission],
                    )?;
                }
            }
            Ok(())
        })
    }

    pub fn delete_role(&self, role_id: i64) -> AppResult<()> {
        let conn = self.lock()?;
        let (is_system, users): (i64, i64) = conn
            .query_row(
                "SELECT r.is_system, (SELECT COUNT(*) FROM app_users u WHERE u.role_id = r.id)
                 FROM roles r WHERE r.id = ?1",
                params![role_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("Role not found."))?;
        if is_system == 1 {
            return Err(AppError::permission(
                "Built-in roles cannot be deleted. Edit their permissions instead.",
            ));
        }
        if users > 0 {
            return Err(AppError::conflict(
                "Move the staff assigned to this role before deleting it.",
            ));
        }
        conn.execute("DELETE FROM roles WHERE id = ?1", params![role_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{Permission, ROLE_ADMINISTRATOR, ROLE_MODERATOR, ROLE_OWNER};

    fn db_with_owner() -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        let id = db
            .create_user(NewUser {
                username: "owner",
                display_name: "Server Owner",
                password_hash: "$argon2id$fake",
                role_key: ROLE_OWNER,
                roblox_user_id: None,
            })
            .unwrap();
        (db, id)
    }

    #[test]
    fn creates_users_and_resolves_owner_access() {
        let (db, id) = db_with_owner();
        assert!(db.has_users().unwrap());
        let access = db.load_access(id).unwrap().unwrap();
        assert!(access.is_owner());
        assert!(access.can(Permission::DatabaseRestore));
        assert_eq!(access.user_id, id);
    }

    #[test]
    fn duplicate_usernames_are_rejected_case_insensitively() {
        let (db, _) = db_with_owner();
        let err = db
            .create_user(NewUser {
                username: "OWNER",
                display_name: "Duplicate",
                password_hash: "$argon2id$fake",
                role_key: ROLE_MODERATOR,
                roblox_user_id: None,
            })
            .unwrap_err();
        assert_eq!(err.code(), "conflict");
    }

    #[test]
    fn validates_username_and_role() {
        let db = Database::open_in_memory().unwrap();
        assert!(db
            .create_user(NewUser {
                username: "x",
                display_name: "Bad",
                password_hash: "h",
                role_key: ROLE_MODERATOR,
                roblox_user_id: None,
            })
            .is_err());
        assert!(db
            .create_user(NewUser {
                username: "validname",
                display_name: "Bad",
                password_hash: "h",
                role_key: "not-a-role",
                roblox_user_id: None,
            })
            .is_err());
    }

    #[test]
    fn overrides_layer_on_role_grants() {
        let (db, id) = db_with_owner();
        db.update_user_role(id, ROLE_MODERATOR).unwrap();
        let access = db.load_access(id).unwrap().unwrap();
        assert!(access.can(Permission::PlayersView));
        assert!(!access.can(Permission::AuditView));

        db.set_user_permission_override(id, "audit.view", Some(true))
            .unwrap();
        let access = db.load_access(id).unwrap().unwrap();
        assert!(access.can(Permission::AuditView));

        db.set_user_permission_override(id, "players.view", Some(false))
            .unwrap();
        let access = db.load_access(id).unwrap().unwrap();
        assert!(!access.can(Permission::PlayersView));
    }

    #[test]
    fn owner_role_cannot_be_limited() {
        let (db, _) = db_with_owner();
        let roles = db.list_roles().unwrap();
        let owner = roles.iter().find(|r| r.key == ROLE_OWNER).unwrap();
        assert!(db.set_role_permissions(owner.id, &[]).is_err());
    }

    #[test]
    fn custom_roles_can_be_created_and_assigned_but_not_system_roles_deleted() {
        let (db, _) = db_with_owner();
        let role_id = db
            .create_role("dispatcher", "Dispatcher", "Calls only", &["dispatch.view".into(), "dispatch.create".into()])
            .unwrap();
        let access = db.user_by_username("owner").unwrap().unwrap();
        db.update_user_role(access.id, "dispatcher").unwrap();
        let resolved = db.load_access(access.id).unwrap().unwrap();
        assert!(resolved.can(Permission::DispatchCreate));
        assert!(!resolved.can(Permission::ModerationBan));

        db.delete_role(role_id).unwrap_err();
        let admin_role = db
            .list_roles()
            .unwrap()
            .into_iter()
            .find(|r| r.key == ROLE_ADMINISTRATOR)
            .unwrap();
        assert!(db.delete_role(admin_role.id).is_err());
    }

    #[test]
    fn tracks_login_failures_and_locks() {
        let (db, id) = db_with_owner();
        let (attempts, locked) = db.record_login_failure(id, 3, 5).unwrap();
        assert_eq!(attempts, 1);
        assert!(locked.is_none());
        db.record_login_failure(id, 3, 5).unwrap();
        let (_, locked) = db.record_login_failure(id, 3, 5).unwrap();
        assert!(locked.is_some());
        db.record_login_success(id).unwrap();
        let user = db.user_by_id(id).unwrap().unwrap();
        assert_eq!(user.failed_attempts, 0);
        assert!(user.locked_until.is_none());
    }

    #[test]
    fn staff_listing_includes_role_and_overrides() {
        let (db, id) = db_with_owner();
        db.set_user_permission_override(id, "audit.view", Some(true))
            .unwrap();
        let users = db.list_staff_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].role_key, ROLE_OWNER);
        assert_eq!(users[0].overrides.len(), 1);
    }
}
