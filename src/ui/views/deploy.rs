use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::{AppState, DeployMode};
use crate::ui::components::{badge, Status};
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line, Role};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("deploy-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let form = &state.deploy_form;
    let mode_label = match form.mode {
        DeployMode::Compose => i18n.t("deploy-mode-compose"),
        DeployMode::GitHub => i18n.t("deploy-mode-github"),
    };

    let mut y = inner.y;
    let subtitle = i18n.t_fmt("deploy-title-mode", &[("mode", &mode_label)]);
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y += 1;
    frame.render_widget(
        Paragraph::new(i18n.t("deploy-mode-hint")).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y += 2;

    match form.mode {
        DeployMode::Compose => render_compose_fields(frame, inner, &mut y, state),
        DeployMode::GitHub => render_github_fields(frame, inner, &mut y, state),
    }

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("m", &i18n.t("deploy-shortcut-mode")),
                ("Tab", &i18n.t("deploy-shortcut-tab")),
                ("Space", &i18n.t("deploy-shortcut-https")),
                ("Enter", &i18n.t("deploy-shortcut-deploy")),
                ("r", &i18n.t("deploy-shortcut-refresh")),
                ("Esc", &i18n.t("shortcut-back")),
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

fn render_compose_fields(
    frame: &mut Frame,
    inner: ratatui::layout::Rect,
    y: &mut u16,
    state: &AppState,
) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;
    let on_off = if form.https {
        i18n.t("deploy-on")
    } else {
        i18n.t("deploy-off")
    };
    let compose_value = if form.active_field == 5 {
        i18n.t("deploy-compose-editing")
    } else {
        let count = form.compose.lines().count().to_string();
        i18n.t_fmt("deploy-compose-lines", &[("count", &count)])
    };
    let https_status = if form.https {
        Status::Success
    } else {
        Status::Muted
    };
    let https_badge = badge(theme, &on_off, https_status);
    let mut https_line = Line::from(vec![Span::styled(
        format!("{}: ", i18n.t("deploy-field-https")),
        theme.style(Role::Text),
    )]);
    https_line.spans.extend(https_badge.spans);

    let fields: [(String, Line<'static>, bool); 6] = [
        (
            i18n.t("deploy-field-remote-dir"),
            Line::from(Span::raw(form.remote_dir.clone())),
            form.active_field == 0,
        ),
        (
            i18n.t("deploy-field-domain"),
            Line::from(Span::raw(form.domain.clone())),
            form.active_field == 1,
        ),
        (
            i18n.t("deploy-field-port"),
            Line::from(Span::raw(form.port.clone())),
            form.active_field == 2,
        ),
        (
            i18n.t("deploy-field-service"),
            Line::from(Span::raw(form.service.clone())),
            form.active_field == 3,
        ),
        (
            i18n.t("deploy-field-https"),
            https_line,
            form.active_field == 4,
        ),
        (
            i18n.t("deploy-field-compose"),
            Line::from(Span::raw(compose_value)),
            form.active_field == 5,
        ),
    ];

    for (label, value, active) in fields {
        let style = if active {
            accent_style(theme)
        } else {
            muted_style(theme)
        };
        let mut line = Line::from(vec![Span::styled(format!("{label}: "), style)]);
        line.spans.extend(value.spans);
        frame.render_widget(
            Paragraph::new(line),
            ratatui::layout::Rect {
                x: inner.x,
                y: *y,
                width: inner.width,
                height: 1,
            },
        );
        *y += 1;
    }

    if form.active_field == 5 {
        *y += 1;
        frame.render_widget(
            Paragraph::new(form.compose.as_str())
                .wrap(Wrap { trim: false })
                .style(muted_style(theme)),
            ratatui::layout::Rect {
                x: inner.x,
                y: *y,
                width: inner.width,
                height: inner.height.saturating_sub((*y - inner.y) as u16 + 2),
            },
        );
    }
}

fn render_github_fields(
    frame: &mut Frame,
    inner: ratatui::layout::Rect,
    y: &mut u16,
    state: &AppState,
) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;

    let repo_display = if form.github_repos.is_empty() {
        if form.gh_owner.is_empty() {
            i18n.t("deploy-gh-no-repos")
        } else {
            format!("{}/{}", form.gh_owner, form.gh_repo)
        }
    } else {
        form.github_repos
            .get(form.selected_repo)
            .map(|r| {
                let priv_tag = if r.private { " private" } else { "" };
                format!("{}{priv_tag}", r.full_name)
            })
            .unwrap_or_else(|| format!("{}/{}", form.gh_owner, form.gh_repo))
    };

    let branch_display = if form.github_branches.is_empty() {
        form.gh_branch.clone()
    } else {
        form.github_branches
            .get(form.selected_branch)
            .cloned()
            .unwrap_or_else(|| form.gh_branch.clone())
    };

    let on_off = |v: bool| {
        if v {
            i18n.t("deploy-on")
        } else {
            i18n.t("deploy-off")
        }
    };

    let rows: Vec<(String, String, bool)> = vec![
        (
            i18n.t("deploy-field-remote-dir"),
            form.remote_dir.clone(),
            form.active_field == 0,
        ),
        (
            i18n.t("deploy-field-repo"),
            repo_display,
            form.active_field == 1,
        ),
        (
            i18n.t("deploy-field-branch"),
            branch_display,
            form.active_field == 2,
        ),
        (
            i18n.t("deploy-field-compose-path"),
            form.gh_compose_path.clone(),
            form.active_field == 3,
        ),
        (
            i18n.t("deploy-field-app-name"),
            form.app_name.clone(),
            form.active_field == 4,
        ),
        (
            i18n.t("deploy-field-domain"),
            form.domain.clone(),
            form.active_field == 5,
        ),
        (
            i18n.t("deploy-field-port"),
            form.port.clone(),
            form.active_field == 6,
        ),
        (
            i18n.t("deploy-field-service"),
            form.service.clone(),
            form.active_field == 7,
        ),
        (
            i18n.t("deploy-field-https"),
            on_off(form.https),
            form.active_field == 8,
        ),
        (
            i18n.t("deploy-field-auto-deploy"),
            on_off(form.auto_deploy),
            form.active_field == 9,
        ),
    ];

    for (label, value, active) in rows {
        let style = if active {
            accent_style(theme)
        } else {
            muted_style(theme)
        };
        frame.render_widget(
            Paragraph::new(format!("{label}: {value}")).style(style),
            ratatui::layout::Rect {
                x: inner.x,
                y: *y,
                width: inner.width,
                height: 1,
            },
        );
        *y += 1;
    }
}
