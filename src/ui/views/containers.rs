use ratatui::Frame;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Containers ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(header_line(theme, "docker ps")),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = if state.containers.is_empty() {
        vec![ListItem::new(if state.loading {
            "loading…"
        } else {
            "no containers — connect to a server under Projects first"
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
                    "{prefix}{} — {} ({})",
                    c.name, c.status, c.image
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
                ("j/k", "select"),
                ("S", "start"),
                ("s", "stop"),
                ("R", "restart"),
                ("r", "remove"),
                ("b", "back"),
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
