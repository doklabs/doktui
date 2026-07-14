use anyhow::{bail, Context, Result};

use super::routing::DomainSpec;
use super::ssh::{CommandOutput, SshBackend};

pub struct DockerController;

impl DockerController {
    pub async fn probe(session: &mut dyn SshBackend) -> Result<bool> {
        let out = session.exec("command -v docker").await?;
        Ok(out.exit_code == 0 && !out.stdout.trim().is_empty())
    }

    pub async fn compose_available(session: &mut dyn SshBackend) -> Result<bool> {
        let out = session.exec("docker compose version").await?;
        Ok(out.exit_code == 0)
    }

    pub async fn list_containers(session: &mut dyn SshBackend) -> Result<Vec<ContainerInfo>> {
        let out = session
            .exec("docker ps -a --format '{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}'")
            .await?;
        if out.exit_code != 0 {
            bail!("docker ps failed: {}", out.stderr.trim());
        }
        let containers = out
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(parse_container_line)
            .collect();
        Ok(containers)
    }

    pub async fn start(session: &mut dyn SshBackend, name: &str) -> Result<CommandOutput> {
        session.exec(&format!("docker start {name}")).await
    }

    pub async fn stop(session: &mut dyn SshBackend, name: &str) -> Result<CommandOutput> {
        session.exec(&format!("docker stop {name}")).await
    }

    pub async fn restart(session: &mut dyn SshBackend, name: &str) -> Result<CommandOutput> {
        session.exec(&format!("docker restart {name}")).await
    }

    pub async fn remove(session: &mut dyn SshBackend, name: &str) -> Result<CommandOutput> {
        session.exec(&format!("docker rm -f {name}")).await
    }

    pub async fn deploy_compose(
        session: &mut dyn SshBackend,
        remote_dir: &str,
        compose_content: &str,
        env_vars: &[(String, String)],
        routing: Option<&DomainSpec>,
    ) -> Result<DeployReport> {
        session
            .exec(&format!("mkdir -p {remote_dir}"))
            .await
            .context("failed to create remote directory")?;

        session
            .write_remote_file(
                &format!("{remote_dir}/docker-compose.yml"),
                compose_content.as_bytes(),
            )
            .await
            .context("failed to upload compose file")?;

        let secrets_count = env_vars.len();
        if !env_vars.is_empty() {
            let env_body = env_vars
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n");
            session
                .write_remote_file(&format!("{remote_dir}/.env"), env_body.as_bytes())
                .await
                .context("failed to upload .env file")?;
        }

        let up = session
            .exec(&format!("cd {remote_dir} && docker compose up -d"))
            .await?;
        if up.exit_code != 0 {
            bail!("docker compose up failed: {}", up.stderr.trim());
        }

        verify_deploy(session, remote_dir, routing, secrets_count).await
    }

    pub async fn stream_logs_prefix(
        session: &mut dyn SshBackend,
        name: &str,
        lines: u16,
    ) -> Result<String> {
        let out = session
            .exec(&format!("docker logs --tail {lines} {name} 2>&1"))
            .await?;
        Ok(out.stdout)
    }

    pub async fn container_stats(session: &mut dyn SshBackend) -> Result<Vec<ContainerStats>> {
        let out = session
            .exec(
                "docker stats --no-stream --format '{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}'",
            )
            .await?;
        if out.exit_code != 0 {
            bail!("docker stats failed: {}", out.stderr.trim());
        }
        Ok(out
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(parse_stats_line)
            .collect())
    }

    pub async fn list_restart_schedules(session: &mut dyn SshBackend) -> Result<Vec<ScheduleInfo>> {
        let out = session
            .exec("docker ps -a --format '{{.Names}}|{{.Status}}'")
            .await?;
        if out.exit_code != 0 {
            bail!("docker ps failed: {}", out.stderr.trim());
        }
        let mut schedules = Vec::new();
        for line in out.stdout.lines().filter(|l| !l.trim().is_empty()) {
            let mut parts = line.splitn(2, '|');
            let name = parts.next().unwrap_or("").trim_start_matches('/');
            let status = parts.next().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let inspect = session
                .exec(&format!(
                    "docker inspect --format '{{{{.HostConfig.RestartPolicy.Name}}}}' {name}"
                ))
                .await?;
            let policy = inspect.stdout.trim();
            schedules.push(ScheduleInfo {
                name: name.to_string(),
                restart_policy: if policy.is_empty() {
                    "no".into()
                } else {
                    policy.to_string()
                },
                status,
            });
        }
        Ok(schedules)
    }

