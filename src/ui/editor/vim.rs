use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{CanvasEditor, EditorAction, VimMode};

pub fn handle_vim_key(editor: &mut CanvasEditor, key: KeyEvent) -> EditorAction {
    match editor.vim.mode {
        VimMode::Normal => handle_vim_normal(editor, key),
        VimMode::Insert => handle_vim_insert(editor, key),
    }
}

pub fn handle_normal_key(editor: &mut CanvasEditor, key: KeyEvent) -> EditorAction {
    match key.code {
        KeyCode::Esc => EditorAction::Quit,
        KeyCode::Backspace => {
            editor.buffer.backspace();
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Enter => {
            editor.buffer.insert_char('\n');
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Left => {
            editor.buffer.move_left();
            EditorAction::None
        }
        KeyCode::Right => {
            editor.buffer.move_right();
            EditorAction::None
        }
        KeyCode::Up => {
            editor.buffer.move_up();
            if editor.buffer.cursor_line_col().0 < editor.scroll_row {
                editor.scroll_row = editor.buffer.cursor_line_col().0;
            }
            EditorAction::None
        }
        KeyCode::Down => {
            editor.buffer.move_down();
            EditorAction::None
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.buffer.insert_char(c);
            editor.dirty = true;
            EditorAction::None
        }
        _ => EditorAction::None,
    }
}

fn handle_vim_normal(editor: &mut CanvasEditor, key: KeyEvent) -> EditorAction {
    match key.code {
        KeyCode::Char('h') | KeyCode::Left => {
            editor.buffer.move_left();
            EditorAction::None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            editor.buffer.move_right();
            EditorAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            editor.buffer.move_up();
            EditorAction::None
        }
        KeyCode::Char('j') | KeyCode::Down => {
            editor.buffer.move_down();
            EditorAction::None
        }
        KeyCode::Char('i') => {
            editor.vim.mode = VimMode::Insert;
            EditorAction::None
        }
        KeyCode::Char('a') => {
            editor.buffer.move_right();
            editor.vim.mode = VimMode::Insert;
            EditorAction::None
        }
        KeyCode::Char('o') => {
            editor.buffer.move_line_end();
            editor.buffer.insert_char('\n');
            editor.vim.mode = VimMode::Insert;
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Char('0') | KeyCode::Home => {
            editor.buffer.move_line_start();
            EditorAction::None
        }
        KeyCode::Char('$') | KeyCode::End => {
            editor.buffer.move_line_end();
            EditorAction::None
        }
        KeyCode::Char('x') => {
            editor.buffer.delete_char();
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Char('d') if editor.pending_dd => {
            editor.buffer.delete_line();
            editor.dirty = true;
            editor.pending_dd = false;
            EditorAction::None
        }
        KeyCode::Char('d') => {
            editor.pending_dd = true;
            EditorAction::None
        }
        KeyCode::Esc => EditorAction::Quit,
        _ => {
            editor.pending_dd = false;
            EditorAction::None
        }
    }
}

fn handle_vim_insert(editor: &mut CanvasEditor, key: KeyEvent) -> EditorAction {
    match key.code {
        KeyCode::Esc => {
            editor.vim.mode = VimMode::Normal;
            EditorAction::None
        }
        KeyCode::Backspace => {
            editor.buffer.backspace();
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Enter => {
            editor.buffer.insert_char('\n');
            editor.dirty = true;
            EditorAction::None
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            editor.buffer.insert_char(c);
            editor.dirty = true;
            EditorAction::None
        }
        _ => EditorAction::None,
    }
}
