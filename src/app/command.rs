use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::config::{AppConfig, ServerConfig};
use crate::services::docker::DockerController;
use crate::services::provision::RemoteProvisioner;
use crate::services::routing::{self, DomainSpec};
use crate::services::traefik::{AcmeConfig, TraefikProvisioner, TraefikStatus};
use crate::services::secrets::{self, SecretsManager};
use crate::services::ssh::{SshConnectError, SshManager, SshSession};
use crate::services::updater::Updater;

use super::event::Message;
use super::state::HostKeyAfterAction;

async fn build_acme_config(
    config: &Arc<Mutex<AppConfig>>,
    secrets: &Arc<Mutex<SecretsManager>>,
) -> AcmeConfig {
    let (email, challenge) = {
        let cfg = config.lock().await;
        (cfg.acme_email.clone(), cfg.acme_challenge.clone())
    };
    let dns_api_token = {
        let mgr = secrets.lock().await;
        mgr.get("CF_DNS_API_TOKEN")
            .or_else(|| mgr.get("CLOUDFLARE_DNS_API_TOKEN"))
            .map(|v| v.to_string())
    };
    AcmeConfig {
        email,
        challenge,
        dns_api_token,
    }
}

pub struct CommandBus {
    tx: mpsc::UnboundedSender<Message>,
    config: Arc<Mutex<AppConfig>>,
    secrets: Arc<Mutex<SecretsManager>>,
    ssh_manager: SshManager,
    sessions: Arc<Mutex<std::collections::HashMap<Uuid, SshSession>>>,
}

