use uuid::Uuid;

use crate::services::docker::{ContainerInfo, ContainerStats, DeployReport, ScheduleInfo};
use crate::services::routing::DomainSpec;
use crate::services::provision::{ProvisionProgress, ProvisionResult};
use crate::services::ssh::SshStatus;
use crate::app::state::{HostKeyAfterAction, NavSection};
use crate::services::updater::UpdateNotice;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Quit,
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),

    // Navigation
    GoWelcome,
    GoHome,
    CopyPublicKey,
    GoNav(NavSection),
    GoServerList,
    GoAddServer,
    GoContainers,
    GoLogs,
    GoDeploy,
    GoSecrets,
    GoEditor,

    // Shell UI
    SidebarNext,
    SidebarPrev,
    ToggleSidebarFocus,
    ToggleSearch,
    CloseSearch,
    ToggleErrorPanel,
    CloseErrorPanel,
    ErrorScrollUp,
    ErrorScrollDown,
    ToggleUiMode,
    SidebarNarrow,
    SidebarWiden,
    SidebarResizeEnd,
    SearchChar(char),
    SearchBackspace,

    // Server management
    SubmitServerForm,
    SelectServer(Uuid),
    ConnectServer(Uuid),
    ProvisionServer(Uuid),

    // Host key
    HostKeyRequired {
        server_id: Uuid,
        host: String,
        port: u16,
        fingerprint: String,
        after_accept: HostKeyAfterAction,
    },
    AcceptHostKey,
    RejectHostKey,

    // SSH events
    SshStatus(SshStatus),
    ProvisionProgress(ProvisionProgress),
    ProvisionDone(Result<ProvisionResult, String>),
    ContainersLoaded(Result<Vec<ContainerInfo>, String>),
    MetricsLoaded(Result<Vec<ContainerStats>, String>),
    SchedulesLoaded(Result<Vec<ScheduleInfo>, String>),
    RunCronJob(Uuid),
    CronJobDone {
        id: Uuid,
        result: Result<String, String>,
    },
    OpenCronForm,
    CloseCronForm,
    SubmitCronForm,
    DeleteCronJob(Uuid),
    ToggleCronJob(Uuid),
    CronNext,
    CronPrev,
    CronFormNextField,
    CronFormPrevField,
    CronFormBackspace,
    CronFormChar(char),
    CronFormToggleAction,
    LogsLoaded(Result<Vec<String>, String>),
    SecretsLoaded(Vec<String>),
    SubmitSecretForm,
    DeleteSecret(String),
    ContainerNext,
    ContainerPrev,
    SubmitDeploy {
        server_id: Uuid,
        remote_dir: String,
        compose: String,
        routing: Option<DomainSpec>,
    },
    DeployDone(Result<DeployReport, String>),

    // Container actions
    RequestRemoveContainer(String),
    ConfirmDestructive,
    CancelDestructive,

    // Container lifecycle
    StartContainer { server_id: Uuid, name: String },
    StopContainer { server_id: Uuid, name: String },
    RestartContainer { server_id: Uuid, name: String },
    RemoveContainer { server_id: Uuid, name: String },

    // Status
    SetStatus(String),
    SetError(String),
    ClearError,
    UpdateAvailable(UpdateNotice),

    // Form editing
    FormBackspace,
    FormChar(char),
    FormNextField,
    FormPrevField,
    ToggleDeployHttps,
    EditorKey(crossterm::event::KeyEvent),
    EditorSaved,
}
