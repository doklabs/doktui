use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::services::ssh::ConnectionState;
use crate::ui::components::{health_bar, sparkline};
use crate::ui::theme::{header_line, muted_style, panel_block, shortcut_line, text_style, Role};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("nav-monitoring"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(inner);

    let subtitle = i18n.t("monitoring-title");
    frame.render_widget(Paragraph::new(header_line(theme, &subtitle)), chunks[0]);

    let server = state
        .selected_server_config()
        .map(|s| format!("{} ({})", s.name, s.host))
        .unwrap_or_else(|| i18n.t("monitoring-no-server"));

    let connected = state
        .selected_server
        .map(|id| state.connection_state(id) == ConnectionState::Connected)
        .unwrap_or(false);

    let body = if state.loading && state.metrics.is_empty() {
        Text::from(vec![Line::from(i18n.t_fmt(
            "monitoring-loading",
            &[("server", &server)],
        ))])
    } else if !connected {
        Text::from(vec![Line::from(i18n.t_fmt(
            "monitoring-not-connected",
            &[("server", &server)],
        ))])
    } else if state.metrics.is_empty() {
        Text::from(vec![Line::from(i18n.t_fmt(
            "monitoring-no-containers",
            &[("server", &server)],
        ))])
    } else {
        let col_name = i18n.t("monitoring-col-name");
        let col_cpu = i18n.t("monitoring-col-cpu");
        let col_mem = i18n.t("monitoring-col-mem");
        let col_mem_pct = i18n.t("monitoring-col-mem-pct");
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(Span::styled(
            format!("Server: {server}"),
            theme.style_bold(Role::Text),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(format!("{col_name:<20} "), muted_style(theme)),
            Span::styled(format!("{col_cpu:>8} "), muted_style(theme)),
            Span::styled(format!("{col_mem:>16} "), muted_style(theme)),
            Span::styled(format!("{col_mem_pct:>8}"), muted_style(theme)),
        ]));
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
            let cpu_role = role_for_percent(cpu_pct);
            let mem_role = role_for_percent(mem_pct);
            let cpu_bar = health_bar(cpu_pct, 8, theme);
            let mem_bar = health_bar(mem_pct, 8, theme);
            let history = state.metrics_history.get(&m.name).map(|v| v.as_slice()).unwrap_or(&[]);
            let cpu_spark = sparkline(history, 8, theme);

            lines.push(Line::from(vec![Span::styled(
                format!("{:<20}", m.name),
                theme.style(Role::Text),
            )]));

            let mut cpu_line = Line::from(vec![Span::styled("  ", muted_style(theme))]);
            cpu_line.spans.extend(cpu_bar.spans);
            cpu_line.spans.push(Span::raw(" "));
            cpu_line.spans.extend(cpu_spark.spans);
            cpu_line
                .spans
                .push(Span::styled(format!(" {cpu_pct}%"), theme.style(cpu_role)));
            lines.push(cpu_line);

            let mut mem_line = Line::from(vec![Span::styled("  ", muted_style(theme))]);
            mem_line.spans.extend(mem_bar.spans);
            mem_line.spans.push(Span::raw(" "));
            mem_line.spans.push(Span::styled(
                m.mem_usage.clone(),
                theme.style(Role::TextMuted),
            ));
            mem_line
                .spans
                .push(Span::styled(format!(" {}", m.mem_percent), theme.style(mem_role)));
            lines.push(mem_line);
        }
        Text::from(lines)
    };

    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .style(text_style(theme)),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("b", &i18n.t("shortcut-back")),
                ("q", &i18n.t("shortcut-quit")),
            ],
        )),
        chunks[2],
    );
}

fn role_for_percent(pct: u8) -> Role {
    match pct {
        0..=59 => Role::Success,
        60..=84 => Role::Warning,
        _ => Role::Danger,
    }
}


