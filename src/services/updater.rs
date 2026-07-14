use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;

use sha2::{Digest, Sha256};

use minisign_verify::{PublicKey, Signature};

use crate::config::{InstallMethod, read_install_marker};

const GITHUB_RELEASES: &str = "https://api.github.com/repos/doklabs/doktui/releases/latest";

/// Minisign public key (base64) for release verification. Set when releases are signed.
const RELEASE_PUBLIC_KEY: &str = "";

#[derive(Debug, Clone)]
pub struct UpdateNotice {
    pub current: String,
    pub latest: String,
    pub changelog: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub struct Updater;

impl Updater {
    pub async fn check_for_update(current: &str) -> Result<Option<UpdateNotice>> {
        let Some(release) = fetch_latest_release().await? else {
            return Ok(None);
        };
        Ok(compare_release(current, release))
    }

    pub fn package_manager_hint(method: InstallMethod) -> Option<&'static str> {
        match method {
            InstallMethod::Homebrew => Some("brew upgrade doktui"),
            InstallMethod::Winget => Some("winget upgrade doktui"),
            InstallMethod::Scoop => Some("scoop update doktui"),
            InstallMethod::Aur => Some("update via your AUR helper"),
            InstallMethod::Script | InstallMethod::Unknown => None,
        }
    }

    pub async fn self_update(current: &str) -> Result<()> {
        let method = read_install_marker();
        if let Some(hint) = Self::package_manager_hint(method) {
            bail!(
                "DokTUI was installed via a package manager.\nUpdate with: {hint}"
            );
        }

        let Some(release) = fetch_latest_release().await? else {
            println!("DokTUI v{current} — no published GitHub releases yet.");
            return Ok(());
        };

        let Some(notice) = compare_release(current, release.clone()) else {
            println!("DokTUI v{current} is already the latest published version.");
            return Ok(());
        };

        println!("Updating {} → {}", notice.current, notice.latest);

        let triple = release_target_triple();
        let binary_name = asset_binary_name(&triple);
        let checksum_name = format!("{binary_name}.sha256");

        let binary_asset = release
            .assets
            .iter()
            .find(|a| a.name == binary_name)
            .with_context(|| {
                format!(
                    "release {} has no asset `{binary_name}` for this platform ({triple})",
                    notice.latest
                )
            })?;

        let bytes = download_bytes(&binary_asset.browser_download_url).await?;
        if let Some(checksum_asset) = release.assets.iter().find(|a| a.name == checksum_name) {
            let checksum_text = download_text(&checksum_asset.browser_download_url).await?;
            let expected = checksum_text
                .lines()
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if !expected.is_empty() {
                verify_sha256(&bytes, expected)?;
                println!("SHA-256 verified.");
            }
        } else {
            tracing::warn!("no {checksum_name} sidecar — skipping checksum verification");
        }

        let sig_name = format!("{binary_name}.minisig");
        if !RELEASE_PUBLIC_KEY.is_empty() {
            if let Some(sig_asset) = release.assets.iter().find(|a| a.name == sig_name) {
                let sig_text = download_text(&sig_asset.browser_download_url).await?;
                verify_minisign(&bytes, &sig_text, RELEASE_PUBLIC_KEY)?;
                println!("Minisign signature verified.");
            } else {
                tracing::warn!("no {sig_name} sidecar — skipping minisign verification");
            }
        }

        swap_binary(&bytes)?;
        if !notice.changelog.is_empty() {
            println!("\nChangelog:\n{}", notice.changelog);
        }
        println!("Update complete. Restart DokTUI to use {}.", notice.latest);
        Ok(())
    }
}

fn compare_release(current: &str, release: GitHubRelease) -> Option<UpdateNotice> {
    let latest_tag = release.tag_name.trim_start_matches('v');
    let current_v = Version::parse(current).unwrap_or_else(|_| Version::new(0, 1, 0));
    let latest_v = Version::parse(latest_tag).unwrap_or_else(|_| Version::new(0, 0, 0));

    if latest_v > current_v {
        Some(UpdateNotice {
            current: current.to_string(),
            latest: release.tag_name,
            changelog: release.body.unwrap_or_default(),
        })
    } else {
        None
    }
}

