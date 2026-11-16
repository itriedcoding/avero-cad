//! Small shared helpers: timestamps, text validation, secret redaction, logging.

use crate::error::{AppError, AppResult};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Fresh, process-unique temporary directory for tests.
///
/// A monotonic counter avoids the millisecond collisions that made parallel
/// tests share (and delete) each other's database directories.
#[cfg(test)]
pub fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "erlc-cad-{prefix}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test temp dir");
    path
}

/// Replace anything that looks like a credential with a fixed marker.
///
/// Handles both `server-key: VALUE` and `server-key VALUE` shapes so a secret
/// can never end up in the log file.
pub fn redact(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut hide_next = false;
    for token in input.split_inclusive(|c: char| c.is_whitespace() || c == '&' || c == '?') {
        let core = token.trim_end_matches(|c: char| c.is_whitespace() || c == '&' || c == '?');
        let trailing = &token[core.len()..];
        let lower = core.to_ascii_lowercase();
        let sensitive = lower.contains("server-key")
            || lower.contains("server_key")
            || lower.contains("api-key")
            || lower.contains("api_key")
            || lower.contains("authorization")
            || lower.contains("bearer");

        if hide_next {
            out.push_str("[redacted]");
            hide_next = false;
        } else if sensitive {
            match core.find(':') {
                Some(index) => {
                    let (head, value) = core.split_at(index + 1);
                    out.push_str(head);
                    if value.trim().is_empty() {
                        hide_next = true;
                    } else {
                        out.push_str("[redacted]");
                    }
                }
                None => {
                    out.push_str(core);
                    hide_next = true;
                }
            }
        } else {
            out.push_str(core);
        }
        out.push_str(trailing);
    }
    out
}

/// Truncate user supplied text to a maximum length (in characters).
pub fn clamp_text(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        trimmed.chars().take(max_chars).collect()
    }
}

pub fn require_text(value: &str, field: &str, min: usize, max: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.chars().count() < min {
        return Err(AppError::validation(format!(
            "{field} must be at least {min} characters."
        )));
    }
    if trimmed.chars().count() > max {
        return Err(AppError::validation(format!(
            "{field} must be {max} characters or fewer."
        )));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') || trimmed.chars().any(|c| c == '\0') {
        return Err(AppError::validation(format!(
            "{field} must be a single line."
        )));
    }
    Ok(trimmed.to_string())
}

/// ER:LC usernames are 3-20 characters of letters, digits and underscores.
pub fn is_valid_roblox_username(value: &str) -> bool {
    (3..=20).contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn is_valid_username(value: &str) -> bool {
    (3..=32).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Append a line to the application log. Never pass credentials into this.
pub fn log_line(log_path: &Path, level: &str, message: &str) {
    let line = format!(
        "{} [{level}] {}\n",
        chrono::Utc::now().to_rfc3339(),
        redact(message)
    );
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// Format bytes as a short human readable string.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_headers_but_keeps_context() {
        let line = "GET /v2/server server-key: abc123def failed";
        let redacted = redact(line);
        assert!(!redacted.contains("abc123def"));
        assert!(redacted.contains("GET /v2/server"));
    }

    #[test]
    fn validation_rejects_multiline() {
        assert!(require_text("ok", "Reason", 2, 10).is_ok());
        assert!(require_text("a\nb", "Reason", 1, 10).is_err());
        assert!(require_text("", "Reason", 1, 10).is_err());
        assert!(require_text("01234567890", "Reason", 1, 10).is_err());
    }

    #[test]
    fn roblox_username_rules() {
        assert!(is_valid_roblox_username("Builderman"));
        assert!(!is_valid_roblox_username("has space"));
        assert!(!is_valid_roblox_username("ab"));
    }

    #[test]
    fn clamp_counts_characters() {
        assert_eq!(clamp_text("  hello  ", 10), "hello");
        assert_eq!(clamp_text("abcdef", 3), "abc");
    }
}
