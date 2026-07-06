use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use russh_keys::key::PublicKey;

use crate::config::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyAction {
    AcceptNew,
    AlreadyKnown,
    Changed { old: String, new: String },
}

pub struct KnownHosts {
    entries: HashMap<String, String>,
    path: std::path::PathBuf,
}

impl KnownHosts {
    pub fn load() -> Result<Self> {
        let path = paths::known_hosts_path()?;
        let entries = if path.exists() {
            parse_known_hosts(&fs::read_to_string(&path)?)?
        } else {
            HashMap::new()
        };
        Ok(Self { entries, path })
    }

    pub fn verify(&self, host: &str, port: u16, key: &PublicKey) -> Result<HostKeyAction> {
        let fingerprint = key.fingerprint();
        let lookup = format!("{host}:{port}");

        match self.entries.get(&lookup) {
            None => Ok(HostKeyAction::AcceptNew),
            Some(stored) if stored == &fingerprint => Ok(HostKeyAction::AlreadyKnown),
            Some(stored) => Ok(HostKeyAction::Changed {
                old: stored.clone(),
                new: fingerprint,
            }),
        }
    }

    pub fn trust(&mut self, host: &str, port: u16, key: &PublicKey) -> Result<()> {
        let fingerprint = key.fingerprint();
        let lookup = format!("{host}:{port}");
        self.entries.insert(lookup, fingerprint);
        self.save()
    }

    pub fn trust_fingerprint(&mut self, host: &str, port: u16, fingerprint: &str) -> Result<()> {
        let lookup = format!("{host}:{port}");
        self.entries.insert(lookup, fingerprint.to_string());
        self.save()
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut lines: Vec<String> = self
            .entries
            .iter()
            .map(|(host, fp)| format!("{host} {fp}"))
            .collect();
        lines.sort();
        fs::write(&self.path, lines.join("\n") + "\n")
            .with_context(|| format!("failed to write known_hosts to {}", self.path.display()))
    }
}

fn parse_known_hosts(content: &str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let host = parts.next().context("invalid known_hosts line")?;
        let fp = parts.next().context("invalid known_hosts line")?;
        map.insert(host.to_string(), fp.to_string());
    }
    Ok(map)
}

pub fn require_trust(action: HostKeyAction) -> Result<()> {
    match action {
        HostKeyAction::Changed { old, new } => {
            bail!(
                "host key fingerprint changed (possible MITM)\n  was: {old}\n  now: {new}\nRemove the entry from known_hosts to re-trust."
            );
        }
        _ => Ok(()),
    }
}

pub fn host_label(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_label_omits_default_ssh_port() {
        assert_eq!(host_label("example.com", 22), "example.com");
        assert_eq!(host_label("example.com", 2222), "example.com:2222");
    }
}

#[allow(dead_code)]
pub fn known_hosts_exists() -> bool {
    paths::known_hosts_path()
        .map(|p| Path::new(&p).exists())
        .unwrap_or(false)
}
