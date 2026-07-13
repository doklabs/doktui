use std::cell::{Cell, RefCell};

use uuid::Uuid;

use crate::app::event::Message;
use crate::config::{CronJob, EditorMode, ServerConfig, UiMode as ConfigUiMode};
use crate::i18n::I18n;
use crate::services::docker::{ContainerInfo, ContainerStats};
use crate::services::provision::{ProvisionProgress, ProvisionResult};
use crate::services::ssh::{ConnectionState, SshStatus};
use crate::services::updater::UpdateNotice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Home,
    Projects,
    Deployments,
    Monitoring,
    Schedules,
}

impl NavSection {
    pub const ALL: [NavSection; 5] = [
        Self::Home,
        Self::Projects,
        Self::Deployments,
        Self::Monitoring,
        Self::Schedules,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::Home => Self::Projects,
            Self::Projects => Self::Deployments,
            Self::Deployments => Self::Monitoring,
            Self::Monitoring => Self::Schedules,
            Self::Schedules => Self::Home,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Home => Self::Schedules,
            Self::Projects => Self::Home,
            Self::Deployments => Self::Projects,
            Self::Monitoring => Self::Deployments,
            Self::Schedules => Self::Monitoring,
        }
    }

    pub fn default_screen(self) -> Screen {
        match self {
            Self::Home => Screen::Home,
            Self::Projects => Screen::ServerList,
            Self::Deployments => Screen::DeploymentsHub,
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
    Deploy,
    Secrets,
    Editor,
    ConfirmDestructive,
}

// Screen helpers — keep free of ui imports to avoid cycles.
impl Screen {
    pub fn uses_app_shell(self) -> bool {
        !matches!(
            self,
            Screen::Welcome
                | Screen::AddServer
                | Screen::HostKeyPrompt
                |             Screen::Provisioning
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

#[derive(Debug, Clone)]
pub struct DeployForm {
    pub remote_dir: String,
    pub domain: String,
    pub port: String,
    pub service: String,
    pub https: bool,
    pub compose: String,
    pub active_field: usize,
}

impl Default for DeployForm {
    fn default() -> Self {
        Self {
            remote_dir: "/opt/doktui/apps/myapp".into(),
            domain: String::new(),
            port: "80".into(),
            service: "app".into(),
            https: true,
            compose: DEFAULT_COMPOSE.into(),
            active_field: 0,
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
    pub host_key_prompt: Option<HostKeyPrompt>,
    pub provision_progress: Option<ProvisionProgress>,
    pub provision_result: Option<ProvisionResult>,
    pub containers: Vec<ContainerInfo>,
    pub selected_container: usize,
    pub metrics: Vec<ContainerStats>,
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
    pub pending_action: Option<PendingAction>,
    pub onboarding_complete: bool,
    pub public_key: String,
    pub public_key_fingerprint: String,
    pub loading: bool,
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
            host_key_prompt: None,
            provision_progress: None,
            provision_result: None,
            containers: Vec::new(),
            selected_container: 0,
            metrics: Vec::new(),
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
            pending_action: None,
            onboarding_complete,
            public_key,
            public_key_fingerprint,
            loading: false,
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
        if section == NavSection::Deployments {
            self.selected_deploy_menu = 0;
        }
    }

    pub fn push_click(&self, rect: ratatui::layout::Rect, msg: Message) {
        self.click_regions.borrow_mut().push(ClickRegion { rect, msg });
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
