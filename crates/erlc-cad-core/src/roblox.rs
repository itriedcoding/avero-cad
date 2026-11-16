//! Roblox identity and avatar integration.
//!
//! Uses the official Roblox endpoints:
//! * `POST https://users.roblox.com/v1/usernames/users` — username → id
//! * `GET  https://users.roblox.com/v1/users/{id}` — profile details
//! * `GET  https://thumbnails.roblox.com/v1/users/avatar-headshot` — headshot
//! * `GET  https://thumbnails.roblox.com/v1/users/avatar` — full body
//!
//! Thumbnails are downloaded by the Rust backend, cached on disk for 12 hours and
//! returned as data URLs so the webview never loads an arbitrary remote image.

use crate::error::{AppError, AppResult};
use crate::models::{AvatarView, RobloxUserView};
use crate::util::now_ms;
use base64::Engine;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const USERS_BASE: &str = "https://users.roblox.com";
const THUMBS_BASE: &str = "https://thumbnails.roblox.com";
const AVATAR_TTL_MS: i64 = 12 * 60 * 60 * 1000;
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvatarKind {
    Headshot,
    FullBody,
}

impl AvatarKind {
    fn endpoint(self) -> &'static str {
        match self {
            AvatarKind::Headshot => "avatar-headshot",
            AvatarKind::FullBody => "avatar",
        }
    }

    fn file_key(self) -> &'static str {
        match self {
            AvatarKind::Headshot => "headshot",
            AvatarKind::FullBody => "body",
        }
    }
}

#[derive(Debug, Clone)]
struct CachedAvatar {
    data_url: String,
    fetched_at: i64,
}

