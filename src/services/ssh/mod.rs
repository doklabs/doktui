use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use russh::client::{self, Handler};
use russh::{ChannelMsg, Disconnect};
use russh_keys::key::PublicKey;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::security::{hostkey, keys};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

#[derive(Debug, Clone)]
pub struct SshStatus {
    pub server_id: Uuid,
    pub state: ConnectionState,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}

#[derive(Debug, Clone)]
pub enum SshConnectError {
    HostKeyUnknown { fingerprint: String },
    HostKeyChanged { old: String, new: String },
    Failed(String),
}

#[derive(Default)]
struct HostKeyCapture {
    unknown_fingerprint: Option<String>,
    changed: Option<(String, String)>,
}

struct ClientHandler {
    host: String,
    port: u16,
    known: hostkey::KnownHosts,
    trust_new: bool,
    capture: Arc<Mutex<HostKeyCapture>>,
}

#[async_trait]
impl Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let action = self
            .known
            .verify(&self.host, self.port, server_public_key)
            .context("host key verification failed")?;

        match action {
            hostkey::HostKeyAction::Changed { old, new } => {
                self.capture.lock().expect("host key capture lock").changed = Some((old, new));
                Ok(false)
            }
            hostkey::HostKeyAction::AcceptNew if self.trust_new => {
                self.known
                    .trust(&self.host, self.port, server_public_key)
                    .context("failed to persist host key")?;
                Ok(true)
            }
            hostkey::HostKeyAction::AcceptNew => {
                self.capture
                    .lock()
                    .expect("host key capture lock")
                    .unknown_fingerprint = Some(server_public_key.fingerprint());
                Ok(false)
            }
            hostkey::HostKeyAction::AlreadyKnown => Ok(true),
        }
    }
}

pub struct SshSession {
    pub server_id: Uuid,
    handle: client::Handle<ClientHandler>,
}

impl SshSession {
    pub async fn connect(
        server_id: Uuid,
        host: &str,
        port: u16,
        user: &str,
        trust_new_host: bool,
    ) -> Result<Self, SshConnectError> {
        let capture = Arc::new(Mutex::new(HostKeyCapture::default()));
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            keepalive_interval: Some(Duration::from_secs(15)),
            ..Default::default()
        };
        let config = Arc::new(config);
        let handler = ClientHandler {
            host: host.to_string(),
            port,
            known: hostkey::KnownHosts::load()
                .map_err(|e| SshConnectError::Failed(e.to_string()))?,
            trust_new: trust_new_host,
            capture: capture.clone(),
        };

        let mut session = client::connect(config, (host, port), handler)
            .await
            .map_err(|e| map_connect_error(&capture, e))?;

        let auth_ok = session
            .authenticate_publickey(
                user,
                Arc::new(
                    keys::load_private_key().map_err(|e| SshConnectError::Failed(e.to_string()))?,
                ),
            )
            .await
            .map_err(|e| map_connect_error(&capture, e))?;

        if !auth_ok {
            return Err(SshConnectError::Failed(format!(
                "SSH authentication failed for {user}@{host}:{port}"
            )));
        }

        Ok(Self {
            server_id,
            handle: session,
        })
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.handle
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}

/// Async backend for executing remote commands and uploading files.
///
/// `SshSession` is the real implementation. Tests can implement a mock backend
/// that records commands and returns scripted output, removing the need for a
/// real SSH connection.
#[async_trait]
pub trait SshBackend: Send {
    /// Execute a command on the remote host and capture stdout, stderr, and the exit code.
    async fn exec(&mut self, command: &str) -> Result<CommandOutput>;

    /// Upload `content` to `remote_path` on the remote host.
    async fn write_remote_file(&mut self, remote_path: &str, content: &[u8]) -> Result<()>;
}