impl CommandBus {
    pub fn new(
        tx: mpsc::UnboundedSender<Message>,
        config: Arc<Mutex<AppConfig>>,
        secrets: Arc<Mutex<SecretsManager>>,
        auto_reconnect: bool,
        ssh_status_tx: mpsc::UnboundedSender<crate::services::ssh::SshStatus>,
    ) -> Self {
        Self {
            tx,
            config,
            secrets,
            ssh_manager: SshManager::new(ssh_status_tx, auto_reconnect),
            sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn dispatch(&self, msg: Message) {
        match msg {
            Message::ConnectServer(id) => self.connect_server(id, HostKeyAfterAction::Connect),
            Message::ProvisionServer(id) => self.provision_server(id),
            Message::StartContainer { server_id, name } => {
                self.container_action(server_id, "start", name)
            }
            Message::StopContainer { server_id, name } => {
                self.container_action(server_id, "stop", name)
            }
            Message::RestartContainer { server_id, name } => {
                self.container_action(server_id, "restart", name)
            }
            Message::RemoveContainer { server_id, name } => {
                self.container_action(server_id, "remove", name)
            }
            Message::SubmitDeploy {
                server_id,
                remote_dir,
                compose,
                routing,
            } => self.deploy(server_id, remote_dir, compose, routing),
            Message::RunCronJob(id) => self.run_cron_job(id),
            Message::DeployDone(_) | Message::Quit | Message::CronJobDone { .. } => {}
            _ => {}
        }
    }

    pub fn load_containers(&self, server_id: Uuid) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let Some(session) = guard.get_mut(&server_id) else {
                let _ = tx.send(Message::ContainersLoaded(Err(
                    "no active SSH session — connect to a server first".into(),
                )));
                return;
            };
            let result = DockerController::list_containers(session)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Message::ContainersLoaded(result));
        });
    }

    pub fn load_logs(&self, server_id: Uuid, container_name: Option<String>) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        let secrets = self.secrets.clone();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let Some(session) = guard.get_mut(&server_id) else {
                let _ = tx.send(Message::LogsLoaded(Err(
                    "no active SSH session".into(),
                )));
                return;
            };
            let name = match container_name {
                Some(n) => n,
                None => match DockerController::list_containers(session).await {
                    Ok(list) if !list.is_empty() => list[0].name.clone(),
                    Ok(_) => {
                        let _ = tx.send(Message::LogsLoaded(Ok(vec![
                            "no containers to show logs for".into(),
                        ])));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(Message::LogsLoaded(Err(e.to_string())));
                        return;
                    }
                },
            };
            let logs = DockerController::stream_logs_prefix(session, &name, 100)
                .await
                .map(|s| {
                    s.lines().map(String::from).collect::<Vec<_>>()
                })
                .map_err(|e| e.to_string());
            let logs = match logs {
                Ok(lines) => {
                    let secret_values: Vec<String> = secrets.lock().await.all_values();
                    let refs: Vec<&str> = secret_values.iter().map(String::as_str).collect();
                    Ok(lines
                        .into_iter()
                        .map(|line| secrets::redact(&line, &refs))
                        .collect())
                }
                Err(e) => Err(e),
            };
            let _ = tx.send(Message::LogsLoaded(logs));
        });
    }

    pub fn load_metrics(&self, server_id: Uuid) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let Some(session) = guard.get_mut(&server_id) else {
                let _ = tx.send(Message::MetricsLoaded(Err(
                    "no active SSH session — connect to a server first".into(),
                )));
                return;
            };
            let result = DockerController::container_stats(session)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Message::MetricsLoaded(result));
        });
    }

    pub fn load_schedules(&self, server_id: Uuid) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let Some(session) = guard.get_mut(&server_id) else {
                let _ = tx.send(Message::SchedulesLoaded(Err(
                    "no active SSH session — connect to a server first".into(),
                )));
                return;
            };
            let result = DockerController::list_restart_schedules(session)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Message::SchedulesLoaded(result));
        });
    }

    pub fn load_secrets(&self) {
        let tx = self.tx.clone();
        let secrets = self.secrets.clone();
        tokio::spawn(async move {
            let guard = secrets.lock().await;
            let keys = guard.list_keys();
            let _ = tx.send(Message::SecretsLoaded(keys));
        });
    }

    pub fn save_secret(&self, key: String, value: String) {
        let tx = self.tx.clone();
        let secrets = self.secrets.clone();
        tokio::spawn(async move {
            let save_result = {
                let mut guard = secrets.lock().await;
                guard.set(&key, &value).map_err(|e| e.to_string())
            };
            match save_result {
                Ok(()) => {
                    let keys = secrets.lock().await.list_keys();
                    let _ = tx.send(Message::SecretsLoaded(keys));
                    let _ = tx.send(Message::SetStatus(format!("saved secret `{key}`")));
                }
                Err(e) => {
                    let _ = tx.send(Message::SetError(e));
                }
            }
        });
    }

    pub fn delete_secret(&self, key: String) {
        let tx = self.tx.clone();
        let secrets = self.secrets.clone();
        tokio::spawn(async move {
            let delete_result = {
                let mut guard = secrets.lock().await;
                guard.remove(&key).map_err(|e| e.to_string())
            };
            match delete_result {
                Ok(()) => {
                    let keys = secrets.lock().await.list_keys();
                    let _ = tx.send(Message::SecretsLoaded(keys));
                    let _ = tx.send(Message::SetStatus(format!("removed secret `{key}`")));
                }
                Err(e) => {
                    let _ = tx.send(Message::SetError(e));
                }
            }
        });
    }

    fn connect_server(&self, server_id: Uuid, after_host_key: HostKeyAfterAction) {
        let tx = self.tx.clone();
        let config = self.config.clone();
        let ssh_manager = self.ssh_manager.clone();
        let sessions = self.sessions.clone();

        tokio::spawn(async move {
            let server = {
                let cfg = config.lock().await;
                cfg.server(server_id).cloned()
            };
            let Some(server) = server else {
                let _ = tx.send(Message::SetError("server not found".into()));
                return;
            };

            match ssh_manager
                .connect_with_retry(
                    server_id,
                    &server.host,
                    server.port,
                    &server.user,
                    false,
                )
                .await
            {
                Ok(session) => {
                    sessions.lock().await.insert(server_id, session);
                    let _ = tx.send(Message::SetStatus(format!(
                        "connected to {}",
                        server.name
                    )));
                }
                Err(SshConnectError::HostKeyUnknown { fingerprint }) => {
                    let _ = tx.send(Message::HostKeyRequired {
                        server_id,
                        host: server.host.clone(),
                        port: server.port,
                        fingerprint,
                        after_accept: after_host_key,
                    });
                }
                Err(SshConnectError::HostKeyChanged { old, new }) => {
                    let _ = tx.send(Message::SetError(format!(
                        "host key changed (possible MITM)\n  was: {old}\n  now: {new}"
                    )));
                }
                Err(SshConnectError::Failed(e)) => {
                    let _ = tx.send(Message::SetError(e));
                }
            }
        });
    }

    fn provision_server(&self, server_id: Uuid) {
        let tx = self.tx.clone();
        let config = self.config.clone();
        let secrets = self.secrets.clone();
        let ssh_manager = self.ssh_manager.clone();
        let sessions = self.sessions.clone();

        tokio::spawn(async move {
            let server = {
                let cfg = config.lock().await;
                cfg.server(server_id).cloned()
            };
            let Some(server) = server else {
                let _ = tx.send(Message::ProvisionDone(Err("server not found".into())));
                return;
            };

            let session_result = ssh_manager
                .connect_with_retry(
                    server_id,
                    &server.host,
                    server.port,
                    &server.user,
                    false,
                )
                .await;

            let mut session = match session_result {
                Ok(s) => s,
                Err(SshConnectError::HostKeyUnknown { fingerprint }) => {
                    let _ = tx.send(Message::HostKeyRequired {
                        server_id,
                        host: server.host.clone(),
                        port: server.port,
                        fingerprint,
                        after_accept: HostKeyAfterAction::Provision,
                    });
                    return;
                }
                Err(SshConnectError::HostKeyChanged { old, new }) => {
                    let _ = tx.send(Message::ProvisionDone(Err(format!(
                        "host key changed (possible MITM)\n  was: {old}\n  now: {new}"
                    ))));
                    return;
                }
                Err(SshConnectError::Failed(e)) => {
                    let _ = tx.send(Message::ProvisionDone(Err(e)));
                    return;
                }
            };

            let progress_tx = tx.clone();
            let acme = build_acme_config(&config, &secrets).await;
            let result = RemoteProvisioner::run(&mut session, &acme, |p| {
                let _ = progress_tx.send(Message::ProvisionProgress(p));
            })
            .await;

            match result {
                Ok(res) => {
                    sessions.lock().await.insert(server_id, session);
                    let mut cfg = config.lock().await;
                    if let Some(srv) = cfg.server_mut(server_id) {
                        srv.docker_installed = res.docker_installed;
                        srv.traefik_installed = res.traefik_installed;
                    }
                    let _ = cfg.save();
                    let _ = tx.send(Message::ProvisionDone(Ok(res)));
                }
                Err(e) => {
                    let _ = tx.send(Message::ProvisionDone(Err(e.to_string())));
                }
            }
        });
    }

    fn container_action(&self, server_id: Uuid, action: &str, name: String) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        let action = action.to_string();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let Some(session) = guard.get_mut(&server_id) else {
                let _ = tx.send(Message::SetError("no active SSH session".into()));
                return;
            };
            let result = match action.as_str() {
                "start" => DockerController::start(session, &name).await,
                "stop" => DockerController::stop(session, &name).await,
                "restart" => DockerController::restart(session, &name).await,
                "remove" => DockerController::remove(session, &name).await,
                _ => return,
            };
            match result {
                Ok(out) if out.exit_code == 0 => {
                    let _ = tx.send(Message::SetStatus(format!("{action} {name} OK")));
                    let reload = DockerController::list_containers(session)
                        .await
                        .map_err(|e| e.to_string());
                    let _ = tx.send(Message::ContainersLoaded(reload));
                }
                Ok(out) => {
                    let _ = tx.send(Message::SetError(out.stderr));
                }
                Err(e) => {
                    let _ = tx.send(Message::SetError(e.to_string()));
                }
            }
        });
    }

    pub fn spawn_update_check(&self, current_version: &str, enabled: bool) {
        if !enabled {
            return;
        }
        let tx = self.tx.clone();
        let version = current_version.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Ok(Some(notice)) = Updater::check_for_update(&version).await {
                let _ = tx.send(Message::UpdateAvailable(notice));
            }
        });
    }

    pub fn deploy(
        &self,
        server_id: Uuid,
        remote_dir: String,
        compose: String,
        routing: Option<DomainSpec>,
    ) {
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        let secrets = self.secrets.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            let mut guard = sessions.lock().await;
            let session = match guard.get_mut(&server_id) {
                Some(s) => s,
                None => {
                    let _ = tx.send(Message::DeployDone(Err(
                        "connect to server before deploying".into(),
                    )));
                    return;
                }
            };

            if routing.is_some() {
                let acme = build_acme_config(&config, &secrets).await;
                match TraefikProvisioner::status(session).await {
                    Ok(TraefikStatus::Legacy) => {
                        if let Err(e) = TraefikProvisioner::migrate(session, &acme).await {
                            let _ = tx.send(Message::DeployDone(Err(format!(
                                "Traefik migration failed: {e}"
                            ))));
                            return;
                        }
                    }
                    Ok(TraefikStatus::NotRunning) => {
                        let _ = tx.send(Message::DeployDone(Err(
                            "Traefik is not running — provision the server first".into(),
                        )));
                        return;
                    }
                    Ok(TraefikStatus::Healthy) => {}
                    Err(e) => {
                        let _ = tx.send(Message::DeployDone(Err(e.to_string())));
                        return;
                    }
                }
            }

            let routing_ref = routing.as_ref();
            let final_compose = match routing_ref {
                Some(spec) => match routing::inject_routing(&compose, spec) {
                    Ok(patched) => patched,
                    Err(e) => {
                        let _ = tx.send(Message::DeployDone(Err(e.to_string())));
                        return;
                    }
                },
                None => compose,
            };
            let env_vars: Vec<(String, String)> = {
                let mgr = secrets.lock().await;
                mgr.list_keys()
                    .into_iter()
                    .filter_map(|k| mgr.get(&k).map(|v| (k, v.to_string())))
                    .collect()
            };
            let result = DockerController::deploy_compose(
                session,
                &remote_dir,
                &final_compose,
                &env_vars,
                routing_ref,
            )
            .await
            .map_err(|e| e.to_string());
            let _ = tx.send(Message::DeployDone(result));
        });
    }

    pub fn run_cron_job(&self, job_id: Uuid) {
        let tx = self.tx.clone();
        let config = self.config.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let job = {
                let cfg = config.lock().await;
                cfg.cron_jobs.iter().find(|j| j.id == job_id).cloned()
            };
            let Some(job) = job else {
                let _ = tx.send(Message::CronJobDone {
                    id: job_id,
                    result: Err("cron job not found".into()),
                });
                return;
            };
            if !job.enabled {
                return;
            }

            let mut guard = sessions.lock().await;
            let session = match guard.get_mut(&job.server_id) {
                Some(s) => s,
                None => {
                    let _ = tx.send(Message::CronJobDone {
                        id: job_id,
                        result: Err(format!(
                            "no SSH session for scheduled job `{}` — connect to the server first",
                            job.label
                        )),
                    });
                    return;
                }
            };

            let result = match &job.action {
                crate::config::CronAction::RestartContainer { container } => {
                    DockerController::restart(session, container)
                        .await
                        .map(|_| format!("restarted {container}"))
                        .map_err(|e| e.to_string())
                }
                crate::config::CronAction::Redeploy { remote_dir } => {
                    DockerController::redeploy_compose(session, remote_dir)
                        .await
                        .map(|_| format!("redeployed {remote_dir}"))
                        .map_err(|e| e.to_string())
                }
            };

            if result.is_ok() {
                let now = chrono::Utc::now().to_rfc3339();
                let mut cfg = config.lock().await;
                if let Some(j) = cfg.cron_jobs.iter_mut().find(|j| j.id == job_id) {
                    j.last_run = Some(now);
                }
                let _ = cfg.save();
            }

            let _ = tx.send(Message::CronJobDone {
                id: job_id,
                result,
            });
        });
    }
}

pub async fn save_new_server(config: &Arc<Mutex<AppConfig>>, server: ServerConfig) -> Result<()> {
    let mut cfg = config.lock().await;
    cfg.servers.push(server);
    cfg.onboarding_complete = true;
    cfg.save()
}
