use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    poll, read, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, Mutex};

use crate::config::{bootstrap, AcmeChallenge, AppConfig, CronAction, CronJob, ServerConfig};
use crate::i18n::I18n;
use crate::security::{hostkey, keys};
use crate::services::routing::{self, DomainSpec};
use crate::services::secrets::SecretsManager;
use crate::ui::{self, layout};

use self::command::{save_new_server, CommandBus};
use self::event::{GitHubDeployRequest, Message};
use self::state::{
    clamp_sidebar_width, hit, AppCanvasTab, AppState, CronActionKind, CronForm, DeployForm,
    DeployMode, HostKeyAfterAction, NavSection, NewAppStep, Screen, ServerForm, UiMode,
};

pub mod command;
pub mod event;
pub mod state;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const FRAME_RATE: Duration = Duration::from_millis(66);
const HOUSEKEEPING: Duration = Duration::from_millis(1000);

pub async fn run_tui(theme_override: Option<String>) -> Result<()> {
    let config = bootstrap()?;
    let public_key = keys::load_public_key_openssh()?;
    let public_key_fingerprint = keys::public_key_fingerprint().unwrap_or_default();
    let config = Arc::new(Mutex::new(config));

    let secrets = Arc::new(Mutex::new(SecretsManager::load_or_create()?));

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let (ssh_tx, mut ssh_rx) = mpsc::unbounded_channel();
    let bridge_tx = tx.clone();
    tokio::spawn(async move {
        while let Some(status) = ssh_rx.recv().await {
            let _ = bridge_tx.send(Message::SshStatus(status));
        }
    });

    let auto_reconnect = config.lock().await.auto_reconnect;
    let check_updates = config.lock().await.check_updates;

    let (i18n, mut state) = {
        let cfg = config.lock().await;
        let theme_name = theme_override.unwrap_or_else(|| cfg.theme.clone());
        let theme = ui::theme::ThemeRegistry::active(&theme_name);
        let i18n = I18n::load(&cfg.locale)?;
        let locale_fallback = i18n.used_fallback();
        let locale_tag = cfg.locale.clone();
        let sidebar_width = cfg.sidebar_width;
        let mut s = AppState::new(
            cfg.servers.clone(),
            cfg.onboarding_complete,
            public_key.trim().to_string(),
            public_key_fingerprint,
            cfg.editor_mode.clone(),
            cfg.ui_mode.clone(),
            cfg.cron_jobs.clone(),
            cfg.apps.clone(),
            cfg.git_accounts.clone(),
            theme,
            i18n.clone(),
            sidebar_width,
        );
        if locale_fallback {
            s.status_message = Some(format!("locale `{locale_tag}` unavailable — using English"));
        }
        (i18n, s)
    };

    let bus = CommandBus::new(
        tx.clone(),
        config.clone(),
        secrets,
        i18n,
        auto_reconnect,
        ssh_tx,
    );
    bus.spawn_update_check(VERSION, check_updates);

    enable_raw_mode()?;
    let mouse_enabled = config.lock().await.mouse;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    if mouse_enabled {
        stdout.execute(EnableMouseCapture)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_frame = std::time::Instant::now();
    let mut last_housekeeping = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &state))?;

        let timeout = FRAME_RATE.saturating_sub(last_frame.elapsed());

        if poll(timeout)? {
            while poll(Duration::ZERO)? {
                match read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if let Some(msg) = map_key(key, &state) {
                            update(&mut state, msg.clone(), &config, &bus).await;
                            if spawns_background(&msg) {
                                bus.dispatch(msg);
                            }
                        }
                    }
                    Event::Mouse(me) if mouse_enabled => {
                        if let Some(msg) = handle_mouse(&mut state, me) {
                            update(&mut state, msg.clone(), &config, &bus).await;
                            if spawns_background(&msg) {
                                bus.dispatch(msg);
                            }
                        }
                    }
                    Event::Resize(w, _) => {
                        update(&mut state, Message::Resize(w), &config, &bus).await;
                    }
                    _ => {}
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            update(&mut state, msg, &config, &bus).await;
        }

        if last_frame.elapsed() >= FRAME_RATE {
            state.anim_tick = state.anim_tick.wrapping_add(1);
            last_frame = std::time::Instant::now();
        }

        if last_housekeeping.elapsed() >= HOUSEKEEPING {
            update(&mut state, Message::Tick, &config, &bus).await;
            last_housekeeping = std::time::Instant::now();
        }

        if state.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    if mouse_enabled {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Messages that start async I/O — handled by CommandBus, not re-sent to the channel.
fn spawns_background(msg: &Message) -> bool {
    matches!(
        msg,
        Message::ConnectServer(_)
            | Message::ProvisionServer(_)
            | Message::StartContainer { .. }
            | Message::StopContainer { .. }
            | Message::RestartContainer { .. }
            | Message::RemoveContainer { .. }
            | Message::RedeployApp(_)
            | Message::LoadGitHubRepos(_)
            | Message::LoadGitHubBranches { .. }
            | Message::GitConnectStart
            | Message::GitDeleteAccount(_)
            | Message::RunCronJob(_)
            | Message::SubmitSecretForm
            | Message::DeleteSecret(_)
    )
}

fn is_enter(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter | KeyCode::Char('\r'))
}

fn key_char(code: KeyCode) -> Option<char> {
    if let KeyCode::Char(c) = code {
        Some(c)
    } else {
        None
    }
}

#[allow(dead_code)]
fn key_is(code: KeyCode, ch: char) -> bool {
    key_char(code).is_some_and(|c| c.eq_ignore_ascii_case(&ch))
}

