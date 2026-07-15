use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use uuid::Uuid;

use crate::app::event::Message;
use crate::config::{
    AppDeployment, CronJob, DeploySource, EditorMode, GitAccountMeta, ServerConfig,
    UiMode as ConfigUiMode,
};
use crate::services::github::GitHubRepo;
use crate::i18n::I18n;
use crate::services::docker::{ContainerInfo, ContainerStats};
use crate::services::provision::{ProvisionProgress, ProvisionResult};
use crate::services::ssh::{ConnectionState, SshStatus};
use crate::services::updater::UpdateNotice;

/// Top-level sidebar sections (Dokploy-like: servers vs apps are separate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Home,
    /// SSH targets (machines).
    Servers,
    /// Deployed applications (many per server).
    Apps,
    Monitoring,
    Schedules,
}

impl NavSection {
    pub const ALL: [NavSection; 5] = [
        Self::Home,
        Self::Servers,
        Self::Apps,
        Self::Monitoring,
        Self::Schedules,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::Home => Self::Servers,
            Self::Servers => Self::Apps,
            Self::Apps => Self::Monitoring,
            Self::Monitoring => Self::Schedules,
            Self::Schedules => Self::Home,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Home => Self::Schedules,
            Self::Servers => Self::Home,
            Self::Apps => Self::Servers,
            Self::Monitoring => Self::Apps,
            Self::Schedules => Self::Monitoring,
        }
    }

    pub fn default_screen(self) -> Screen {
        match self {
            Self::Home => Screen::Home,
            Self::Servers => Screen::ServerList,
            Self::Apps => Screen::Apps,
            Self::Monitoring => Screen::Monitoring,
            Self::Schedules => Screen::Schedules,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Overlay,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    AddServer,
    HostKeyPrompt,
    Provisioning,
    Home,
    DeploymentsHub,
    Monitoring,
    Schedules,
    ServerList,
    Containers,
    Logs,
    /// Legacy flat deploy form — primary UX is [`Screen::AppCanvas`].
    #[allow(dead_code)]
    Deploy,
    Apps,
    /// Dokploy-style per-app canvas (tabs: General / Domain / Env / Deploy / Logs).
    AppCanvas,
    /// Step-by-step create flow (type → identity → [account → repo] → canvas).
    NewAppWizard,
    /// Connected GitHub accounts (OAuth Device Flow).
    GitProviders,
    Secrets,
    Editor,
    ConfirmDestructive,
}

/// Steps in the Dokploy-like “Create service” wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewAppStep {
    /// Pick Application (GitHub) or Compose — like Create Service menu.
    Type,
    /// Name / app name / description — like Create Compose modal.
    Identity,
    /// Pick connected GitHub account (Application only).
    Account,
    /// Pick repository for the account (Application only).
    Repo,
}

impl NewAppStep {
    pub fn prev(self, is_github: bool) -> Option<Self> {
        match self {
            Self::Type => None,
            Self::Identity => Some(Self::Type),
            Self::Account => Some(Self::Identity),
            Self::Repo => {
                if is_github {
                    Some(Self::Account)
                } else {
                    Some(Self::Identity)
                }
            }
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Type => 0,
            Self::Identity => 1,
            Self::Account => 2,
            Self::Repo => 3,
        }
    }
}

/// In-progress GitHub Device Flow UI state.
#[derive(Debug, Clone)]
pub struct GitDeviceFlow {
    pub user_code: String,
    pub verification_uri: String,
    pub status: String,
}

/// Tabs inside the app canvas (Dokploy-inspired, TUI-sized subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCanvasTab {
    General,
    Domain,
    Env,
    Deploy,
    Logs,
}

