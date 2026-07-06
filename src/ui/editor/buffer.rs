use ropey::Rope;

/// Rope-backed text buffer with cursor tracking (byte indices).
#[derive(Debug)]
pub struct TextBuffer {
    rope: Rope,
    cursor: usize,
}

#[allow(dead_code)] // reserved for Vim motions and future editor features
impl TextBuffer {
    pub fn from_str(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            cursor: 0,
        }
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.rope.len_chars());
    }

    pub fn line(&self, idx: usize) -> Option<String> {
        if idx >= self.rope.len_lines() {
            return None;
        }
        Some(self.rope.line(idx).to_string())
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let line = self.rope.char_to_line(self.cursor);
        let line_start = self.rope.line_to_char(line);
        (line, self.cursor - line_start)
    }

    pub fn insert_char(&mut self, c: char) {
        self.rope.insert_char(self.cursor, c);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        self.rope.insert(self.cursor, s);
        self.cursor += s.chars().count();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.cursor - 1;
        self.rope.remove(prev..self.cursor);
        self.cursor = prev;
    }

    pub fn delete_char(&mut self) {
        if self.cursor >= self.rope.len_chars() {
            return;
        }
        self.rope.remove(self.cursor..self.cursor + 1);
    }

    pub fn delete_line(&mut self) {
        let line = self.rope.char_to_line(self.cursor);
        if line + 1 < self.rope.len_lines() {
            let start = self.rope.line_to_char(line);
            let end = self.rope.line_to_char(line + 1);
            self.rope.remove(start..end);
            self.cursor = start.min(self.rope.len_chars().saturating_sub(1));
        } else if self.rope.len_lines() > 1 {
            let start = self.rope.line_to_char(line.saturating_sub(1));
            let end = self.rope.len_chars();
            self.rope.remove(start..end);
            self.cursor = start;
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.rope.len_chars() {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        let prev_line = line - 1;
        let prev_start = self.rope.line_to_char(prev_line);
        let prev_len = self.rope.line(prev_line).len_chars();
        self.cursor = prev_start + col.min(prev_len.saturating_sub(1).max(0));
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.rope.len_lines() {
            return;
        }
        let next_line = line + 1;
        let next_start = self.rope.line_to_char(next_line);
        let next_len = self.rope.line(next_line).len_chars();
        self.cursor = next_start + col.min(next_len.saturating_sub(1).max(0));
    }

    pub fn move_line_start(&mut self) {
        let line = self.rope.char_to_line(self.cursor);
        self.cursor = self.rope.line_to_char(line);
    }

    pub fn move_line_end(&mut self) {
        let line = self.rope.char_to_line(self.cursor);
        if line + 1 < self.rope.len_lines() {
            self.cursor = self.rope.line_to_char(line + 1) - 1;
        } else {
            self.cursor = self.rope.len_chars();
        }
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}
