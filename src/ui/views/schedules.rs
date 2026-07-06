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
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("nav-schedules"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6), Constraint::Min(8)])
        .split(inner);

    let subtitle = i18n.t("schedules-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        chunks[0],
    );

    render_restart_policies(frame, chunks[1], state);
    render_cron_jobs(frame, chunks[2], state);

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("a", &i18n.t("schedules-shortcut-add")),
                ("t", &i18n.t("schedules-shortcut-toggle")),
                ("d", &i18n.t("schedules-shortcut-delete")),
                ("j/k", &i18n.t("schedules-shortcut-select")),
            ],
        )),
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
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("schedules-restart-policies"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.loading && state.schedules.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("schedules-loading")).style(muted_style(theme)),
            inner,
        );
        return;
    }

    if state.schedules.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("schedules-no-containers"))
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
            ListItem::new(i18n.t_fmt(
                "schedules-restart-line",
                &[
                    ("name", &s.name),
                    ("policy", &s.restart_policy),
                    ("status", &s.status),
                ],
            ))
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn render_cron_jobs(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("schedules-cron-panel"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.cron_jobs.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("schedules-no-jobs"))
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
                CronAction::RestartContainer { container } => {
                    i18n.t_fmt("schedules-action-restart", &[("target", container)])
                }
                CronAction::Redeploy { remote_dir } => {
                    i18n.t_fmt("schedules-action-redeploy", &[("target", remote_dir)])
                }
            };
            let status = if job.enabled {
                i18n.t("schedules-status-on")
            } else {
                i18n.t("schedules-status-off")
            };
            let never = i18n.t("schedules-last-never");
            let last = job.last_run.as_deref().unwrap_or(&never);
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
    let i18n = &state.i18n;
    let form = state.cron_form.as_ref().expect("cron form");
    let popup = centered_rect(70, 14, area);
    frame.render_widget(Clear, popup);

    let panel_title = format!(" {} ", i18n.t("schedules-cron-form-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let action_label = match form.action_kind {
        CronActionKind::Restart => i18n.t("schedules-form-restart"),
        CronActionKind::Redeploy => i18n.t("schedules-form-redeploy"),
    };
    let target_label = match form.action_kind {
        CronActionKind::Restart => i18n.t("schedules-form-container"),
        CronActionKind::Redeploy => i18n.t("schedules-form-remote-dir"),
    };

    let label_field = i18n.t("schedules-form-label");
    let cron_field = i18n.t("schedules-form-cron");
    let action_field = i18n.t("schedules-form-action");
    let fields: [(String, String, bool); 4] = [
        (label_field, form.label.clone(), form.active_field == 0),
        (cron_field, form.expression.clone(), form.active_field == 1),
        (action_field, action_label, form.active_field == 2),
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