impl AppCanvasTab {
    pub const ALL: [AppCanvasTab; 5] = [
        Self::General,
        Self::Domain,
        Self::Env,
        Self::Deploy,
        Self::Logs,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::General => Self::Domain,
            Self::Domain => Self::Env,
            Self::Env => Self::Deploy,
            Self::Deploy => Self::Logs,
            Self::Logs => Self::General,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::General => Self::Logs,
            Self::Domain => Self::General,
            Self::Env => Self::Domain,
            Self::Deploy => Self::Env,
            Self::Logs => Self::Deploy,
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'g' => Some(Self::General),
            'd' => Some(Self::Domain),
            'e' => Some(Self::Env),
            'p' => Some(Self::Deploy),
            'l' => Some(Self::Logs),
            _ => None,
        }
    }
}

// Screen helpers — keep free of ui imports to avoid cycles.
impl Screen {
    pub fn uses_app_shell(self) -> bool {
        !matches!(
            self,
            Screen::Welcome
                | Screen::AddServer
                | Screen::HostKeyPrompt
                | Screen::Provisioning
                | Screen::ConfirmDestructive
                | Screen::Editor
        )
    }
}

#[derive(Debug, Clone)]
pub struct ServerForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub acme_email: String,
    pub active_field: usize,
}

