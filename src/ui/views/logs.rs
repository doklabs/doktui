use ratatui::Frame;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("logs-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = i18n.t("logs-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let text = if state.logs.is_empty() {
        if state.loading {
            i18n.t("logs-fetching")
        } else {
            i18n.t("logs-empty")
        }
    } else {
        state.logs.join("\n")
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(muted_style(theme))
            .wrap(Wrap { trim: false }),
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
            &[("b", &i18n.t("logs-shortcut-back"))],
        )),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        },
    );
}
