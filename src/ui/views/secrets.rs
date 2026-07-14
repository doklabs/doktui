use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, header_line, muted_style, panel_block, shortcut_line};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("secrets-panel-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let subtitle = i18n.t("secrets-title");
    frame.render_widget(
        Paragraph::new(header_line(theme, &subtitle)),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let items: Vec<ListItem> = if state.secret_keys.is_empty() {
        vec![ListItem::new(i18n.t("secrets-empty"))]
    } else {
        state
            .secret_keys
            .iter()
            .map(|k| ListItem::new(format!("  {k} = ********")))
            .collect()
    };

    frame.render_widget(
        List::new(items),
        ratatui::layout::Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height.saturating_sub(8),
        },
    );

    let form = &state.secret_form;
    let key_style = if form.active_field == 0 {
        accent_style(theme)
    } else {
        muted_style(theme)
    };
    let val_style = if form.active_field == 1 {
        accent_style(theme)
    } else {
        muted_style(theme)
    };

    let form_y = inner.y + inner.height.saturating_sub(6);
    frame.render_widget(
        Paragraph::new(format!("{}   {}", i18n.t("secrets-key"), form.key)).style(key_style),
        ratatui::layout::Rect {
            x: inner.x,
            y: form_y,
            width: inner.width,
            height: 1,
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{} {}",
            i18n.t("secrets-value"),
            if form.value.is_empty() {
                "_"
            } else {
                "********"
            }
        ))
        .style(val_style),
        ratatui::layout::Rect {
            x: inner.x,
            y: form_y + 1,
            width: inner.width,
            height: 1,
        },
    );

    frame.render_widget(
        Paragraph::new(shortcut_line(
            theme,
            &[
                ("Tab", &i18n.t("secrets-shortcut-tab")),
                ("Enter", &i18n.t("secrets-shortcut-save")),
                ("Ctrl+x", &i18n.t("secrets-shortcut-delete")),
                ("Esc", &i18n.t("secrets-shortcut-back")),
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