impl Default for ServerForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "22".into(),
            user: "root".into(),
            acme_email: "admin@example.com".into(),
            active_field: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostKeyPrompt {
    pub server_id: Uuid,
    pub fingerprint: String,
    pub host: String,
    pub port: u16,
    pub after_accept: HostKeyAfterAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyAfterAction {
    Connect,
    Provision,
}

#[derive(Debug, Clone)]
pub struct SecretForm {
    pub key: String,
    pub value: String,
    pub active_field: usize,
}

impl Default for SecretForm {
    fn default() -> Self {
        Self {
            key: String::new(),
            value: String::new(),
            active_field: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployMode {
    Compose,
    GitHub,
}

#[derive(Debug, Clone)]
pub struct DeployForm {
    pub mode: DeployMode,
    pub remote_dir: String,
    pub domain: String,
    pub port: String,
    pub service: String,
    pub https: bool,
    pub compose: String,
    pub active_field: usize,
    /// GitHub owner (e.g. doklabs).
    pub gh_owner: String,
    /// GitHub repo name.
    pub gh_repo: String,
    pub gh_branch: String,
    pub gh_compose_path: String,
    pub auto_deploy: bool,
    pub app_name: String,
    /// Optional blurb from the new-app wizard (not persisted to config yet).
    pub description: String,
    /// Connected GitHub account for deploy (OAuth).
    pub git_account_id: Option<Uuid>,
    pub github_repos: Vec<GitHubRepo>,
    pub selected_repo: usize,
    pub github_branches: Vec<String>,
    pub selected_branch: usize,
}

impl Default for DeployForm {
    fn default() -> Self {
        Self {
            mode: DeployMode::Compose,
            remote_dir: "/opt/doktui/apps/myapp".into(),
            domain: String::new(),
            port: "80".into(),
            service: "app".into(),
            https: true,
            compose: DEFAULT_COMPOSE.into(),
            active_field: 0,
            gh_owner: String::new(),
            gh_repo: String::new(),
            gh_branch: "main".into(),
            gh_compose_path: "docker-compose.yml".into(),
            auto_deploy: true,
            app_name: String::new(),
            description: String::new(),
            git_account_id: None,
            github_repos: Vec::new(),
            selected_repo: 0,
            github_branches: Vec::new(),
            selected_branch: 0,
        }
    }
}

/// Slug for remote dir / app name preview (Dokploy-style app name).
pub fn slugify_app_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if (c.is_whitespace() || c == '-' || c == '_') && !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

impl DeployForm {
    pub fn field_count(&self) -> usize {
        match self.mode {
            DeployMode::Compose => 6,
            DeployMode::GitHub => 10,
        }
    }

    /// Editable fields on the General tab of the app canvas.
    pub fn canvas_general_field_count(&self) -> usize {
        match self.mode {
            DeployMode::Compose => 3, // name, remote_dir, compose
            // name, remote_dir, account, repo, branch, compose_path, auto
            DeployMode::GitHub => 7,
        }
    }

    pub fn canvas_domain_field_count() -> usize {
        4 // domain, port, service, https
    }

    pub fn load_from_app(&mut self, app: &AppDeployment) {
        self.app_name = app.name.clone();
        self.remote_dir = app.remote_dir.clone();
        self.auto_deploy = app.auto_deploy;
        match &app.source {
            DeploySource::ComposePaste => {
                self.mode = DeployMode::Compose;
            }
            DeploySource::GitHub {
                account_id,
                owner,
                repo,
                branch,
                compose_path,
            } => {
                self.mode = DeployMode::GitHub;
                self.git_account_id = *account_id;
                self.gh_owner = owner.clone();
                self.gh_repo = repo.clone();
                self.gh_branch = branch.clone();
                self.gh_compose_path = compose_path.clone();
            }
        }
        if let Some(r) = &app.routing {
            self.domain = r.host.clone();
            self.port = r.port.to_string();
            self.service = r.service.clone();
            self.https = r.https;
        }
        self.active_field = 0;
    }

    pub fn apply_selected_repo(&mut self) {
        if let Some(repo) = self.github_repos.get(self.selected_repo) {
            self.gh_owner = repo.owner.clone();
            self.gh_repo = repo.name.clone();
            self.gh_branch = repo.default_branch.clone();
            if self.app_name.is_empty() {
                self.app_name = repo.name.clone();
            }
            if self.remote_dir == "/opt/doktui/apps/myapp"
                || self.remote_dir.starts_with("/opt/doktui/apps/")
            {
                self.remote_dir = format!("/opt/doktui/apps/{}", repo.name);
            }
        }
    }

    pub fn apply_selected_branch(&mut self) {
        if let Some(b) = self.github_branches.get(self.selected_branch) {
            self.gh_branch = b.clone();
        }
    }
}

const DEFAULT_COMPOSE: &str = r#"services:
  app:
    image: nginx:alpine
    restart: unless-stopped
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronActionKind {
    Restart,
    Redeploy,
}

#[derive(Debug, Clone)]
pub struct CronForm {
    pub label: String,
    pub expression: String,
    pub action_kind: CronActionKind,
    pub target: String,
    pub active_field: usize,
}

impl Default for CronForm {
    fn default() -> Self {
        Self {
            label: String::new(),
            expression: "0 0 3 * * *".into(),
            action_kind: CronActionKind::Redeploy,
            target: "/opt/doktui/apps/myapp".into(),
            active_field: 0,
        }
    }
}

/// Minimum sidebar width in terminal columns.
pub const MIN_SIDEBAR_WIDTH: u16 = 12;
/// Minimum main content width in terminal columns.
pub const MIN_CONTENT_WIDTH: u16 = 10;

pub fn clamp_sidebar_width(requested: u16, terminal_width: u16) -> u16 {
    let max = terminal_width.saturating_sub(MIN_CONTENT_WIDTH);
    requested.clamp(MIN_SIDEBAR_WIDTH, max.max(MIN_SIDEBAR_WIDTH))
}

#[derive(Clone, Debug)]
pub struct ClickRegion {
    pub rect: ratatui::layout::Rect,
    pub msg: Message,
}

/// Hit-test: whether `(col, row)` lies inside `r`.
pub fn hit(r: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    RemoveContainer { name: String },
    RemoveServer { id: Uuid },
}

#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    pub servers: Vec<ServerConfig>,
    pub selected_server: Option<Uuid>,
    pub connection_states: Vec<SshStatus>,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub update_notice: Option<UpdateNotice>,
    pub server_form: ServerForm,
    /// When set, AddServer screen updates this server instead of creating a new one.
    pub editing_server_id: Option<Uuid>,
    pub host_key_prompt: Option<HostKeyPrompt>,
    pub provision_progress: Option<ProvisionProgress>,
    pub provision_result: Option<ProvisionResult>,
    pub containers: Vec<ContainerInfo>,
    pub selected_container: usize,
    pub metrics: Vec<ContainerStats>,
    pub metrics_history: HashMap<String, Vec<u8>>,
    pub metrics_tick: u8,
    pub secret_keys: Vec<String>,
    pub schedules: Vec<crate::services::docker::ScheduleInfo>,
    pub cron_jobs: Vec<CronJob>,
    pub selected_cron: usize,
    pub cron_form: Option<CronForm>,
    pub schedule_check_counter: u32,
    pub secret_form: SecretForm,
    pub logs: Vec<String>,
    pub log_target: Option<String>,
    pub deploy_form: DeployForm,
    pub apps: Vec<AppDeployment>,
    pub selected_app: usize,
    /// When editing an existing app in the canvas; `None` = new draft.
    pub canvas_app_id: Option<Uuid>,
    pub canvas_tab: AppCanvasTab,
    pub wizard_step: NewAppStep,
    /// 0 = Compose, 1 = Application (GitHub).
    pub wizard_type_idx: usize,
    pub git_accounts: Vec<GitAccountMeta>,
    pub selected_git_account: usize,
    pub git_device: Option<GitDeviceFlow>,
    pub auto_deploy_poll_counter: u32,
    pub pending_action: Option<PendingAction>,
    pub onboarding_complete: bool,
    pub public_key: String,
    pub public_key_fingerprint: String,
    pub loading: bool,
    /// True only while a compose deploy is in flight (Home deploy panel).
    pub deploying: bool,
    pub should_quit: bool,
    pub editor: Option<crate::ui::editor::CanvasEditor>,
    pub editor_mode: EditorMode,
    pub nav_section: NavSection,
    pub ui_mode: UiMode,
    pub sidebar_focused: bool,
    pub search_active: bool,
    pub search_query: String,
    pub error_detail: Option<String>,
    pub error_panel_open: bool,
    pub error_scroll: u16,
    pub theme: crate::ui::theme::Theme,
    pub i18n: I18n,
    pub sidebar_width: u16,
    pub shell_body: RefCell<ratatui::layout::Rect>,
    pub sidebar_area: RefCell<ratatui::layout::Rect>,
    pub gutter_rect: RefCell<ratatui::layout::Rect>,
    pub sidebar_resizing: bool,
    /// Animation frame counter (~15 fps); separate from housekeeping tick.
    pub anim_tick: u64,
    pub click_regions: RefCell<Vec<ClickRegion>>,
    pub editor_visible_rows: Cell<usize>,
    pub selected_deploy_menu: usize,
    pub mouse_pos: Option<(u16, u16)>,
    pub achievement: Option<String>,
}

impl AppState {
    pub fn new(
        servers: Vec<ServerConfig>,
        onboarding_complete: bool,
        public_key: String,
        public_key_fingerprint: String,
        editor_mode: EditorMode,
        config_ui_mode: ConfigUiMode,
        cron_jobs: Vec<CronJob>,
        apps: Vec<AppDeployment>,
        git_accounts: Vec<GitAccountMeta>,
        theme: crate::ui::theme::Theme,
        i18n: I18n,
        sidebar_width: u16,
    ) -> Self {
        let screen = if onboarding_complete && !servers.is_empty() {
            Screen::Home
        } else if onboarding_complete {
            Screen::ServerList
        } else {
            Screen::Welcome
        };

        let nav_section = NavSection::Home;

        Self {
            screen,
            nav_section,
            ui_mode: match config_ui_mode {
                ConfigUiMode::Overlay => UiMode::Overlay,
                ConfigUiMode::Compact => UiMode::Compact,
            },
            sidebar_focused: false,
            search_active: false,
            search_query: String::new(),
            error_detail: None,
            error_panel_open: false,
            error_scroll: 0,
            theme,
            i18n,
            sidebar_width,
            shell_body: RefCell::new(ratatui::layout::Rect::default()),
            sidebar_area: RefCell::new(ratatui::layout::Rect::default()),
            gutter_rect: RefCell::new(ratatui::layout::Rect::default()),
            sidebar_resizing: false,
            anim_tick: 0,
            click_regions: RefCell::new(Vec::new()),
            editor_visible_rows: Cell::new(20),
            selected_deploy_menu: 0,
            mouse_pos: None,
            achievement: None,
            servers,
            selected_server: None,
            connection_states: Vec::new(),
            status_message: None,
            error_message: None,
            update_notice: None,
            server_form: ServerForm::default(),
            editing_server_id: None,
            host_key_prompt: None,
            provision_progress: None,
            provision_result: None,
            containers: Vec::new(),
            selected_container: 0,
            metrics: Vec::new(),
            metrics_history: HashMap::new(),
            metrics_tick: 0,
            secret_keys: Vec::new(),
            schedules: Vec::new(),
            cron_jobs,
            selected_cron: 0,
            cron_form: None,
            schedule_check_counter: 0,
            secret_form: SecretForm::default(),
            logs: Vec::new(),
            log_target: None,
            deploy_form: DeployForm::default(),
            apps,
            selected_app: 0,
            canvas_app_id: None,
            canvas_tab: AppCanvasTab::General,
            wizard_step: NewAppStep::Type,
            wizard_type_idx: 0,
            git_accounts,
            selected_git_account: 0,
            git_device: None,
            auto_deploy_poll_counter: 0,
            pending_action: None,
            onboarding_complete,
            public_key,
            public_key_fingerprint,
            loading: false,
            deploying: false,
            should_quit: false,
            editor: None,
            editor_mode,
        }
    }

    pub fn connection_state(&self, id: Uuid) -> ConnectionState {
        self.connection_states
            .iter()
            .find(|s| s.server_id == id)
            .map(|s| s.state)
            .unwrap_or(ConnectionState::Disconnected)
    }

    pub fn connected_server_count(&self) -> (usize, usize) {
        let connected = self
            .servers
            .iter()
            .filter(|s| self.connection_state(s.id) == ConnectionState::Connected)
            .count();
        (connected, self.servers.len())
    }

    pub fn record_metrics_history(&mut self, stats: &[ContainerStats]) {
        for stat in stats {
            let cpu = stat
                .cpu_percent
                .trim_end_matches('%')
                .parse::<u8>()
                .unwrap_or(0);
            let entry = self.metrics_history.entry(stat.name.clone()).or_default();
            entry.push(cpu);
            if entry.len() > 8 {
                entry.remove(0);
            }
        }
    }

    pub fn set_connection(&mut self, status: SshStatus) {
        if let Some(existing) = self
            .connection_states
            .iter_mut()
            .find(|s| s.server_id == status.server_id)
        {
            *existing = status;
        } else {
            self.connection_states.push(status);
        }
    }

    pub fn selected_server_config(&self) -> Option<&ServerConfig> {
        self.selected_server
            .and_then(|id| self.servers.iter().find(|s| s.id == id))
    }

    pub fn selected_container_name(&self) -> Option<String> {
        self.containers
            .get(self.selected_container)
            .map(|c| c.name.clone())
    }

    pub fn go_nav(&mut self, section: NavSection) {
        self.nav_section = section;
        self.screen = section.default_screen();
        self.sidebar_focused = false;
        if section == NavSection::Apps {
            self.selected_deploy_menu = 0;
            if self.selected_app >= self.apps.len() {
                self.selected_app = self.apps.len().saturating_sub(1);
            }
        }
    }

    pub fn push_click(&self, rect: ratatui::layout::Rect, msg: Message) {
        self.click_regions
            .borrow_mut()
            .push(ClickRegion { rect, msg });
    }

    pub fn is_hovered(&self, rect: ratatui::layout::Rect) -> bool {
        matches!(self.mouse_pos, Some((c, r)) if hit(rect, c, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_sidebar_respects_terminal_width() {
        assert_eq!(clamp_sidebar_width(30, 40), 30);
        assert_eq!(clamp_sidebar_width(30, 35), 25);
        assert_eq!(clamp_sidebar_width(8, 40), MIN_SIDEBAR_WIDTH);
    }

    #[test]
    fn config_sidebar_width_defaults() {
        let cfg: crate::config::AppConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.sidebar_width, 22);
        assert_eq!(cfg.locale, "en");
    }
}
