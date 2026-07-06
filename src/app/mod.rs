use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind, poll, read,
};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{ExecutableCommand, execute};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{Mutex, mpsc};

use crate::config::{AcmeChallenge, AppConfig, CronAction, CronJob, ServerConfig, bootstrap};
use crate::security::{hostkey, keys};
use crate::services::secrets::SecretsManager;
use crate::services::routing::{self, DomainSpec};
use crate::ui::{self, layout};

use self::command::{CommandBus, save_new_server};
use self::event::Message;
use self::state::{AppState, CronActionKind, CronForm, DeployForm, HostKeyAfterAction, NavSection, Screen, ServerForm, UiMode, hit};

pub mod command;
pub mod event;
pub mod state;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const FRAME_RATE: Duration = Duration::from_millis(66);
const HOUSEKEEPING: Duration = Duration::from_millis(1000);

pub async fn run_tui() -> Result<()> {
    let config = bootstrap()?;
    let public_key = keys::load_public_key_openssh()?;
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
    let bus = CommandBus::new(
        tx.clone(),
        config.clone(),
        secrets,
        auto_reconnect,
        ssh_tx,
    );
    bus.spawn_update_check(VERSION, check_updates);

    let mut state = {
        let cfg = config.lock().await;
        let theme = ui::theme::ThemeRegistry::active(&cfg.theme);
        AppState::new(
            cfg.servers.clone(),
            cfg.onboarding_complete,
            public_key.trim().to_string(),
            cfg.editor_mode.clone(),
            cfg.ui_mode.clone(),
            cfg.cron_jobs.clone(),
            theme,
        )
    };

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

        let timeout = FRAME_RATE
            .saturating_sub(last_frame.elapsed());

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
                    Event::Resize(w, h) => {
                        update(&mut state, Message::Resize(w, h), &config, &bus).await;
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
            | Message::SubmitDeploy { .. }
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

    if state.screen.uses_app_shell() {
        if key.code == KeyCode::Tab {
            return Some(Message::ToggleSidebarFocus);
        }
        if key.code == KeyCode::Char('m') {
            return Some(Message::ToggleUiMode);
        }

        if state.sidebar_focused {
            return match key.code {
                KeyCode::Down | KeyCode::Char('j') => Some(Message::SidebarNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Message::SidebarPrev),
                code if is_enter(code) => Some(Message::GoNav(state.nav_section)),
                KeyCode::Right | KeyCode::Char('l') => Some(Message::ToggleSidebarFocus),
                KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
                KeyCode::Char(c) => layout::section_from_char(c).map(Message::GoNav),
                _ => None,
            };
        }

        if let KeyCode::Char(c) = key.code {
            if let Some(section) = layout::section_from_char(c) {
                return Some(Message::GoNav(section));
            }
        }
        if matches!(key.code, KeyCode::Left | KeyCode::Char('h')) {
            return Some(Message::ToggleSidebarFocus);
        }
    }

    match state.screen {
        Screen::Welcome => match key.code {
            code if is_enter(code) => Some(Message::GoAddServer),
            KeyCode::Char('c') => Some(Message::CopyPublicKey),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::AddServer => match key.code {
            KeyCode::Esc => Some(Message::GoServerList),
            KeyCode::Tab => Some(Message::FormNextField),
            KeyCode::BackTab => Some(Message::FormPrevField),
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
            KeyCode::Char('d') => Some(Message::GoDeploy),
            KeyCode::Char('c') => Some(Message::GoContainers),
            KeyCode::Char('l') => Some(Message::GoLogs),
            KeyCode::Char('v') => Some(Message::GoSecrets),
            KeyCode::Char('e') => Some(Message::GoEditor),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Monitoring => match key.code {
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::Schedules => match key.code {
            KeyCode::Char('a') => Some(Message::OpenCronForm),
            KeyCode::Char('d') => state
                .cron_jobs
                .get(state.selected_cron)
                .map(|j| Message::DeleteCronJob(j.id)),
            KeyCode::Char('t') => state
                .cron_jobs
                .get(state.selected_cron)
                .map(|j| Message::ToggleCronJob(j.id)),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::CronNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::CronPrev),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => Some(Message::Quit),
            _ => None,
        },
        Screen::ServerList => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoHome),
            KeyCode::Char('a') => Some(Message::GoAddServer),
            KeyCode::Char('c') if state.selected_server.is_some() => {
                state.selected_server.map(Message::ConnectServer)
            }
            KeyCode::Char('p') if state.selected_server.is_some() => {
                state.selected_server.map(Message::ProvisionServer)
            }
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
            _ => None,
        },
        Screen::Containers => {
            let name = state.selected_container_name();
            let server_id = state.selected_server;
            match key.code {
                KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Deployments)),
                KeyCode::Down | KeyCode::Char('j') => Some(Message::ContainerNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Message::ContainerPrev),
                KeyCode::Char('r') if name.is_some() && server_id.is_some() => {
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
                KeyCode::Char('R') if name.is_some() && server_id.is_some() => {
                    Some(Message::RestartContainer {
                        server_id: server_id.unwrap(),
                        name: name.unwrap(),
                    })
                }
                _ => None,
            }
        }
        Screen::Logs => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Deployments)),
            _ => None,
        },
        Screen::Deploy => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Deployments)),
            KeyCode::Char('e') if state.deploy_form.active_field == 5 => Some(Message::GoEditor),
            KeyCode::Tab => Some(Message::FormNextField),
            KeyCode::BackTab => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            KeyCode::Char(' ') if state.deploy_form.active_field == 4 => {
                Some(Message::ToggleDeployHttps)
            }
            KeyCode::Enter if state.deploy_form.active_field < 5 => Some(Message::FormNextField),
            KeyCode::Enter => {
                if let Some(id) = state
                    .selected_server
                    .or_else(|| state.servers.first().map(|s| s.id))
                {
                    let routing = deploy_routing_from_form(&state.deploy_form);
                    if let Some(ref spec) = routing {
                        if let Err(e) = routing::validate_domain_spec(spec) {
                            return Some(Message::SetError(e.to_string()));
                        }
                    }
                    Some(Message::SubmitDeploy {
                        server_id: id,
                        remote_dir: state.deploy_form.remote_dir.clone(),
                        compose: state.deploy_form.compose.clone(),
                        routing,
                    })
                } else {
                    Some(Message::SetError("add a server first".into()))
                }
            }
            KeyCode::Char(c) if state.deploy_form.active_field != 4 => Some(Message::FormChar(c)),
            _ => None,
        },
        Screen::Secrets => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(Message::GoNav(NavSection::Deployments)),
            KeyCode::Tab => Some(Message::FormNextField),
            KeyCode::BackTab => Some(Message::FormPrevField),
            KeyCode::Backspace => Some(Message::FormBackspace),
            code if is_enter(code) => Some(Message::SubmitSecretForm),
            KeyCode::Char('d') if !state.secret_keys.is_empty() => {
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
    }
}

