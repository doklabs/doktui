use ratatui::Frame;
use ratatui::widgets::{Block, Gauge, Paragraph, Wrap};

use crate::app::state::AppState;
use crate::services::provision::ProvisionStep;
use crate::ui::theme::{header_line, muted_style, panel_block, success_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("provision-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let progress = state.provision_progress.as_ref();
    let starting = i18n.t("provision-starting");
    let (msg, pct) = progress
        .map(|p| {
            (
                provision_step_label(i18n, &p.step, &p.message),
                p.percent,
            )
        })
        .unwrap_or((starting, 0));

    let subtitle = i18n.t("provision-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
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
        let os_info = i18n.t_fmt("provision-os", &[("os", &res.os_info)]);
        frame.render_widget(
            Paragraph::new(os_info)
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

fn provision_step_label(
    i18n: &crate::i18n::I18n,
    step: &ProvisionStep,
    fallback: &str,
) -> String {
    let key = match step {
        ProvisionStep::DetectOs => "provision-detect-os",
        ProvisionStep::CheckDocker => "provision-check-docker",
        ProvisionStep::InstallDocker => "provision-install-docker",
        ProvisionStep::CheckTraefik => "provision-check-traefik",
        ProvisionStep::MigrateTraefik => "provision-migrate-traefik",
        ProvisionStep::InstallTraefik => "provision-install-traefik",
        ProvisionStep::Verify => "provision-verify",
        ProvisionStep::Done => "provision-ready",
    };
    let translated = i18n.t(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
