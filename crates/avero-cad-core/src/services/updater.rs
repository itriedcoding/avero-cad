//! Updates delivered through GitHub Releases.
//!
//! The updater reads the repository's latest published release from the public
//! GitHub API, compares its tag with the running build and can download the
//! NSIS installer, verify it against the SHA-256 digest GitHub reports for the
//! asset (falling back to a `SHA256SUMS.txt` asset when a digest is missing) and
//! start it. Nothing is installed unless the digest matches byte for byte.
//!
//! The check is read-only; installing requires `settings.manage`, an explicit
//! confirmation and is recorded in the audit log.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::error::{AppError, AppResult};
use crate::models::{UpdateCheckView, UpdateInstallView};
use crate::permissions::{Access, Permission};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

const RELEASES_API: &str = "https://api.github.com/repos/itriedcoding/avero-cad/releases/latest";
const RELEASE_PREFIX: &str = "https://github.com/itriedcoding/avero-cad/releases/download/";
const SUMS_ASSET: &str = "SHA256SUMS.txt";
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

fn http_client() -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(format!("AveroCAD/{}", current_version()))
        .build()
        .map_err(|err| AppError::network(format!("Could not initialise the update client: {err}")))
}

/// The version of the running build.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

async fn latest_release(client: &Client) -> AppResult<GhRelease> {
    let response = client
        .get(RELEASES_API)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|err| AppError::network(format!("GitHub Releases could not be reached: {err}")))?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::not_found(
            "No published release was found for this repository.",
        ));
    }
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        let detail = detail.chars().take(200).collect::<String>();
        return Err(AppError::network(format!(
            "GitHub Releases answered HTTP {}. {}",
            status.as_u16(),
            if detail.trim().is_empty() {
                "The check will be retried later."
            } else {
                detail.as_str()
            }
        )));
    }
    response
        .json::<GhRelease>()
        .await
        .map_err(|err| AppError::network(format!("GitHub returned an unreadable release: {err}")))
}

/// Parse `v1.2.3`, `1.2` or `1` into numeric components.
pub fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    let core = trimmed.split(['-', '+']).next().unwrap_or(trimmed);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn select_installer(assets: &[GhAsset]) -> Option<&GhAsset> {
    assets
        .iter()
        .find(|asset| asset.name.to_ascii_lowercase().ends_with("setup.exe"))
        .or_else(|| {
            assets
                .iter()
                .find(|asset| asset.name.to_ascii_lowercase().ends_with(".msi"))
        })
}

fn digest_checksum(digest: &str) -> Option<String> {
    let hex = digest.trim().strip_prefix("sha256:")?;
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

/// Read `"<sha256>  <filename>"` lines and return the checksum for `name`.
pub fn checksum_from_sums(text: &str, name: &str) -> Option<String> {
    let wanted = name.trim().to_ascii_lowercase();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(hex) = parts.next() else { continue };
        let Some(file) = parts.next() else { continue };
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let file = file.trim_start_matches('*').to_ascii_lowercase();
        if file == wanted {
            return Some(hex.to_ascii_lowercase());
        }
    }
    None
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    if cleaned.is_empty() {
        "avero-cad-update.exe".to_string()
    } else {
        cleaned
    }
}

async fn download(client: &Client, url: &str) -> AppResult<Vec<u8>> {
    if !url.starts_with(RELEASE_PREFIX) {
        return Err(AppError::validation(
            "The update asset is not hosted on this project's GitHub release.",
        ));
    }
    let response = client.get(url).send().await.map_err(|err| {
        AppError::network(format!("The installer could not be downloaded: {err}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::network(format!(
            "The installer download failed with HTTP {}.",
            status.as_u16()
        )));
    }
    if let Some(length) = response.content_length() {
        if length > MAX_DOWNLOAD_BYTES {
            return Err(AppError::validation(
                "The update download is unexpectedly large.",
            ));
        }
    }
    let bytes = response.bytes().await.map_err(|err| {
        AppError::network(format!("The installer download was interrupted: {err}"))
    })?;
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(AppError::validation(
            "The update download is unexpectedly large.",
        ));
    }
    Ok(bytes.to_vec())
}

async fn expected_checksum(
    client: &Client,
    release: &GhRelease,
    asset: &GhAsset,
) -> AppResult<String> {
    if let Some(digest) = asset.digest.as_deref() {
        if let Some(hex) = digest_checksum(digest) {
            return Ok(hex);
        }
    }
    let sums = release
        .assets
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(SUMS_ASSET))
        .ok_or_else(|| {
            AppError::validation(
                "This release does not publish a SHA-256 digest for its installer, so it cannot be verified.",
            )
        })?;
    let text = download(client, &sums.browser_download_url).await?;
    let text = String::from_utf8_lossy(&text);
    checksum_from_sums(&text, &asset.name).ok_or_else(|| {
        AppError::validation(format!(
            "The release's checksum file does not list {}.",
            asset.name
        ))
    })
}

