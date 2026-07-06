use ratatui::Frame;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("deploy-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let form = &state.deploy_form;
    let on_off = if form.https {
        i18n.t("deploy-on")
    } else {
        i18n.t("deploy-off")
    };
    let compose_value = if form.active_field == 5 {
        i18n.t("deploy-compose-editing")
    } else {
        let count = form.compose.lines().count().to_string();
        i18n.t_fmt("deploy-compose-lines", &[("count", &count)])
    };
    let fields: [(String, String, bool); 6] = [
        (
            i18n.t("deploy-field-remote-dir"),
            form.remote_dir.clone(),
            form.active_field == 0,
        ),
        (
            i18n.t("deploy-field-domain"),
            form.domain.clone(),
            form.active_field == 1,
        ),
        (
            i18n.t("deploy-field-port"),
            form.port.clone(),
            form.active_field == 2,
        ),
        (
            i18n.t("deploy-field-service"),
            form.service.clone(),
            form.active_field == 3,
        ),
        (
            i18n.t("deploy-field-https"),
            on_off,
            form.active_field == 4,
        ),
        (
            i18n.t("deploy-field-compose"),
            compose_value,
            form.active_field == 5,
        ),
    ];

    let mut y = inner.y;
    let subtitle = i18n.t("deploy-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
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
                ("Tab", &i18n.t("deploy-shortcut-tab")),
                ("Space", &i18n.t("deploy-shortcut-https")),
                ("Enter", &i18n.t("deploy-shortcut-deploy")),
                ("e", &i18n.t("deploy-shortcut-editor")),
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
