use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::components::{badge, container_status};
use crate::ui::theme::{Role, accent_style, header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("containers-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = i18n.t("containers-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = if state.containers.is_empty() {
        vec![ListItem::new(if state.loading {
            i18n.t("containers-loading")
        } else {
            i18n.t("containers-empty")
        })]
    } else {
        state
            .containers
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let selected = i == state.selected_container;
                let prefix = if selected { "▸ " } else { "  " };
                let style = if selected {
                    accent_style(theme)
                } else {
                    muted_style(theme)
                };
                let status = container_status(&c.status);
                let short_status = c.status.split_whitespace().next().unwrap_or(&c.status);
                let badge = badge(theme, short_status, status);
                let mut line = Line::from(vec![
                    Span::styled(prefix.to_string(), style),
                    Span::styled(format!("{} — ", c.name), theme.style(Role::Text)),
                    Span::styled(format!("{} ", c.image), muted_style(theme)),
                    Span::styled(format!("[{}] ", short_container_id(&c.id)), muted_style(theme)),
                ]);
                line.spans.extend(badge.spans);
                ListItem::new(line).style(style)
            })
            .collect()
    };

    frame.render_widget(
        List::new(items),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height.saturating_sub(3),
        },
    );

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("j/k", &i18n.t("containers-shortcut-select")),
                ("s", &i18n.t("containers-shortcut-stop")),
                ("S", &i18n.t("containers-shortcut-start")),
                ("r", &i18n.t("containers-shortcut-restart")),
                ("x", &i18n.t("containers-shortcut-remove")),
                ("l", &i18n.t("containers-shortcut-logs")),
                ("b", &i18n.t("containers-shortcut-back")),
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

fn short_container_id(id: &str) -> &str {
    if id.len() > 12 {
        &id[..12]
    } else {
        id
    }
}
