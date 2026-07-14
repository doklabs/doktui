use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::{AppState, PendingAction};
use crate::ui::components::card_with_role;
use crate::ui::theme::{error_style, Role};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let action = match &state.pending_action {
        Some(PendingAction::RemoveContainer { name }) => {
            i18n.t_fmt("confirm-remove-container", &[("name", name)])
        }
        Some(PendingAction::RemoveServer { id }) => {
            let name = state
                .servers
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| id.to_string());
            i18n.t_fmt("confirm-remove-server", &[("name", &name)])
        }
        None => i18n.t("confirm-generic"),
    };

    let panel_title = i18n.t("confirm-title");
    let block = card_with_role(&panel_title, theme, Role::Danger);
    let text = Text::from(vec![
        action.into(),
        "".into(),
        i18n.t("confirm-hint").into(),
    ]);
    let p = Paragraph::new(text)
        .style(error_style(theme))
        .wrap(Wrap { trim: true })
        .block(block);
    frame.render_widget(p, area);
}
