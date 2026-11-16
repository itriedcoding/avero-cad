//! Credential handling.
//!
//! The ER:LC server key is the most sensitive value in the application: it can
//! execute commands on a live server. It is protected with Windows DPAPI
//! (per-user, per-machine encryption tied to the Windows account) and is never
//! written to the database, the log file or the frontend.

pub mod passwords;
pub mod secrets;

pub use passwords::{
    hash_password, new_session_token, tokens_equal, validate_password_strength, verify_password,
};
pub use secrets::{fingerprint, mask_key, SecretStore};
