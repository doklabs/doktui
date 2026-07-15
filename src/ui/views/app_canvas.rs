use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::event::Message;
use crate::app::state::{AppCanvasTab, AppState, DeployMode};
use crate::services::ssh::ConnectionState;
use crate::ui::components::{badge, Status};
use crate::ui::theme::{
    accent_style, connection_badge, header_line, muted_style, panel_block, shortcut_line, text_style,
    Role,
};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("canvas-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 {
        return;
    }

    let mut y = inner.y;
    render_header(frame, inner, &mut y, state);

    y += 1;
    render_tab_strip(frame, inner, &mut y, state);

    y += 1;
    let body_bottom = inner.y + inner.height.saturating_sub(2);
    let body_height = body_bottom.saturating_sub(y);
    let body = ratatui::layout::Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: body_height,
    };

    match state.canvas_tab {
        AppCanvasTab::General => render_general(frame, body, state),
        AppCanvasTab::Domain => render_domain(frame, body, state),
        AppCanvasTab::Env => render_env(frame, body, state),
        AppCanvasTab::Deploy => render_deploy_tab(frame, body, state),
        AppCanvasTab::Logs => render_logs_tab(frame, body, state),
    }

    let action_y = inner.y + inner.height.saturating_sub(1);
    let mut actions = vec![
        ("Ctrl+D", i18n.t("canvas-action-deploy")),
        ("Esc", i18n.t("canvas-action-back")),
    ];
    if state.canvas_app_id.is_some() {
        actions.insert(1, ("r", i18n.t("canvas-action-redeploy")));
    }
    let action_refs: Vec<(&str, &str)> = actions
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    frame.render_widget(
        Paragraph::new(shortcut_line(theme, &action_refs)),
        ratatui::layout::Rect {
            x: inner.x,
            y: action_y,
            width: inner.width,
            height: 1,
        },
    );
}

fn render_header(
    frame: &mut Frame,
    inner: ratatui::layout::Rect,
    y: &mut u16,
    state: &AppState,
) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;

    let name = if form.app_name.trim().is_empty() {
        i18n.t("canvas-draft-name")
    } else {
        form.app_name.clone()
    };
    let short_id = state
        .canvas_app_id
        .map(|id| {
            let s = id.to_string();
            s[..8.min(s.len())].to_string()
        })
        .unwrap_or_else(|| i18n.t("canvas-new-id"));

    let server_name = state
        .selected_server_config()
        .map(|s| s.name.as_str())
        .unwrap_or("?");
    let conn = state
        .selected_server
        .map(|id| state.connection_state(id))
        .unwrap_or(ConnectionState::Disconnected);
    let (conn_label, conn_style) = connection_badge(theme, i18n, conn);

    let mode = match form.mode {
        DeployMode::Compose => i18n.t("wizard-type-compose"),
        DeployMode::GitHub => i18n.t("wizard-type-application"),
    };

    let mut line = Line::from(vec![
        Span::styled(
            format!(" {mode} "),
            Style::default()
                .fg(theme.color(Role::Bg))
                .bg(theme.color(Role::Accent))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(name, theme.style_bold(Role::Text)),
        Span::styled(format!("  {short_id}  "), muted_style(theme)),
        Span::styled(
            format!("▣ {server_name}"),
            theme.style(Role::Primary).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(conn_label, conn_style),
    ]);

    if state.deploying {
        line.spans.push(Span::raw("  "));
        line.spans
            .extend(badge(theme, &i18n.t("canvas-deploying"), Status::Warning).spans);
    }

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

    let desc = if form.description.trim().is_empty() {
        form.remote_dir.clone()
    } else {
        format!("{}  ·  {}", form.description.trim(), form.remote_dir)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", theme.glyphs.info), muted_style(theme)),
            Span::styled(desc, muted_style(theme)),
        ])),
        ratatui::layout::Rect {
            x: inner.x,
            y: *y,
            width: inner.width,
            height: 1,
        },
    );
    *y += 1;
}

fn render_tab_strip(
    frame: &mut Frame,
    inner: ratatui::layout::Rect,
    y: &mut u16,
    state: &AppState,
) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let labels = [
        ("g", i18n.t("canvas-tab-general")),
        ("d", i18n.t("canvas-tab-domain")),
        ("e", i18n.t("canvas-tab-env")),
        ("p", i18n.t("canvas-tab-deploy")),
        ("l", i18n.t("canvas-tab-logs")),
    ];

    let mut spans = Vec::new();
    let mut cursor_x = inner.x;
    for (i, tab) in AppCanvasTab::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", muted_style(theme)));
            cursor_x += 3;
        }
        let (key, label) = &labels[i];
        let active = state.canvas_tab == *tab;
        let text = if active {
            format!(" ▸ {label} ")
        } else {
            format!(" [{key}]{label} ")
        };
        let w = text.chars().count() as u16;
        let style = if active {
            Style::default()
                .fg(theme.color(Role::Bg))
                .bg(theme.color(Role::Primary))
                .add_modifier(Modifier::BOLD)
        } else {
            muted_style(theme)
        };
        spans.push(Span::styled(text, style));
        if w > 0 {
            state.push_click(
                ratatui::layout::Rect {
                    x: cursor_x,
                    y: *y,
                    width: w,
                    height: 1,
                },
                Message::CanvasSetTab(*tab),
            );
        }
        cursor_x += w;
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        ratatui::layout::Rect {
            x: inner.x,
            y: *y,
            width: inner.width,
            height: 1,
        },
    );
    *y += 1;

    // Accent underline under the tab strip
    let rule: String = "─".repeat(inner.width as usize);
    frame.render_widget(
        Paragraph::new(rule).style(theme.style(Role::Border)),
        ratatui::layout::Rect {
            x: inner.x,
            y: *y,
            width: inner.width,
            height: 1,
        },
    );
    *y += 1;
}

