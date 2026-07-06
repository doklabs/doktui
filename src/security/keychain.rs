use anyhow::{Context, Result};

const SERVICE: &str = "doktui";
const USER: &str = "ssh-private-key";

/// Store the SSH private key PEM in the OS keychain when available.
pub fn store_key_pem(pem: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER).context("failed to open keychain entry")?;
    entry.set_password(pem).context("failed to store key in keychain")
}

/// Retrieve the SSH private key PEM from the OS keychain.
pub fn load_key_pem() -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE, USER).context("failed to open keychain entry")?;
    match entry.get_password() {
        Ok(pem) => Ok(Some(pem)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_key_pem() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, USER).context("failed to open keychain entry")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
