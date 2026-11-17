//! Password hashing (Argon2id) and session token generation.

use crate::error::{AppError, AppResult};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;

/// Minimum local account password policy.
pub fn validate_password_strength(password: &str) -> AppResult<()> {
    if password.chars().count() < 10 {
        return Err(AppError::validation(
            "Use at least 10 characters for your password.",
        ));
    }
    if password.chars().count() > 200 {
        return Err(AppError::validation("Password is too long."));
    }
    if !password.chars().any(|c| c.is_alphabetic()) {
        return Err(AppError::validation(
            "Your password must contain at least one letter.",
        ));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(AppError::validation(
            "Your password must contain at least one number.",
        ));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> AppResult<String> {
    validate_password_strength(password)?;
    let mut salt_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AppError::security(format!("Could not generate a password salt: {e}")))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::security(format!("Could not hash the password: {e}")))
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// 256-bit session token rendered as hex.
pub fn new_session_token() -> AppResult<String> {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    if bytes.iter().all(|b| *b == 0) {
        return Err(AppError::security("Random number generator failure."));
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Constant-time comparison for secrets.
pub fn tokens_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.as_bytes().iter().zip(b.as_bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies() {
        let hash = hash_password("StrongPass123").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password(&hash, "StrongPass123"));
        assert!(!verify_password(&hash, "wrong-password"));
        assert!(!verify_password("not-a-hash", "StrongPass123"));
    }

    #[test]
    fn enforces_password_policy() {
        assert!(validate_password_strength("short1").is_err());
        assert!(validate_password_strength("alllettershere").is_err());
        assert!(validate_password_strength("1234567890").is_err());
        assert!(validate_password_strength("GoodPassw0rd").is_ok());
    }

    #[test]
    fn tokens_are_unique_and_long() {
        let a = new_session_token().unwrap();
        let b = new_session_token().unwrap();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(tokens_equal(&a, &a));
        assert!(!tokens_equal(&a, &b));
    }
}
