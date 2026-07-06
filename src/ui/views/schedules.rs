use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Clear, List, ListItem, Paragraph, Wrap};

use crate::app::state::{AppState, CronActionKind};
use crate::config::CronAction;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line, text_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    if state.cron_form.is_some() {
        render_cron_form(frame, area, state);
        return;
    }

    let theme = &state.theme;
    let block = panel_block(" Schedules ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6), Constraint::Min(8)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(header_line(theme, "restart policies & cron jobs")),
        chunks[0],
    );

    render_restart_policies(frame, chunks[1], state);
    render_cron_jobs(frame, chunks[2], state);

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
            ("a", "add cron"),
            ("t", "toggle"),
            ("d", "delete"),
            ("j/k", "select"),
        ])),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        },
    );
}

fn render_restart_policies(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Docker restart policies ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.loading && state.schedules.is_empty() {
        frame.render_widget(
            Paragraph::new("Loading…").style(muted_style(theme)),
            inner,
        );
        return;
    }

    if state.schedules.is_empty() {
        frame.render_widget(
            Paragraph::new("No containers — connect to a server under Projects.")
                .style(muted_style(theme))
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .schedules
        .iter()
        .map(|s| {
            ListItem::new(format!(
                "{} — restart: {} ({})",
                s.name, s.restart_policy, s.status
            ))
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn render_cron_jobs(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Cron jobs (runs while DokTUI is open) ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.cron_jobs.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No cron jobs yet.\nPress a to schedule container restarts or compose redeploys.\nExample: 0 0 3 * * * = daily at 03:00 UTC.",
            )
            .style(muted_style(theme))
            .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .cron_jobs
        .iter()
        .enumerate()
        .map(|(idx, job)| {
            let action = match &job.action {
                CronAction::RestartContainer { container } => format!("restart {container}"),
                CronAction::Redeploy { remote_dir } => format!("redeploy {remote_dir}"),
            };
            let status = if job.enabled { "on" } else { "off" };
            let last = job.last_run.as_deref().unwrap_or("never");
            let line = format!(
                "{} {} — {} — {} [{}] last: {}",
                if idx == state.selected_cron {
                    "▸"
                } else {
                    " "
                },
                job.label,
                job.expression,
                action,
                status,
                last,
            );
            let style = if idx == state.selected_cron {
                accent_style(theme)
            } else {
                text_style(theme)
            };
            ListItem::new(line).style(style)
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn render_cron_form(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let form = state.cron_form.as_ref().expect("cron form");
    let popup = centered_rect(70, 14, area);
    frame.render_widget(Clear, popup);

    let block = panel_block(" New cron job (Esc cancel, Enter save) ", theme);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let action_label = match form.action_kind {
        CronActionKind::Restart => "restart container",
        CronActionKind::Redeploy => "redeploy compose dir",
    };
    let target_label = match form.action_kind {
        CronActionKind::Restart => "Container name",
        CronActionKind::Redeploy => "Remote dir",
    };

    let fields: [(&str, String, bool); 4] = [
        ("Label", form.label.clone(), form.active_field == 0),
        ("Cron", form.expression.clone(), form.active_field == 1),
        (
            "Action (Space toggles)",
            action_label.into(),
            form.active_field == 2,
        ),
        (target_label, form.target.clone(), form.active_field == 3),
    ];

    let mut y = inner.y;
    for (label, value, active) in fields {
        let style = if active { accent_style(theme) } else { muted_style(theme) };
        frame.render_widget(
            Paragraph::new(format!("{label}: {value}")).style(style),
            ratatui::layout::Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }
}

fn centered_rect(percent_x: u16, height: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect {
        x,
        y,
        width: popup_width.max(30),
        height,
    }
}
