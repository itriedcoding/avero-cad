//! Async ER:LC API client.
//!
//! Behaviour that matters for safety:
//! * the server key is sent only in the `server-key` header and never logged;
//! * `GET` requests retry carefully on 429/5xx/network errors;
//! * `POST /v2/server/command` is **never** retried automatically, and the
//!   documented 1-request-per-5-seconds command limit is enforced locally.

use crate::api::parse_bans;
use crate::error::{AppError, AppResult};
use crate::models::{CommandApiResponse, LiveBanView, RateLimitView, ServerInfo, ServerSummary};
use crate::util::{log_line, now_ms};
use reqwest::{Method, Response, StatusCode};
use serde_json::Value;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const COMMAND_COOLDOWN: Duration = Duration::from_secs(5);
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(250);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Which optional blocks `/v2/server` should include.
#[derive(Debug, Clone, Copy, Default)]
pub struct ServerQuery {
    pub players: bool,
    pub staff: bool,
    pub join_logs: bool,
    pub queue: bool,
    pub kill_logs: bool,
    pub command_logs: bool,
    pub mod_calls: bool,
    pub emergency_calls: bool,
    pub vehicles: bool,
}

impl ServerQuery {
    pub fn everything() -> Self {
        Self {
            players: true,
            staff: true,
            join_logs: true,
            queue: true,
            kill_logs: true,
            command_logs: true,
            mod_calls: true,
            emergency_calls: true,
            vehicles: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    pub status: u16,
    pub latency_ms: u64,
    pub rate_limit: RateLimitView,
    pub fetched_at: i64,
}

#[derive(Debug)]
struct LimiterState {
    last_request: Option<Instant>,
    blocked_until: Option<Instant>,
    rate_limit: RateLimitView,
}

impl std::fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The API key is deliberately redacted from debug output.
        f.debug_struct("ApiClient")
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .finish()
    }
}

pub struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    limiter: Mutex<LimiterState>,
    command_guard: Mutex<Option<Instant>>,
    log_path: Option<std::path::PathBuf>,
}

impl ApiClient {
    pub fn new(base_url: &str, api_key: &str, log_path: Option<&Path>) -> AppResult<Self> {
        let trimmed_base = base_url.trim().trim_end_matches('/');
        if !trimmed_base.starts_with("https://") {
            return Err(AppError::validation("The API base URL must use HTTPS."));
        }
        if api_key.trim().is_empty() {
            return Err(AppError::validation("A server key is required."));
        }
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!(
                "averocad/{} (Windows desktop CAD)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| AppError::network(format!("Could not initialise the HTTP client: {e}")))?;
        Ok(Self {
            http,
            base_url: trimmed_base.to_string(),
            api_key: api_key.trim().to_string(),
            limiter: Mutex::new(LimiterState {
                last_request: None,
                blocked_until: None,
                rate_limit: RateLimitView::default(),
            }),
            command_guard: Mutex::new(None),
            log_path: log_path.map(|p| p.to_path_buf()),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn log(&self, level: &str, message: &str) {
        if let Some(path) = &self.log_path {
            log_line(path, level, message);
        }
    }

    /// Wait until the client is allowed to issue another request.
    async fn acquire(&self) -> AppResult<()> {
        loop {
            let wait = {
                let mut state = self
                    .limiter
                    .lock()
                    .map_err(|_| AppError::internal("Rate limiter lock poisoned"))?;
                let now = Instant::now();
                let mut wait = Duration::ZERO;
                if let Some(last) = state.last_request {
                    let elapsed = now.saturating_duration_since(last);
                    if elapsed < MIN_REQUEST_INTERVAL {
                        wait = MIN_REQUEST_INTERVAL - elapsed;
                    }
                }
                if let Some(blocked) = state.blocked_until {
                    if now < blocked {
                        let remaining = blocked - now;
                        if remaining > wait {
                            wait = remaining;
                        }
                    }
                }
                if wait.is_zero() {
                    state.last_request = Some(now);
                    return Ok(());
                }
                wait
            };
            tokio::time::sleep(wait).await;
        }
    }

    fn read_rate_headers(&self, response: &Response) {
        let get_i64 = |name: &str| -> Option<i64> {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<i64>().ok())
        };
        let bucket = response
            .headers()
            .get("x-ratelimit-bucket")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let rate = RateLimitView {
            bucket,
            limit: get_i64("x-ratelimit-limit"),
            remaining: get_i64("x-ratelimit-remaining"),
            reset: get_i64("x-ratelimit-reset"),
        };
        if let Ok(mut state) = self.limiter.lock() {
            if rate.limit.is_some() || rate.remaining.is_some() || rate.reset.is_some() {
                state.rate_limit = rate;
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let retry_after = get_i64("retry-after").unwrap_or(5).max(1) as u64;
                state.blocked_until = Some(Instant::now() + Duration::from_secs(retry_after));
            }
        }
    }

    fn current_rate(&self) -> RateLimitView {
        self.limiter
            .lock()
            .map(|s| s.rate_limit.clone())
            .unwrap_or_default()
    }

    fn retry_delay(&self) -> Duration {
        let blocked = self.limiter.lock().ok().and_then(|s| {
            s.blocked_until
                .map(|b| b.saturating_duration_since(Instant::now()))
        });
        blocked.unwrap_or(Duration::from_secs(2))
    }

    async fn send_once(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, bool)],
        body: Option<&Value>,
    ) -> AppResult<(StatusCode, Option<Value>, ResponseMeta)> {
        self.acquire().await?;
        let mut url = format!("{}{}", self.base_url, path);
        let params: Vec<String> = query
            .iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(key, _)| format!("{key}=true"))
            .collect();
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        let mut request = self
            .http
            .request(method.clone(), &url)
            .header("server-key", &self.api_key)
            .header("accept", "application/json");
        if let Some(payload) = body {
            request = request.json(payload);
        }

        let started = Instant::now();
        let response = request.send().await?;
        let latency_ms = started.elapsed().as_millis() as u64;
        let status = response.status();
        self.read_rate_headers(&response);

        let text = response.text().await.unwrap_or_default();
        let parsed = if text.trim().is_empty() {
            None
        } else {
            serde_json::from_str::<Value>(&text).ok()
        };

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = self.retry_delay().as_secs().max(1);
            return Err(AppError::RateLimited {
                message: format!(
                    "ER:LC is rate limiting this application. Retry in about {retry_after} second(s)."
                ),
                retry_after: Some(retry_after),
            });
        }