fn map_key(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Message> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
        return Some(Message::ToggleSearch);
    }

    if state.search_active {
        return match key.code {
            KeyCode::Esc => Some(Message::CloseSearch),
            KeyCode::Backspace => Some(Message::SearchBackspace),
            KeyCode::Char(c) => Some(Message::SearchChar(c)),
            _ => None,
        };
    }

    if state.error_panel_open {
        return match key.code {
            KeyCode::Esc => Some(Message::CloseErrorPanel),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::ErrorScrollUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::ErrorScrollDown),
            _ => None,
        };
    }

    if state.cron_form.is_some() {
        return match key.code {
            KeyCode::Esc => Some(Message::CloseCronForm),
            KeyCode::Tab => Some(Message::CronFormNextField),
            KeyCode::BackTab => Some(Message::CronFormPrevField),
            KeyCode::Backspace => Some(Message::CronFormBackspace),
            KeyCode::Char(' ') => Some(Message::CronFormToggleAction),
            code if is_enter(code) => Some(Message::SubmitCronForm),
            KeyCode::Char(c) => Some(Message::CronFormChar(c)),
            _ => None,
        };
    }

    if state.error_detail.is_some() && matches!(key.code, KeyCode::Char('E')) {
        return Some(Message::ToggleErrorPanel);
    }

    if state.error_message.is_some() && !state.error_panel_open && key.code == KeyCode::Esc {
        return Some(Message::ClearError);
    }

    if state.screen.uses_app_shell() {
        if key.code == KeyCode::Tab
            && !matches!(state.screen, Screen::AppCanvas | Screen::NewAppWizard)
        {
            return Some(Message::ToggleSidebarFocus);
        }
        if key.code == KeyCode::Char('m')
            && !matches!(
                state.screen,
                Screen::Deploy | Screen::AppCanvas | Screen::NewAppWizard
            )
        {
            return Some(Message::ToggleUiMode);
        }

        if state.sidebar_focused {
            return match key.code {
                KeyCode::Down | KeyCode::Char('j') => Some(Message::SidebarNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Message::SidebarPrev),
                code if is_enter(code) => Some(Message::GoNav(state.nav_section)),
                KeyCode::Right | KeyCode::Char('l') => Some(Message::ToggleSidebarFocus),
                KeyCode::Char('[') => Some(Message::SidebarNarrow),
                KeyCode::Char(']') => Some(Message::SidebarWiden),
                KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
                KeyCode::Char(c) => layout::section_from_char(c).map(Message::GoNav),
                _ => None,
            };
        }
    }

    let screen_msg = match state.screen {
        Screen::Welcome => match key.code {
            code if is_enter(code) => Some(Message::GoAddServer),
            KeyCode::Char('c') => Some(Message::CopyPublicKey),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::AddServer => match key.code {
            KeyCode::Esc => Some(Message::GoServerList),
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => Some(Message::FormNextField),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            code if is_enter(code) => Some(Message::SubmitServerForm),
            KeyCode::Char(c) => Some(Message::FormChar(c)),
            _ => None,
        },
        Screen::HostKeyPrompt => match key.code {
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'y') => Some(Message::AcceptHostKey),
            code if is_enter(code) => Some(Message::AcceptHostKey),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'n') => Some(Message::RejectHostKey),
            KeyCode::Esc => Some(Message::RejectHostKey),
            _ => None,
        },
        Screen::Provisioning => None,
        Screen::Home => match key.code {
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::DeploymentsHub => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoApps),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::DeployHubNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::DeployHubPrev),
            code if is_enter(code) => deploy_hub_message(state.selected_deploy_menu),
            KeyCode::Char('d') => Some(Message::GoDeploy),
            KeyCode::Char('c') => Some(Message::GoContainers),
            KeyCode::Char('l') => Some(Message::GoLogs),
            KeyCode::Char('v') | KeyCode::Char('s') => Some(Message::GoSecrets),
            KeyCode::Char('g') => Some(Message::GoGitProviders),
            KeyCode::Char('e') => Some(Message::GoEditor),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Apps => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoHome),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::AppsNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::AppsPrev),
            code if is_enter(code) => {
                if state.apps.is_empty() {
                    Some(Message::NewAppCanvas)
                } else {
                    Some(Message::OpenAppCanvas)
                }
            }
            KeyCode::Char('x') => state
                .apps
                .get(state.selected_app)
                .map(|a| Message::DeleteApp(a.id)),
            KeyCode::Char('n') | KeyCode::Char('d') => Some(Message::NewAppCanvas),
            KeyCode::Char('t') => Some(Message::GoAppTools),
            KeyCode::Char('c') if state.selected_server.is_some() => {
                state.selected_server.map(Message::ConnectServer)
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::AppCanvas => map_canvas_key(key.code, key.modifiers, state),
        Screen::NewAppWizard => map_wizard_key(key.code, state),
        Screen::GitProviders => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => {
                if state.git_device.is_some() {
                    Some(Message::GitConnectCancel)
                } else {
                    Some(Message::GoApps)
                }
            }
            KeyCode::Char('c') | KeyCode::Char('n') if state.git_device.is_none() => {
                Some(Message::GitConnectStart)
            }
            KeyCode::Down | KeyCode::Char('j') if state.git_device.is_none() => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectAccount(
                        (state.selected_git_account + 1) % state.git_accounts.len(),
                    ))
                }
            }
            KeyCode::Up | KeyCode::Char('k') if state.git_device.is_none() => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectAccount(
                        state
                            .selected_git_account
                            .checked_sub(1)
                            .unwrap_or(state.git_accounts.len() - 1),
                    ))
                }
            }
            KeyCode::Char('x') if state.git_device.is_none() => state
                .git_accounts
                .get(state.selected_git_account)
                .map(|a| Message::GitDeleteAccount(a.id)),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Monitoring => match key.code {
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Schedules => match key.code {
            KeyCode::Char('a') => Some(Message::OpenCronForm),
            code if is_enter(code) => Some(Message::OpenCronForm),
            KeyCode::Char('x') => state
                .cron_jobs
                .get(state.selected_cron)
                .map(|j| Message::DeleteCronJob(j.id)),
            KeyCode::Char('t') => state
                .cron_jobs
                .get(state.selected_cron)
                .map(|j| Message::ToggleCronJob(j.id)),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::CronNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::CronPrev),
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoHome),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::ServerList => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoHome),
            KeyCode::Char('a') => Some(Message::GoAddServer),
            KeyCode::Char('e') if state.selected_server.is_some() => Some(Message::GoEditServer),
            KeyCode::Char('c') if state.selected_server.is_some() => {
                state.selected_server.map(Message::ConnectServer)
            }
            KeyCode::Char('p') if state.selected_server.is_some() => {
                state.selected_server.map(Message::ProvisionServer)
            }
            KeyCode::Char('x') if state.selected_server.is_some() => {
                Some(Message::RequestRemoveServer(state.selected_server.unwrap()))
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Message::ServerNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::ServerPrev),
            code if is_enter(code) => Some(Message::GoNav(NavSection::Apps)),
            KeyCode::Char(d @ '1'..='9') => {
                let idx = (d as u8 - b'1') as usize;
                state
                    .servers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| {
                        layout::filter_match(&s.name, &state.search_query)
                            && layout::filter_match(&s.host, &state.search_query)
                    })
                    .nth(idx)
                    .map(|(_, s)| Message::SelectServer(s.id))
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Containers => {
            let name = state.selected_container_name();
            let server_id = state.selected_server;
            match key.code {
                KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Apps)),
                KeyCode::Down | KeyCode::Char('j') => Some(Message::ContainerNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Message::ContainerPrev),
                KeyCode::Char('x') if name.is_some() && server_id.is_some() => {
                    Some(Message::RequestRemoveContainer(name.unwrap()))
                }
                KeyCode::Char('s') if name.is_some() && server_id.is_some() => {
                    Some(Message::StopContainer {
                        server_id: server_id.unwrap(),
                        name: name.unwrap(),
                    })
                }
                KeyCode::Char('S') if name.is_some() && server_id.is_some() => {
                    Some(Message::StartContainer {
                        server_id: server_id.unwrap(),
                        name: name.unwrap(),
                    })
                }
                KeyCode::Char('r') if name.is_some() && server_id.is_some() => {
                    Some(Message::RestartContainer {
                        server_id: server_id.unwrap(),
                        name: name.unwrap(),
                    })
                }
                KeyCode::Char('l') if name.is_some() => Some(Message::GoLogs),
                KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
                _ => None,
            }
        }
        Screen::Logs => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Apps)),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Deploy => map_deploy_key(key.code, state),
        Screen::Secrets => match key.code {
            KeyCode::Esc => Some(Message::GoNav(NavSection::Apps)),
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => Some(Message::FormNextField),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            code if is_enter(code) => Some(Message::SubmitSecretForm),
            KeyCode::Char('x')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && !state.secret_keys.is_empty() =>
            {
                Some(Message::DeleteSecret(
                    state.secret_keys[state.secret_keys.len() - 1].clone(),
                ))
            }
            KeyCode::Char(c) => Some(Message::FormChar(c)),
            _ => None,
        },
        Screen::Editor => Some(Message::EditorKey(key)),
        Screen::ConfirmDestructive => match key.code {
            KeyCode::Char('y') => Some(Message::ConfirmDestructive),
            KeyCode::Char('n') | KeyCode::Esc => Some(Message::CancelDestructive),
            _ => None,
        },
    };

    if screen_msg.is_some() {
        return screen_msg;
    }

    // Don't steal digits / h while typing in forms or the create wizard.
    let typing_screen = matches!(
        state.screen,
        Screen::NewAppWizard
            | Screen::AppCanvas
            | Screen::Deploy
            | Screen::Secrets
            | Screen::AddServer
            | Screen::GitProviders
    );
    if state.screen.uses_app_shell() && !state.sidebar_focused && !typing_screen {
        if let KeyCode::Char(c) = key.code {
            if let Some(section) = layout::section_from_char(c) {
                return Some(Message::GoNav(section));
            }
        }
        if matches!(key.code, KeyCode::Left | KeyCode::Char('h')) {
            return Some(Message::ToggleSidebarFocus);
        }
    }

    None
}

fn deploy_hub_message(index: usize) -> Option<Message> {
    match index {
        0 => Some(Message::NewAppCanvas),
        1 => Some(Message::GoContainers),
        2 => Some(Message::GoLogs),
        3 => Some(Message::GoSecrets),
        4 => Some(Message::GoGitProviders),
        5 => Some(Message::GoEditor),
        _ => None,
    }
}

fn map_wizard_key(code: KeyCode, state: &AppState) -> Option<Message> {
    match state.wizard_step {
        NewAppStep::Type => match code {
            KeyCode::Esc => Some(Message::GoApps),
            KeyCode::Down | KeyCode::Char('j') => {
                Some(Message::WizardSelectType((state.wizard_type_idx + 1) % 2))
            }
            KeyCode::Up | KeyCode::Char('k') => Some(Message::WizardSelectType(
                state.wizard_type_idx.checked_sub(1).unwrap_or(1),
            )),
            KeyCode::Char('1') => Some(Message::WizardSelectType(0)),
            KeyCode::Char('2') => Some(Message::WizardSelectType(1)),
            code if is_enter(code) || matches!(code, KeyCode::Tab | KeyCode::Right) => {
                Some(Message::WizardNext)
            }
            KeyCode::Char(_) => None,
            _ => None,
        },
        NewAppStep::Identity => match code {
            KeyCode::Esc => Some(Message::WizardPrev),
            KeyCode::Tab | KeyCode::Down => Some(Message::FormNextField),
            KeyCode::BackTab | KeyCode::Up => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            code if is_enter(code) => {
                if state.deploy_form.active_field < 2 {
                    Some(Message::FormNextField)
                } else {
                    Some(Message::WizardNext)
                }
            }
            KeyCode::Char(c) => Some(Message::FormChar(c)),
            _ => None,
        },
        NewAppStep::Account => match code {
            KeyCode::Esc => Some(Message::WizardPrev),
            KeyCode::Down | KeyCode::Char('j') => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectAccount(
                        (state.selected_git_account + 1) % state.git_accounts.len(),
                    ))
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectAccount(
                        state
                            .selected_git_account
                            .checked_sub(1)
                            .unwrap_or(state.git_accounts.len() - 1),
                    ))
                }
            }
            KeyCode::Char('c') | KeyCode::Char('g') => Some(Message::GoGitProviders),
            code if is_enter(code) || matches!(code, KeyCode::Tab | KeyCode::Right) => {
                if state.git_accounts.is_empty() {
                    Some(Message::GoGitProviders)
                } else {
                    Some(Message::WizardNext)
                }
            }
            _ => None,
        },
        NewAppStep::Repo => match code {
            KeyCode::Esc => Some(Message::WizardPrev),
            KeyCode::Down | KeyCode::Char('j') => {
                if state.deploy_form.github_repos.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectRepo(
                        (state.deploy_form.selected_repo + 1)
                            % state.deploy_form.github_repos.len(),
                    ))
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.deploy_form.github_repos.is_empty() {
                    None
                } else {
                    Some(Message::WizardSelectRepo(
                        state
                            .deploy_form
                            .selected_repo
                            .checked_sub(1)
                            .unwrap_or(state.deploy_form.github_repos.len() - 1),
                    ))
                }
            }
            KeyCode::Char('r') => {
                Some(Message::LoadGitHubRepos(state.deploy_form.git_account_id))
            }
            code if is_enter(code) => Some(Message::WizardFinish),
            _ => None,
        },
    }
}

