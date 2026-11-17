//! Application settings and non-secret API configuration metadata.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::SettingsView;
use crate::util::now_ms;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ApiConfigRow {
    pub base_url: String,
    pub key_storage: String,
    pub key_fingerprint: Option<String>,
    pub key_last4: Option<String>,
    pub last_verified_at: Option<i64>,
    pub server_name: Option<String>,
    pub server_id: Option<i64>,
}

pub const DEFAULT_API_BASE_URL: &str = "https://api.erlc.gg";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUpdateRow {
    pub version: String,
    pub started_at: i64,
    pub installer_path: String,
    pub sha256: String,
}

fn parse_bool(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key)
        .map(|v| matches!(v.trim(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

fn parse_u64(map: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    map.get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_f64(map: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    map.get(key)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

impl Database {
    pub fn settings_map(&self) -> AppResult<HashMap<String, String>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
    }

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.lock()?;
        Ok(conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?)
    }

    pub fn set_settings(&self, values: &[(String, String)]) -> AppResult<()> {
        let now = now_ms();
        self.with_tx(|tx| {
            for (key, value) in values {
                tx.execute(
                    "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                    params![key, value, now],
                )?;
            }
            Ok(())
        })
    }

    pub fn settings_view(&self) -> AppResult<SettingsView> {
        let map = self.settings_map()?;
        let last_backup = map
            .get("last_backup_at")
            .and_then(|v| v.trim().parse::<i64>().ok());
        Ok(SettingsView {
            refresh_interval_seconds: parse_u64(&map, "refresh_interval_seconds", 15).clamp(5, 300),
            startup_page: map
                .get("startup_page")
                .cloned()
                .unwrap_or_else(|| "dashboard".into()),
            auto_connect: parse_bool(&map, "auto_connect", true),
            confirm_destructive: parse_bool(&map, "confirm_destructive", true),
            compact_tables: parse_bool(&map, "compact_tables", false),
            map_image: map
                .get("map_image")
                .cloned()
                .unwrap_or_else(|| "https://api.erlc.gg/maps/summer_postals.png".into()),
            map_scale: parse_f64(&map, "map_scale", 3.0).clamp(0.2, 20.0),
            session_idle_minutes: parse_u64(&map, "session_idle_minutes", 60).clamp(5, 480),
            backup_directory: map.get("backup_directory").cloned().unwrap_or_default(),
            auto_backup_hours: parse_u64(&map, "auto_backup_hours", 24).min(720),
            notify_emergency_calls: parse_bool(&map, "notify_emergency_calls", true),
            notify_mod_calls: parse_bool(&map, "notify_mod_calls", true),
            notify_connection: parse_bool(&map, "notify_connection", true),
            last_backup_at: last_backup,
        })
    }

    pub fn api_config(&self) -> AppResult<ApiConfigRow> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                "SELECT base_url, key_storage, key_fingerprint, key_last4, last_verified_at,
                        server_name, server_id
                 FROM api_config WHERE id = 1",
                [],
                |row| {
                    Ok(ApiConfigRow {
                        base_url: row.get(0)?,
                        key_storage: row.get(1)?,
                        key_fingerprint: row.get(2)?,
                        key_last4: row.get(3)?,
                        last_verified_at: row.get(4)?,
                        server_name: row.get(5)?,
                        server_id: row.get(6)?,
                    })
                },
            )
            .optional()?
            .unwrap_or(ApiConfigRow {
                base_url: DEFAULT_API_BASE_URL.to_string(),
                key_storage: "windows_dpapi".into(),
                key_fingerprint: None,
                key_last4: None,
                last_verified_at: None,
                server_name: None,
                server_id: None,
            });
        Ok(row)
    }

    pub fn update_api_config(
        &self,
        base_url: &str,
        key_storage: &str,
        fingerprint: Option<&str>,
        last4: Option<&str>,
    ) -> AppResult<()> {
        let base = base_url.trim().trim_end_matches('/');
        if !(base.starts_with("https://")) {
            return Err(AppError::validation("The API base URL must use HTTPS."));
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO api_config (id, base_url, key_storage, key_fingerprint, key_last4, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET base_url = excluded.base_url,
                key_storage = excluded.key_storage,
                key_fingerprint = excluded.key_fingerprint,
                key_last4 = excluded.key_last4,
                updated_at = excluded.updated_at",
            params![base, key_storage, fingerprint, last4, now_ms()],
        )?;
        Ok(())
    }

    pub fn mark_api_verified(&self, server_name: &str, server_id: Option<i64>) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE api_config SET last_verified_at = ?1, server_name = ?2, server_id = ?3 WHERE id = 1",
            params![now_ms(), server_name, server_id],
        )?;
        Ok(())
    }

    pub fn set_last_backup_at(&self, at: i64) -> AppResult<()> {
        self.set_settings(&[("last_backup_at".into(), at.to_string())])
    }

    /// The most recent update whose installer was started but not yet observed
    /// running. Kept across restarts so a cancelled elevation prompt is visible.
    pub fn pending_update(&self) -> AppResult<Option<PendingUpdateRow>> {
        let map = self.settings_map()?;
        let version = map
            .get("update_pending_version")
            .cloned()
            .unwrap_or_default();
        let path = map
            .get("update_pending_installer")
            .cloned()
            .unwrap_or_default();
        if version.trim().is_empty() || path.trim().is_empty() {
            return Ok(None);
        }
        let started_at = map
            .get("update_pending_started_at")
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let sha256 = map
            .get("update_pending_sha256")
            .cloned()
            .unwrap_or_default();
        Ok(Some(PendingUpdateRow {
            version,
            started_at,
            installer_path: path,
            sha256,
        }))
    }

    pub fn set_pending_update(&self, row: &PendingUpdateRow) -> AppResult<()> {
        self.set_settings(&[
            ("update_pending_version".into(), row.version.clone()),
            (
                "update_pending_started_at".into(),
                row.started_at.to_string(),
            ),
            (
                "update_pending_installer".into(),
                row.installer_path.clone(),
            ),
            ("update_pending_sha256".into(), row.sha256.clone()),
        ])
    }

    pub fn clear_pending_update(&self) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM settings WHERE key LIKE 'update_pending_%'", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_default_settings() {
        let db = Database::open_in_memory().unwrap();
        let settings = db.settings_view().unwrap();
        assert_eq!(settings.refresh_interval_seconds, 15);
        assert!(settings.auto_connect);
        assert!(settings.confirm_destructive);
        assert_eq!(settings.map_scale, 3.0);
    }

    #[test]
    fn updates_and_reads_settings() {
        let db = Database::open_in_memory().unwrap();
        db.set_settings(&[
            ("refresh_interval_seconds".into(), "30".into()),
            ("compact_tables".into(), "true".into()),
            ("map_scale".into(), "4.5".into()),
        ])
        .unwrap();
        let settings = db.settings_view().unwrap();
        assert_eq!(settings.refresh_interval_seconds, 30);
        assert!(settings.compact_tables);
        assert_eq!(settings.map_scale, 4.5);
    }

    #[test]
    fn clamps_unsafe_values() {
        let db = Database::open_in_memory().unwrap();
        db.set_settings(&[
            ("refresh_interval_seconds".into(), "1".into()),
            ("map_scale".into(), "9999".into()),
        ])
        .unwrap();
        let settings = db.settings_view().unwrap();
        assert_eq!(settings.refresh_interval_seconds, 5);
        assert_eq!(settings.map_scale, 20.0);
    }

    #[test]
    fn remembers_and_clears_a_pending_update() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.pending_update().unwrap().is_none());
        let row = PendingUpdateRow {
            version: "1.3.0".into(),
            started_at: 1_700_000_000_000,
            installer_path: "C:\\updates\\setup.exe".into(),
            sha256: "abc".into(),
        };
        db.set_pending_update(&row).unwrap();
        assert_eq!(db.pending_update().unwrap(), Some(row));
        db.clear_pending_update().unwrap();
        assert!(db.pending_update().unwrap().is_none());
        // Unrelated settings survive clearing the record.
        db.set_settings(&[("refresh_interval_seconds".into(), "30".into())])
            .unwrap();
        db.clear_pending_update().unwrap();
        assert_eq!(db.settings_view().unwrap().refresh_interval_seconds, 30);
    }

    #[test]
    fn api_config_stores_metadata_only() {
        let db = Database::open_in_memory().unwrap();
        db.update_api_config(
            "https://api.erlc.gg",
            "windows_dpapi",
            Some("abc123"),
            Some("••••1234"),
        )
        .unwrap();
        let config = db.api_config().unwrap();
        assert_eq!(config.base_url, "https://api.erlc.gg");
        assert_eq!(config.key_fingerprint.as_deref(), Some("abc123"));
        assert!(db
            .update_api_config("http://insecure", "windows_dpapi", None, None)
            .is_err());
    }
}
