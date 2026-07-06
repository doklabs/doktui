use ratatui::Frame;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let block = panel_block(" Deploy ", theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let form = &state.deploy_form;
    let fields: [(&str, String, bool); 6] = [
        ("Remote dir", form.remote_dir.clone(), form.active_field == 0),
        ("Domain (or *.example.com)", form.domain.clone(), form.active_field == 1),
        ("Port", form.port.clone(), form.active_field == 2),
        ("Service", form.service.clone(), form.active_field == 3),
        (
            "HTTPS (Let's Encrypt)",
            if form.https { "on" } else { "off" }.into(),
            form.active_field == 4,
        ),
        (
            "Compose",
            if form.active_field == 5 {
                "(editing — press e for canvas editor)".into()
            } else {
                format!("{} lines", form.compose.lines().count())
            },
            form.active_field == 5,
        ),
    ];

    let mut y = inner.y;
    frame.render_widget(
        Paragraph::new(header_line(theme, "docker compose + traefik routing")),
        ratatui::layout::Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y += 2;

    for (label, value, active) in fields {
        let style = if active {
            accent_style(theme)
        } else {
            muted_style(theme)
        };
        frame.render_widget(
            Paragraph::new(format!("{label}: {value}")).style(style),
            ratatui::layout::Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    if form.active_field == 5 {
        y += 1;
        frame.render_widget(
            Paragraph::new(form.compose.as_str())
                .wrap(Wrap { trim: false })
                .style(muted_style(theme)),
            ratatui::layout::Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: inner.height.saturating_sub((y - inner.y) as u16 + 2),
            },
        );
    }

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("Tab", "field"),
                ("Space", "toggle HTTPS"),
                ("Enter", "deploy"),
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
