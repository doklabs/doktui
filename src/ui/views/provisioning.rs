use ratatui::Frame;
use ratatui::widgets::{Block, Gauge, Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{header_line, muted_style, panel_block, success_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Provisioning Server ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let progress = state.provision_progress.as_ref();
    let (msg, pct) = progress
        .map(|p| (p.message.as_str(), p.percent))
        .unwrap_or(("Starting…", 0));

    frame.render_widget(
        Paragraph::new(header_line(theme, "installing Docker + Traefik")),
        inner,
    );

    let gauge = Gauge::default()
        .block(Block::default().title(msg))
        .gauge_style(success_style(theme))
        .percent(pct as u16);
    frame.render_widget(
        gauge,
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 3,
        },
    );

    if let Some(res) = &state.provision_result {
        frame.render_widget(
            Paragraph::new(format!("OS: {}", res.os_info))
                .wrap(Wrap { trim: true })
                .style(muted_style(theme)),
            ratatui::layout::Rect {
                x: inner.x,
                y: inner.y + 6,
                width: inner.width,
                height: inner.height.saturating_sub(6),
            },
        );
    }
}
