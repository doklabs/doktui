use std::path::PathBuf;

use anyhow::{Context, Result};

/// Resolve the DokTUI config directory for the current platform.
pub fn config_dir() -> Result<PathBuf> {
    directories::ProjectDirs::from("com", "doklabs", "doktui")
        .map(|d| d.config_dir().to_path_buf())
        .context("could not determine config directory")
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    directories::ProjectDirs::from("com", "doklabs", "doktui")
        .map(|d| d.data_dir().to_path_buf())
        .context("could not determine data directory")
}

pub fn ssh_key_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("doktui_key"))
}

pub fn ssh_key_pub_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("doktui_key.pub"))
}

pub fn known_hosts_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("known_hosts"))
}

pub fn secrets_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("secrets.enc"))
}

pub fn install_marker_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("install_method"))
}

pub fn themes_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("themes"))
}

pub fn locales_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("locales"))
}

pub fn ensure_dirs() -> Result<()> {
    let config = config_dir()?;
    let data = data_dir()?;
    std::fs::create_dir_all(&config)?;
    std::fs::create_dir_all(&data)?;
    let _ = std::fs::create_dir_all(themes_dir()?);
    let _ = std::fs::create_dir_all(locales_dir()?);
    Ok(())
}
