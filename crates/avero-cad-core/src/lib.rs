//! Core library for the Avero CAD desktop application.
//!
//! Everything that touches the ER:LC API, the SQLite database, credentials or
//! permissions lives here so it can be compiled and tested independently of the
//! Tauri/webview shell.

pub mod api;
pub mod db;
pub mod error;
pub mod models;
pub mod permissions;
pub mod roblox;
pub mod security;
pub mod services;
pub mod util;

pub use error::AppError;