fn map_canvas_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Option<Message> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('d') | KeyCode::Char('D') => Some(Message::CanvasDeploy),
            KeyCode::Char('m') | KeyCode::Char('M')
                if state.canvas_tab == AppCanvasTab::General =>
            {
                Some(Message::ToggleDeployMode)
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if state.canvas_tab == AppCanvasTab::General
                    && state.deploy_form.mode == DeployMode::GitHub =>
            {
                Some(Message::LoadGitHubRepos(state.deploy_form.git_account_id))
            }
            KeyCode::Char('e') | KeyCode::Char('E')
                if state.canvas_tab == AppCanvasTab::General
                    && state.deploy_form.mode == DeployMode::Compose =>
            {
                Some(Message::GoEditor)
            }
            _ => None,
        };
    }

    // Text fields win: every printable char types; no letter shortcuts.
    if canvas_allows_typing(state) {
        return match code {
            KeyCode::Esc => Some(Message::GoApps),
            KeyCode::Tab | KeyCode::Down => Some(Message::FormNextField),
            KeyCode::BackTab | KeyCode::Up => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            KeyCode::Enter => Some(Message::FormNextField),
            KeyCode::Char(c) => Some(Message::FormChar(c)),
            _ => None,
        };
    }

    // Account / repo / branch pickers (GitHub General).
    if canvas_gh_pick(state) {
        let field = state.deploy_form.active_field;
        return match code {
            KeyCode::Esc => Some(Message::GoApps),
            KeyCode::Tab => Some(Message::FormNextField),
            KeyCode::BackTab => Some(Message::FormPrevField),
            KeyCode::Down | KeyCode::Char(']') if field == 2 => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    let next = (state.selected_git_account + 1) % state.git_accounts.len();
                    Some(Message::SelectGitAccount(state.git_accounts[next].id))
                }
            }
            KeyCode::Up | KeyCode::Char('[') if field == 2 => {
                if state.git_accounts.is_empty() {
                    None
                } else {
                    let prev = state
                        .selected_git_account
                        .checked_sub(1)
                        .unwrap_or(state.git_accounts.len() - 1);
                    Some(Message::SelectGitAccount(state.git_accounts[prev].id))
                }
            }
            KeyCode::Down | KeyCode::Char(']') => Some(Message::FormChar(']')),
            KeyCode::Up | KeyCode::Char('[') => Some(Message::FormChar('[')),
            KeyCode::Enter => Some(Message::FormNextField),
            KeyCode::Char('r') => {
                Some(Message::LoadGitHubRepos(state.deploy_form.git_account_id))
            }
            KeyCode::Char(c) if field == 4 => Some(Message::FormChar(c)),
            KeyCode::Backspace if field == 4 => Some(Message::FormBackspace),
            _ => None,
        };
    }

    if canvas_on_toggle_field(state) {
        return match code {
            KeyCode::Esc => Some(Message::GoApps),
            KeyCode::Char(' ') => canvas_space_message(state),
            KeyCode::Tab | KeyCode::Down => Some(Message::FormNextField),
            KeyCode::BackTab | KeyCode::Up => Some(Message::FormPrevField),
            KeyCode::Enter => Some(Message::FormNextField),
            KeyCode::Char(c) => AppCanvasTab::from_char(c)
                .map(Message::CanvasSetTab)
                .or(None),
            _ => None,
        };
    }

    // Non-typing tabs (Env / Deploy / Logs) and navigation chrome.
    if let KeyCode::Char(c) = code {
        if let Some(tab) = AppCanvasTab::from_char(c) {
            return Some(Message::CanvasSetTab(tab));
        }
    }

    match code {
        KeyCode::Esc => Some(Message::GoApps),
        KeyCode::Tab | KeyCode::Right => Some(Message::CanvasTabNext),
        KeyCode::BackTab | KeyCode::Left => Some(Message::CanvasTabPrev),
            KeyCode::Char('r')
                if state.canvas_tab == AppCanvasTab::General
                    && state.deploy_form.mode == DeployMode::GitHub =>
            {
                Some(Message::LoadGitHubRepos(state.deploy_form.git_account_id))
            }
        KeyCode::Char('r') if state.canvas_app_id.is_some() => {
            state.canvas_app_id.map(Message::RedeployApp)
        }
        KeyCode::Char('s') if state.canvas_tab == AppCanvasTab::Env => Some(Message::GoSecrets),
        KeyCode::Char('m') if state.canvas_tab == AppCanvasTab::General => {
            Some(Message::ToggleDeployMode)
        }
        KeyCode::Enter if state.canvas_tab == AppCanvasTab::Deploy => Some(Message::CanvasDeploy),
        _ => None,
    }
}

fn canvas_allows_typing(state: &AppState) -> bool {
    let f = &state.deploy_form;
    match state.canvas_tab {
        AppCanvasTab::General => match f.mode {
            DeployMode::Compose => true,
            // Account (2) / repo (3) are pickers; auto (6) is toggle.
            DeployMode::GitHub => matches!(f.active_field, 0 | 1 | 4 | 5),
        },
        AppCanvasTab::Domain => !canvas_on_toggle_field(state),
        _ => false,
    }
}

fn canvas_on_toggle_field(state: &AppState) -> bool {
    let f = &state.deploy_form;
    match state.canvas_tab {
        AppCanvasTab::General if f.mode == DeployMode::GitHub && f.active_field == 6 => true,
        AppCanvasTab::Domain if f.active_field == 3 => true,
        _ => false,
    }
}

fn canvas_gh_pick(state: &AppState) -> bool {
    state.canvas_tab == AppCanvasTab::General
        && state.deploy_form.mode == DeployMode::GitHub
        && matches!(state.deploy_form.active_field, 2 | 3 | 4)
}

fn canvas_space_message(state: &AppState) -> Option<Message> {
    let f = &state.deploy_form;
    match state.canvas_tab {
        AppCanvasTab::General if f.mode == DeployMode::GitHub && f.active_field == 6 => {
            Some(Message::ToggleDeployAuto)
        }
        AppCanvasTab::Domain if f.active_field == 3 => Some(Message::ToggleDeployHttps),
        _ => None,
    }
}

fn canvas_field_count(state: &AppState) -> usize {
    match state.canvas_tab {
        AppCanvasTab::General => state.deploy_form.canvas_general_field_count(),
        AppCanvasTab::Domain => DeployForm::canvas_domain_field_count(),
        _ => 0,
    }
}

fn map_deploy_key(code: KeyCode, state: &AppState) -> Option<Message> {
    let form = &state.deploy_form;
    let max_field = form.field_count().saturating_sub(1);
    let gh_pick = form.mode == DeployMode::GitHub && matches!(form.active_field, 1 | 2);
    let on_toggle = (form.mode == DeployMode::Compose && form.active_field == 4)
        || (form.mode == DeployMode::GitHub && matches!(form.active_field, 8 | 9));

    match code {
        KeyCode::Esc => Some(Message::GoNav(NavSection::Apps)),
        KeyCode::Tab | KeyCode::Down if gh_pick && matches!(code, KeyCode::Down) => {
            Some(Message::FormChar(']'))
        }
        KeyCode::BackTab | KeyCode::Up if gh_pick && matches!(code, KeyCode::Up) => {
            Some(Message::FormChar('['))
        }
        KeyCode::Tab | KeyCode::Down => Some(Message::FormNextField),
        KeyCode::BackTab | KeyCode::Up => Some(Message::FormPrevField),
        KeyCode::Backspace => Some(Message::FormBackspace),
        KeyCode::Char(' ') if on_toggle => {
            if form.mode == DeployMode::GitHub && form.active_field == 9 {
                Some(Message::ToggleDeployAuto)
            } else {
                Some(Message::ToggleDeployHttps)
            }
        }
        KeyCode::Enter if form.active_field < max_field => Some(Message::FormNextField),
        KeyCode::Enter => build_submit_deploy(state),
        // Letter shortcuts only when not typing into a value field.
        KeyCode::Char('m') if on_toggle => Some(Message::ToggleDeployMode),
        KeyCode::Char('r') if on_toggle && form.mode == DeployMode::GitHub => {
            Some(Message::LoadGitHubRepos(form.git_account_id))
        }
        KeyCode::Char(c) if !on_toggle => Some(Message::FormChar(c)),
        _ => None,
    }
}

fn build_submit_deploy(state: &AppState) -> Option<Message> {
    let form = &state.deploy_form;
    let Some(id) = state
        .selected_server
        .or_else(|| state.servers.first().map(|s| s.id))
    else {
        return Some(Message::SetError(state.i18n.t("error-add-server-first")));
    };
    let routing = deploy_routing_from_form(form);
    if let Some(ref spec) = routing {
        if let Err(e) = routing::validate_domain_spec(spec) {
            return Some(Message::SetError(e.to_string()));
        }
    }
    let github = if form.mode == DeployMode::GitHub {
        let (owner, repo) = if form.gh_owner.contains('/') {
            let (o, r) = form.gh_owner.split_once('/').unwrap();
            (o.trim().to_string(), r.trim().to_string())
        } else {
            (
                form.gh_owner.trim().to_string(),
                form.gh_repo.trim().to_string(),
            )
        };
        if owner.is_empty() || repo.is_empty() {
            return Some(Message::SetError(
                state.i18n.t("error-github-repo-required"),
            ));
        }
        if form.git_account_id.is_none() {
            return Some(Message::SetError(
                state.i18n.t("error-git-account-required"),
            ));
        }
        Some(GitHubDeployRequest {
            account_id: form.git_account_id,
            owner,
            repo,
            branch: if form.gh_branch.trim().is_empty() {
                "main".into()
            } else {
                form.gh_branch.trim().to_string()
            },
            compose_path: if form.gh_compose_path.trim().is_empty() {
                "docker-compose.yml".into()
            } else {
                form.gh_compose_path.trim().to_string()
            },
        })
    } else {
        None
    };
    Some(Message::SubmitDeploy {
        server_id: id,
        remote_dir: form.remote_dir.clone(),
        compose: form.compose.clone(),
        routing,
        github,
        app_name: form.app_name.clone(),
        auto_deploy: form.auto_deploy,
    })
}

fn apply_sidebar_drag(state: &mut AppState, column: u16) {
    let body = *state.shell_body.borrow();
    let requested = column.saturating_sub(body.x);
    state.sidebar_width = clamp_sidebar_width(requested, body.width);
}

fn handle_mouse(state: &mut AppState, me: MouseEvent) -> Option<Message> {
    match me.kind {
        MouseEventKind::Moved => {
            state.mouse_pos = Some((me.column, me.row));
            if state.sidebar_resizing {
                apply_sidebar_drag(state, me.column);
            }
            None
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if layout::gutter_hit(state, me.column, me.row) {
                state.sidebar_resizing = true;
                apply_sidebar_drag(state, me.column);
                return None;
            }
            state
                .click_regions
                .borrow()
                .iter()
                .find(|c| hit(c.rect, me.column, me.row))
                .map(|c| c.msg.clone())
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if state.sidebar_resizing {
                apply_sidebar_drag(state, me.column);
            }
            None
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if state.sidebar_resizing {
                state.sidebar_resizing = false;
                return Some(Message::SidebarResizeEnd);
            }
            None
        }
        MouseEventKind::ScrollDown => scroll_message(state, true),
        MouseEventKind::ScrollUp => scroll_message(state, false),
        _ => None,
    }
}

fn scroll_message(state: &AppState, down: bool) -> Option<Message> {
    if state.error_panel_open {
        return Some(if down {
            Message::ErrorScrollDown
        } else {
            Message::ErrorScrollUp
        });
    }
    if state.sidebar_focused {
        return Some(if down {
            Message::SidebarNext
        } else {
            Message::SidebarPrev
        });
    }
    match state.screen {
        Screen::Containers => Some(if down {
            Message::ContainerNext
        } else {
            Message::ContainerPrev
        }),
        Screen::Schedules if state.cron_form.is_none() => Some(if down {
            Message::CronNext
        } else {
            Message::CronPrev
        }),
        _ => None,
    }
}

