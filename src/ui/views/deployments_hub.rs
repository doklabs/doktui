use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

const DEPLOY_MENU_ITEMS: [&str; 6] = [
    "deploy-hub-item-deploy",
    "deploy-hub-item-apps",
    "deploy-hub-item-containers",
    "deploy-hub-item-logs",
    "deploy-hub-item-secrets",
    "deploy-hub-item-editor",
];

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("nav-deployments"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = i18n.t("deploy-hub-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = DEPLOY_MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let label = i18n.t(key);
            let selected = i == state.selected_deploy_menu;
            let prefix = if selected { "▸ " } else { "  " };
            let style = if selected {
                accent_style(theme)
            } else {
                muted_style(theme)
            };
            ListItem::new(Line::from(vec![Span::styled(
                format!("{prefix}{label}"),
                style,
            )]))
        })
        .collect();

    frame.render_widget(
        List::new(items),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height.saturating_sub(4),
        },
    );

    let server = state
        .selected_server_config()
        .map(|s| i18n.t_fmt("deploy-hub-target", &[("name", &s.name)]))
        .unwrap_or_else(|| i18n.t("deploy-hub-no-target"));

    frame.render_widget(
        Paragraph::new(server),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(2),
            width: inner.width,
            height: 1,
        },
    );

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("j/k", &i18n.t("deploy-hub-shortcut-nav")),
                ("Enter", &i18n.t("deploy-hub-shortcut-open")),
                ("b", &i18n.t("shortcut-back")),
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