fn update_path(ctx: &AppContext, name: &str) -> AppResult<PathBuf> {
    let directory = ctx.paths.data_dir.join("updates");
    std::fs::create_dir_all(&directory)?;
    Ok(directory.join(safe_file_name(name)))
}

pub async fn check(ctx: &AppContext, access: &Access) -> AppResult<UpdateCheckView> {
    access.require(Permission::SettingsManage)?;
    let client = http_client()?;
    let current = current_version().to_string();
    let release = match latest_release(&client).await {
        Ok(release) => release,
        Err(err) => {
            return Ok(UpdateCheckView {
                current_version: current,
                latest_version: None,
                available: false,
                release_url: None,
                published_at: None,
                notes: None,
                asset_name: None,
                asset_size: None,
                can_install: false,
                detail: err.message().to_string(),
            });
        }
    };
    let latest = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    let asset = select_installer(&release.assets);
    let available = !release.draft && !release.prerelease && is_newer(&release.tag_name, &current);
    let detail = if available {
        format!("Version {latest} is available.")
    } else if release.prerelease || release.draft {
        "The latest release is a draft or pre-release.".to_string()
    } else if parse_version(&release.tag_name).is_none() {
        format!(
            "The latest release tag \"{}\" is not a version number.",
            release.tag_name
        )
    } else {
        format!("You are running the newest published version ({current}).")
    };
    let _ = ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "system",
        action: "update.checked",
        target: &latest,
        details: "",
    });
    Ok(UpdateCheckView {
        current_version: current,
        latest_version: Some(latest),
        available,
        can_install: available && asset.is_some(),
        release_url: Some(release.html_url),
        published_at: release.published_at,
        notes: release.body.map(|body| body.chars().take(4000).collect()),
        asset_name: asset.map(|asset| asset.name.clone()),
        asset_size: asset.map(|asset| asset.size),
        detail,
    })
}

pub async fn install(
    ctx: &AppContext,
    access: &Access,
    confirmed: bool,
) -> AppResult<UpdateInstallView> {
    access.require(Permission::SettingsManage)?;
    if !confirmed {
        return Err(AppError::validation(
            "Confirm that you want to download and start the installer.",
        ));
    }
    if !cfg!(windows) {
        return Err(AppError::validation(
            "Automatic updates are only available in the Windows build.",
        ));
    }
    let client = http_client()?;
    let release = latest_release(&client).await?;
    if release.draft || release.prerelease {
        return Err(AppError::conflict(
            "Draft and pre-release builds are never installed automatically.",
        ));
    }
    if !is_newer(&release.tag_name, current_version()) {
        return Err(AppError::conflict(format!(
            "This installation is already on the newest published version ({}).",
            current_version()
        )));
    }
    let version = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    let asset = select_installer(&release.assets).ok_or_else(|| {
        AppError::not_found("The latest release has no Windows installer attached.")
    })?;
    let expected = expected_checksum(&client, &release, asset).await?;
    let bytes = download(&client, &asset.browser_download_url).await?;
    if bytes.len() as u64 != asset.size {
        return Err(AppError::validation(
            "The downloaded installer size does not match the release asset, so it was discarded.",
        ));
    }
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(AppError::validation(format!(
            "The installer failed its SHA-256 check (expected {expected}, got {actual}) and was discarded."
        )));
    }
    let path = update_path(ctx, &asset.name)?;
    std::fs::write(&path, &bytes)?;
    ctx.log(
        "info",
        &format!(
            "Update {version} downloaded and verified ({} bytes, sha256 {actual})",
            bytes.len()
        ),
    );
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "system",
        action: "update.verified",
        target: &version,
        details: &format!("sha256 {actual}"),
    })?;
    spawn_installer(&path)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "system",
        action: "update.installer_started",
        target: &version,
        details: &path.display().to_string(),
    })?;
    Ok(UpdateInstallView {
        version,
        installer_path: path.display().to_string(),
        sha256: actual,
        size_bytes: bytes.len() as u64,
    })
}

#[cfg(windows)]
fn spawn_installer(path: &Path) -> AppResult<()> {
    std::process::Command::new(path)
        .arg("/S")
        .spawn()
        .map(|_| ())
        .map_err(|err| {
            AppError::internal(format!(
                "The installer was verified but Windows could not start it: {err}"
            ))
        })
}