async fn update(
    state: &mut AppState,
    msg: Message,
    config: &Arc<Mutex<AppConfig>>,
    bus: &CommandBus,
) {
    match msg {
        Message::Quit => {
            bus.disconnect_all();
            state.should_quit = true;
        }
        Message::Tick => {
            if matches!(state.screen, Screen::Home | Screen::Monitoring) {
                state.metrics_tick = state.metrics_tick.wrapping_add(1);
            }
            if state.screen == Screen::Monitoring {
                if let Some(id) = state.selected_server {
                    bus.load_metrics(id);
                }
            }
            state.schedule_check_counter = state.schedule_check_counter.saturating_add(1);
            if state.schedule_check_counter >= 60 {
                state.schedule_check_counter = 0;
                let due: Vec<uuid::Uuid> = {
                    let cfg = config.lock().await;
                    cfg.cron_jobs
                        .iter()
                        .filter(|j| j.enabled)
                        .filter(|j| {
                            crate::services::cron::is_due(&j.expression, j.last_run.as_deref())
                                .unwrap_or(false)
                        })
                        .map(|j| j.id)
                        .collect()
                };
                for id in due {
                    bus.dispatch(Message::RunCronJob(id));
                }
            }
            // Auto-deploy poll (~60s) for GitHub apps while DokTUI is open.
            state.auto_deploy_poll_counter = state.auto_deploy_poll_counter.saturating_add(1);
            if state.auto_deploy_poll_counter >= 60 && !state.deploying {
                state.auto_deploy_poll_counter = 0;
                let due = bus.poll_auto_deploy_candidates().await;
                for id in due {
                    if !state.deploying {
                        state.loading = true;
                        state.deploying = true;
                        state.status_message =
                            Some(state.i18n.t("status-auto-deploy"));
                        bus.dispatch(Message::RedeployApp(id));
                    }
                }
            }
        }
        Message::GoHome => {
            state.go_nav(NavSection::Home);
            state.error_message = None;
        }
        Message::CopyPublicKey => {
            match arboard::Clipboard::new()
                .and_then(|mut clip| clip.set_text(state.public_key.clone()))
            {
                Ok(()) => state.status_message = Some(state.i18n.t("status-copied-key")),
                Err(e) => push_error(
                    state,
                    state
                        .i18n
                        .t_fmt("error-clipboard", &[("err", &e.to_string())]),
                ),
            }
        }
        Message::GoNav(section) => {
            state.go_nav(section);
            state.error_message = None;
            state.error_detail = None;
            state.error_panel_open = false;
            if section == NavSection::Servers && state.selected_server.is_none() {
                state.selected_server = state.servers.first().map(|s| s.id);
            }
            if section == NavSection::Monitoring {
                state.loading = true;
                if let Some(id) = state.selected_server {
                    bus.load_metrics(id);
                }
            }
            if section == NavSection::Schedules {
                state.loading = true;
                if let Some(id) = state.selected_server {
                    bus.load_schedules(id);
                } else {
                    state.loading = false;
                    push_error(state, state.i18n.t("error-select-server"));
                }
            }
        }
        Message::SidebarNext => {
            state.nav_section = state.nav_section.next();
        }
        Message::SidebarPrev => {
            state.nav_section = state.nav_section.prev();
        }
        Message::ToggleSidebarFocus => {
            state.sidebar_focused = !state.sidebar_focused;
        }
        Message::ToggleSearch => {
            state.search_active = !state.search_active;
            if !state.search_active {
                state.search_query.clear();
            }
        }
        Message::CloseSearch => {
            state.search_active = false;
            state.search_query.clear();
        }
        Message::SearchChar(c) => state.search_query.push(c),
        Message::SearchBackspace => {
            state.search_query.pop();
        }
        Message::ToggleUiMode => {
            state.ui_mode = match state.ui_mode {
                UiMode::Overlay => UiMode::Compact,
                UiMode::Compact => UiMode::Overlay,
            };
            let mut cfg = config.lock().await;
            cfg.ui_mode = match state.ui_mode {
                UiMode::Overlay => crate::config::UiMode::Overlay,
                UiMode::Compact => crate::config::UiMode::Compact,
            };
            let _ = cfg.save();
        }
        Message::SidebarNarrow => {
            let body = *state.shell_body.borrow();
            state.sidebar_width =
                clamp_sidebar_width(state.sidebar_width.saturating_sub(1), body.width);
            persist_sidebar_width(state, config).await;
        }
        Message::SidebarWiden => {
            let body = *state.shell_body.borrow();
            state.sidebar_width =
                clamp_sidebar_width(state.sidebar_width.saturating_add(1), body.width);
            persist_sidebar_width(state, config).await;
        }
        Message::SidebarResizeEnd => {
            persist_sidebar_width(state, config).await;
        }
        Message::GoServerList => {
            state.editing_server_id = None;
            state.go_nav(NavSection::Servers);
            if state.selected_server.is_none() {
                state.selected_server = state.servers.first().map(|s| s.id);
            }
        }
        Message::GoAddServer => {
            state.screen = Screen::AddServer;
            state.editing_server_id = None;
            let acme = config.lock().await.acme_email.clone();
            state.server_form = ServerForm {
                acme_email: acme,
                ..Default::default()
            };
        }
        Message::GoEditServer => {
            let Some(id) = state.selected_server else {
                push_error(state, state.i18n.t("error-select-server"));
                return;
            };
            let Some(srv) = state.servers.iter().find(|s| s.id == id).cloned() else {
                push_error(state, state.i18n.t("error-select-server"));
                return;
            };
            let acme = config.lock().await.acme_email.clone();
            state.editing_server_id = Some(id);
            state.server_form = ServerForm {
                name: srv.name,
                host: srv.host,
                port: srv.port.to_string(),
                user: srv.user,
                acme_email: acme,
                active_field: 0,
            };
            state.screen = Screen::AddServer;
        }
        Message::GoContainers => {
            state.nav_section = NavSection::Apps;
            state.screen = Screen::Containers;
            state.selected_container = 0;
            state.loading = true;
            if let Some(id) = state.selected_server {
                bus.load_containers(id);
            } else {
                state.loading = false;
                push_error(state, state.i18n.t("error-select-server"));
            }
        }
        Message::GoLogs => {
            state.nav_section = NavSection::Apps;
            state.screen = Screen::Logs;
            state.loading = true;
            state.log_target = state.selected_container_name();
            if let Some(id) = state.selected_server {
                let container = state.log_target.clone();
                bus.load_logs(id, container);
            } else {
                state.loading = false;
                push_error(state, state.i18n.t("error-select-server"));
            }
        }
        Message::GoSecrets => {
            state.nav_section = NavSection::Apps;
            state.screen = Screen::Secrets;
            state.secret_form = Default::default();
            bus.load_secrets();
        }
        Message::GoDeploy | Message::NewAppCanvas => {
            open_new_app_wizard(state);
        }
        Message::WizardSelectType(idx) => {
            state.wizard_type_idx = idx.min(1);
            state.deploy_form.mode = if state.wizard_type_idx == 1 {
                DeployMode::GitHub
            } else {
                DeployMode::Compose
            };
        }
        Message::WizardNext => match state.wizard_step {
            NewAppStep::Type => {
                state.deploy_form.mode = if state.wizard_type_idx == 1 {
                    DeployMode::GitHub
                } else {
                    DeployMode::Compose
                };
                state.wizard_step = NewAppStep::Identity;
                state.deploy_form.active_field = 0;
            }
            NewAppStep::Identity => {
                if state.wizard_type_idx == 1 || state.deploy_form.mode == DeployMode::GitHub {
                    state.wizard_step = NewAppStep::Account;
                    if state.selected_git_account >= state.git_accounts.len() {
                        state.selected_git_account = 0;
                    }
                    if let Some(acc) = state.git_accounts.get(state.selected_git_account) {
                        state.deploy_form.git_account_id = Some(acc.id);
                    }
                } else {
                    finish_new_app_wizard(state, bus);
                }
            }
            NewAppStep::Account => {
                if let Some(acc) = state.git_accounts.get(state.selected_git_account).cloned() {
                    state.deploy_form.git_account_id = Some(acc.id);
                    state.wizard_step = NewAppStep::Repo;
                    state.status_message = Some(state.i18n.t("status-loading-github"));
                    bus.dispatch(Message::LoadGitHubRepos(Some(acc.id)));
                } else {
                    push_error(state, state.i18n.t("error-git-account-required"));
                }
            }
            NewAppStep::Repo => {
                finish_new_app_wizard(state, bus);
            }
        },
        Message::WizardPrev => {
            let is_github =
                state.wizard_type_idx == 1 || state.deploy_form.mode == DeployMode::GitHub;
            match state.wizard_step.prev(is_github) {
                Some(step) => {
                    state.wizard_step = step;
                    state.deploy_form.active_field = 0;
                }
                None => state.go_nav(NavSection::Apps),
            }
        },
        Message::WizardSelectAccount(idx) => {
            if !state.git_accounts.is_empty() {
                state.selected_git_account = idx % state.git_accounts.len();
                if let Some(acc) = state.git_accounts.get(state.selected_git_account) {
                    state.deploy_form.git_account_id = Some(acc.id);
                }
            }
        }
        Message::WizardSelectRepo(idx) => {
            if !state.deploy_form.github_repos.is_empty() {
                state.deploy_form.selected_repo = idx % state.deploy_form.github_repos.len();
                state.deploy_form.apply_selected_repo();
            }
        }
        Message::WizardFinish => {
            finish_new_app_wizard(state, bus);
        }
        Message::GoGitProviders => {
            state.nav_section = NavSection::Apps;
            state.screen = Screen::GitProviders;
            state.git_device = None;
        }
        Message::GitConnectStart => {
            state.git_device = Some(state::GitDeviceFlow {
                user_code: String::new(),
                verification_uri: String::new(),
                status: state.i18n.t("git-device-starting"),
            });
            state.status_message = Some(state.i18n.t("git-device-starting"));
        }
        Message::GitConnectCancel => {
            state.git_device = None;
        }
        Message::GitDeviceStarted {
            user_code,
            verification_uri,
        } => {
            state.git_device = Some(state::GitDeviceFlow {
                user_code,
                verification_uri,
                status: state.i18n.t("git-device-waiting"),
            });
        }
        Message::GitConnectFailed(err) => {
            state.git_device = None;
            push_error(state, err);
        }
        Message::GitAccountConnected(account) => {
            state.git_device = None;
            if let Some(idx) = state.git_accounts.iter().position(|a| a.id == account.id) {
                state.git_accounts[idx] = account.clone();
                state.selected_git_account = idx;
            } else {
                state.git_accounts.push(account.clone());
                state.selected_git_account = state.git_accounts.len() - 1;
            }
            state.deploy_form.git_account_id = Some(account.id);
            state.status_message = Some(state.i18n.t_fmt(
                "status-git-connected",
                &[("login", &account.login)],
            ));
        }
        Message::GitDeleteAccount(id) => {
            state.git_accounts.retain(|a| a.id != id);
            if state.deploy_form.git_account_id == Some(id) {
                state.deploy_form.git_account_id = None;
            }
            if state.selected_git_account >= state.git_accounts.len() {
                state.selected_git_account = state.git_accounts.len().saturating_sub(1);
            }
            state.status_message = Some(state.i18n.t("status-git-account-removed"));
        }
        Message::SelectGitAccount(id) => {
            state.deploy_form.git_account_id = Some(id);
            if let Some(idx) = state.git_accounts.iter().position(|a| a.id == id) {
                state.selected_git_account = idx;
            }
            state.status_message = Some(state.i18n.t("status-loading-github"));
            bus.dispatch(Message::LoadGitHubRepos(Some(id)));
        }
        Message::OpenAppCanvas => {
            open_selected_app_canvas(state, bus);
        }
        Message::CanvasTabNext => {
            state.canvas_tab = state.canvas_tab.next();
            state.deploy_form.active_field = 0;
            canvas_on_tab_enter(state, bus);
        }
        Message::CanvasTabPrev => {
            state.canvas_tab = state.canvas_tab.prev();
            state.deploy_form.active_field = 0;
            canvas_on_tab_enter(state, bus);
        }
        Message::CanvasSetTab(tab) => {
            state.canvas_tab = tab;
            state.deploy_form.active_field = 0;
            canvas_on_tab_enter(state, bus);
        }
        Message::CanvasDeploy => match build_submit_deploy(state) {
            Some(Message::SetError(e)) => push_error(state, e),
            Some(Message::SubmitDeploy {
                server_id,
                remote_dir,
                compose,
                routing,
                github,
                app_name,
                auto_deploy,
            }) => {
                if let Some(ref spec) = routing {
                    if spec.is_wildcard() {
                        let challenge = config.lock().await.acme_challenge;
                        if challenge != AcmeChallenge::DnsCloudflare {
                            push_error(state, state.i18n.t("error-wildcard-dns"));
                            return;
                        }
                    }
                }
                state.loading = true;
                state.deploying = true;
                bus.dispatch(Message::SubmitDeploy {
                    server_id,
                    remote_dir,
                    compose,
                    routing,
                    github,
                    app_name,
                    auto_deploy,
                });
            }
            _ => {}
        }
        Message::GoApps => {
            state.go_nav(NavSection::Apps);
        }
        Message::GoAppTools => {
            state.nav_section = NavSection::Apps;
            state.screen = Screen::DeploymentsHub;
            state.selected_deploy_menu = 0;
            if state.selected_server.is_none() {
                state.selected_server = state.servers.first().map(|s| s.id);
            }
        }
        Message::AppsNext => {
            if !state.apps.is_empty() {
                state.selected_app = (state.selected_app + 1).min(state.apps.len() - 1);
            }
        }
        Message::AppsPrev => {
            state.selected_app = state.selected_app.saturating_sub(1);
        }
        Message::DeleteApp(id) => {
            state.apps.retain(|a| a.id != id);
            let mut cfg = config.lock().await;
            cfg.apps.retain(|a| a.id != id);
            let _ = cfg.save();
            if state.selected_app >= state.apps.len() {
                state.selected_app = state.apps.len().saturating_sub(1);
            }
            state.status_message = Some(state.i18n.t("status-app-removed"));
        }
        Message::AppUpserted(app) => {
            if state.screen == Screen::AppCanvas {
                state.canvas_app_id = Some(app.id);
            }
            if let Some(idx) = state.apps.iter().position(|a| a.id == app.id) {
                state.apps[idx] = app;
                state.selected_app = idx;
            } else {
                state.apps.push(app);
                state.selected_app = state.apps.len() - 1;
            }
        }
        Message::ToggleDeployMode => {
            state.deploy_form.mode = match state.deploy_form.mode {
                DeployMode::Compose => DeployMode::GitHub,
                DeployMode::GitHub => DeployMode::Compose,
            };
            state.deploy_form.active_field = 0;
        }
        Message::ToggleDeployAuto => {
            state.deploy_form.auto_deploy = !state.deploy_form.auto_deploy;
        }
        Message::LoadGitHubRepos(_) => {
            state.status_message = Some(state.i18n.t("status-loading-github"));
        }
        Message::LoadGitHubBranches { .. } => {}
        Message::GitHubReposLoaded(result) => match result {
            Ok(repos) => {
                state.deploy_form.github_repos = repos;
                // Prefer already-selected repo if still present.
                if let Some(idx) = state.deploy_form.github_repos.iter().position(|r| {
                    r.owner == state.deploy_form.gh_owner && r.name == state.deploy_form.gh_repo
                }) {
                    state.deploy_form.selected_repo = idx;
                } else {
                    state.deploy_form.selected_repo = 0;
                    state.deploy_form.apply_selected_repo();
                }
                state.status_message = Some(state.i18n.t("status-github-repos-loaded"));
                if !state.deploy_form.gh_owner.is_empty() {
                    bus.dispatch(Message::LoadGitHubBranches {
                        account_id: state.deploy_form.git_account_id,
                        owner: state.deploy_form.gh_owner.clone(),
                        repo: state.deploy_form.gh_repo.clone(),
                    });
                }
            }
            Err(e) => push_error(state, e),
        },
        Message::GitHubBranchesLoaded(result) => match result {
            Ok(branches) => {
                state.deploy_form.github_branches = branches;
                state.deploy_form.selected_branch = state
                    .deploy_form
                    .github_branches
                    .iter()
                    .position(|b| b == &state.deploy_form.gh_branch)
                    .unwrap_or(0);
                state.deploy_form.apply_selected_branch();
            }
            Err(e) => push_error(state, e),
        },
        Message::RedeployApp(_) => {
            state.loading = true;
            state.deploying = true;
        }
        Message::GoEditor => {
            let content = state.deploy_form.compose.clone();
            let path = format!(
                "{}/docker-compose.yml",
                state.deploy_form.remote_dir.trim_end_matches('/')
            );
            state.editor = Some(crate::ui::editor::CanvasEditor::open(
                path,
                &content,
                state.editor_mode.clone(),
            ));
            state.screen = Screen::Editor;
        }
        Message::EditorKey(key) => {
            use crate::ui::editor::EditorAction;
            let action = if let Some(editor) = state.editor.as_mut() {
                let action = editor.handle_key(key);
                let visible = state.editor_visible_rows.get().max(1);
                editor.clamp_scroll(visible);
                action
            } else {
                EditorAction::None
            };
            match action {
                EditorAction::Quit => {
                    if let Some(ed) = state.editor.take() {
                        state.deploy_form.compose = ed.content();
                    }
                    state.screen = Screen::AppCanvas;
                    state.nav_section = NavSection::Apps;
                    state.canvas_tab = AppCanvasTab::General;
                }
                EditorAction::Saved => {
                    state.status_message = Some(state.i18n.t("status-editor-saved"));
                }
                EditorAction::None => {}
            }
        }
        Message::SubmitServerForm => {
            let form = state.server_form.clone();
            if form.name.is_empty() || form.host.is_empty() {
                push_error(state, state.i18n.t("error-name-host-required"));
            } else if let Some(edit_id) = state.editing_server_id.take() {
                let port: u16 = form.port.parse().unwrap_or(22);
                let mut cfg = config.lock().await;
                if let Some(srv) = cfg.server_mut(edit_id) {
                    srv.name = form.name.clone();
                    srv.host = form.host.clone();
                    srv.port = port;
                    srv.user = form.user.clone();
                }
                if !form.acme_email.is_empty() {
                    cfg.acme_email = form.acme_email.clone();
                }
                if cfg.save().is_err() {
                    drop(cfg);
                    push_error(state, state.i18n.t("error-save-server"));
                    state.editing_server_id = Some(edit_id);
                    return;
                }
                drop(cfg);
                if let Some(srv) = state.servers.iter_mut().find(|s| s.id == edit_id) {
                    srv.name = form.name;
                    srv.host = form.host;
                    srv.port = port;
                    srv.user = form.user;
                }
                state.selected_server = Some(edit_id);
                state.status_message = Some(state.i18n.t("status-server-updated"));
                state.screen = Screen::ServerList;
            } else {
                let port: u16 = form.port.parse().unwrap_or(22);
                let server = ServerConfig::new(
                    form.name.clone(),
                    form.host.clone(),
                    port,
                    form.user.clone(),
                );
                state.selected_server = Some(server.id);
                if save_new_server(config, server.clone()).await.is_ok() {
                    let mut cfg = config.lock().await;
                    if !form.acme_email.is_empty() {
                        cfg.acme_email = form.acme_email.clone();
                        let _ = cfg.save();
                    }
                    drop(cfg);
                    state.servers.push(server);
                    state.onboarding_complete = true;
                    state.screen = Screen::Provisioning;
                    state.loading = true;
                    state.provision_progress = None;
                    if let Some(id) = state.selected_server {
                        bus.dispatch(Message::ProvisionServer(id));
                    }
                } else {
                    push_error(state, state.i18n.t("error-save-server"));
                }
            }
        }
        Message::SelectServer(id) => {
            if state.selected_server != Some(id) {
                bus.disconnect_server(state.selected_server);
            }
            state.selected_server = Some(id);
        }
        Message::ServerNext => {
            let filtered: Vec<&crate::config::ServerConfig> = state
                .servers
                .iter()
                .filter(|s| {
                    layout::filter_match(&s.name, &state.search_query)
                        && layout::filter_match(&s.host, &state.search_query)
                })
                .collect();
            if filtered.is_empty() {
                state.selected_server = None;
            } else if let Some(id) = state.selected_server {
                let current = filtered.iter().position(|s| s.id == id).unwrap_or(0);
                let next = (current + 1) % filtered.len();
                state.selected_server = Some(filtered[next].id);
            } else {
                state.selected_server = filtered.first().map(|s| s.id);
            }
        }
        Message::ServerPrev => {
            let filtered: Vec<&crate::config::ServerConfig> = state
                .servers
                .iter()
                .filter(|s| {
                    layout::filter_match(&s.name, &state.search_query)
                        && layout::filter_match(&s.host, &state.search_query)
                })
                .collect();
            if filtered.is_empty() {
                state.selected_server = None;
            } else if let Some(id) = state.selected_server {
                let current = filtered.iter().position(|s| s.id == id).unwrap_or(0);
                let prev = (current + filtered.len() - 1) % filtered.len();
                state.selected_server = Some(filtered[prev].id);
            } else {
                state.selected_server = filtered.last().map(|s| s.id);
            }
        }
        Message::RequestRemoveServer(id) => {
            state.pending_action = Some(crate::app::state::PendingAction::RemoveServer { id });
            state.screen = Screen::ConfirmDestructive;
        }
        Message::ProvisionServer(id) => {
            state.screen = Screen::Provisioning;
            state.loading = true;
            state.selected_server = Some(id);
        }
        Message::ProvisionProgress(p) => {
            state.provision_progress = Some(p);
        }
        Message::ProvisionDone(result) => {
            state.loading = false;
            match result {
                Ok(res) => {
                    state.provision_result = Some(res);
                    state.go_nav(NavSection::Home);
                    state.status_message = Some(state.i18n.t("status-provisioned"));
                }
                Err(e) => {
                    push_error(state, e);
                    state.screen = Screen::ServerList;
                }
            }
        }
        Message::SshStatus(status) => {
            if let Some(msg) = status.message.clone() {
                state.status_message = Some(msg);
            }
            state.set_connection(status);
        }
        Message::ContainersLoaded(result) => {
            state.loading = false;
            match result {
                Ok(list) => {
                    state.containers = list;
                    if state.selected_container >= state.containers.len() {
                        state.selected_container = state.containers.len().saturating_sub(1);
                    }
                }
                Err(e) => push_error(state, e),
            }
        }
        Message::MetricsLoaded(result) => {
            state.loading = false;
            match result {
                Ok(stats) => {
                    state.record_metrics_history(&stats);
                    state.metrics = stats;
                }
                Err(e) => push_error(state, e),
            }
        }
        Message::SchedulesLoaded(result) => {
            state.loading = false;
            match result {
                Ok(list) => state.schedules = list,
                Err(e) => push_error(state, e),
            }
        }
        Message::OpenCronForm => {
            state.cron_form = Some(CronForm::default());
        }
        Message::CloseCronForm => {
            state.cron_form = None;
        }
        Message::CronNext => {
            if !state.cron_jobs.is_empty() {
                state.selected_cron = (state.selected_cron + 1).min(state.cron_jobs.len() - 1);
            }
        }
        Message::CronPrev => {
            state.selected_cron = state.selected_cron.saturating_sub(1);
        }
        Message::DeployHubNext => {
            state.selected_deploy_menu = (state.selected_deploy_menu + 1).min(5);
        }
        Message::DeployHubPrev => {
            state.selected_deploy_menu = state.selected_deploy_menu.saturating_sub(1);
        }
        Message::CronFormNextField => {
            if let Some(form) = &mut state.cron_form {
                form.active_field = (form.active_field + 1).min(3);
            }
        }
        Message::CronFormPrevField => {
            if let Some(form) = &mut state.cron_form {
                form.active_field = form.active_field.saturating_sub(1);
            }
        }
        Message::CronFormBackspace => cron_form_backspace(state),
        Message::CronFormChar(c) => cron_form_char(state, c),
        Message::CronFormToggleAction => {
            if let Some(form) = &mut state.cron_form {
                form.action_kind = match form.action_kind {
                    CronActionKind::Restart => CronActionKind::Redeploy,
                    CronActionKind::Redeploy => CronActionKind::Restart,
                };
            }
        }
        Message::SubmitCronForm => {
            let Some(form) = state.cron_form.take() else {
                return;
            };
            if form.label.trim().is_empty() {
                push_error(state, state.i18n.t("error-schedule-label"));
                state.cron_form = Some(form);
                return;
            }
            if let Err(e) = crate::services::cron::validate_expression(&form.expression) {
                push_error(state, e.to_string());
                state.cron_form = Some(form);
                return;
            }
            let Some(server_id) = state
                .selected_server
                .or_else(|| state.servers.first().map(|s| s.id))
            else {
                push_error(state, state.i18n.t("error-select-server"));
                state.cron_form = Some(form);
                return;
            };
            let action = match form.action_kind {
                CronActionKind::Restart => CronAction::RestartContainer {
                    container: form.target.trim().to_string(),
                },
                CronActionKind::Redeploy => CronAction::Redeploy {
                    remote_dir: form.target.trim().to_string(),
                },
            };
            let job = CronJob {
                id: uuid::Uuid::new_v4(),
                label: form.label.trim().to_string(),
                server_id,
                expression: form.expression.trim().to_string(),
                action,
                enabled: true,
                last_run: None,
            };
            let mut cfg = config.lock().await;
            cfg.cron_jobs.push(job);
            let _ = cfg.save();
            state.cron_jobs = cfg.cron_jobs.clone();
            state.status_message = Some(state.i18n.t("status-cron-saved"));
        }
        Message::DeleteCronJob(id) => {
            let mut cfg = config.lock().await;
            cfg.cron_jobs.retain(|j| j.id != id);
            let _ = cfg.save();
            state.cron_jobs = cfg.cron_jobs.clone();
            state.selected_cron = state
                .selected_cron
                .min(state.cron_jobs.len().saturating_sub(1));
            state.status_message = Some(state.i18n.t("status-cron-deleted"));
        }
        Message::ToggleCronJob(id) => {
            let mut cfg = config.lock().await;
            if let Some(job) = cfg.cron_jobs.iter_mut().find(|j| j.id == id) {
                job.enabled = !job.enabled;
            }
            let _ = cfg.save();
            state.cron_jobs = cfg.cron_jobs.clone();
        }
        Message::CronJobDone { id, result } => {
            let label = state
                .cron_jobs
                .iter()
                .find(|j| j.id == id)
                .map(|j| j.label.clone())
                .unwrap_or_else(|| id.to_string());
            match result {
                Ok(msg) => {
                    state.status_message = Some(
                        state
                            .i18n
                            .t_fmt("cmd-schedule-result", &[("label", &label), ("msg", &msg)]),
                    );
                }
                Err(e) => push_error(state, e),
            }
        }
        Message::RunCronJob(_) => {}
        Message::SecretsLoaded(keys) => {
            state.secret_keys = keys;
        }
        Message::SubmitSecretForm => {
            let form = &state.secret_form;
            if form.key.is_empty() {
                push_error(state, state.i18n.t("error-secret-key"));
            } else {
                bus.save_secret(form.key.clone(), form.value.clone());
                state.secret_form = Default::default();
            }
        }
        Message::DeleteSecret(key) => {
            bus.delete_secret(key);
        }
        Message::ContainerNext => {
            if !state.containers.is_empty() {
                state.selected_container =
                    (state.selected_container + 1).min(state.containers.len() - 1);
            }
        }
        Message::ContainerPrev => {
            state.selected_container = state.selected_container.saturating_sub(1);
        }
        Message::LogsLoaded(result) => {
            state.loading = false;
            match result {
                Ok(lines) => state.logs = lines,
                Err(e) => push_error(state, e),
            }
        }
        Message::SubmitDeploy {
            server_id,
            remote_dir,
            compose,
            routing,
            github,
            app_name,
            auto_deploy,
        } => {
            if let Some(ref spec) = routing {
                if spec.is_wildcard() {
                    let challenge = config.lock().await.acme_challenge;
                    if challenge != AcmeChallenge::DnsCloudflare {
                        push_error(state, state.i18n.t("error-wildcard-dns"));
                        return;
                    }
                }
            }
            if let Some(ref gh) = github {
                if gh.owner.is_empty() || gh.repo.is_empty() {
                    push_error(state, state.i18n.t("error-github-repo-required"));
                    return;
                }
            }
            state.loading = true;
            state.deploying = true;
            bus.dispatch(Message::SubmitDeploy {
                server_id,
                remote_dir,
                compose,
                routing,
                github,
                app_name,
                auto_deploy,
            });
        }
        Message::ToggleDeployHttps => {
            state.deploy_form.https = !state.deploy_form.https;
        }
        Message::DeployDone(result) => {
            state.loading = false;
            state.deploying = false;
            match result {
                Ok(report) => {
                    if report.all_ok() {
                        state.status_message = Some(report.summary());
                        state.error_message = None;
                        state.error_detail = None;
                        if state.deploy_form.https && !state.deploy_form.domain.trim().is_empty() {
                            state.achievement = Some(state.i18n.t("achievement-first-https"));
                        }
                    } else {
                        state.status_message = Some(state.i18n.t("status-deploy-warnings"));
                        push_error(state, report.summary());
                    }
                    if state.screen == Screen::AppCanvas {
                        state.canvas_tab = AppCanvasTab::Deploy;
                    } else {
                        state.go_nav(NavSection::Apps);
                    }
                }
                Err(e) => push_error(state, e),
            }
        }
        Message::RequestRemoveContainer(name) => {
            state.pending_action = Some(state::PendingAction::RemoveContainer { name });
            state.screen = Screen::ConfirmDestructive;
        }
        Message::ConfirmDestructive => match state.pending_action.take() {
            Some(state::PendingAction::RemoveContainer { name }) => {
                state.loading = true;
                if let Some(id) = state.selected_server {
                    bus.dispatch(Message::RemoveContainer {
                        server_id: id,
                        name,
                    });
                }
                state.screen = Screen::Containers;
            }
            Some(state::PendingAction::RemoveServer { id }) => {
                let name = state
                    .servers
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.name.clone());
                state.servers.retain(|s| s.id != id);
                {
                    let mut cfg = config.lock().await;
                    cfg.servers.retain(|s| s.id != id);
                    let _ = cfg.save();
                }
                state.connection_states.retain(|s| s.server_id != id);
                state.metrics.clear();
                state.metrics_history.clear();
                bus.disconnect_server(Some(id));
                state.selected_server = None;
                state.go_nav(NavSection::Servers);
                state.selected_server = state.servers.first().map(|s| s.id);
                if let Some(name) = name {
                    state.status_message = Some(
                        state
                            .i18n
                            .t_fmt("status-server-removed", &[("name", &name)]),
                    );
                }
            }
            None => {}
        },
        Message::CancelDestructive => {
            state.screen = match &state.pending_action {
                Some(state::PendingAction::RemoveServer { .. }) => Screen::ServerList,
                _ => Screen::Containers,
            };
            state.pending_action = None;
        }
        Message::SetStatus(s) => {
            state.status_message = Some(s);
            state.error_message = None;
        }
        Message::SetError(e) => push_error(state, e),
        Message::ClearError => {
            state.error_message = None;
            state.error_detail = None;
            state.error_panel_open = false;
            state.error_scroll = 0;
        }
        Message::ToggleErrorPanel => {
            if state.error_detail.is_some() {
                state.error_panel_open = !state.error_panel_open;
                state.error_scroll = 0;
            }
        }
        Message::CloseErrorPanel => {
            state.error_panel_open = false;
            state.error_scroll = 0;
        }
        Message::ErrorScrollUp => {
            state.error_scroll = state.error_scroll.saturating_sub(1);
        }
        Message::ErrorScrollDown => {
            if let Some(detail) = &state.error_detail {
                let max = detail.lines().count().saturating_sub(1) as u16;
                state.error_scroll = (state.error_scroll + 1).min(max);
            }
        }
        Message::UpdateAvailable(notice) => state.update_notice = Some(notice),
        Message::FormBackspace => edit_backspace(state),
        Message::FormChar(c) => edit_char(state, c),
        Message::FormNextField => edit_next_field(state),
        Message::FormPrevField => edit_prev_field(state),
        Message::AcceptHostKey => {
            if let Some(prompt) = state.host_key_prompt.take() {
                match hostkey::KnownHosts::load() {
                    Ok(mut known) => {
                        if let Err(e) =
                            known.trust_fingerprint(&prompt.host, prompt.port, &prompt.fingerprint)
                        {
                            push_error(state, e.to_string());
                        } else {
                            state.screen = Screen::ServerList;
                            match prompt.after_accept {
                                HostKeyAfterAction::Connect => {
                                    bus.dispatch(Message::ConnectServer(prompt.server_id));
                                }
                                HostKeyAfterAction::Provision => {
                                    state.screen = Screen::Provisioning;
                                    state.loading = true;
                                    bus.dispatch(Message::ProvisionServer(prompt.server_id));
                                }
                            }
                        }
                    }
                    Err(e) => push_error(state, e.to_string()),
                }
            }
        }
        Message::RejectHostKey => {
            state.host_key_prompt = None;
            state.screen = Screen::ServerList;
            push_error(state, state.i18n.t("error-hostkey-rejected"));
        }
        Message::HostKeyRequired {
            server_id,
            host,
            port,
            fingerprint,
            after_accept,
        } => {
            state.loading = false;
            state.host_key_prompt = Some(state::HostKeyPrompt {
                server_id,
                host: host.clone(),
                port,
                fingerprint,
                after_accept,
            });
            state.screen = Screen::HostKeyPrompt;
        }
        Message::ConnectServer(id) => {
            state.selected_server = Some(id);
            state.status_message = Some(state.i18n.t("status-connecting"));
        }
        Message::StartContainer { .. }
        | Message::StopContainer { .. }
        | Message::RestartContainer { .. }
        | Message::RemoveContainer { .. } => {
            state.loading = true;
        }
        Message::Resize(w) => {
            state.sidebar_width = clamp_sidebar_width(state.sidebar_width, w);
            let mut cfg = config.lock().await;
            cfg.sidebar_width = state.sidebar_width;
            let _ = cfg.save();
        }
    }
}

