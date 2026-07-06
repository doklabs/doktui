use ratatui::Frame;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::theme::{header_line, panel_block, shortcut_line};

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

    let items = vec![
        ListItem::new(i18n.t("deploy-hub-item-deploy")),
        ListItem::new(i18n.t("deploy-hub-item-containers")),
        ListItem::new(i18n.t("deploy-hub-item-logs")),
        ListItem::new(i18n.t("deploy-hub-item-secrets")),
        ListItem::new(i18n.t("deploy-hub-item-editor")),
    ];

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
                ("d", &i18n.t("deploy-hub-shortcut-deploy")),
                ("c", &i18n.t("deploy-hub-shortcut-containers")),
                ("l", &i18n.t("deploy-hub-shortcut-logs")),
                ("v", &i18n.t("deploy-hub-shortcut-secrets")),
                ("e", &i18n.t("deploy-hub-shortcut-editor")),
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