#[cfg(not(windows))]
fn spawn_installer(_path: &Path) -> AppResult<()> {
    Err(AppError::validation(
        "Automatic updates are only available in the Windows build.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, digest: Option<&str>) -> GhAsset {
        GhAsset {
            name: name.to_string(),
            size: 1024,
            browser_download_url: format!("{RELEASE_PREFIX}v9.9.9/{name}"),
            digest: digest.map(str::to_string),
        }
    }

    #[test]
    fn parses_versions_with_and_without_prefixes() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("2"), Some((2, 0, 0)));
        assert_eq!(parse_version("v1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("nightly"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("v1.2.0", "1.1.0"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(!is_newer("1.1.0", "1.1.0"));
        assert!(!is_newer("1.0.9", "1.1.0"));
        assert!(!is_newer("latest", "1.1.0"));
    }

    #[test]
    fn accepts_github_digests_only() {
        let hex = "a".repeat(64);
        assert_eq!(digest_checksum(&format!("sha256:{hex}")), Some(hex.clone()));
        assert_eq!(digest_checksum("sha512:abc"), None);
        assert_eq!(digest_checksum("sha256:xyz"), None);
    }

    #[test]
    fn reads_checksums_from_a_sums_file() {
        let hex = "b".repeat(64);
        let text = format!("{hex}  Avero.CAD_1.2.0_x64-setup.exe\nccc  other.exe\n");
        assert_eq!(
            checksum_from_sums(&text, "Avero.CAD_1.2.0_x64-setup.exe").as_deref(),
            Some(hex.as_str())
        );
        assert!(checksum_from_sums(&text, "missing.exe").is_none());
        // Binary-mode markers are tolerated.
        let marked = format!("{hex} *installer.exe");
        assert!(checksum_from_sums(&marked, "installer.exe").is_some());
    }

    #[test]
    fn prefers_the_nsis_installer_and_falls_back_to_msi() {
        let assets = vec![asset("notes.txt", None), asset("app.msi", None)];
        assert_eq!(
            select_installer(&assets).map(|a| a.name.as_str()),
            Some("app.msi")
        );
        let assets = vec![
            asset("app.msi", None),
            asset("Avero.CAD_9_x64-setup.exe", None),
        ];
        assert_eq!(
            select_installer(&assets).map(|a| a.name.as_str()),
            Some("Avero.CAD_9_x64-setup.exe")
        );
        assert!(select_installer(&[]).is_none());
    }

    #[test]
    fn parses_the_published_release_payload() {
        // Shape captured from the live v1.2.0 release, including asset digests.
        let json = r#"{
          "tag_name": "v1.2.0",
          "html_url": "https://github.com/itriedcoding/avero-cad/releases/tag/v1.2.0",
          "published_at": "2026-11-17T04:45:54Z",
          "body": "Windows x64 installers for Avero CAD 1.2.0.",
          "draft": false,
          "prerelease": false,
          "assets": [
            {
              "name": "Avero.CAD_1.2.0_x64-setup.exe",
              "size": 2488532,
              "browser_download_url": "https://github.com/itriedcoding/avero-cad/releases/download/v1.2.0/Avero.CAD_1.2.0_x64-setup.exe",
              "digest": "sha256:a1a24c5cf98a904e6b32f1b54a479137a38a9b59dcdaec7c3a97c9f8b4417379"
            },
            {
              "name": "Avero.CAD_1.2.0_x64_en-US.msi",
              "size": 3297280,
              "browser_download_url": "https://github.com/itriedcoding/avero-cad/releases/download/v1.2.0/Avero.CAD_1.2.0_x64_en-US.msi",
              "digest": "sha256:80537d2763df52be43f36da71b1781f1b86c228511a6b4702fa9ba3d18c3fe01"
            }
          ]
        }"#;
        let release: GhRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
        assert!(!release.draft && !release.prerelease);
        let asset = select_installer(&release.assets).unwrap();
        assert!(asset.name.ends_with("-setup.exe"));
        assert_eq!(asset.size, 2_488_532);
        assert_eq!(
            digest_checksum(asset.digest.as_deref().unwrap()).as_deref(),
            Some("a1a24c5cf98a904e6b32f1b54a479137a38a9b59dcdaec7c3a97c9f8b4417379")
        );
        assert!(is_newer(&release.tag_name, "1.1.0"));
        assert!(!is_newer(&release.tag_name, "1.2.0"));
    }

    #[test]
    fn hashes_bytes_and_sanitises_file_names() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(safe_file_name("../../evil name.exe"), "....evilname.exe");
        assert_eq!(safe_file_name(""), "avero-cad-update.exe");
    }
}