async fn persist_sidebar_width(state: &AppState, config: &Arc<Mutex<AppConfig>>) {
    let mut cfg = config.lock().await;
    cfg.sidebar_width = state.sidebar_width;
    let _ = cfg.save();
}

fn edit_backspace(state: &mut AppState) {
    match state.screen {
        Screen::AddServer => {
            let field = &mut state.server_form;
            match field.active_field {
                0 => {
                    field.name.pop();
                }
                1 => {
                    field.host.pop();
                }
                2 => {
                    field.port.pop();
                }
                3 => {
                    field.user.pop();
                }
                _ => {
                    field.acme_email.pop();
                }
            }
        }
        Screen::Deploy => deploy_form_backspace(state),
        Screen::AppCanvas => canvas_form_backspace(state),
        Screen::NewAppWizard => wizard_form_backspace(state),
        Screen::Secrets => {
            let field = &mut state.secret_form;
            match field.active_field {
                0 => {
                    field.key.pop();
                }
                _ => {
                    field.value.pop();
                }
            }
        }
        _ => {}
    }
}

fn edit_char(state: &mut AppState, c: char) {
    match state.screen {
        Screen::AddServer => {
            let field = &mut state.server_form;
            match field.active_field {
                0 => field.name.push(c),
                1 => field.host.push(c),
                2 if c.is_ascii_digit() => field.port.push(c),
                3 => field.user.push(c),
                4 => field.acme_email.push(c),
                _ => {}
            }
        }
        Screen::Deploy => deploy_form_char(state, c),
        Screen::AppCanvas => canvas_form_char(state, c),
        Screen::NewAppWizard => wizard_form_char(state, c),
        Screen::Secrets => {
            let field = &mut state.secret_form;
            match field.active_field {
                0 => field.key.push(c),
                _ => field.value.push(c),
            }
        }
        _ => {}
    }
}

