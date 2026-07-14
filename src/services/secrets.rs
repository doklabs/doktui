use std::collections::HashMap;

use anyhow::{Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::config::paths;

const NONCE_LEN: usize = 12;

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecretStore {
    values: HashMap<String, String>,
}

pub struct SecretsManager {
    cipher: ChaCha20Poly1305,
    store: SecretStore,
}

impl SecretsManager {
    pub fn load_or_create() -> Result<Self> {
        paths::ensure_dirs()?;
        let key = load_or_create_master_key()?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key).context("invalid master key length")?;

        let path = paths::secrets_path()?;
        let store = if path.exists() {
            let blob = std::fs::read(&path)?;
            decrypt_store(&cipher, &blob)?
        } else {
            SecretStore::default()
        };

        Ok(Self { cipher, store })
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        self.store.values.insert(key.to_string(), value.to_string());
        self.save()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.values.get(key).map(String::as_str)
    }

    pub fn remove(&mut self, key: &str) -> Result<()> {
        self.store.values.remove(key);
        self.save()
    }

    pub fn list_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.store.values.keys().cloned().collect();
        keys.sort();
        keys
    }

    pub fn all_values(&self) -> Vec<String> {
        self.store.values.values().cloned().collect()
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::secrets_path()?;
        let blob = encrypt_store(&self.cipher, &self.store)?;
        std::fs::write(path, blob).context("failed to write secrets file")
    }
}

fn load_or_create_master_key() -> Result<[u8; 32]> {
    let path = paths::data_dir()?.join("master.key");
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    std::fs::write(&path, key)?;
    Ok(key)
}

fn encrypt_store(cipher: &ChaCha20Poly1305, store: &SecretStore) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(store)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;
    let mut out = nonce_bytes.to_vec();
    out.extend(ciphertext);
    Ok(out)
}

fn decrypt_store(cipher: &ChaCha20Poly1305, blob: &[u8]) -> Result<SecretStore> {
    if blob.len() <= NONCE_LEN {
        anyhow::bail!("secrets file is corrupt");
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("failed to decrypt secrets (corrupt or wrong key)"))?;
    Ok(serde_json::from_slice(&plaintext)?)
}

/// Redact known secret values from log output.
pub fn redact(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret, "[REDACTED]");
        }
    }
    out
}