        if !status.is_success() {
            self.log(
                "warn",
                &format!(
                    "{method} {path} -> {status}: {}",
                    text.chars().take(300).collect::<String>()
                ),
            );
            return Err(map_error(status, parsed.as_ref(), &text));
        }

        Ok((
            status,
            parsed,
            ResponseMeta {
                status: status.as_u16(),
                latency_ms,
                rate_limit: self.current_rate(),
                fetched_at: now_ms(),
            },
        ))
    }

    /// GET with limited, safe retries (only for idempotent reads).
    async fn get(&self, path: &str, query: &[(&str, bool)]) -> AppResult<(Value, ResponseMeta)> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.send_once(Method::GET, path, query, None).await {
                Ok((_, value, meta)) => {
                    let value = value.unwrap_or(Value::Null);
                    return Ok((value, meta));
                }
                Err(err) => {
                    let retryable =
                        matches!(err, AppError::RateLimited { .. } | AppError::Network(_))
                            || matches!(err.http_status(), Some(status) if status >= 500);
                    if !retryable || attempt >= 3 {
                        return Err(err);
                    }
                    let wait = match &err {
                        AppError::RateLimited { retry_after, .. } => {
                            Duration::from_secs(retry_after.unwrap_or(2).min(30))
                        }
                        _ => Duration::from_millis(400 * attempt as u64),
                    };
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    pub async fn fetch_server(&self, query: ServerQuery) -> AppResult<(ServerInfo, ResponseMeta)> {
        let params = [
            ("Players", query.players),
            ("Staff", query.staff),
            ("JoinLogs", query.join_logs),
            ("Queue", query.queue),
            ("KillLogs", query.kill_logs),
            ("CommandLogs", query.command_logs),
            ("ModCalls", query.mod_calls),
            ("EmergencyCalls", query.emergency_calls),
            ("Vehicles", query.vehicles),
        ];
        let (value, meta) = self.get("/v2/server", &params).await?;
        let info: ServerInfo = serde_json::from_value(value).map_err(|e| {
            AppError::api(
                Some(200),
                format!("The ER:LC API returned an unexpected server payload: {e}"),
            )
        })?;
        Ok((info, meta))
    }

    pub async fn fetch_summary(&self) -> AppResult<(ServerSummary, ResponseMeta)> {
        let (info, meta) = self.fetch_server(ServerQuery::default()).await?;
        Ok((ServerSummary::from(&info), meta))
    }

    pub async fn fetch_bans(&self) -> AppResult<(Vec<LiveBanView>, ResponseMeta)> {
        let (value, meta) = self.get("/v1/server/bans", &[]).await?;
        Ok((parse_bans(&value), meta))
    }

    /// Execute a server command. Exactly one HTTP request is ever issued.
    pub async fn execute_command(
        &self,
        command: &str,
    ) -> AppResult<(String, Option<String>, ResponseMeta)> {
        {
            let mut guard = self
                .command_guard
                .lock()
                .map_err(|_| AppError::internal("Command guard lock poisoned"))?;
            let now = Instant::now();
            if let Some(last) = *guard {
                let elapsed = now.saturating_duration_since(last);
                if elapsed < COMMAND_COOLDOWN {
                    let remaining = (COMMAND_COOLDOWN - elapsed).as_secs() + 1;
                    return Err(AppError::conflict(format!(
                        "ER:LC accepts one command every 5 seconds. Try again in {remaining}s."
                    )));
                }
            }
            *guard = Some(now);
        }

        let body = serde_json::json!({ "command": command });
        let (_, value, meta) = match self
            .send_once(Method::POST, "/v2/server/command", &[], Some(&body))
            .await
        {
            Ok(result) => result,
            Err(err) => {
                // A failed attempt still consumed the cooldown slot; surface the
                // error without retrying, so a command can never run twice.
                return Err(err);
            }
        };
        let parsed: CommandApiResponse = value
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(CommandApiResponse {
                message: None,
                command_id: None,
            });
        let message = parsed
            .message
            .unwrap_or_else(|| "Command accepted by the ER:LC API.".to_string());
        Ok((message, parsed.command_id, meta))
    }
}

fn map_error(status: StatusCode, parsed: Option<&Value>, raw: &str) -> AppError {
    let message = parsed
        .and_then(|v| {
            v.get("message")
                .or_else(|| v.get("error"))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                format!("ER:LC API request failed with HTTP {}.", status.as_u16())
            } else {
                trimmed.chars().take(300).collect()
            }
        });
    let code = parsed.and_then(|v| v.get("code").and_then(|c| c.as_i64()));
    let command_id = parsed.and_then(|v| {
        v.get("commandId")
            .and_then(|c| c.as_str())
            .map(str::to_string)
    });

    match status {
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => AppError::Unauthorized(format!(
            "The ER:LC API rejected the server key (HTTP {}). {}",
            status.as_u16(),
            message
        )),
        StatusCode::UNPROCESSABLE_ENTITY => AppError::Api {
            status: Some(status.as_u16()),
            message: if message.is_empty() {
                "The private server is offline. ER:LC only executes commands while players are in the server.".into()
            } else {
                message
            },
            code,
            command_id,
        },
        _ => AppError::Api {
            status: Some(status.as_u16()),
            message,
            code,
            command_id,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_https_base_urls() {
        let err = ApiClient::new("http://api.erlc.gg", "key", None).unwrap_err();
        assert_eq!(err.code(), "validation");
    }

    #[test]
    fn rejects_blank_key() {
        let err = ApiClient::new("https://api.erlc.gg", "   ", None).unwrap_err();
        assert_eq!(err.code(), "validation");
    }

    #[test]
    fn accepts_documented_base_url_and_trims_slash() {
        let client = ApiClient::new("https://api.erlc.gg/", "test-key", None).unwrap();
        assert_eq!(client.base_url(), "https://api.erlc.gg");
    }

    #[test]
    fn maps_forbidden_to_unauthorized() {
        let err = map_error(StatusCode::FORBIDDEN, None, "");
        assert_eq!(err.code(), "unauthorized");
        assert!(err.message().contains("rejected the server key"));
    }

    #[test]
    fn maps_offline_server_message() {
        let body = serde_json::json!({"message": "The private server is currently offline.", "commandId": "abc"});
        let err = map_error(StatusCode::UNPROCESSABLE_ENTITY, Some(&body), "");
        assert_eq!(err.command_id().as_deref(), Some("abc"));
        assert!(err.message().contains("offline"));
    }

    #[test]
    fn command_never_retried_on_error_is_enforced_by_cooldown() {
        let client = ApiClient::new("https://api.erlc.gg", "test-key", None).unwrap();
        // Simulate a command having just been sent.
        *client.command_guard.lock().unwrap() = Some(Instant::now());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = runtime
            .block_on(client.execute_command(":h test"))
            .unwrap_err();
        assert_eq!(err.code(), "conflict");
    }
}
