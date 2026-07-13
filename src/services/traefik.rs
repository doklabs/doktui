use anyhow::{Context, Result, bail};

use crate::config::AcmeChallenge;

use super::ssh::SshSession;

pub const NETWORK_NAME: &str = "doktui-network";
const TRAEFIK_CONTAINER: &str = "doktui-traefik";
const REMOTE_DIR: &str = "/opt/doktui/traefik";

/// ACME / TLS configuration for Traefik static config.
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    pub email: String,
    pub challenge: AcmeChallenge,
    /// Cloudflare API token when using DNS-01.
    pub dns_api_token: Option<String>,
}

/// Health of the Traefik installation on a remote server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraefikStatus {
    /// Container not running.
    NotRunning,
    /// Running but missing doktui-network and/or provider network config.
    Legacy,
    /// Running on doktui-network with expected static config.
    Healthy,
}

fn acme_challenge_lines(challenge: AcmeChallenge) -> Vec<&'static str> {
    match challenge {
        AcmeChallenge::Http01 => vec![
            "--certificatesresolvers.le.acme.httpchallenge=true",
            "--certificatesresolvers.le.acme.httpchallenge.entrypoint=web",
        ],
        AcmeChallenge::DnsCloudflare => vec![
            "--certificatesresolvers.le.acme.dnschallenge=true",
            "--certificatesresolvers.le.acme.dnschallenge.provider=cloudflare",
        ],
    }
}

/// Build Traefik compose with configurable ACME email, challenge, HTTP→HTTPS redirect, and shared network.
pub fn traefik_compose(acme: &AcmeConfig) -> String {
    let challenge_lines = acme_challenge_lines(acme.challenge);
    let challenge_block = challenge_lines
        .into_iter()
        .map(|line| format!("      - \"{line}\""))
        .collect::<Vec<_>>()
        .join("\n");

    let env_block = if acme.challenge == AcmeChallenge::DnsCloudflare {
        r#"    env_file:
      - .env
"#
    } else {
        ""
    };

    format!(
        r#"services:
  traefik:
    image: traefik:v3.7
    container_name: {TRAEFIK_CONTAINER}
    restart: unless-stopped
    networks:
      - {NETWORK_NAME}
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik-certs:/letsencrypt
    labels:
      - "doktui.traefik/managed=true"
      - "doktui.traefik/network={NETWORK_NAME}"
{env_block}    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--providers.docker.network={NETWORK_NAME}"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.web.http.redirections.entrypoint.scheme=https"
{challenge_block}
      - "--certificatesresolvers.le.acme.email={email}"
      - "--certificatesresolvers.le.acme.storage=/letsencrypt/acme.json"
networks:
  {NETWORK_NAME}:
    external: true
volumes:
  traefik-certs:
"#,
        email = acme.email,
    )
}

/// Create shared network if it does not exist yet.
pub async fn ensure_network(session: &mut SshSession) -> Result<()> {
    let exists = session
        .exec(&format!(
            "docker network ls --filter name=^{NETWORK_NAME}$ --format '{{{{.Name}}}}'"
        ))
        .await?;
    if exists.exit_code != 0 {
        bail!("failed to list docker networks: {}", exists.stderr.trim());
    }
    if !exists.stdout.contains(NETWORK_NAME) {
        let out = session
            .exec(&format!("docker network create {NETWORK_NAME}"))
            .await?;
        if out.exit_code != 0 {
            bail!("failed to create {NETWORK_NAME}: {}", out.stderr.trim());
        }
    }
    Ok(())
}

pub struct TraefikProvisioner;

impl TraefikProvisioner {
    pub async fn status(session: &mut SshSession) -> Result<TraefikStatus> {
        let out = session
            .exec(&format!(
                "docker ps --filter name=^{TRAEFIK_CONTAINER}$ --format '{{{{.Names}}}}'"
            ))
            .await?;
        if out.exit_code != 0 || !out.stdout.contains(TRAEFIK_CONTAINER) {
            return Ok(TraefikStatus::NotRunning);
        }

        let on_network = Self::on_shared_network(session).await?;
        let provider_ok = Self::has_network_provider(session).await?;

        if on_network && provider_ok {
            Ok(TraefikStatus::Healthy)
        } else {
            Ok(TraefikStatus::Legacy)
        }
    }

    pub async fn install(session: &mut SshSession, acme: &AcmeConfig) -> Result<()> {
        Self::write_compose_and_up(session, acme).await
    }

    /// Recreate Traefik with the current doktui-network configuration.
    pub async fn migrate(session: &mut SshSession, acme: &AcmeConfig) -> Result<()> {
        Self::write_compose_and_up(session, acme).await
    }