fn edit_next_field(state: &mut AppState) {
    match state.screen {
        Screen::AddServer => {
            state.server_form.active_field = (state.server_form.active_field + 1).min(4);
        }
        Screen::Deploy => {
            let max = state.deploy_form.field_count().saturating_sub(1);
            state.deploy_form.active_field = (state.deploy_form.active_field + 1).min(max);
        }
        Screen::AppCanvas => {
            let max = canvas_field_count(state).saturating_sub(1);
            if max > 0 || canvas_field_count(state) > 0 {
                state.deploy_form.active_field = (state.deploy_form.active_field + 1).min(max);
            }
        }
        Screen::NewAppWizard => {
            state.deploy_form.active_field = (state.deploy_form.active_field + 1).min(2);
        }
        Screen::Secrets => {
            state.secret_form.active_field = (state.secret_form.active_field + 1).min(1);
        }
        _ => {}
    }
}

fn edit_prev_field(state: &mut AppState) {
    match state.screen {
        Screen::AddServer => {
            state.server_form.active_field = state.server_form.active_field.saturating_sub(1);
        }
        Screen::Deploy | Screen::AppCanvas | Screen::NewAppWizard => {
            state.deploy_form.active_field = state.deploy_form.active_field.saturating_sub(1);
        }
        Screen::Secrets => {
            state.secret_form.active_field = state.secret_form.active_field.saturating_sub(1);
        }
        _ => {}
    }
}