    /// Pull latest images and recreate containers (for scheduled redeploy).
    pub async fn redeploy_compose(session: &mut dyn SshBackend, remote_dir: &str) -> Result<()> {
        let out = session
            .exec(&format!(
                "cd {remote_dir} && docker compose pull && docker compose up -d"
            ))
            .await?;
        if out.exit_code != 0 {
            bail!("redeploy failed: {}", out.stderr.trim());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DeployReport {
    pub lines: Vec<String>,
    pub container_ok: bool,
    pub route_ok: Option<bool>,
}

impl DeployReport {
    pub fn summary(&self) -> String {
        self.lines.join("\n")
    }

    pub fn all_ok(&self) -> bool {
        self.container_ok && self.route_ok.unwrap_or(true)
    }
}

async fn verify_deploy(
    session: &mut dyn SshBackend,
    remote_dir: &str,
    routing: Option<&DomainSpec>,
    secrets_count: usize,
) -> Result<DeployReport> {
    let service = routing.map(|r| r.service.as_str()).unwrap_or("app");
    let mut lines = vec!["Deploy finished".to_string()];

    let ps = session
        .exec(&format!(
            "cd {remote_dir} && docker compose ps --status running --format '{{{{.Service}}}}|{{{{.State}}}}'"
        ))
        .await?;
    let container_ok = ps.exit_code == 0
        && ps.stdout.lines().any(|line| {
            line.split('|')
                .next()
                .is_some_and(|svc| svc.trim() == service)
        });
    lines.push(format!(
        "• container `{service}`: {}",
        if container_ok {
            "running ✓"
        } else {
            "NOT running ✗"
        }
    ));

    let mut route_ok = None;
    if let Some(spec) = routing {
        let scheme = if spec.https { "https" } else { "http" };
        let curl_cmd = if spec.https {
            format!(
                "curl -sk -o /dev/null -w '%{{http_code}}' -H 'Host: {}' --connect-timeout 8 {scheme}://127.0.0.1/ 2>/dev/null || echo 000",
                spec.host
            )
        } else {
            format!(
                "curl -sf -o /dev/null -w '%{{http_code}}' -H 'Host: {}' --connect-timeout 8 {scheme}://127.0.0.1/ 2>/dev/null || echo 000",
                spec.host
            )
        };
        let out = session.exec(&curl_cmd).await?;
        let code = out.stdout.trim();
        let ok = code.starts_with('2') || code.starts_with('3');
        route_ok = Some(ok);
        lines.push(format!(
            "• route {scheme}://{}: {}",
            spec.host,
            if ok {
                format!("reachable ✓ (HTTP {code})")
            } else {
                "not reachable ✗ (check DNS / Traefik labels)".into()
            }
        ));
    }

    if secrets_count > 0 {
        lines.push(format!(
            "• secrets: {secrets_count} vars uploaded to .env ✓"
        ));
    }

    let report = DeployReport {
        lines,
        container_ok,
        route_ok,
    };

    if !report.container_ok {
        bail!(
            "{}\n\ncontainer failed to start — check logs with Deployments → Logs",
            report.summary()
        );
    }

    Ok(report)
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub image: String,
}

#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub name: String,
    pub cpu_percent: String,
    pub mem_usage: String,
    pub mem_percent: String,
}

#[derive(Debug, Clone)]
pub struct ScheduleInfo {
    pub name: String,
    pub restart_policy: String,
    pub status: String,
}

fn parse_stats_line(line: &str) -> Option<ContainerStats> {
    let mut parts = line.splitn(4, '|');
    Some(ContainerStats {
        name: parts.next()?.to_string(),
        cpu_percent: parts.next()?.to_string(),
        mem_usage: parts.next()?.to_string(),
        mem_percent: parts.next()?.to_string(),
    })
}

fn parse_container_line(line: &str) -> Option<ContainerInfo> {
    let mut parts = line.splitn(4, '|');
    Some(ContainerInfo {
        id: parts.next()?.to_string(),
        name: parts.next()?.to_string(),
        status: parts.next()?.to_string(),
        image: parts.next()?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct MockSshBackend {
        commands: Vec<String>,
        responses: HashMap<String, CommandOutput>,
    }

    #[async_trait]
    impl SshBackend for MockSshBackend {
        async fn exec(&mut self, command: &str) -> Result<CommandOutput> {
            self.commands.push(command.to_string());
            Ok(self
                .responses
                .get(command)
                .cloned()
                .unwrap_or(CommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                }))
        }

        async fn write_remote_file(&mut self, _remote_path: &str, _content: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn probe_detects_docker() {
        let mut backend = MockSshBackend {
            responses: HashMap::from([(
                "command -v docker".to_string(),
                CommandOutput {
                    stdout: "/usr/bin/docker\n".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                },
            )]),
            ..Default::default()
        };

        assert!(DockerController::probe(&mut backend).await.unwrap());
        assert_eq!(backend.commands, vec!["command -v docker"]);
    }

    #[tokio::test]
    async fn list_containers_parses_output() {
        let mut backend = MockSshBackend {
            responses: HashMap::from([(
                "docker ps -a --format '{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}'".to_string(),
                CommandOutput {
                    stdout: "abc123|my-app|Up 2 hours|nginx:latest".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                },
            )]),
            ..Default::default()
        };

        let containers = DockerController::list_containers(&mut backend)
            .await
            .unwrap();

        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].id, "abc123");
        assert_eq!(containers[0].name, "my-app");
        assert_eq!(containers[0].status, "Up 2 hours");
        assert_eq!(containers[0].image, "nginx:latest");
    }

    #[tokio::test]
    async fn list_containers_filters_empty_lines() {
        let mut backend = MockSshBackend {
            responses: HashMap::from([(
                "docker ps -a --format '{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}'".to_string(),
                CommandOutput {
                    stdout: "\n\nabc123|my-app|Up|nginx\n\n".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                },
            )]),
            ..Default::default()
        };

        let containers = DockerController::list_containers(&mut backend)
            .await
            .unwrap();
        assert_eq!(containers.len(), 1);
    }
}