#[derive(Debug, Deserialize)]
struct UsernameLookupResponse {
    #[serde(default)]
    data: Vec<UsernameLookupEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsernameLookupEntry {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct ThumbnailResponse {
    #[serde(default)]
    data: Vec<ThumbnailEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThumbnailEntry {
    #[serde(default)]
    state: String,
    #[serde(default)]
    image_url: Option<String>,
}

pub struct RobloxClient {
    http: reqwest::Client,
    cache: Mutex<HashMap<(u64, AvatarKind), CachedAvatar>>,
    cache_dir: PathBuf,
}

impl RobloxClient {
    pub fn new(cache_dir: impl AsRef<Path>) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(8))
            .user_agent(format!(
                "erlccad/{} (Windows desktop CAD)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| AppError::network(format!("Could not initialise the HTTP client: {e}")))?;
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            http,
            cache: Mutex::new(HashMap::new()),
            cache_dir,
        })
    }

    /// Resolve a Roblox user ID or username to a full Roblox profile.
    pub async fn resolve(&self, identifier: &str) -> AppResult<RobloxUserView> {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation("Enter a Roblox username or user ID."));
        }
        if let Ok(id) = trimmed.parse::<u64>() {
            return self.user_by_id(id).await;
        }
        let matches = self.search_usernames(trimmed).await?;
        let exact = matches
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(trimmed))
            .or_else(|| matches.first());
        match exact {
            Some(entry) => self.user_by_id(entry.id).await,
            None => Err(AppError::not_found(format!(
                "Roblox has no user named \"{trimmed}\"."
            ))),
        }
    }

    async fn search_usernames(&self, username: &str) -> AppResult<Vec<UsernameLookupEntry>> {
        let url = format!("{USERS_BASE}/v1/usernames/users");
        let body = serde_json::json!({
            "usernames": [username],
            "excludeBannedUsers": false
        });
        let response = self.http.post(url).json(&body).send().await?;
        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::RateLimited {
                message: "Roblox is rate limiting username lookups. Try again shortly.".into(),
                retry_after: response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok()),
            });
        }
        if !status.is_success() {
            return Err(AppError::api(
                Some(status.as_u16()),
                format!("Roblox username lookup failed (HTTP {}).", status.as_u16()),
            ));
        }
        let parsed: UsernameLookupResponse = response.json().await.map_err(|e| {
            AppError::api(
                Some(status.as_u16()),
                format!("Roblox returned an unexpected username payload: {e}"),
            )
        })?;
        Ok(parsed.data)
    }

    pub async fn user_by_id(&self, id: u64) -> AppResult<RobloxUserView> {
        let url = format!("{USERS_BASE}/v1/users/{id}");
        let response = self.http.get(url).send().await?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::not_found(format!(
                "Roblox has no user with ID {id}."
            )));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::RateLimited {
                message: "Roblox is rate limiting requests. Try again shortly.".into(),
                retry_after: response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok()),
            });
        }
        if !status.is_success() {
            return Err(AppError::api(
                Some(status.as_u16()),
                format!("Roblox user lookup failed (HTTP {}).", status.as_u16()),
            ));
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UserPayload {
            id: u64,
            name: String,
            #[serde(default)]
            display_name: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            created: String,
            #[serde(default)]
            is_banned: bool,
            #[serde(default)]
            has_verified_badge: bool,
        }
        let user: UserPayload = response.json().await.map_err(|e| {
            AppError::api(
                Some(status.as_u16()),
                format!("Roblox returned an unexpected user payload: {e}"),
            )
        })?;
        Ok(RobloxUserView {
            id: user.id,
            name: user.name,
            display_name: if user.display_name.is_empty() {
                String::new()
            } else {
                user.display_name
            },
            description: user.description,
            created: user.created,
            is_banned: user.is_banned,
            has_verified_badge: user.has_verified_badge,
        })
    }

    /// Fetch (or load from cache) a Roblox avatar thumbnail.
    pub async fn avatar(&self, user_id: u64, kind: AvatarKind, force: bool) -> AppResult<AvatarView> {
        if !force {
            if let Some(cached) = self.memory_get(user_id, kind) {
                if now_ms() - cached.fetched_at < AVATAR_TTL_MS {
                    return Ok(AvatarView {
                        roblox_user_id: user_id,
                        data_url: cached.data_url,
                        cached: true,
                        fetched_at: cached.fetched_at,
                    });
                }
            }
            if let Some(view) = self.disk_get(user_id, kind)? {
                self.memory_put(user_id, kind, &view);
                return Ok(AvatarView {
                    roblox_user_id: user_id,
                    data_url: view.data_url,
                    cached: true,
                    fetched_at: view.fetched_at,
                });
            }
        }

        let url = format!(
            "{THUMBS_BASE}/v1/users/{}?userIds={user_id}&size=420x420&format=Png&isCircular=false",
            kind.endpoint()
        );
        let response = self.http.get(url).send().await?;
        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::RateLimited {
                message: "Roblox is rate limiting thumbnail requests. Try again shortly.".into(),
                retry_after: response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok()),
            });
        }
        if !status.is_success() {
            return Err(AppError::api(
                Some(status.as_u16()),
                format!("Roblox thumbnail lookup failed (HTTP {}).", status.as_u16()),
            ));
        }
        let payload: ThumbnailResponse = response.json().await.map_err(|e| {
            AppError::api(
                Some(status.as_u16()),
                format!("Roblox returned an unexpected thumbnail payload: {e}"),
            )
        })?;
        let entry = payload.data.into_iter().next().ok_or_else(|| {
            AppError::not_found(format!("Roblox returned no thumbnail for user {user_id}."))
        })?;
        let image_url = entry.image_url.filter(|u| !u.trim().is_empty());
        let image_url = match (entry.state.as_str(), image_url) {
            ("Completed", Some(url)) => url,
            (state, _) => {
                return Err(AppError::not_found(format!(
                    "Roblox has no rendered avatar for user {user_id} yet (state: {state})."
                )));
            }
        };

        let image_response = self.http.get(image_url).send().await?;
        if !image_response.status().is_success() {
            return Err(AppError::network(format!(
                "Could not download the Roblox avatar image (HTTP {}).",
                image_response.status().as_u16()
            )));
        }
        if let Some(length) = image_response.content_length() {
            if length as usize > MAX_IMAGE_BYTES {
                return Err(AppError::internal("Roblox avatar image is unexpectedly large."));
            }
        }
        let bytes = image_response.bytes().await?;
        if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
            return Err(AppError::internal(
                "Roblox returned an unusable avatar image.",
            ));
        }
        let mime = detect_mime(&bytes).ok_or_else(|| {
            AppError::internal("Roblox returned an avatar image in an unsupported format.")
        })?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_url = format!("data:{mime};base64,{encoded}");
        let fetched_at = now_ms();

        let path = self.cache_path(user_id, kind);
        let temp = path.with_extension("tmp");
        if std::fs::write(&temp, &bytes).is_ok() {
            let _ = std::fs::rename(&temp, &path);
        }
        self.memory_put(
            user_id,
            kind,
            &CachedAvatar {
                data_url: data_url.clone(),
                fetched_at,
            },
        );
        Ok(AvatarView {
            roblox_user_id: user_id,
            data_url,
            cached: false,
            fetched_at,
        })
    }

    fn cache_path(&self, user_id: u64, kind: AvatarKind) -> PathBuf {
        self.cache_dir
            .join(format!("{}-{user_id}.img", kind.file_key()))
    }

    fn memory_get(&self, user_id: u64, kind: AvatarKind) -> Option<CachedAvatar> {
        self.cache
            .lock()
            .ok()
            .and_then(|cache| cache.get(&(user_id, kind)).cloned())
    }

    fn memory_put(&self, user_id: u64, kind: AvatarKind, view: &CachedAvatar) {
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() > 512 {
                cache.clear();
            }
            cache.insert((user_id, kind), view.clone());
        }
    }

    fn disk_get(&self, user_id: u64, kind: AvatarKind) -> AppResult<Option<CachedAvatar>> {
        let path = self.cache_path(user_id, kind);
        if !path.exists() {
            return Ok(None);
        }
        let metadata = std::fs::metadata(&path)?;
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if now_ms() - modified_ms >= AVATAR_TTL_MS {
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        }
        let bytes = std::fs::read(&path)?;
        let Some(mime) = detect_mime(&bytes) else {
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(Some(CachedAvatar {
            data_url: format!("data:{mime};base64,{encoded}"),
            fetched_at: modified_ms,
        }))
    }
}

fn detect_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_images() {
        assert_eq!(detect_mime(&[0x89, b'P', b'N', b'G', 0, 0]), Some("image/png"));
        assert_eq!(detect_mime(&[0xFF, 0xD8, 0xFF, 0, 0]), Some("image/jpeg"));
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(detect_mime(&webp), Some("image/webp"));
        assert_eq!(detect_mime(b"GIF89a"), None);
    }

    #[test]
    fn cache_paths_are_scoped_per_user_and_kind() {
        let dir = std::env::temp_dir().join(format!("erlc-cad-roblox-{}", std::process::id()));
        let client = RobloxClient::new(&dir).unwrap();
        assert!(client
            .cache_path(42, AvatarKind::Headshot)
            .to_string_lossy()
            .ends_with("headshot-42.img"));
        assert!(client
            .cache_path(42, AvatarKind::FullBody)
            .to_string_lossy()
            .ends_with("body-42.img"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