fn open_new_app_wizard(state: &mut AppState) {
    state.nav_section = NavSection::Apps;
    if state.selected_server.is_none() {
        state.selected_server = state.servers.first().map(|s| s.id);
    }
    state.canvas_app_id = None;
    state.wizard_step = NewAppStep::Type;
    state.wizard_type_idx = 0;
    state.deploy_form = DeployForm::default();
    state.deploy_form.mode = DeployMode::Compose;
    state.deploy_form.active_field = 0;
    state.screen = Screen::NewAppWizard;
}

fn finish_new_app_wizard(state: &mut AppState, bus: &CommandBus) {
    if state.deploy_form.app_name.trim().is_empty() {
        push_error(state, state.i18n.t("error-wizard-name-required"));
        state.wizard_step = NewAppStep::Identity;
        state.deploy_form.active_field = 0;
        return;
    }
    let slug = state::slugify_app_name(&state.deploy_form.app_name);
    let slug = if slug.is_empty() {
        "myapp".to_string()
    } else {
        slug
    };
    if state.deploy_form.remote_dir == DeployForm::default().remote_dir
        || state
            .deploy_form
            .remote_dir
            .starts_with("/opt/doktui/apps/")
    {
        state.deploy_form.remote_dir = format!("/opt/doktui/apps/{slug}");
    }
    state.deploy_form.mode = if state.wizard_type_idx == 1 {
        DeployMode::GitHub
    } else {
        DeployMode::Compose
    };
    if state.deploy_form.mode == DeployMode::GitHub {
        state.deploy_form.apply_selected_repo();
    }
    state.canvas_app_id = None;
    state.canvas_tab = AppCanvasTab::General;
    state.deploy_form.active_field = 0;
    state.screen = Screen::AppCanvas;
    state.status_message = Some(match state.deploy_form.mode {
        DeployMode::Compose => state.i18n.t("status-wizard-compose-created"),
        DeployMode::GitHub => state.i18n.t("status-wizard-app-created"),
    });
    bus.load_secrets();
    if state.deploy_form.mode == DeployMode::GitHub {
        bus.dispatch(Message::LoadGitHubRepos(state.deploy_form.git_account_id));
    }
}

fn wizard_form_backspace(state: &mut AppState) {
    let field = &mut state.deploy_form;
    match field.active_field {
        0 => {
            field.app_name.pop();
            sync_wizard_remote_dir(field);
        }
        1 => {
            field.description.pop();
        }
        2 => {
            field.remote_dir.pop();
        }
        _ => {}
    }
}

fn wizard_form_char(state: &mut AppState, c: char) {
    let field = &mut state.deploy_form;
    match field.active_field {
        0 => {
            field.app_name.push(c);
            sync_wizard_remote_dir(field);
        }
        1 => field.description.push(c),
        2 => field.remote_dir.push(c),
        _ => {}
    }
}

fn sync_wizard_remote_dir(form: &mut DeployForm) {
    let slug = state::slugify_app_name(&form.app_name);
    let slug = if slug.is_empty() {
        "myapp".to_string()
    } else {
        slug
    };
    if form.remote_dir.starts_with("/opt/doktui/apps/") || form.remote_dir.is_empty() {
        form.remote_dir = format!("/opt/doktui/apps/{slug}");
    }
}

