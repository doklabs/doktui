//! GitHub OAuth Device Flow for connecting accounts in DokTUI.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const SCOPE: &str = "repo read:user";

#[derive(Debug, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeApi {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenApi {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    interval: Option<u64>,
}

/// Start Device Flow — returns codes the UI should display.
pub async fn request_device_code(client_id: &str) -> Result<DeviceCodeResponse> {
    if client_id.trim().is_empty() {
        bail!(
            "GitHub OAuth client ID is not set. Set github_oauth_client_id in config.toml \
             or DOKTUI_GITHUB_CLIENT_ID (create an OAuth App with Device Flow enabled)."
        );
    }
    let client = reqwest::Client::builder()
        .user_agent("doktui")
        .build()
        .context("failed to build OAuth HTTP client")?;
    let resp = client
        .post(DEVICE_CODE_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await
        .context("failed to reach GitHub device code endpoint")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("GitHub device code error ({status}): {}", body.trim());
    }
    let parsed: DeviceCodeApi =
        serde_json::from_str(&body).context("invalid device code JSON from GitHub")?;
    Ok(DeviceCodeResponse {
        device_code: parsed.device_code,
        user_code: parsed.user_code,
        verification_uri: parsed.verification_uri,
        expires_in: parsed.expires_in,
        interval: parsed.interval.max(5),
    })
}

#[derive(Debug)]
pub enum PollResult {
    Pending,
    SlowDown(u64),
    AccessToken(String),
    Expired,
    Denied,
    Error(String),
}

/// One poll attempt for an access token.
pub async fn poll_access_token(
    client_id: &str,
    device_code: &str,
) -> Result<PollResult> {
    let client = reqwest::Client::builder()
        .user_agent("doktui")
        .build()
        .context("failed to build OAuth HTTP client")?;
    let resp = client
        .post(ACCESS_TOKEN_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .context("failed to poll GitHub access token")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() && status.as_u16() != 400 {
        bail!("GitHub token poll error ({status}): {}", body.trim());
    }
    let parsed: TokenApi =
        serde_json::from_str(&body).context("invalid access token JSON from GitHub")?;
    if let Some(token) = parsed.access_token.filter(|t| !t.is_empty()) {
        return Ok(PollResult::AccessToken(token));
    }
    Ok(match parsed.error.as_deref() {
        Some("authorization_pending") | None => PollResult::Pending,
        Some("slow_down") => PollResult::SlowDown(parsed.interval.unwrap_or(10)),
        Some("expired_token") => PollResult::Expired,
        Some("access_denied") => PollResult::Denied,
        Some(other) => PollResult::Error(
            parsed
                .error_description
                .unwrap_or_else(|| other.to_string()),
        ),
    })
}

/// Poll until token, expiry, or denial. Returns access token.
pub async fn wait_for_token(
    client_id: &str,
    device: &DeviceCodeResponse,
) -> Result<String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(device.expires_in);
    let mut interval = Duration::from_secs(device.interval.max(5));
    loop {
        if tokio::time::Instant::now() >= deadline {
            bail!("GitHub device code expired — try Connect again");
        }
        tokio::time::sleep(interval).await;
        match poll_access_token(client_id, &device.device_code).await? {
            PollResult::AccessToken(t) => return Ok(t),
            PollResult::Pending => {}
            PollResult::SlowDown(secs) => {
                interval = Duration::from_secs(secs.max(5));
            }
            PollResult::Expired => bail!("GitHub device code expired — try Connect again"),
            PollResult::Denied => bail!("GitHub authorization was denied"),
            PollResult::Error(e) => bail!("GitHub OAuth error: {e}"),
        }
    }
}

/// Best-effort open verification URL in the system browser.
pub fn open_browser(url: &str) -> Result<()> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.spawn().context("failed to open browser")?;
    Ok(())
}
