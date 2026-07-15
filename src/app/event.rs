use uuid::Uuid;

use crate::app::state::{HostKeyAfterAction, NavSection};
use crate::config::AppDeployment;
use crate::services::docker::{ContainerInfo, ContainerStats, DeployReport, ScheduleInfo};
use crate::services::github::GitHubRepo;
use crate::services::provision::{ProvisionProgress, ProvisionResult};
use crate::services::routing::DomainSpec;
use crate::services::ssh::SshStatus;
use crate::services::updater::UpdateNotice;

/// GitHub source fields for a deploy request.
#[derive(Debug, Clone)]
pub struct GitHubDeployRequest {
    pub account_id: Option<Uuid>,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub compose_path: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Quit,
    Resize(u16),

    // Navigation
    GoHome,
    CopyPublicKey,
    GoNav(NavSection),
    GoServerList,
    GoAddServer,
    GoEditServer,
    GoContainers,
    GoLogs,
    GoDeploy,
    GoApps,
    /// Open app canvas for the selected (or given) app.
    OpenAppCanvas,
    /// Start Dokploy-style step-by-step create wizard.
    NewAppCanvas,
    WizardNext,
    WizardPrev,
    WizardSelectType(usize),
    /// Finish wizard → open AppCanvas with form filled.
    WizardFinish,
    WizardSelectAccount(usize),
    WizardSelectRepo(usize),
    GoGitProviders,
    GitConnectStart,
    GitConnectCancel,
    GitDeviceStarted {
        user_code: String,
        verification_uri: String,
    },
    GitConnectFailed(String),
    GitAccountConnected(crate::config::GitAccountMeta),
    GitDeleteAccount(Uuid),
    /// Select git account in canvas / providers list.
    SelectGitAccount(Uuid),
    CanvasTabNext,
    CanvasTabPrev,
    CanvasSetTab(crate::app::state::AppCanvasTab),
    /// Submit deploy from the canvas action bar / Ctrl+D.
    CanvasDeploy,
    /// Containers / logs / secrets / editor hub for the selected server.
    GoAppTools,
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
    ServerNext,
    ServerPrev,
    RequestRemoveServer(Uuid),

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
    DeployHubNext,
    DeployHubPrev,
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
        github: Option<GitHubDeployRequest>,
        app_name: String,
        auto_deploy: bool,
    },
    DeployDone(Result<DeployReport, String>),
    RedeployApp(Uuid),
    AppUpserted(AppDeployment),
    LoadGitHubRepos(Option<Uuid>),
    GitHubReposLoaded(Result<Vec<GitHubRepo>, String>),
    LoadGitHubBranches {
        account_id: Option<Uuid>,
        owner: String,
        repo: String,
    },
    GitHubBranchesLoaded(Result<Vec<String>, String>),
    ToggleDeployMode,
    ToggleDeployAuto,
    AppsNext,
    AppsPrev,
    DeleteApp(Uuid),

    // Container actions
    RequestRemoveContainer(String),
    ConfirmDestructive,
    CancelDestructive,

    // Container lifecycle
    StartContainer {
        server_id: Uuid,
        name: String,
    },
    StopContainer {
        server_id: Uuid,
        name: String,
    },
    RestartContainer {
        server_id: Uuid,
        name: String,
    },
    RemoveContainer {
        server_id: Uuid,
        name: String,
    },

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
}
