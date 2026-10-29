//! 編輯器文件緩衝區與多游標操作 (EditorBuffer)
//!
//! 支援：
//! - 多行文字緩衝區與行列索引
//! - 多游標 (Multi-Cursor) 編輯與選取
//! - 行操作（複製行、刪除行、上下移動行、註解切換、大小寫轉換）
//! - 書籤系統 (Bookmarks)
//! - 復原 / 重做 (Undo / Redo) 歷史紀錄

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn point(pos: Position) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn start(&self) -> Position {
        if self.anchor.line < self.head.line
            || (self.anchor.line == self.head.line && self.anchor.col <= self.head.col)
        {
            self.anchor
        } else {
            self.head
        }
    }

    pub fn end(&self) -> Position {
        if self.anchor.line > self.head.line
            || (self.anchor.line == self.head.line && self.anchor.col > self.head.col)
        {
            self.anchor
        } else {
            self.head
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditorBuffer {
    pub lines: Vec<String>,
    pub selections: Vec<Selection>,
    pub bookmarks: Vec<usize>, // 0-based 行號
    undo_stack: Vec<Vec<String>>,
    redo_stack: Vec<Vec<String>>,
    pub modified: bool,
    pub file_path: Option<String>,
    pub line_ending: &'static str, // "\n" or "\r\n"
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            selections: vec![Selection::point(Position { line: 0, col: 0 })],
            bookmarks: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
            file_path: None,
            line_ending: "\n",
        }
    }

    pub fn from_text(text: &str) -> Self {
        let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let normalized = text.replace("\r\n", "\n");
        let mut lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }

        Self {
            lines,
            selections: vec![Selection::point(Position { line: 0, col: 0 })],
            bookmarks: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
            file_path: None,
            line_ending: ending,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join(self.line_ending)
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// 以 `\n` 連接的文字（供 GUI 編輯元件使用，不受原檔 CRLF 影響）
    pub fn edit_text(&self) -> String {
        self.lines.join("\n")
    }

    /// 以 GUI 編輯後的整份文字取代緩衝區（保留復原紀錄與原始換行格式）
    pub fn replace_all_text(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n");
        let new_lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
        if new_lines == self.lines {
            return;
        }
        self.record_undo();
        self.lines = if new_lines.is_empty() { vec![String::new()] } else { new_lines };
        self.clamp_cursors();
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line < self.lines.len() {
            self.lines[line].chars().count()
        } else {
            0
        }
    }

    fn record_undo(&mut self) {
        self.undo_stack.push(self.lines.clone());
        self.redo_stack.clear();
        self.modified = true;
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.lines.clone());
            self.lines = prev;
            self.clamp_cursors();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.lines.clone());
            self.lines = next;
            self.clamp_cursors();
            true
        } else {
            false
        }
    }

    pub fn clamp_cursors(&mut self) {
        let max_line = if self.lines.is_empty() { 0 } else { self.lines.len() - 1 };
        let lines = &self.lines;
        for sel in &mut self.selections {
            sel.anchor.line = sel.anchor.line.min(max_line);
            sel.head.line = sel.head.line.min(max_line);
            let anchor_len = lines.get(sel.anchor.line).map_or(0, |l| l.chars().count());
            let head_len = lines.get(sel.head.line).map_or(0, |l| l.chars().count());
            sel.anchor.col = sel.anchor.col.min(anchor_len);
            sel.head.col = sel.head.col.min(head_len);
        }
    }

    /// 在所有游標處插入文字
    pub fn insert_text(&mut self, text: &str) {
        self.record_undo();

        // 簡化單一游標常見情境
        if self.selections.len() == 1 {
            let sel = self.selections[0];
            let pos = sel.start();
            let mut line = self.lines[pos.line].clone();
            let char_indices: Vec<(usize, char)> = line.char_indices().collect();

            let byte_idx = if pos.col >= char_indices.len() {
                line.len()
            } else {
                char_indices[pos.col].0
            };

            line.insert_str(byte_idx, text);
            self.lines[pos.line] = line;

            let new_col = pos.col + text.chars().count();
            self.selections[0] = Selection::point(Position {
                line: pos.line,
                col: new_col,
            });
        }
    }

    /// 刪除當前行 (Ctrl+Shift+L)
    pub fn delete_current_line(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        self.record_undo();
        let cur_line = self.selections[0].head.line;
        if self.lines.len() > 1 {
            self.lines.remove(cur_line);
        } else {
            self.lines[0].clear();
        }
        self.clamp_cursors();
    }

    /// 複製當前行至下一行 (Ctrl+D)
    pub fn duplicate_current_line(&mut self) {
        self.record_undo();
        let cur_line = self.selections[0].head.line;
        let line_content = self.lines[cur_line].clone();
        self.lines.insert(cur_line + 1, line_content);
        self.selections[0].head.line += 1;
        self.selections[0].anchor = self.selections[0].head;
    }

    /// 上移當前行 (Alt+Up)
    pub fn move_line_up(&mut self) {
        let cur_line = self.selections[0].head.line;
        if cur_line > 0 {
            self.record_undo();
            self.lines.swap(cur_line, cur_line - 1);
            self.selections[0].head.line -= 1;
            self.selections[0].anchor = self.selections[0].head;
        }
    }

    /// 下移當前行 (Alt+Down)
    pub fn move_line_down(&mut self) {
        let cur_line = self.selections[0].head.line;
        if cur_line + 1 < self.lines.len() {
            self.record_undo();
            self.lines.swap(cur_line, cur_line + 1);
            self.selections[0].head.line += 1;
            self.selections[0].anchor = self.selections[0].head;
        }
    }

    /// 切換註解（依註解符號如 "// " 或 "# "）
    pub fn toggle_comment(&mut self, prefix: &str) {
        self.record_undo();
        let cur_line = self.selections[0].head.line;
        let line = &self.lines[cur_line];
        let trimmed = line.trim_start();
        if trimmed.starts_with(prefix) {
            let offset = line.len() - trimmed.len();
            let mut new_line = line[0..offset].to_string();
            new_line.push_str(trimmed.strip_prefix(prefix).unwrap_or(trimmed));
            self.lines[cur_line] = new_line;
        } else {
            let indent = line.len() - trimmed.len();
            let mut new_line = line[0..indent].to_string();
            new_line.push_str(prefix);
            new_line.push(' ');
            new_line.push_str(trimmed);
            self.lines[cur_line] = new_line;
        }
    }

    /// 切換書籤 (Ctrl+F2)
    pub fn toggle_bookmark(&mut self, line: usize) {
        if let Some(idx) = self.bookmarks.iter().position(|&l| l == line) {
            self.bookmarks.remove(idx);
        } else {
            self.bookmarks.push(line);
            self.bookmarks.sort();
        }
    }

    /// 跳至下一書籤 (F2)
    pub fn next_bookmark(&self, from_line: usize) -> Option<usize> {
        self.bookmarks
            .iter()
            .find(|&&l| l > from_line)
            .copied()
            .or_else(|| self.bookmarks.first().copied())
    }

    /// 跳至上一書籤 (Shift+F2)
    pub fn prev_bookmark(&self, from_line: usize) -> Option<usize> {
        self.bookmarks
            .iter()
            .rev()
            .find(|&&l| l < from_line)
            .copied()
            .or_else(|| self.bookmarks.last().copied())
    }
}
