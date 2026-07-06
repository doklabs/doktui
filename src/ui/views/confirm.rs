use ratatui::Frame;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::{AppState, PendingAction};
use crate::ui::theme::{error_style, panel_block, title_style};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let action = match &state.pending_action {
        Some(PendingAction::RemoveContainer { name }) => {
            format!("Remove container '{name}'? This cannot be undone.")
        }
        None => "Confirm action?".into(),
    };

    let block = panel_block(" Confirm ", theme).style(title_style(theme));
    let p = Paragraph::new(vec![
        action.into(),
        "".into(),
        "Press [y] to confirm, [n] or Esc to cancel.".into(),
    ])
    .style(error_style(theme))
    .wrap(Wrap { trim: true })
    .block(block);
    frame.render_widget(p, area);
}
