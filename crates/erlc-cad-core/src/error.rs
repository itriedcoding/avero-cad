use serde::Serialize;

/// Application-wide error type.
///
/// Every error is converted into a stable, machine readable `code` plus a human
/// readable `message` before crossing the IPC boundary. Credentials are never
/// included in messages.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    PermissionDenied(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{message}")]
    RateLimited {
        message: String,
        retry_after: Option<u64>,
    },
    #[error("{message}")]
    Api {
        status: Option<u16>,
        message: String,
        code: Option<i64>,
        command_id: Option<String>,
    },
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Security(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn permission(message: impl Into<String>) -> Self {
        Self::PermissionDenied(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn security(message: impl Into<String>) -> Self {
        Self::Security(message.into())
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    pub fn api(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
            code: None,
            command_id: None,
        }
    }

    /// Stable machine code consumed by the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::PermissionDenied(_) => "permission_denied",
            Self::Unauthorized(_) => "unauthorized",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::RateLimited { .. } => "rate_limited",
            Self::Api { .. } => "api_error",
            Self::Network(_) => "network",
            Self::Database(_) => "database",
            Self::Security(_) => "security",
            Self::Internal(_) => "internal",
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => *status,
            Self::RateLimited { .. } => Some(429),
            // Session/auth failures are internal to the app, not HTTP responses.
            Self::Unauthorized(_) => None,
            Self::PermissionDenied(_) => Some(403),
            Self::NotFound(_) => Some(404),
            _ => None,
        }
    }

    pub fn command_id(&self) -> Option<String> {
        match self {
            Self::Api { command_id, .. } => command_id.clone(),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        match value {
            rusqlite::Error::SqliteFailure(err, msg) => {
                let detail = msg.unwrap_or_else(|| err.to_string());
                Self::Database(detail)
            }
            other => Self::Database(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Internal(format!("Serialization error: {value}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(format!("I/O error: {value}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Self::Network("The request to the ER:LC API timed out.".into())
        } else if value.is_connect() {
            Self::Network("Could not reach the ER:LC API. Check your internet connection.".into())
        } else if value.is_decode() {
            Self::Api {
                status: None,
                message: "The ER:LC API returned an unexpected response format.".into(),
                code: None,
                command_id: None,
            }
        } else {
            Self::Network(format!("HTTP request failed: {value}"))
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    code: &'static str,
    message: String,
    retry_after: Option<u64>,
    status: Option<u16>,
    command_id: Option<String>,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = ErrorPayload {
            code: self.code(),
            message: self.message(),
            retry_after: self.retry_after(),
            status: self.http_status(),
            command_id: self.command_id(),
        };
        payload.serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_serializes_with_code() {
        let err = AppError::permission("Nope");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "permission_denied");
        assert_eq!(json["status"], 403);
        assert_eq!(json["message"], "Nope");
    }

    #[test]
    fn rate_limit_exposes_retry_after() {
        let err = AppError::RateLimited {
            message: "Slow down".into(),
            retry_after: Some(12),
        };
        assert_eq!(err.retry_after(), Some(12));
        assert_eq!(err.code(), "rate_limited");
    }
}
