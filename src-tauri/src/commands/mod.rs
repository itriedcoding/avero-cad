//! Thin Tauri command layer.
//!
//! Every command authenticates the caller, enforces permissions inside the Rust
//! core and only then calls a service. The frontend never performs privileged
//! work on its own.

pub mod auth;
pub mod console;
pub mod dispatch;
pub mod moderation;
pub mod operations;
pub mod players;
pub mod settings;
pub mod staff;
