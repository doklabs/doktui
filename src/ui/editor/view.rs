use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, muted_style};

use super::highlight::highlight_line;
use super::VimMode;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let Some(editor) = &state.editor else {
        frame.render_widget(
            Paragraph::new(i18n.t("editor-no-session")),
            area,
        );
        return;
    };

    let mode_label = if editor.vim.enabled {
        match editor.vim.mode {
            VimMode::Normal => i18n.t("editor-mode-normal"),
            VimMode::Insert => i18n.t("editor-mode-insert"),
        }
    } else {
        i18n.t("editor-mode-edit")
    };

    let dirty = if editor.dirty { " •" } else { "" };
    let title = format!(" {} [{mode_label}]{dirty} ", editor.path);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_rows = inner.height as usize;
    editor_clamp_scroll(state, visible_rows);

    let source = editor.content();
    let cursor_line = editor.buffer.cursor_line_col().0;

    let lines: Vec<Line> = (editor.scroll_row..editor.scroll_row + visible_rows)
        .map(|i| highlight_line(editor.language, &source, i))
        .collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    let _cursor_line = cursor_line;

    if let Some(status) = &editor.status {
        let status_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(status.as_str()).style(muted_style(theme)),
            status_area,
        );
    }
}

fn editor_clamp_scroll(state: &AppState, visible_rows: usize) {
    // AppState is immutable in render — scroll clamping happens in handle_key via mutable ref.
    let _ = (state, visible_rows);
}

pub fn render_footer_hint(theme: &crate::ui::theme::Theme, i18n: &crate::i18n::I18n) -> Line<'static> {
    Line::from(vec![
        ratatui::text::Span::styled(i18n.t("editor-footer"), accent_style(theme)),
    ])
}