fn render_general(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;
    let mut y = area.y;

    frame.render_widget(
        Paragraph::new(i18n.t("canvas-general-hint")).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );
    y += 2;

    match form.mode {
        DeployMode::Compose => {
            let compose_value = if form.active_field == 2 {
                i18n.t("deploy-compose-editing")
            } else {
                let count = form.compose.lines().count().to_string();
                i18n.t_fmt("deploy-compose-lines", &[("count", &count)])
            };
            let rows = [
                (i18n.t("deploy-field-app-name"), form.app_name.clone(), 0),
                (i18n.t("deploy-field-remote-dir"), form.remote_dir.clone(), 1),
                (i18n.t("deploy-field-compose"), compose_value, 2),
            ];
            for (label, value, idx) in rows {
                render_field_row(frame, area.x, &mut y, area.width, state, label, &value, idx);
            }
            if form.active_field == 2 {
                y += 1;
                let h = area.height.saturating_sub(y.saturating_sub(area.y) + 1);
                frame.render_widget(
                    Paragraph::new(form.compose.as_str())
                        .wrap(Wrap { trim: false })
                        .style(muted_style(theme)),
                    ratatui::layout::Rect {
                        x: area.x,
                        y,
                        width: area.width,
                        height: h,
                    },
                );
            }
        }
        DeployMode::GitHub => {
            let account_display = state
                .git_accounts
                .iter()
                .find(|a| Some(a.id) == form.git_account_id)
                .map(|a| format!("{} (@{})", a.label, a.login))
                .or_else(|| {
                    state
                        .git_accounts
                        .get(state.selected_git_account)
                        .map(|a| format!("{} (@{})", a.label, a.login))
                })
                .unwrap_or_else(|| {
                    if state.git_accounts.is_empty() {
                        i18n.t("deploy-gh-no-account")
                    } else {
                        i18n.t("deploy-gh-pick-account")
                    }
                });
            let repo_display = if form.github_repos.is_empty() {
                if form.gh_owner.is_empty() {
                    i18n.t("deploy-gh-no-repos")
                } else {
                    format!("{}/{}", form.gh_owner, form.gh_repo)
                }
            } else {
                form.github_repos
                    .get(form.selected_repo)
                    .map(|r| r.full_name.clone())
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
            let auto = if form.auto_deploy {
                i18n.t("deploy-on")
            } else {
                i18n.t("deploy-off")
            };
            let rows = [
                (i18n.t("deploy-field-app-name"), form.app_name.clone(), 0),
                (i18n.t("deploy-field-remote-dir"), form.remote_dir.clone(), 1),
                (i18n.t("deploy-field-account"), account_display, 2),
                (i18n.t("deploy-field-repo"), repo_display, 3),
                (i18n.t("deploy-field-branch"), branch_display, 4),
                (
                    i18n.t("deploy-field-compose-path"),
                    form.gh_compose_path.clone(),
                    5,
                ),
                (i18n.t("deploy-field-auto-deploy"), auto, 6),
            ];
            for (label, value, idx) in rows {
                render_field_row(frame, area.x, &mut y, area.width, state, label, &value, idx);
            }
            frame.render_widget(
                Paragraph::new(shortcut_line(
                    theme,
                    &[
                        ("Ctrl+M", &i18n.t("deploy-shortcut-mode")),
                        ("Ctrl+R", &i18n.t("deploy-shortcut-refresh")),
                        ("↑↓", &i18n.t("canvas-shortcut-pick")),
                        ("Tab", &i18n.t("deploy-shortcut-tab")),
                    ],
                )),
                ratatui::layout::Rect {
                    x: area.x,
                    y: area.y + area.height.saturating_sub(1),
                    width: area.width,
                    height: 1,
                },
            );
        }
    }
}

fn render_domain(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;
    let mut y = area.y;

    frame.render_widget(
        Paragraph::new(i18n.t("canvas-domain-hint")).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );
    y += 2;

    let https = if form.https {
        i18n.t("deploy-on")
    } else {
        i18n.t("deploy-off")
    };
    let rows = [
        (i18n.t("deploy-field-domain"), form.domain.clone(), 0),
        (i18n.t("deploy-field-port"), form.port.clone(), 1),
        (i18n.t("deploy-field-service"), form.service.clone(), 2),
        (i18n.t("deploy-field-https"), https, 3),
    ];
    for (label, value, idx) in rows {
        render_field_row(frame, area.x, &mut y, area.width, state, label, &value, idx);
    }
}

fn render_env(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let mut y = area.y;

    frame.render_widget(
        Paragraph::new(i18n.t("canvas-env-hint")).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: area.x,
            y,
            width: area.width,
            height: 2,
        },
    );
    y += 3;

    if state.secret_keys.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("canvas-env-empty")).style(muted_style(theme)),
            ratatui::layout::Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
        );
    } else {
        for key in &state.secret_keys {
            frame.render_widget(
                Paragraph::new(format!("• {key}")).style(theme.style(Role::Text)),
                ratatui::layout::Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
            );
            y += 1;
            if y >= area.y + area.height {
                break;
            }
        }
    }

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[("s", &i18n.t("canvas-shortcut-secrets"))],
        )),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        },
    );
}

