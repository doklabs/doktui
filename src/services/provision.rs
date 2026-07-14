use anyhow::{bail, Context, Result};

use super::docker::DockerController;
use super::ssh::SshBackend;
use super::traefik::{ensure_network, AcmeConfig, TraefikProvisioner, TraefikStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionStep {
    DetectOs,
    CheckDocker,
    InstallDocker,
    CheckTraefik,
    MigrateTraefik,
    InstallTraefik,
    Verify,
    Done,
}

#[derive(Debug, Clone)]
pub struct ProvisionProgress {
    pub step: ProvisionStep,
    pub message: String,
    pub percent: u8,
}

#[derive(Debug, Clone, Default)]
pub struct ProvisionResult {
    pub docker_installed: bool,
    pub traefik_installed: bool,
    pub os_info: String,
}

pub struct RemoteProvisioner;

impl RemoteProvisioner {
    pub async fn run(
        session: &mut dyn SshBackend,
        acme: &AcmeConfig,
        progress: impl Fn(ProvisionProgress),
    ) -> Result<ProvisionResult> {
        progress(ProvisionProgress {
            step: ProvisionStep::DetectOs,
            message: "Detecting server OS…".into(),
            percent: 5,
        });
        let os = detect_os(session).await?;

        progress(ProvisionProgress {
            step: ProvisionStep::CheckDocker,
            message: "Checking Docker…".into(),
            percent: 20,
        });
        let mut docker_ok = DockerController::probe(session).await?;

        if !docker_ok {
            progress(ProvisionProgress {
                step: ProvisionStep::InstallDocker,
                message: "Installing Docker (this may take a few minutes)…".into(),
                percent: 40,
            });
            install_docker(session).await?;
            docker_ok = DockerController::probe(session).await?;
            if !docker_ok {
                bail!("Docker installation did not succeed — install Docker manually and retry");
            }
        }

        progress(ProvisionProgress {
            step: ProvisionStep::CheckTraefik,
            message: "Checking Traefik…".into(),
            percent: 65,
        });
        ensure_network(session).await?;

        let traefik_status = TraefikProvisioner::status(session).await?;
        let mut traefik_ok = traefik_status == TraefikStatus::Healthy;

        if traefik_status == TraefikStatus::Legacy {
            progress(ProvisionProgress {
                step: ProvisionStep::MigrateTraefik,
                message: "Migrating Traefik to doktui-network (auto-upgrade)…".into(),
                percent: 75,
            });
            TraefikProvisioner::migrate(session, acme).await?;
            traefik_ok = TraefikProvisioner::status(session).await? == TraefikStatus::Healthy;
        } else if traefik_status == TraefikStatus::NotRunning {
            progress(ProvisionProgress {
                step: ProvisionStep::InstallTraefik,
                message: "Deploying Traefik…".into(),
                percent: 80,
            });
            TraefikProvisioner::install(session, acme).await?;
            traefik_ok = TraefikProvisioner::status(session).await? == TraefikStatus::Healthy;
        }

        progress(ProvisionProgress {
            step: ProvisionStep::Verify,
            message: "Verifying setup…".into(),
            percent: 95,
        });

        let compose_ok = DockerController::compose_available(session).await?;
        if !compose_ok {
            bail!("docker compose is not available after provisioning");
        }

        progress(ProvisionProgress {
            step: ProvisionStep::Done,
            message: "Server ready".into(),
            percent: 100,
        });

        Ok(ProvisionResult {
            docker_installed: docker_ok,
            traefik_installed: traefik_ok,
            os_info: os,
        })
    }
}

async fn detect_os(session: &mut dyn SshBackend) -> Result<String> {
    let out = session
        .exec("cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"'")
        .await?;
    let os = out.stdout.trim();
    if os.is_empty() {
        Ok("Unknown Linux".into())
    } else {
        Ok(os.to_string())
    }
}

async fn install_docker(session: &mut dyn SshBackend) -> Result<()> {
    let out = session
        .exec("curl -fsSL https://get.docker.com | sh")
        .await
        .context("Docker install script failed")?;
    if out.exit_code != 0 {
        bail!(
            "Docker install failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        );
    }
    Ok(())
}
