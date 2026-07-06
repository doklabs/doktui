use ratatui::Frame;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::state::AppState;
use crate::ui::theme::{header_line, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Deployments ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(header_line(theme, "deploy & runtime")),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items = vec![
        ListItem::new("[d] Deploy — docker compose to server"),
        ListItem::new("[c] Containers — start/stop/restart"),
        ListItem::new("[l] Logs — stream container output"),
        ListItem::new("[v] Secrets — env vars (encrypted locally)"),
        ListItem::new("[e] Editor — edit compose file"),
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
        .map(|s| format!("Target: {}", s.name))
        .unwrap_or_else(|| "Target: (none — pick server in Projects)".into());

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
                ("d", "deploy"),
                ("c", "containers"),
                ("l", "logs"),
                ("v", "secrets"),
                ("e", "editor"),
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