fn open_selected_app_canvas(state: &mut AppState, bus: &CommandBus) {
    state.nav_section = NavSection::Apps;
    if let Some(app) = state.apps.get(state.selected_app).cloned() {
        state.selected_server = Some(app.server_id);
        state.canvas_app_id = Some(app.id);
        state.deploy_form.load_from_app(&app);
        if let Some(id) = state.deploy_form.git_account_id {
            if let Some(idx) = state.git_accounts.iter().position(|a| a.id == id) {
                state.selected_git_account = idx;
            }
        }
    } else {
        state.canvas_app_id = None;
        state.deploy_form = DeployForm::default();
    }
    state.canvas_tab = AppCanvasTab::General;
    state.deploy_form.active_field = 0;
    state.screen = Screen::AppCanvas;
    bus.load_secrets();
    if state.deploy_form.mode == DeployMode::GitHub {
        bus.dispatch(Message::LoadGitHubRepos(state.deploy_form.git_account_id));
    }
}

fn canvas_on_tab_enter(state: &mut AppState, bus: &CommandBus) {
    match state.canvas_tab {
        AppCanvasTab::Env => bus.load_secrets(),
        AppCanvasTab::Logs => {
            let connected = state.selected_server.map(|id| {
                state.connection_state(id) == crate::services::ssh::ConnectionState::Connected
            });
            if connected == Some(true) {
                state.loading = true;
                if state.log_target.is_none() {
                    state.log_target = state.selected_container_name().or_else(|| {
                        state
                            .containers
                            .first()
                            .map(|c| c.name.clone())
                    });
                }
                if let Some(id) = state.selected_server {
                    bus.load_logs(id, state.log_target.clone());
                }
            }
        }
        _ => {}
    }
}

fn canvas_form_backspace(state: &mut AppState) {
    let field = &mut state.deploy_form;
    match state.canvas_tab {
        AppCanvasTab::General => match field.mode {
            DeployMode::Compose => match field.active_field {
                0 => {
                    field.app_name.pop();
                }
                1 => {
                    field.remote_dir.pop();
                }
                2 => {
                    field.compose.pop();
                }
                _ => {}
            },
            DeployMode::GitHub => match field.active_field {
                0 => {
                    field.app_name.pop();
                }
                1 => {
                    field.remote_dir.pop();
                }
                4 => {
                    field.gh_branch.pop();
                }
                5 => {
                    field.gh_compose_path.pop();
                }
                _ => {}
            },
        },
        AppCanvasTab::Domain => match field.active_field {
            0 => {
                field.domain.pop();
            }
            1 => {
                field.port.pop();
            }
            2 => {
                field.service.pop();
            }
            _ => {}
        },
        _ => {}
    }
}

fn canvas_form_char(state: &mut AppState, c: char) {
    match state.canvas_tab {
        AppCanvasTab::General => match state.deploy_form.mode {
            DeployMode::Compose => match state.deploy_form.active_field {
                0 => state.deploy_form.app_name.push(c),
                1 => state.deploy_form.remote_dir.push(c),
                2 => state.deploy_form.compose.push(c),
                _ => {}
            },
            DeployMode::GitHub => match state.deploy_form.active_field {
                0 => state.deploy_form.app_name.push(c),
                1 => state.deploy_form.remote_dir.push(c),
                // Account (2) is cycled via SelectGitAccount in map_canvas_key.
                3 if c == ']' && !state.deploy_form.github_repos.is_empty() => {
                    let n = state.deploy_form.github_repos.len();
                    state.deploy_form.selected_repo = (state.deploy_form.selected_repo + 1) % n;
                    state.deploy_form.apply_selected_repo();
                }
                3 if c == '[' && !state.deploy_form.github_repos.is_empty() => {
                    let n = state.deploy_form.github_repos.len();
                    state.deploy_form.selected_repo = state
                        .deploy_form
                        .selected_repo
                        .checked_sub(1)
                        .unwrap_or(n - 1);
                    state.deploy_form.apply_selected_repo();
                }
                4 if c == ']' && !state.deploy_form.github_branches.is_empty() => {
                    let n = state.deploy_form.github_branches.len();
                    state.deploy_form.selected_branch =
                        (state.deploy_form.selected_branch + 1) % n;
                    state.deploy_form.apply_selected_branch();
                }
                4 if c == '[' && !state.deploy_form.github_branches.is_empty() => {
                    let n = state.deploy_form.github_branches.len();
                    state.deploy_form.selected_branch = state
                        .deploy_form
                        .selected_branch
                        .checked_sub(1)
                        .unwrap_or(n - 1);
                    state.deploy_form.apply_selected_branch();
                }
                4 if c != '[' && c != ']' => state.deploy_form.gh_branch.push(c),
                5 => state.deploy_form.gh_compose_path.push(c),
                _ => {}
            },
        },
        AppCanvasTab::Domain => match state.deploy_form.active_field {
            0 => state.deploy_form.domain.push(c),
            1 if c.is_ascii_digit() => state.deploy_form.port.push(c),
            2 => state.deploy_form.service.push(c),
            _ => {}
        },
        _ => {}
    }
}

fn deploy_form_backspace(state: &mut AppState) {
    let field = &mut state.deploy_form;
    match field.mode {
        DeployMode::Compose => match field.active_field {
            0 => {
                field.remote_dir.pop();
            }
            1 => {
                field.domain.pop();
            }
            2 => {
                field.port.pop();
            }
            3 => {
                field.service.pop();
            }
            5 => {
                field.compose.pop();
            }
            _ => {}
        },
        DeployMode::GitHub => match field.active_field {
            0 => {
                field.remote_dir.pop();
            }
            1 => {
                if !field.gh_repo.is_empty() {
                    field.gh_repo.pop();
                } else {
                    field.gh_owner.pop();
                }
            }
            2 => {
                field.gh_branch.pop();
            }
            3 => {
                field.gh_compose_path.pop();
            }
            4 => {
                field.app_name.pop();
            }
            5 => {
                field.domain.pop();
            }
            6 => {
                field.port.pop();
            }
            7 => {
                field.service.pop();
            }
            _ => {}
        },
    }
}

fn deploy_form_char(state: &mut AppState, c: char) {
    let field = &mut state.deploy_form;
    match field.mode {
        DeployMode::Compose => match field.active_field {
            0 => field.remote_dir.push(c),
            1 => field.domain.push(c),
            2 if c.is_ascii_digit() => field.port.push(c),
            3 => field.service.push(c),
            5 => field.compose.push(c),
            _ => {}
        },
        DeployMode::GitHub => match field.active_field {
            0 => field.remote_dir.push(c),
            1 if c == ']' && !field.github_repos.is_empty() => {
                field.selected_repo = (field.selected_repo + 1) % field.github_repos.len();
                field.apply_selected_repo();
            }
            1 if c == '[' && !field.github_repos.is_empty() => {
                field.selected_repo = field
                    .selected_repo
                    .checked_sub(1)
                    .unwrap_or(field.github_repos.len() - 1);
                field.apply_selected_repo();
            }
            1 if c != '[' && c != ']' && field.github_repos.is_empty() => {
                if field.gh_repo.is_empty() {
                    field.gh_owner.push(c);
                    if let Some((o, r)) = field.gh_owner.clone().split_once('/') {
                        field.gh_owner = o.to_string();
                        field.gh_repo = r.to_string();
                    }
                } else {
                    field.gh_repo.push(c);
                }
            }
            2 if c == ']' && !field.github_branches.is_empty() => {
                field.selected_branch =
                    (field.selected_branch + 1) % field.github_branches.len();
                field.apply_selected_branch();
            }
            2 if c == '[' && !field.github_branches.is_empty() => {
                field.selected_branch = field
                    .selected_branch
                    .checked_sub(1)
                    .unwrap_or(field.github_branches.len() - 1);
                field.apply_selected_branch();
            }
            2 if c != '[' && c != ']' => field.gh_branch.push(c),
            3 => field.gh_compose_path.push(c),
            4 => field.app_name.push(c),
            5 => field.domain.push(c),
            6 if c.is_ascii_digit() => field.port.push(c),
            7 => field.service.push(c),
            _ => {}
        },
    }
}

pub async fn run_update() -> Result<()> {
    crate::services::updater::Updater::self_update(VERSION).await
}

fn cron_form_backspace(state: &mut AppState) {
    let Some(form) = &mut state.cron_form else {
        return;
    };
    match form.active_field {
        0 => {
            form.label.pop();
        }
        1 => {
            form.expression.pop();
        }
        3 => {
            form.target.pop();
        }
        _ => {}
    }
}

fn cron_form_char(state: &mut AppState, c: char) {
    let Some(form) = &mut state.cron_form else {
        return;
    };
    match form.active_field {
        0 => form.label.push(c),
        1 => form.expression.push(c),
        3 => form.target.push(c),
        _ => {}
    }
}

fn deploy_routing_from_form(form: &DeployForm) -> Option<DomainSpec> {
    if form.domain.trim().is_empty() {
        return None;
    }
    Some(DomainSpec {
        service: if form.service.trim().is_empty() {
            "app".into()
        } else {
            form.service.clone()
        },
        host: form.domain.trim().to_string(),
        port: form.port.parse().unwrap_or(80),
        path: None,
        https: form.https,
    })
}

fn push_error(state: &mut AppState, detail: String) {
    let first_line = detail.lines().next().unwrap_or(&detail).to_string();
    state.error_message = Some(first_line);
    state.error_detail = Some(detail);
    state.error_panel_open = false;
    state.error_scroll = 0;
}

#[cfg(test)]
mod map_key_tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::config::{EditorMode, ServerConfig, UiMode as ConfigUiMode};
    use crate::i18n::I18n;

    use super::*;
    use crate::app::state::{AppState, NavSection, Screen};

    fn test_state(screen: Screen) -> AppState {
        let i18n = I18n::load("en").unwrap();
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        let mut state = AppState::new(
            vec![ServerConfig::new(
                "srv-a".into(),
                "10.0.0.1".into(),
                22,
                "root".into(),
            )],
            true,
            String::new(),
            String::new(),
            EditorMode::Normal,
            ConfigUiMode::Overlay,
            vec![],
            vec![],
            vec![],
            theme,
            i18n,
            22,
        );
        state.screen = screen;
        state.sidebar_focused = false;
        state
    }

    #[test]
    fn server_list_digit_selects_server_not_sidebar() {
        let state = test_state(Screen::ServerList);
        let key = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty());
        let msg = map_key(key, &state);
        assert!(matches!(msg, Some(Message::SelectServer(_))));
    }

    #[test]
    fn home_digit_falls_back_to_sidebar_nav() {
        let state = test_state(Screen::Home);
        let key = KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty());
        let msg = map_key(key, &state);
        assert!(matches!(msg, Some(Message::GoNav(NavSection::Apps))));
    }
}