/// Returns `Ok(None)` when the repo has no published releases yet (HTTP 404).
async fn fetch_latest_release() -> Result<Option<GitHubRelease>> {
    let client = reqwest::Client::builder()
        .user_agent("doktui-updater")
        .build()?;
    let resp = client.get(GITHUB_RELEASES).send().await?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        tracing::debug!("no GitHub releases published yet");
        return Ok(None);
    }

    if !resp.status().is_success() {
        bail!("release check failed: HTTP {}", resp.status());
    }

    Ok(Some(resp.json().await.context("failed to parse release metadata")?))
}

async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent("doktui-updater")
        .build()?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("failed to download release asset")?;
    if !resp.status().is_success() {
        bail!("download failed: HTTP {}", resp.status());
    }
    Ok(resp.bytes().await?.to_vec())
}

async fn download_text(url: &str) -> Result<String> {
    String::from_utf8(download_bytes(url).await?)
        .context("release metadata is not valid UTF-8")
}

fn release_target_triple() -> String {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu".into(),
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu".into(),
        ("macos", "x86_64") => "x86_64-apple-darwin".into(),
        ("macos", "aarch64") => "aarch64-apple-darwin".into(),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".into(),
        (os, arch) => format!("{arch}-unknown-{os}"),
    }
}

fn asset_binary_name(triple: &str) -> String {
    if cfg!(windows) {
        format!("doktui-{triple}.exe")
    } else {
        format!("doktui-{triple}")
    }
}

fn swap_binary(bytes: &[u8]) -> Result<()> {
    let current = std::env::current_exe().context("cannot resolve current executable path")?;
    let staging = current.with_extension("new");

    std::fs::write(&staging, bytes).context("failed to write staged binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&staging)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&staging, perms)?;
    }

    #[cfg(windows)]
    {
        let backup = current.with_extension("exe.old");
        let _ = std::fs::remove_file(&backup);
        if current.exists() {
            std::fs::rename(&current, &backup).context("failed to backup current binary")?;
        }
        std::fs::rename(&staging, &current).context("failed to activate new binary")?;
        let _ = std::fs::remove_file(backup);
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(&staging, &current).context("failed to replace binary")?;
    }

    Ok(())
}

/// Verify SHA-256 checksum of a release artifact.
pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let digest = Sha256::digest(bytes);
    let hex = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
    if hex.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        bail!("SHA-256 mismatch: expected {expected_hex}, got {hex}");
    }
}

/// Verify minisign signature when a `.minisig` file accompanies the release.
pub fn verify_minisign(bytes: &[u8], sig_text: &str, pubkey_b64: &str) -> Result<()> {
    if pubkey_b64.is_empty() {
        bail!("minisign release public key not configured");
    }
    let public_key =
        PublicKey::from_base64(pubkey_b64).context("invalid minisign public key encoding")?;
    let signature = Signature::decode(sig_text).context("invalid minisign signature file")?;
    public_key
        .verify(bytes, &signature, true)
        .context("minisign signature verification failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_release_triggers_notice() {
        let notice = compare_release(
            "0.1.0",
            GitHubRelease {
                tag_name: "v0.2.0".into(),
                body: Some("fixes".into()),
                assets: vec![],
            },
        );
        assert!(notice.is_some());
        assert_eq!(notice.unwrap().latest, "v0.2.0");
    }

    #[test]
    fn same_version_is_not_an_update() {
        let notice = compare_release(
            "0.1.0",
            GitHubRelease {
                tag_name: "v0.1.0".into(),
                body: None,
                assets: vec![],
            },
        );
        assert!(notice.is_none());
    }

    #[test]
    fn asset_name_includes_exe_on_windows() {
        let name = asset_binary_name("x86_64-pc-windows-msvc");
        if cfg!(windows) {
            assert!(name.ends_with(".exe"));
        } else {
            assert!(!name.contains(".exe"));
        }
    }
}
