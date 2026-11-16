//! Server key storage.
//!
//! * Windows: encrypted with DPAPI (`CryptProtectData`) using an application
//!   entropy value, then written to `%APPDATA%\com.erlccad.desktop\secrets`.
//! * Other platforms (development and automated tests only): a clearly labelled
//!   fallback file with owner-only permissions. The UI surfaces this state so it
//!   is never mistaken for DPAPI protection.

use crate::error::{AppError, AppResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const KEY_FILE_NAME: &str = "server-key.bin";

/// Binds the ciphertext to this application (DPAPI optional entropy).
const ENTROPY: &[u8] = b"erlc-cad/server-key/v1";

pub struct SecretStore {
    dir: PathBuf,
    storage: &'static str,
}

impl SecretStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            dir: app_data_dir.join("secrets"),
            storage: if cfg!(windows) {
                "windows_dpapi"
            } else {
                "file_fallback"
            },
        }
    }

    pub fn storage_kind(&self) -> &'static str {
        self.storage
    }

    fn file_path(&self) -> PathBuf {
        self.dir.join(KEY_FILE_NAME)
    }

    pub fn store(&self, key: &str) -> AppResult<()> {
        let trimmed = key.trim();
        if trimmed.len() < 8 {
            return Err(AppError::validation(
                "The server key looks too short. Copy it from your ER:LC server settings.",
            ));
        }
        if trimmed.len() > 512 || trimmed.contains(char::is_whitespace) {
            return Err(AppError::validation(
                "The server key must be a single token without spaces.",
            ));
        }
        let protected = platform::protect(trimmed.as_bytes())?;
        fs::create_dir_all(&self.dir)?;
        restrict_dir(&self.dir);
        let target = self.file_path();
        let temp = target.with_extension("tmp");
        fs::write(&temp, protected)?;
        restrict_file(&temp);
        fs::rename(&temp, &target)?;
        Ok(())
    }

    pub fn load(&self) -> AppResult<Option<String>> {
        let path = self.file_path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)?;
        if bytes.is_empty() {
            return Err(AppError::security(
                "The stored server key file is empty. Re-enter your server key.",
            ));
        }
        let plain = platform::unprotect(&bytes).map_err(|err| {
            AppError::security(format!(
                "The stored server key could not be decrypted ({}). Re-enter it in API Configuration.",
                err.message()
            ))
        })?;
        let text = String::from_utf8(plain)
            .map_err(|_| AppError::security("The stored server key is not valid UTF-8."))?;
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    }

    pub fn clear(&self) -> AppResult<()> {
        let path = self.file_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn exists(&self) -> bool {
        self.file_path().exists()
    }
}

/// Non-reversible fingerprint used to confirm which key is stored.
pub fn fingerprint(key: &str) -> String {
    let digest = Sha256::digest(key.trim().as_bytes());
    digest[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// Display-safe mask: never returns more than the final four characters.
pub fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    let last4: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("••••{last4}")
}

fn restrict_dir(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
}

fn restrict_file(file: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(file, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = file;
    }
}

#[cfg(windows)]
mod platform {
    use crate::error::{AppError, AppResult};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    pub fn protect(plain: &[u8]) -> AppResult<Vec<u8>> {
        let entropy = super::ENTROPY;
        let input = blob(plain);
        let entropy_blob = blob(entropy);
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &entropy_blob,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(AppError::security(format!(
                "Windows DPAPI could not encrypt the server key: {}",
                std::io::Error::last_os_error()
            )));
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData as *mut core::ffi::c_void);
        }
        Ok(bytes)
    }

    pub fn unprotect(cipher: &[u8]) -> AppResult<Vec<u8>> {
        let entropy = super::ENTROPY;
        let input = blob(cipher);
        let entropy_blob = blob(entropy);
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &entropy_blob,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(AppError::security(format!(
                "Windows DPAPI could not decrypt the server key: {}",
                std::io::Error::last_os_error()
            )));
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData as *mut core::ffi::c_void);
        }
        Ok(bytes)
    }
}

#[cfg(not(windows))]
mod platform {
    use crate::error::{AppError, AppResult};
    use base64::Engine;

    const MARKER: &[u8] = b"ERLCCAD-FALLBACK-V1:";

    pub fn protect(plain: &[u8]) -> AppResult<Vec<u8>> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(plain);
        let mut out = MARKER.to_vec();
        out.extend_from_slice(encoded.as_bytes());
        Ok(out)
    }

    pub fn unprotect(cipher: &[u8]) -> AppResult<Vec<u8>> {
        if !cipher.starts_with(MARKER) {
            return Err(AppError::security(
                "This key was protected with Windows DPAPI and can only be read on Windows.",
            ));
        }
        let encoded = std::str::from_utf8(&cipher[MARKER.len()..])
            .map_err(|_| AppError::security("Corrupt key file."))?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| AppError::security("Corrupt key file."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "erlc-cad-test-{}-{}-{}",
                name,
                std::process::id(),
                crate::util::now_ms()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn round_trips_a_key_and_clears_it() {
        let dir = TempDir::new("secrets");
        let store = SecretStore::new(&dir.0);
        assert!(store.load().unwrap().is_none());
        store.store("abcdef1234567890").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("abcdef1234567890"));
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn rejects_short_or_spaced_keys() {
        let dir = TempDir::new("secrets-invalid");
        let store = SecretStore::new(&dir.0);
        assert!(store.store("short").is_err());
        assert!(store.store("has a space inside").is_err());
    }

    #[test]
    fn stored_file_does_not_contain_the_plaintext_key() {
        let dir = TempDir::new("secrets-cipher");
        let store = SecretStore::new(&dir.0);
        store.store("super-secret-key-value").unwrap();
        let bytes = fs::read(dir.0.join("secrets").join(KEY_FILE_NAME)).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("super-secret-key-value"));
        assert!(fingerprint("super-secret-key-value").len() == 16);
    }

    #[test]
    fn mask_only_reveals_last_four_characters() {
        assert_eq!(mask_key("0123456789"), "••••6789");
    }
}
