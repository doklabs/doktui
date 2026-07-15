use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::config::DeploySource;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line, Role};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("apps-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(header_line(theme, &i18n.t("apps-title"))),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let target = state
        .selected_server_config()
        .map(|s| {
            i18n.t_fmt(
                "apps-target-server",
                &[("name", &s.name), ("host", &s.host)],
            )
        })
        .unwrap_or_else(|| i18n.t("apps-target-none"));
    frame.render_widget(
        Paragraph::new(target).style(muted_style(theme)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = if state.apps.is_empty() {
        vec![ListItem::new(i18n.t("apps-empty")).style(muted_style(theme))]
    } else {
        state
            .apps
            .iter()
            .enumerate()
            .map(|(i, app)| {
                let selected = i == state.selected_app;
                let prefix = if selected { "▸ " } else { "  " };
                let on_selected_server = state.selected_server == Some(app.server_id);
                let source = match &app.source {
                    DeploySource::ComposePaste => i18n.t("apps-source-compose"),
                    DeploySource::GitHub {
                        owner, repo, branch, ..
                    } => format!("{owner}/{repo}@{branch}"),
                };
                let auto = if app.auto_deploy {
                    i18n.t("apps-auto-on")
                } else {
                    i18n.t("apps-auto-off")
                };
                let sha = app
                    .last_commit_sha
                    .as_deref()
                    .map(|s| if s.len() > 7 { &s[..7] } else { s })
                    .unwrap_or("-");
                let server = state
                    .servers
                    .iter()
                    .find(|s| s.id == app.server_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("?");
                let marker = if on_selected_server { "●" } else { "○" };
                let line = format!(
                    "{prefix}{marker} {}  [{server}]  {source}  {auto}  {sha}",
                    app.name
                );
                let style = if selected {
                    accent_style(theme)
                } else if on_selected_server {
                    theme.style(Role::Text)
                } else {
                    muted_style(theme)
                };
                ListItem::new(Line::from(Span::styled(line, style)))
            })
            .collect()
    };

    frame.render_widget(
        List::new(items),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: inner.height.saturating_sub(5),
        },
    );

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("↑↓", &i18n.t("apps-shortcut-nav")),
                ("n", &i18n.t("apps-shortcut-new")),
                ("Enter", &i18n.t("apps-shortcut-open")),
                ("t", &i18n.t("apps-shortcut-tools")),
                ("x", &i18n.t("apps-shortcut-delete")),
                ("2", &i18n.t("apps-shortcut-servers")),
                ("q", &i18n.t("shortcut-quit")),
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
