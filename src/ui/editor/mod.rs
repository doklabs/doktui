mod buffer;
mod highlight;
pub mod view;
mod vim;

use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::EditorMode;

pub use buffer::TextBuffer;
pub use highlight::EditorLanguage;

/// Canvas code editor — rope buffer, syntax highlighting, Vim/non-Vim modes.
#[derive(Debug)]
pub struct CanvasEditor {
    pub path: String,
    pub buffer: TextBuffer,
    pub language: EditorLanguage,
    pub vim: VimState,
    pub dirty: bool,
    pub scroll_row: usize,
    pub status: Option<String>,
    pub pending_dd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy)]
pub struct VimState {
    pub enabled: bool,
    pub mode: VimMode,
}

impl VimState {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            mode: if enabled {
                VimMode::Normal
            } else {
                VimMode::Insert
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    None,
    Quit,
    Saved,
}

impl CanvasEditor {
    pub fn open(path: impl Into<String>, content: &str, editor_mode: EditorMode) -> Self {
        let path = path.into();
        let language = EditorLanguage::from_path(&path);
        let vim_enabled = editor_mode == EditorMode::Vim;
        Self {
            path,
            buffer: TextBuffer::from_str(content),
            language,
            vim: VimState::new(vim_enabled),
            dirty: false,
            scroll_row: 0,
            status: None,
            pending_dd: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> EditorAction {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            return self.save();
        }

        if self.vim.enabled {
            vim::handle_vim_key(self, key)
        } else {
            vim::handle_normal_key(self, key)
        }
    }

    pub fn save(&mut self) -> EditorAction {
        self.dirty = false;
        self.status = Some(format!("saved {}", self.path));
        EditorAction::Saved
    }

    pub fn content(&self) -> String {
        self.buffer.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let max = self.line_count().saturating_sub(visible_rows);
        self.scroll_row = self.scroll_row.min(max);
    }
}

impl EditorLanguage {
    pub fn from_path(path: &str) -> Self {
        match Path::new(path).extension().and_then(|e| e.to_str()) {
            Some("yml" | "yaml") => Self::Yaml,
            Some("toml") => Self::Toml,
            Some("env") => Self::Env,
            Some("dockerfile") | Some("Dockerfile") => Self::Dockerfile,
            Some("json") => Self::Json,
            _ if path.ends_with("Dockerfile") => Self::Dockerfile,
            _ if path.ends_with(".env") => Self::Env,
            _ => Self::Plain,
        }
    }
}

pub use view::render as render_editor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_compose_language() {
        assert_eq!(
            EditorLanguage::from_path("docker-compose.yml"),
            EditorLanguage::Yaml
        );
    }

    #[test]
    fn buffer_roundtrip() {
        let ed = CanvasEditor::open("test.env", "FOO=bar\n", EditorMode::Normal);
        assert_eq!(ed.content(), "FOO=bar\n");
    }
}
