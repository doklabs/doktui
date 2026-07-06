use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::anim;
use crate::ui::components::health_bar;
use crate::ui::theme::{Role, muted_style, panel_block, success_style, text_style, warning_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" HOME ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let server_count = state.servers.len();
    let app_count = state.containers.len();
    let running = state
        .containers
        .iter()
        .filter(|c| c.status.to_lowercase().contains("up"))
        .count();
    let deploy_note = if state.loading { "1 deploy running" } else { "0 deploy running" };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(if state.achievement.is_some() { 4 } else { 0 }),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "HOME",
                theme.style_bold(Role::Primary).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(
                    "{server_count} servers · {app_count} apps · {deploy_note}"
                ),
                muted_style(theme),
            )),
        ]),
        chunks[0],
    );

    render_stat_row(frame, chunks[1], state, running, app_count);

    if state.loading {
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
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    let apps_label = format!("{running:02}/{total:02}");
    stat_card(
        frame,
        cols[0],
        theme,
        "Apps Online",
        Line::from(apps_label),
        Role::Success,
    );

    let cpu = state
        .metrics
        .first()
        .and_then(|m| m.cpu_percent.trim_end_matches('%').parse::<u8>().ok())
        .unwrap_or(0);
    let cpu_bar = health_bar(cpu, 12, theme);
    let mut cpu_spans = cpu_bar.spans;
    cpu_spans.push(Span::styled(format!(" {cpu}%"), theme.style(Role::Warning)));
    stat_card(
        frame,
        cols[1],
        theme,
        "CPU",
        Line::from(cpu_spans),
        Role::Warning,
    );

    stat_card(
        frame,
        cols[2],
        theme,
        "Uptime",
        Line::from("99.9%"),
        Role::Accent,
    );
}

fn stat_card(
    frame: &mut Frame,
    area: Rect,
    theme: &crate::ui::theme::Theme,
    title: &str,
    value: Line<'_>,
    value_role: Role,
) {
    let block = panel_block(title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(value).style(theme.style_bold(value_role)),
        inner,
    );
}

fn render_deploy_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let domain = if state.deploy_form.domain.trim().is_empty() {
        "app.example.com".to_string()
    } else {
        state.deploy_form.domain.clone()
    };
    let spin = anim::spinner(theme, state.anim_tick);
    let bar = anim::progress_bar(theme, state.anim_tick, 28, 72);

    let title = format!("{} DEPLOYING · {domain}", theme.glyphs.arrow);
    let block = panel_block(&title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(bar, theme.style(Role::Primary)),
            Span::styled(format!("  building {spin}"), muted_style(theme)),
        ]),
        Line::from(Span::styled(
            format!("{} doktui-network attached", theme.glyphs.check),
            success_style(theme),
        )),
        Line::from(Span::styled(
            format!(
                "{} traefik route → Host(`{domain}`) tls:le",
                theme.glyphs.check
            ),
            success_style(theme),
        )),
        Line::from(vec![
            Span::styled(
                format!("{} pulling image… ", theme.glyphs.arrow),
                warning_style(theme),
            ),
            Span::styled(spin.to_string(), theme.style(Role::Accent)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_overview(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let body = if let Some(srv) = state.selected_server_config() {
        let dot = theme.glyphs.dot_on.clone();
        format!(
            "Active server: {} @ {}:{}\nStatus: {dot} connected\n\nUse Projects to manage SSH servers.\nUse Deployments to deploy apps.",
            srv.name, srv.host, srv.port
        )
    } else if state.servers.is_empty() {
        "No servers yet.\n\nGo to Projects → press [a] to register an SSH server.".into()
    } else {
        format!(
            "{} server(s) registered.\nSelect one under Projects.",
            state.servers.len()
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
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(theme.style(Role::Warning))
        .style(Style::default().bg(theme.color(Role::Surface)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("{} ACHIEVEMENT", theme.glyphs.star), theme.style_bold(Role::Warning)),
                Span::raw(" — "),
                Span::styled(text.to_string(), text_style(theme)),
            ]),
            Line::from(Span::styled(
                "cert Let's Encrypt terbit · +50 XP",
                muted_style(theme),
            )),
        ]),
        inner,
    );
}
