use std::path::Path;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::AppState;
use crate::ui::theme::{accent_style, muted_style, Role, Theme};

use super::highlight::{highlight_line, EditorLanguage};
use super::VimMode;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let Some(editor) = &state.editor else {
        frame.render_widget(Paragraph::new(i18n.t("editor-no-session")), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_tabline(frame, chunks[0], editor.path.as_str(), editor.dirty, theme);

    let line_count = editor.line_count();
    let gutter_width = gutter_width(line_count);
    let visible_rows = chunks[1].height as usize;
    state.editor_visible_rows.set(visible_rows.max(1));

    let source = editor.content();
    let (cursor_line, cursor_col) = editor.buffer.cursor_line_col();

    let mut cursor_screen: Option<(u16, u16)> = None;
    for row in 0..visible_rows {
        let line_idx = editor.scroll_row + row;
        let y = chunks[1].y + row as u16;
        let line_area = Rect {
            x: chunks[1].x,
            y,
            width: chunks[1].width,
            height: 1,
        };

        if line_idx < line_count {
            let gutter = format!("{:>gutter_width$} ", line_idx + 1);
            let mut spans = vec![Span::styled(gutter, muted_style(theme))];
            let highlighted = highlight_line(editor.language, &source, line_idx, theme);
            spans.extend(highlighted.spans);
            frame.render_widget(Paragraph::new(Line::from(spans)), line_area);

            if line_idx == cursor_line {
                let x = chunks[1].x + gutter_width as u16 + cursor_col as u16;
                if x < line_area.x + line_area.width {
                    cursor_screen = Some((x, y));
                }
            }
        } else {
            let gutter = format!("{:>gutter_width$} ", " ");
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(gutter, muted_style(theme)),
                    Span::styled("~", muted_style(theme)),
                ])),
                line_area,
            );
        }
    }

    if let Some((x, y)) = cursor_screen {
        frame.set_cursor_position((x, y));
    }

    render_statusline(
        frame,
        chunks[2],
        editor,
        theme,
        i18n,
        cursor_line,
        cursor_col,
    );
}

fn gutter_width(line_count: usize) -> usize {
    let digits = line_count.max(1).ilog10() as usize + 1;
    digits.max(4)
}

fn render_tabline(frame: &mut Frame, area: Rect, path: &str, dirty: bool, theme: &Theme) {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    let dirty_mark = if dirty { " +" } else { "" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{name}{dirty_mark}"), accent_style(theme)),
        ]))
        .style(theme.style_bg(Role::Surface)),
        area,
    );
}

fn render_statusline(
    frame: &mut Frame,
    area: Rect,
    editor: &super::CanvasEditor,
    theme: &Theme,
    i18n: &crate::i18n::I18n,
    cursor_line: usize,
    cursor_col: usize,
) {
    let mode = if editor.vim.enabled {
        match editor.vim.mode {
            VimMode::Normal => i18n.t("editor-mode-normal"),
            VimMode::Insert => i18n.t("editor-mode-insert"),
        }
    } else {
        i18n.t("editor-mode-edit")
    };

    let filetype = language_label(editor.language);
    let pos = i18n.t_fmt(
        "editor-status-position",
        &[
            ("line", &(cursor_line + 1).to_string()),
            ("col", &(cursor_col + 1).to_string()),
        ],
    );
    let dirty = if editor.dirty {
        i18n.t("editor-status-modified")
    } else {
        String::new()
    };

    let mut segments = vec![
        Path::new(&editor.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&editor.path)
            .to_string(),
        mode,
        filetype.to_string(),
        pos,
    ];
    if !dirty.is_empty() {
        segments.push(dirty);
    }

    let status = if let Some(msg) = &editor.status {
        msg.clone()
    } else {
        segments.join(" │ ")
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {status} "),
            theme
                .style(Role::TextMuted)
                .add_modifier(Modifier::REVERSED),
        )))
        .style(theme.style_bg(Role::Surface)),
        area,
    );
}

fn language_label(language: EditorLanguage) -> &'static str {
    match language {
        EditorLanguage::Yaml => "yaml",
        EditorLanguage::Toml => "toml",
        EditorLanguage::Env => "env",
        EditorLanguage::Dockerfile => "dockerfile",
        EditorLanguage::Json => "json",
        EditorLanguage::Plain => "text",
    }
}
