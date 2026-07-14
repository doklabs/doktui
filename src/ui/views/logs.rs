use ratatui::Frame;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{error_style, header_line, muted_style, panel_block, shortcut_line, warning_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("logs-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = if let Some(name) = &state.log_target {
        i18n.t_fmt("logs-target", &[("name", name)])
    } else {
        i18n.t("logs-title")
    };
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
        Text::from(vec![Line::from(if state.loading {
            i18n.t("logs-fetching")
        } else {
            i18n.t("logs-empty")
        })])
    } else {
        Text::from(
            state
                .logs
                .iter()
                .map(|line| log_line(theme, line))
                .collect::<Vec<_>>(),
        )
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

fn log_line(theme: &crate::ui::theme::Theme, line: &str) -> Line<'static> {
    let lower = line.to_lowercase();
    let style = if lower.contains("error") {
        error_style(theme)
    } else if lower.contains("warn") {
        warning_style(theme)
    } else {
        return Line::from(Span::raw(line.to_string()));
    };
    Line::from(Span::styled(line.to_string(), style))
}
