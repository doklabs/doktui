use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::services::ssh::ConnectionState;
use crate::ui::components::{health_bar, sparkline};
use crate::ui::theme::{header_line, muted_style, panel_block};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Monitoring ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(4)])
        .split(inner);

    frame.render_widget(Paragraph::new(header_line(theme, "resource usage")), chunks[0]);

    let server = state
        .selected_server_config()
        .map(|s| format!("{} ({})", s.name, s.host))
        .unwrap_or_else(|| "no server selected".into());

    let connected = state
        .selected_server
        .map(|id| state.connection_state(id) == ConnectionState::Connected)
        .unwrap_or(false);

    let body = if state.loading && state.metrics.is_empty() {
        format!("Server: {server}\n\nLoading metrics…")
    } else if !connected {
        format!(
            "Server: {server}\n\nNot connected — press [c] under Projects to connect."
        )
    } else if state.metrics.is_empty() {
        format!(
            "Server: {server}\n\nNo running containers to measure.\nDeploy an app or start containers first."
        )
    } else {
        let mut lines = vec![format!("Server: {server}"), String::new()];
        lines.push(format!(
            "{:<20} {:>8} {:>16} {:>8}",
            "NAME", "CPU", "MEM", "MEM%"
        ));
        for m in &state.metrics {
            let cpu_pct = m
                .cpu_percent
                .trim_end_matches('%')
                .parse::<u8>()
                .unwrap_or(0);
            let mem_pct = m
                .mem_percent
                .trim_end_matches('%')
                .parse::<u8>()
                .unwrap_or(0);
            let cpu_bar = health_bar(cpu_pct, 8, theme);
            let mem_bar = health_bar(mem_pct, 8, theme);
            let cpu_spark = sparkline(
                &synthetic_history(cpu_pct, state.anim_tick),
                8,
                theme,
            );
            lines.push(format!("{:<20}", m.name));
            lines.push(format!(
                "  CPU {} {}",
                line_to_string(&cpu_bar),
                line_to_string(&cpu_spark)
            ));
            lines.push(format!(
                "  MEM {} {}  {}",
                line_to_string(&mem_bar),
                m.mem_usage,
                m.mem_percent
            ));
        }
        lines.join("\n")
    };

    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: true })
            .style(muted_style(theme)),
        chunks[1],
    );
}

fn synthetic_history(current: u8, tick: u64) -> Vec<u8> {
    (0..8)
        .map(|i| {
            let wave = ((tick + i as u64) % 8) as u8 * 8;
            current.saturating_sub(10).saturating_add(wave.min(20))
        })
        .collect()
}

fn line_to_string(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<String>()
}