#[async_trait]
impl SshBackend for SshSession {
    async fn exec(&mut self, command: &str) -> Result<CommandOutput> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;
        drain_channel(&mut channel).await
    }

    async fn write_remote_file(&mut self, remote_path: &str, content: &[u8]) -> Result<()> {
        let path_escaped = remote_path.replace('\'', "'\\''");
        let cmd = format!("cat > '{path_escaped}'");

        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(false, cmd).await?;

        for chunk in content.chunks(32 * 1024) {
            channel.data(chunk).await?;
        }
        channel.eof().await?;

        let out = drain_channel(&mut channel).await?;
        if out.exit_code != 0 {
            anyhow::bail!(
                "failed to write {} (exit {}): {}",
                remote_path,
                out.exit_code,
                out.stderr.trim()
            );
        }
        Ok(())
    }
}

async fn drain_channel(channel: &mut russh::Channel<client::Msg>) -> Result<CommandOutput> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_code = None;

    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { data }) => {
                stdout.push_str(&String::from_utf8_lossy(&data));
            }
            Some(ChannelMsg::ExtendedData { data, .. }) => {
                stderr.push_str(&String::from_utf8_lossy(&data));
            }
            Some(ChannelMsg::ExitStatus { exit_status }) => {
                exit_code = Some(exit_status);
                break;
            }
            Some(ChannelMsg::Eof) => {
                // Keep waiting — some servers send EOF before ExitStatus.
            }
            None => break,
            _ => {}
        }
    }

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code: exit_code.unwrap_or(255),
    })
}

#[derive(Clone)]
pub struct SshManager {
    status_tx: mpsc::UnboundedSender<SshStatus>,
    auto_reconnect: bool,
}

impl SshManager {
    pub fn new(status_tx: mpsc::UnboundedSender<SshStatus>, auto_reconnect: bool) -> Self {
        Self {
            status_tx,
            auto_reconnect,
        }
    }

    pub fn emit(&self, server_id: Uuid, state: ConnectionState, message: Option<String>) {
        let _ = self.status_tx.send(SshStatus {
            server_id,
            state,
            message,
        });
    }

    pub async fn connect_with_retry(
        &self,
        server_id: Uuid,
        host: &str,
        port: u16,
        user: &str,
        trust_new: bool,
    ) -> Result<SshSession, SshConnectError> {
        let mut attempt = 0u32;
        loop {
            let state = if attempt == 0 {
                ConnectionState::Connecting
            } else {
                ConnectionState::Reconnecting
            };
            self.emit(
                server_id,
                state,
                Some(format!("connecting to {user}@{host}:{port}")),
            );

            match SshSession::connect(server_id, host, port, user, trust_new).await {
                Ok(session) => {
                    self.emit(server_id, ConnectionState::Connected, None);
                    return Ok(session);
                }
                Err(e @ SshConnectError::HostKeyUnknown { .. })
                | Err(e @ SshConnectError::HostKeyChanged { .. }) => {
                    self.emit(server_id, ConnectionState::Disconnected, None);
                    return Err(e);
                }
                Err(SshConnectError::Failed(e)) if self.auto_reconnect && attempt < 8 => {
                    let delay = backoff_delay(attempt);
                    self.emit(
                        server_id,
                        ConnectionState::Reconnecting,
                        Some(format!("retry in {}s: {e}", delay.as_secs())),
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(SshConnectError::Failed(e)) => {
                    self.emit(server_id, ConnectionState::Disconnected, Some(e.clone()));
                    return Err(SshConnectError::Failed(e));
                }
            }
        }
    }
}

fn map_connect_error(
    capture: &Arc<Mutex<HostKeyCapture>>,
    err: impl std::fmt::Display,
) -> SshConnectError {
    let cap = capture.lock().expect("host key capture lock");
    if let Some(fp) = &cap.unknown_fingerprint {
        return SshConnectError::HostKeyUnknown {
            fingerprint: fp.clone(),
        };
    }
    if let Some((old, new)) = &cap.changed {
        return SshConnectError::HostKeyChanged {
            old: old.clone(),
            new: new.clone(),
        };
    }
    SshConnectError::Failed(err.to_string())
}

pub fn backoff_delay(attempt: u32) -> Duration {
    let base = 1u64 << attempt.min(6);
    let jitter = (attempt as u64) % 3;
    Duration::from_secs(base + jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_with_attempts() {
        assert!(backoff_delay(0) <= backoff_delay(3));
    }
}
