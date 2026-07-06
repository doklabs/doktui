use ratatui::Frame;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

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
                ListItem::new(format!(
                    "{prefix}{} — {} ({}) [{}]",
                    c.name,
                    c.status,
                    c.image,
                    short_container_id(&c.id)
                ))
                .style(style)
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
                ("S", &i18n.t("containers-shortcut-start")),
                ("s", &i18n.t("containers-shortcut-stop")),
                ("R", &i18n.t("containers-shortcut-restart")),
                ("r", &i18n.t("containers-shortcut-remove")),
                ("b", &i18n.t("containers-shortcut-back")),
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