fn render_deploy_tab(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;
    let mut y = area.y;

    let server = state
        .selected_server_config()
        .map(|s| format!("{} ({})", s.name, s.host))
        .unwrap_or_else(|| i18n.t("apps-target-none"));
    let source = match form.mode {
        DeployMode::Compose => i18n.t("apps-source-compose"),
        DeployMode::GitHub => {
            if form.gh_owner.is_empty() {
                i18n.t("deploy-mode-github")
            } else {
                format!("{}/{}@{}", form.gh_owner, form.gh_repo, form.gh_branch)
            }
        }
    };
    let domain = if form.domain.trim().is_empty() {
        i18n.t("canvas-no-domain")
    } else {
        let scheme = if form.https { "https" } else { "http" };
        format!("{scheme}://{}", form.domain)
    };
    let auto = if form.auto_deploy {
        i18n.t("deploy-on")
    } else {
        i18n.t("deploy-off")
    };

    let lines = vec![
        i18n.t_fmt("canvas-summary-server", &[("value", &server)]),
        i18n.t_fmt("canvas-summary-source", &[("value", &source)]),
        i18n.t_fmt("canvas-summary-dir", &[("value", &form.remote_dir)]),
        i18n.t_fmt("canvas-summary-domain", &[("value", &domain)]),
        i18n.t_fmt("canvas-summary-auto", &[("value", &auto)]),
    ];

    frame.render_widget(
        Paragraph::new(header_line(theme, &i18n.t("canvas-deploy-title"))),
        ratatui::layout::Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );
    y += 2;

    for line in lines {
        frame.render_widget(
            Paragraph::new(line).style(theme.style(Role::Text)),
            ratatui::layout::Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
        );
        y += 1;
    }

    y += 1;
    let status = if state.deploying {
        i18n.t("canvas-deploying")
    } else {
        i18n.t("canvas-ready")
    };
    frame.render_widget(
        Paragraph::new(status).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );

    frame.render_widget(
        Paragraph::new(i18n.t("canvas-deploy-hint")).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        },
    );
}

fn render_logs_tab(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;

    let connected = state
        .selected_server
        .map(|id| state.connection_state(id) == ConnectionState::Connected)
        .unwrap_or(false);

    let subtitle = if let Some(name) = &state.log_target {
        i18n.t_fmt("logs-target", &[("name", name)])
    } else {
        i18n.t("canvas-logs-title")
    };
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    let text = if !connected {
        Text::from(i18n.t("canvas-logs-offline"))
    } else if state.logs.is_empty() {
        Text::from(if state.loading {
            i18n.t("logs-fetching")
        } else {
            i18n.t("canvas-logs-empty")
        })
    } else {
        Text::from(
            state
                .logs
                .iter()
                .rev()
                .take(area.height.saturating_sub(2) as usize)
                .rev()
                .map(|line| Line::from(Span::raw(line.clone())))
                .collect::<Vec<_>>(),
        )
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(muted_style(theme))
            .wrap(Wrap { trim: false }),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y + 2,
            width: area.width,
            height: area.height.saturating_sub(2),
        },
    );
}

fn render_field_row(
    frame: &mut Frame,
    x: u16,
    y: &mut u16,
    width: u16,
    state: &AppState,
    label: String,
    value: &str,
    idx: usize,
) {
    let theme = &state.theme;
    let active = state.deploy_form.active_field == idx;
    let mark = if active { "▸" } else { " " };
    let label_style = if active {
        accent_style(theme).add_modifier(Modifier::BOLD)
    } else {
        muted_style(theme)
    };
    let value_style = if active {
        text_style(theme)
    } else {
        muted_style(theme)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{mark} {label}  "), label_style),
            Span::styled(value.to_string(), value_style),
            if active {
                Span::styled(" ▌", accent_style(theme))
            } else {
                Span::raw("")
            },
        ])),
        ratatui::layout::Rect {
            x,
            y: *y,
            width,
            height: 1,
        },
    );
    *y += 1;
}