    async fn write_compose_and_up(session: &mut SshSession, acme: &AcmeConfig) -> Result<()> {
        if acme.challenge == AcmeChallenge::DnsCloudflare && acme.dns_api_token.as_deref().unwrap_or("").is_empty() {
            bail!(
                "DNS-01 (Cloudflare) requires CF_DNS_API_TOKEN in Secrets — add it under Deployments → Secrets"
            );
        }

        ensure_network(session).await?;

        session.exec(&format!("mkdir -p {REMOTE_DIR}")).await?;

        let compose = traefik_compose(acme);
        session
            .write_remote_file(&format!("{REMOTE_DIR}/docker-compose.yml"), compose.as_bytes())
            .await?;

        if acme.challenge == AcmeChallenge::DnsCloudflare {
            let token = acme.dns_api_token.as_deref().unwrap_or("");
            let env_body = format!("CF_DNS_API_TOKEN={token}\n");
            session
                .write_remote_file(&format!("{REMOTE_DIR}/.env"), env_body.as_bytes())
                .await?;
        }

        // Drop any stopped legacy container so compose cannot resurrect old config.
        let _ = session
            .exec(&format!("docker rm -f {TRAEFIK_CONTAINER} 2>/dev/null || true"))
            .await;

        let up = session
            .exec(&format!(
                "cd {REMOTE_DIR} && docker compose up -d --force-recreate"
            ))
            .await
            .context("failed to start Traefik")?;
        if up.exit_code != 0 {
            bail!("Traefik start failed: {}", up.stderr.trim());
        }

        match Self::wait_for_healthy(session).await? {
            TraefikStatus::Healthy => Ok(()),
            TraefikStatus::Legacy => {
                let detail = Self::legacy_detail(session).await;
                bail!(
                    "Traefik is running but not healthy ({detail}) — check docker logs {TRAEFIK_CONTAINER}"
                );
            }
            TraefikStatus::NotRunning => bail!("Traefik container failed to start"),
        }
    }

    /// Poll status briefly — network attach and container Cmd can lag `compose up`.
    async fn wait_for_healthy(session: &mut SshSession) -> Result<TraefikStatus> {
        use std::time::Duration;

        const ATTEMPTS: u32 = 5;
        let mut last = TraefikStatus::NotRunning;
        for attempt in 0..ATTEMPTS {
            last = Self::status(session).await?;
            if last == TraefikStatus::Healthy {
                return Ok(last);
            }
            if attempt + 1 < ATTEMPTS {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        Ok(last)
    }

    async fn legacy_detail(session: &mut SshSession) -> String {
        let on_network = Self::on_shared_network(session).await.unwrap_or(false);
        let provider_ok = Self::has_network_provider(session).await.unwrap_or(false);
        match (on_network, provider_ok) {
            (false, false) => format!(
                "not on {NETWORK_NAME} and missing providers.docker.network={NETWORK_NAME}"
            ),
            (false, true) => format!("not attached to {NETWORK_NAME}"),
            (true, false) => format!("missing providers.docker.network={NETWORK_NAME} in Cmd"),
            (true, true) => "unexpected legacy state".into(),
        }
    }

    async fn on_shared_network(session: &mut SshSession) -> Result<bool> {
        let out = session
            .exec(&format!(
                "docker inspect {TRAEFIK_CONTAINER} --format '{{{{range $k, $_ := .NetworkSettings.Networks}}}}{{$k}}|{{{{end}}}}'"
            ))
            .await?;
        if out.exit_code != 0 {
            return Ok(false);
        }
        let attached = networks_from_inspect(&out.stdout).any(|n| n == NETWORK_NAME);
        Ok(attached)
    }

    async fn has_network_provider(session: &mut SshSession) -> Result<bool> {
        let out = session
            .exec(&format!(
                "docker inspect {TRAEFIK_CONTAINER} --format '{{{{join .Config.Cmd \"|\"}}}}'"
            ))
            .await?;
        if out.exit_code != 0 {
            return Ok(false);
        }
        Ok(cmd_has_network_provider(&out.stdout))
    }
}

fn cmd_has_network_provider(stdout: &str) -> bool {
    stdout
        .trim()
        .contains(&format!("providers.docker.network={NETWORK_NAME}"))
}

/// Parse network names from `docker inspect --format '{{range ...}}{{$k}}|{{end}}'`.
/// SSH exec stdout always ends with `\n`; the template also leaves a trailing `|`.
fn networks_from_inspect(stdout: &str) -> impl Iterator<Item = &str> {
    stdout.split('|').map(str::trim).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn networks_from_inspect_handles_trailing_newline() {
        assert!(networks_from_inspect("doktui-network|\n").any(|n| n == NETWORK_NAME));
    }

    #[test]
    fn networks_from_inspect_finds_network_among_many() {
        let stdout = "bridge|doktui-network|\n";
        assert!(networks_from_inspect(stdout).any(|n| n == NETWORK_NAME));
    }

    #[test]
    fn networks_from_inspect_rejects_other_networks_only() {
        assert!(!networks_from_inspect("bridge|\n").any(|n| n == NETWORK_NAME));
    }

    #[test]
    fn cmd_has_network_provider_matches_flag() {
        let stdout = "--providers.docker=true|--providers.docker.network=doktui-network|\n";
        assert!(cmd_has_network_provider(stdout));
    }

    #[test]
    fn cmd_has_network_provider_rejects_missing_flag() {
        assert!(!cmd_has_network_provider("--providers.docker=true|\n"));
    }

    #[test]
    fn dns_compose_includes_cloudflare_provider() {
        let compose = traefik_compose(&AcmeConfig {
            email: "admin@test.com".into(),
            challenge: AcmeChallenge::DnsCloudflare,
            dns_api_token: Some("token".into()),
        });
        assert!(compose.contains("dnschallenge.provider=cloudflare"));
        assert!(compose.contains("env_file:"));
    }

    #[test]
    fn http_compose_uses_http_challenge() {
        let compose = traefik_compose(&AcmeConfig {
            email: "admin@test.com".into(),
            challenge: AcmeChallenge::Http01,
            dns_api_token: None,
        });
        assert!(compose.contains("httpchallenge=true"));
        assert!(!compose.contains("dnschallenge"));
    }
}
