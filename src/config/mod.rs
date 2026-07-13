pub mod paths;

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::security::keys;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EditorMode {
    Vim,
    Normal,
}

impl Default for EditorMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    Overlay,
    Compact,
}

impl Default for UiMode {
    fn default() -> Self {
        Self::Overlay
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcmeChallenge {
    #[default]
    Http01,
    DnsCloudflare,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CronAction {
    RestartContainer { container: String },
    Redeploy { remote_dir: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: Uuid,
    pub label: String,
    pub server_id: Uuid,
    /// Standard cron expression (6 fields: sec min hour dom month dow), e.g. `0 0 3 * * *`.
    pub expression: String,
    pub action: CronAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_run: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallMethod {
    Script,
    Homebrew,
    Winget,
    Scoop,
    Aur,
    Unknown,
}

impl Default for InstallMethod {
    fn default() -> Self {
        Self::Script
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub docker_installed: bool,
    pub traefik_installed: bool,
}

impl ServerConfig {
    pub fn new(name: String, host: String, port: u16, user: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            host,
            port,
            user,
            docker_installed: false,
            traefik_installed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub editor_mode: EditorMode,
    #[serde(default)]
    pub ui_mode: UiMode,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub check_updates: bool,
    #[serde(default)]
    pub install_method: InstallMethod,
    #[serde(default = "default_acme_email")]
    pub acme_email: String,
    #[serde(default)]
    pub acme_challenge: AcmeChallenge,
    #[serde(default)]
    pub cron_jobs: Vec<CronJob>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
    #[serde(default = "default_true")]
    pub mouse: bool,
    #[serde(default)]
    pub onboarding_complete: bool,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

fn default_true() -> bool {
    true
}

fn default_acme_email() -> String {
    "admin@example.com".into()
}

fn default_theme() -> String {
    "gruvbox-material".into()
}

fn default_locale() -> String {
    "en".into()
}

fn default_sidebar_width() -> u16 {
    22
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            editor_mode: EditorMode::default(),
            ui_mode: UiMode::default(),
            auto_reconnect: true,
            check_updates: true,
            install_method: InstallMethod::default(),
            acme_email: default_acme_email(),
            acme_challenge: AcmeChallenge::default(),
            cron_jobs: Vec::new(),
            theme: default_theme(),
            locale: default_locale(),
            sidebar_width: default_sidebar_width(),
            mouse: true,
            onboarding_complete: false,
            servers: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        paths::ensure_dirs()?;
        let path = paths::config_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        match toml::from_str(&content) {
            Ok(cfg) => Ok(cfg),
            Err(e) => {
                tracing::warn!("config corrupt, using defaults: {e}");
                Ok(Self::default())
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        paths::ensure_dirs()?;
        let path = paths::config_file()?;
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write config to {}", path.display()))
    }

    pub fn server_mut(&mut self, id: Uuid) -> Option<&mut ServerConfig> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    pub fn server(&self, id: Uuid) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }
}

pub fn bootstrap() -> Result<AppConfig> {
    paths::ensure_dirs()?;
    keys::ensure_keypair()?;
    if let Ok(marker_path) = paths::install_marker_path() {
        if !Path::new(&marker_path).exists() {
            write_install_marker(InstallMethod::Script)?;
        }
    }
    AppConfig::load()
}

/// Wrapper so the install marker serializes as a valid TOML table (`method = "script"`).
#[derive(Debug, Serialize, Deserialize)]
struct InstallMarker {
    method: InstallMethod,
}

pub fn write_install_marker(method: InstallMethod) -> Result<()> {
    let path = paths::install_marker_path()?;
    let marker = InstallMarker { method };
    let content = toml::to_string(&marker)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn read_install_marker() -> InstallMethod {
    let Ok(path) = paths::install_marker_path() else {
        return InstallMethod::Unknown;
    };
    if !Path::new(&path).exists() {
        return InstallMethod::Script;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return InstallMethod::Unknown;
    };
    toml::from_str::<InstallMarker>(&content)
        .map(|m| m.method)
        .unwrap_or(InstallMethod::Unknown)
}
