use ratatui::Frame;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::{AppState, PendingAction};
use crate::ui::theme::{error_style, panel_block, title_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let action = match &state.pending_action {
        Some(PendingAction::RemoveContainer { name }) => {
            i18n.t_fmt("confirm-remove-container", &[("name", name)])
        }
        None => i18n.t("confirm-generic"),
    };

    let panel_title = format!(" {} ", i18n.t("confirm-title"));
    let block = panel_block(&panel_title, theme).style(title_style(theme));
    let p = Paragraph::new(vec![
        action.into(),
        "".into(),
        i18n.t("confirm-hint").into(),
    ])
    .style(error_style(theme))
    .wrap(Wrap { trim: true })
    .block(block);
    frame.render_widget(p, area);
}
