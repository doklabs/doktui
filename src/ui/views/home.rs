use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::anim;
use crate::ui::components::{health_bar, metric_bar, stat};
use crate::ui::theme::{Role, muted_style, panel_block, success_style, text_style, warning_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("home-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let server_count = state.servers.len();
    let app_count = state.containers.len();
    let running = state
        .containers
        .iter()
        .filter(|c| c.status.to_lowercase().contains("up"))
        .count();
    let deploy_note = if state.deploying {
        i18n.t("home-deploy-running-1")
    } else {
        i18n.t("home-deploy-running-0")
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(if state.achievement.is_some() { 4 } else { 0 }),
        ])
        .split(inner);

    let summary = i18n.t_fmt(
        "home-summary",
        &[
            ("servers", &server_count.to_string()),
            ("apps", &app_count.to_string()),
            ("deploy", &deploy_note),
        ],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                i18n.t("home-title"),
                theme.style_bold(Role::Primary).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(summary, muted_style(theme))),
        ]),
        chunks[0],
    );

    render_stat_row(frame, chunks[1], state, running, app_count);

    if state.deploying {
        render_deploy_panel(frame, chunks[2], state);
    } else {
        render_overview(frame, chunks[2], state);
    }

    if let Some(achievement) = &state.achievement {
        render_achievement(frame, chunks[3], state, achievement);
    }
}

fn render_stat_row(frame: &mut Frame, area: Rect, state: &AppState, running: usize, total: usize) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    let pulse = anim::pulse(theme, state.metrics_tick as u64);
    let apps_pct = if total > 0 {
        (running * 100 / total) as u8
    } else {
        0
    };
    let apps_label = format!("{running:02}/{total:02}");
    let mut apps_line = metric_bar(theme, 12, apps_pct, true);
    apps_line.spans.push(Span::styled(
        format!(" {apps_label}"),
        theme.style(Role::Success).add_modifier(if pulse > 0.5 {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }),
    ));
    stat(frame, cols[0], &i18n.t("home-stat-apps"), apps_line, Role::Success, theme);

    let cpu = state
        .metrics
        .first()
        .and_then(|m| m.cpu_percent.trim_end_matches('%').parse::<u8>().ok())
        .unwrap_or(0);
    let mut cpu_line = health_bar(cpu, 12, theme);
    cpu_line.spans.push(Span::styled(
        format!(" {cpu}%"),
        theme.style(Role::Warning),
    ));
    stat(frame, cols[1], &i18n.t("home-stat-cpu"), cpu_line, Role::Warning, theme);

    let (connected, server_total) = state.connected_server_count();
    let healthy_pct = if server_total > 0 {
        (connected * 100 / server_total) as u8
    } else {
        0
    };
    let healthy_label = format!("{connected:02}/{server_total:02}");
    let mut healthy_line = metric_bar(theme, 12, healthy_pct, true);
    healthy_line.spans.push(Span::styled(
        format!(" {healthy_label}"),
        theme.style(Role::Accent).add_modifier(if pulse > 0.5 {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }),
    ));
    stat(frame, cols[2], &i18n.t("home-stat-healthy"), healthy_line, Role::Accent, theme);
}

fn render_deploy_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let domain = if state.deploy_form.domain.trim().is_empty() {
        i18n.t("home-deploy-placeholder")
    } else {
        state.deploy_form.domain.clone()
    };
    let spin = anim::spinner(theme, state.anim_tick);
    let bar = anim::progress_bar(theme, state.anim_tick, 28, 72);

    let title = i18n.t_fmt(
        "home-deploying-title",
        &[
            ("arrow", &theme.glyphs.arrow),
            ("domain", &domain),
        ],
    );
    let block = panel_block(&title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(bar, theme.style(Role::Primary)),
            Span::styled(
                format!("  {}", i18n.t_fmt("home-deploy-building", &[("spin", &spin.to_string())])),
                muted_style(theme),
            ),
        ]),
        Line::from(Span::styled(
            i18n.t_fmt(
                "home-deploy-network-attached",
                &[("check", &theme.glyphs.check)],
            ),
            success_style(theme),
        )),
        Line::from(Span::styled(
            i18n.t_fmt(
                "home-deploy-traefik-route",
                &[("check", &theme.glyphs.check), ("domain", &domain)],
            ),
            success_style(theme),
        )),
        Line::from(vec![
            Span::styled(
                i18n.t_fmt(
                    "home-deploy-pulling",
                    &[("arrow", &theme.glyphs.arrow)],
                ),
                warning_style(theme),
            ),
            Span::styled(spin.to_string(), theme.style(Role::Accent)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_overview(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let body = if let Some(srv) = state.selected_server_config() {
        let dot = theme.glyphs.dot_on.clone();
        i18n.t_fmt(
            "home-active-server",
            &[
                ("name", &srv.name),
                ("host", &srv.host),
                ("port", &srv.port.to_string()),
                ("dot", &dot),
            ],
        )
    } else if state.servers.is_empty() {
        i18n.t("home-no-servers")
    } else {
        i18n.t_fmt(
            "home-servers-registered",
            &[("count", &state.servers.len().to_string())],
        )
    };

    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: true })
            .style(muted_style(theme)),
        area,
    );
}

fn render_achievement(frame: &mut Frame, area: Rect, state: &AppState, text: &str) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(theme.style(Role::Warning))
        .style(Style::default().bg(theme.color(Role::Surface)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} {}",
                        theme.glyphs.star,
                        i18n.t("home-achievement-label")
                    ),
                    theme.style_bold(Role::Warning),
                ),
                Span::raw(" — "),
                Span::styled(text.to_string(), text_style(theme)),
            ]),
            Line::from(Span::styled(
                i18n.t("home-achievement-https"),
                muted_style(theme),
            )),
        ]),
        inner,
    );
}
