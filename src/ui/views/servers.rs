use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::services::ssh::ConnectionState;
use crate::ui::components::{Status, badge};
use crate::ui::layout;
use crate::ui::theme::{header_line, muted_style, panel_block, shortcut_line, Role};

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
            let state = state.connection_state(s.id);
            let (status, status_label) = connection_status(&state, i18n);
            let status_badge = badge(theme, &status_label, status);
            let prefix = if selected { "▸ " } else { "  " };
            let mut line = Line::from(vec![
                Span::styled(prefix.to_string(), theme.style(Role::Text)),
                Span::styled(format!("[{}] ", i + 1), muted_style(theme)),
                Span::styled(format!("{} — ", s.name), theme.style(Role::Text)),
                Span::styled(format!("{}@{}:{} ", s.user, s.host, s.port), muted_style(theme)),
            ]);
            line.spans.extend(status_badge.spans);
            ListItem::new(line)
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
                ("j/k", &i18n.t("servers-shortcut-select")),
                ("Enter", &i18n.t("servers-shortcut-open")),
                ("a", &i18n.t("servers-shortcut-add")),
                ("c", &i18n.t("servers-shortcut-connect")),
                ("p", &i18n.t("servers-shortcut-provision")),
                ("x", &i18n.t("servers-shortcut-remove")),
                ("b", &i18n.t("servers-shortcut-back")),
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

fn connection_status(state: &ConnectionState, i18n: &crate::i18n::I18n) -> (Status, String) {
    match state {
        ConnectionState::Connected => (
            Status::Success,
            i18n.t_fmt("conn-online", &[("dot", "")]).trim().to_string(),
        ),
        ConnectionState::Connecting => (
            Status::Warning,
            i18n.t_fmt("conn-connecting", &[("dot", "")]).trim().to_string(),
        ),
        ConnectionState::Reconnecting => (Status::Warning, i18n.t("conn-reconnecting")),
        ConnectionState::Disconnected => (
            Status::Danger,
            i18n.t_fmt("conn-offline", &[("dot", "")]).trim().to_string(),
        ),
    }
}
