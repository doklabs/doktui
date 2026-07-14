use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use russh_keys::key::KeyPair;
use russh_keys::{decode_secret_key, encode_pkcs8_pem, load_public_key, PublicKeyBase64};

use crate::config::paths;
use crate::security::keychain;

const KEY_COMMENT: &str = "doktui@doklabs";

/// Generate or load the dedicated DokTUI Ed25519 keypair.
pub fn ensure_keypair() -> Result<()> {
    let priv_path = paths::ssh_key_path()?;
    let pub_path = paths::ssh_key_pub_path()?;

    if priv_path.exists() && pub_path.exists() {
        enforce_key_permissions(&priv_path)?;
        if keychain::load_key_pem()?.is_none() {
            if let Ok(pem) = fs::read_to_string(&priv_path) {
                let _ = keychain::store_key_pem(&pem);
            }
        }
        return Ok(());
    }

    let key = KeyPair::generate_ed25519();
    let _ = keychain::delete_key_pem();
    let mut priv_pem = Vec::new();
    encode_pkcs8_pem(&key, &mut priv_pem).context("failed to encode private key")?;
    let pem_str = String::from_utf8(priv_pem.clone()).context("invalid private key encoding")?;
    fs::write(&priv_path, &priv_pem)?;
    let _ = keychain::store_key_pem(&pem_str);

    let public = key.clone_public_key()?;
    let pub_line = format!("ssh-ed25519 {} {KEY_COMMENT}", public.public_key_base64());
    fs::write(&pub_path, pub_line)?;

    enforce_key_permissions(&priv_path)?;
    tracing::info!("generated dedicated DokTUI SSH keypair");
    Ok(())
}

pub fn load_private_key() -> Result<KeyPair> {
    if let Some(pem) = keychain::load_key_pem()? {
        if let Ok(key) = decode_secret_key(&pem, None) {
            return Ok(key);
        }
    }
    let path = paths::ssh_key_path()?;
    enforce_key_permissions(&path)?;
    let pem = fs::read_to_string(&path)
        .with_context(|| format!("failed to read private key at {}", path.display()))?;
    decode_secret_key(&pem, None).context("failed to decode private key")
}

pub fn load_public_key_openssh() -> Result<String> {
    let path = paths::ssh_key_pub_path()?;
    fs::read_to_string(&path)
        .with_context(|| format!("failed to read public key at {}", path.display()))
}

pub fn public_key_fingerprint() -> Result<String> {
    let path = paths::ssh_key_pub_path()?;
    let pk = load_public_key(&path).context("failed to load public key")?;
    Ok(pk.fingerprint())
}

#[cfg(unix)]
fn enforce_key_permissions(path: &Path) -> Result<()> {
    use anyhow::bail;
    use std::os::unix::fs::PermissionsExt;

    let meta = fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "SSH private key at {} has insecure permissions ({:o}); expected 0600 or tighter",
            path.display(),
            mode
        );
    }
    Ok(())
}

#[cfg(windows)]
fn enforce_key_permissions(_path: &Path) -> Result<()> {
    Ok(())
}