fn handle_mouse(state: &mut AppState, me: MouseEvent) -> Option<Message> {
    match me.kind {
        MouseEventKind::Moved => {
            state.mouse_pos = Some((me.column, me.row));
            None
        }
        MouseEventKind::Down(MouseButton::Left) => state
            .click_regions
            .borrow()
            .iter()
            .find(|c| hit(c.rect, me.column, me.row))
            .map(|c| c.msg.clone()),
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
            state.should_quit = true;
        }
        Message::Tick => {
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
        }
        Message::GoWelcome => state.screen = Screen::Welcome,
        Message::GoHome => {
            state.go_nav(NavSection::Home);
            state.error_message = None;
        }
        Message::CopyPublicKey => {
            match arboard::Clipboard::new().and_then(|mut clip| clip.set_text(state.public_key.clone()))
            {
                Ok(()) => state.status_message = Some("SSH public key copied".into()),
                Err(e) => push_error(state, format!("clipboard copy failed: {e}")),
            }
        }
        Message::GoNav(section) => {
            state.go_nav(section);
            state.error_message = None;
            state.error_detail = None;
            state.error_panel_open = false;
            if section == NavSection::Projects && state.selected_server.is_none() {
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
                    push_error(state, "select a server under Projects first".into());
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
        Message::GoServerList => {
            state.go_nav(NavSection::Projects);
            if state.selected_server.is_none() {
                state.selected_server = state.servers.first().map(|s| s.id);
            }
        }
        Message::GoAddServer => {
            state.screen = Screen::AddServer;
            let acme = config.lock().await.acme_email.clone();
            state.server_form = ServerForm {
                acme_email: acme,
                ..Default::default()
            };
        }
        Message::GoContainers => {
            state.nav_section = NavSection::Deployments;
            state.screen = Screen::Containers;
            state.selected_container = 0;
            state.loading = true;
            if let Some(id) = state.selected_server {
                bus.load_containers(id);
            } else {
                state.loading = false;
                push_error(state, "select a server under Projects first".into());
            }
        }
        Message::GoLogs => {
            state.nav_section = NavSection::Deployments;
            state.screen = Screen::Logs;
            state.loading = true;
            if let Some(id) = state.selected_server {
                let container = state.selected_container_name();
                bus.load_logs(id, container);
            } else {
                state.loading = false;
                push_error(state, "select a server under Projects first".into());
            }
        }
        Message::GoSecrets => {
            state.nav_section = NavSection::Deployments;
            state.screen = Screen::Secrets;
            state.secret_form = Default::default();
            bus.load_secrets();
        }
        Message::GoDeploy => {
            state.nav_section = NavSection::Deployments;
            state.screen = Screen::Deploy;
            if state.selected_server.is_none() {
                state.selected_server = state.servers.first().map(|s| s.id);
            }
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
            if let Some(editor) = &mut state.editor {
                use crate::ui::editor::EditorAction;
                match editor.handle_key(key) {
                    EditorAction::Quit => {
                        if let Some(ed) = state.editor.take() {
                            state.deploy_form.compose = ed.content();
                        }
                        state.screen = Screen::Deploy;
                    }
                    EditorAction::Saved => {
                        state.status_message = editor.status.clone();
                    }
                    EditorAction::None => {}
                }
            }
        }
        Message::EditorSaved => {
            state.status_message = Some("editor saved".into());
        }
        Message::SubmitServerForm => {
            let form = &state.server_form;
            if form.name.is_empty() || form.host.is_empty() {
                push_error(state, "name and host are required".into());
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
                    push_error(state, "failed to save server".into());
                }
            }
        }
        Message::SelectServer(id) => {
            state.selected_server = Some(id);
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
                    state.status_message = Some("server provisioned successfully".into());
                }
                Err(e) => {
                    push_error(state, e);
                    state.screen = Screen::ServerList;
                }
            }
        }
        Message::SshStatus(status) => {
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
                Ok(stats) => state.metrics = stats,
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
                push_error(state, "schedule label is required".into());
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
                push_error(state, "select a server under Projects first".into());
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
            state.status_message = Some("cron job saved".into());
        }
        Message::DeleteCronJob(id) => {
            let mut cfg = config.lock().await;
            cfg.cron_jobs.retain(|j| j.id != id);
            let _ = cfg.save();
            state.cron_jobs = cfg.cron_jobs.clone();
            state.selected_cron = state
                .selected_cron
                .min(state.cron_jobs.len().saturating_sub(1));
            state.status_message = Some("cron job deleted".into());
        }
        Message::ToggleCronJob(id) => {
            let mut cfg = config.lock().await;
            if let Some(job) = cfg.cron_jobs.iter_mut().find(|j| j.id == id) {
                job.enabled = !job.enabled;
            }
            let _ = cfg.save();
            state.cron_jobs = cfg.cron_jobs.clone();
        }
        Message::CronJobDone { id: _, result } => {
            match result {
                Ok(msg) => state.status_message = Some(format!("schedule: {msg}")),
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
                push_error(state, "secret key is required".into());
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
                state.selected_container = (state.selected_container + 1).min(state.containers.len() - 1);
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
        } => {
            if let Some(ref spec) = routing {
                if spec.is_wildcard() {
                    let challenge = config.lock().await.acme_challenge;
                    if challenge != AcmeChallenge::DnsCloudflare {
                        push_error(
                            state,
                            "wildcard domains require DNS-01: set acme_challenge = \"dns_cloudflare\" in config and add CF_DNS_API_TOKEN in Secrets".into(),
                        );
                        return;
                    }
                }
            }
            state.loading = true;
            bus.deploy(server_id, remote_dir, compose, routing);
        }
        Message::ToggleDeployHttps => {
            state.deploy_form.https = !state.deploy_form.https;
        }
        Message::DeployDone(result) => {
            state.loading = false;
            match result {
                Ok(report) => {
                    if report.all_ok() {
                        state.status_message = Some(report.summary());
                        state.error_message = None;
                        state.error_detail = None;
                        if state.deploy_form.https && !state.deploy_form.domain.trim().is_empty() {
                            state.achievement = Some("First HTTPS Deploy".into());
                        }
                    } else {
                        state.status_message = Some("deploy completed with warnings".into());
                        push_error(state, report.summary());
                    }
                    state.go_nav(NavSection::Deployments);
                }
                Err(e) => push_error(state, e),
            }
        }
        Message::RequestRemoveContainer(name) => {
            state.pending_action = Some(state::PendingAction::RemoveContainer { name });
            state.screen = Screen::ConfirmDestructive;
        }
        Message::ConfirmDestructive => {
            if let Some(state::PendingAction::RemoveContainer { name }) =
                state.pending_action.take()
            {
                state.loading = true;
                if let Some(id) = state.selected_server {
                    bus.dispatch(Message::RemoveContainer { server_id: id, name });
                }
            }
            state.screen = Screen::Containers;
        }
        Message::CancelDestructive => {
            state.pending_action = None;
            state.screen = Screen::Containers;
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
                        if let Err(e) = known.trust_fingerprint(
                            &prompt.host,
                            prompt.port,
                            &prompt.fingerprint,
                        ) {
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
            push_error(state, "host key rejected — connection aborted".into());
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
            state.status_message = Some("connecting…".into());
        }
        Message::StartContainer { .. }
        | Message::StopContainer { .. }
        | Message::RestartContainer { .. }
        | Message::RemoveContainer { .. } => {
            state.loading = true;
        }
        Message::Key(_) | Message::Resize(_, _) => {}
    }
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
        Screen::Deploy => {
            let field = &mut state.deploy_form;
            match field.active_field {
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
            }
        }
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
        Screen::Deploy => {
            let field = &mut state.deploy_form;
            match field.active_field {
                0 => field.remote_dir.push(c),
                1 => field.domain.push(c),
                2 if c.is_ascii_digit() => field.port.push(c),
                3 => field.service.push(c),
                5 => field.compose.push(c),
                _ => {}
            }
        }
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
            state.deploy_form.active_field = (state.deploy_form.active_field + 1).min(5);
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
        Screen::Deploy => {
            state.deploy_form.active_field = state.deploy_form.active_field.saturating_sub(1);
        }
        Screen::Secrets => {
            state.secret_form.active_field = state.secret_form.active_field.saturating_sub(1);
        }
        _ => {}
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
