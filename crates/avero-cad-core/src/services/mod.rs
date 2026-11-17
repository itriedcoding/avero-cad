//! Application services and shared runtime context.

pub mod auth;
pub mod bolos;
pub mod command;
pub mod dispatch;
pub mod moderation;
pub mod players;
pub mod settings;
pub mod shifts;
pub mod snapshot;
pub mod staff;
pub mod stats;
pub mod updater;

use crate::api::ApiClient;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::SessionView;
use crate::permissions::Access;
use crate::roblox::RobloxClient;
use crate::security::SecretStore;
use crate::util::{log_line, now_ms};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub log_path: PathBuf,
    pub avatar_cache_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        Self {
            db_path: data_dir.join("avero-cad.sqlite"),
            log_path: data_dir.join("logs").join("avero-cad.log"),
            avatar_cache_dir: data_dir.join("avatar-cache"),
            data_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: i64,
    pub issued_at: i64,
    pub expires_at: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotCache {
    pub current: Option<snapshot::CachedSnapshot>,
    pub last_attempt_at: i64,
    pub last_error: Option<String>,
    pub notifications: Vec<crate::models::AppNotification>,
    pub seen_emergency: Vec<i64>,
    pub seen_mod_calls: Vec<i64>,
    pub last_metric_at: i64,
    pub mirrored_calls: Vec<i64>,
}

pub struct AppContext {
    pub db: Database,
    pub paths: AppPaths,
    pub secrets: SecretStore,
    pub roblox: RobloxClient,
    sessions: Mutex<HashMap<String, Session>>,
    api: Mutex<Option<Arc<ApiClient>>>,
    pub cache: Mutex<SnapshotCache>,
}

impl AppContext {
    pub fn new(data_dir: impl AsRef<Path>) -> AppResult<Self> {
        let paths = AppPaths::new(data_dir);
        std::fs::create_dir_all(&paths.data_dir)?;
        if let Some(parent) = paths.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Database::open(&paths.db_path)?;
        let secrets = SecretStore::new(&paths.data_dir);
        let roblox = RobloxClient::new(&paths.avatar_cache_dir)?;
        Ok(Self {
            db,
            secrets,
            roblox,
            paths,
            sessions: Mutex::new(HashMap::new()),
            api: Mutex::new(None),
            cache: Mutex::new(SnapshotCache::default()),
        })
    }

    pub fn log(&self, level: &str, message: &str) {
        log_line(&self.paths.log_path, level, message);
    }

    /// Stored server key, if the owner has configured one.
    pub fn server_key(&self) -> AppResult<Option<String>> {
        self.secrets.load()
    }

    pub fn api_key_last4(&self) -> AppResult<Option<String>> {
        Ok(self
            .secrets
            .load()?
            .as_deref()
            .map(crate::security::mask_key))
    }

    /// Build (and cache) the ER:LC API client from the stored key.
    pub fn api_client(&self) -> AppResult<Option<Arc<ApiClient>>> {
        if let Ok(guard) = self.api.lock() {
            if let Some(client) = guard.as_ref() {
                return Ok(Some(client.clone()));
            }
        }
        let Some(key) = self.server_key()? else {
            return Ok(None);
        };
        let config = self.db.api_config()?;
        let client = Arc::new(ApiClient::new(
            &config.base_url,
            &key,
            Some(&self.paths.log_path),
        )?);
        if let Ok(mut guard) = self.api.lock() {
            *guard = Some(client.clone());
        }
        Ok(Some(client))
    }

    pub fn invalidate_api_client(&self) {
        if let Ok(mut guard) = self.api.lock() {
            *guard = None;
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.last_error = None;
        }
    }

    fn idle_timeout_ms(&self) -> i64 {
        self.db
            .settings_view()
            .map(|s| s.session_idle_minutes as i64 * 60_000)
            .unwrap_or(60 * 60_000)
    }

    pub fn create_session(&self, access: &Access) -> AppResult<SessionView> {
        let token = crate::security::new_session_token()?;
        let now = now_ms();
        let expires_at = now + 12 * 60 * 60_000;
        let session = Session {
            token: token.clone(),
            user_id: access.user_id,
            issued_at: now,
            expires_at,
            last_seen: now,
        };
        {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| AppError::internal("Session store unavailable"))?;
            sessions.retain(|_, s| s.expires_at > now);
            sessions.insert(token.clone(), session);
        }
        let user = self
            .db
            .list_staff_users()?
            .into_iter()
            .find(|u| u.id == access.user_id)
            .ok_or_else(|| AppError::not_found("Staff account not found."))?;
        Ok(SessionView {
            token,
            user,
            permissions: access.effective_permissions(),
            expires_at,
            is_owner: access.is_owner(),
        })
    }

    /// Validate a session token and reload the actor's current permissions.
    pub fn authenticate(&self, token: &str) -> AppResult<Access> {
        if token.trim().is_empty() {
            return Err(AppError::unauthorized("Sign in to continue."));
        }
        let now = now_ms();
        let idle = self.idle_timeout_ms();
        let user_id = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| AppError::internal("Session store unavailable"))?;
            sessions.retain(|_, s| s.expires_at > now);
            let session = sessions
                .get_mut(token)
                .ok_or_else(|| AppError::unauthorized("Your session has ended. Sign in again."))?;
            if now - session.last_seen > idle {
                sessions.remove(token);
                return Err(AppError::unauthorized(
                    "Your session timed out after a period of inactivity.",
                ));
            }
            session.last_seen = now;
            session.user_id
        };
        let user = self
            .db
            .user_by_id(user_id)?
            .ok_or_else(|| AppError::unauthorized("Staff account no longer exists."))?;
        if !user.is_active {
            return Err(AppError::unauthorized(
                "Your staff account has been deactivated.",
            ));
        }
        self.db
            .load_access(user_id)?
            .ok_or_else(|| AppError::unauthorized("Staff account no longer exists."))
    }

    pub fn logout(&self, token: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(token);
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn session_expiry(&self, token: &str) -> Option<i64> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(token).map(|session| session.expires_at))
    }

    /// Drop every active session (used after a database restore).
    pub fn clear_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::users::NewUser;
    use crate::permissions::ROLE_OWNER;

    pub fn test_context() -> (AppContext, tempdir::TempDirGuard) {
        let guard = tempdir::TempDirGuard::new();
        let ctx = AppContext::new(&guard.path).unwrap();
        ctx.db
            .create_user(NewUser {
                username: "owner",
                display_name: "Owner",
                password_hash: &crate::security::hash_password("OwnerPass123").unwrap(),
                role_key: ROLE_OWNER,
                roblox_user_id: None,
            })
            .unwrap();
        (ctx, guard)
    }

    pub mod tempdir {
        use std::path::PathBuf;

        pub struct TempDirGuard {
            pub path: PathBuf,
        }

        impl TempDirGuard {
            pub fn new() -> Self {
                Self {
                    path: crate::util::unique_temp_dir("ctx"),
                }
            }
        }

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    #[test]
    fn sessions_round_trip_and_revoke() {
        let (ctx, _guard) = test_context();
        let user = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(user.id).unwrap().unwrap();
        let session = ctx.create_session(&access).unwrap();
        assert_eq!(session.token.len(), 64);
        let resolved = ctx.authenticate(&session.token).unwrap();
        assert!(resolved.is_owner());
        ctx.logout(&session.token);
        assert!(ctx.authenticate(&session.token).is_err());
    }

    #[test]
    fn api_client_is_none_without_a_key() {
        let (ctx, _guard) = test_context();
        assert!(ctx.api_client().unwrap().is_none());
    }

    #[test]
    fn deactivated_users_lose_sessions() {
        let (ctx, _guard) = test_context();
        let user = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(user.id).unwrap().unwrap();
        let session = ctx.create_session(&access).unwrap();
        ctx.db.set_user_active(user.id, false).unwrap();
        let err = ctx.authenticate(&session.token).unwrap_err();
        assert_eq!(err.code(), "unauthorized");
    }
}
