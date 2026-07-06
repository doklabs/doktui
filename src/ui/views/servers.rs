use ratatui::Frame;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::layout;
use crate::ui::theme::{connection_badge, header_line, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("nav-projects"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = i18n.t("servers-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = state
        .servers
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            layout::filter_match(&s.name, &state.search_query)
                || layout::filter_match(&s.host, &state.search_query)
                || layout::filter_match(&s.user, &state.search_query)
        })
        .map(|(i, s)| {
            let selected = state.selected_server == Some(s.id);
            let (badge, _) = connection_badge(theme, i18n, state.connection_state(s.id));
            let prefix = if selected { "▸ " } else { "  " };
            ListItem::new(format!(
                "{prefix}[{}] {} — {}@{}:{} {badge}",
                i + 1,
                s.name,
                s.user,
                s.host,
                s.port
            ))
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

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("1-9", &i18n.t("servers-shortcut-select")),
                ("c", &i18n.t("servers-shortcut-connect")),
                ("p", &i18n.t("servers-shortcut-provision")),
                ("a", &i18n.t("servers-shortcut-add")),
                ("b", &i18n.t("servers-shortcut-back")),
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
