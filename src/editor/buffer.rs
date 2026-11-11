//! 編輯器文件緩衝區與多游標操作 (EditorBuffer)
//!
//! 支援：
//! - 多行文字緩衝區與行列索引
//! - 多游標 (Multi-Cursor) 編輯與選取
//! - 行操作（複製行、刪除行、上下移動行、註解切換、大小寫轉換）
//! - 書籤系統 (Bookmarks)
//! - 復原 / 重做 (Undo / Redo) 歷史紀錄

use super::encoding::FileEncoding;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

/// 一筆復原紀錄：只保存「被換掉的那幾行」與「換上的那幾行」，而不是整份文件
#[derive(Debug, Clone)]
struct Edit {
    id: u64,
    start: usize,
    removed: Vec<String>,
    inserted: Vec<String>,
    /// 逐字輸入：時間相近的連續輸入在復原時視為一步
    typing: bool,
    at: Instant,
    bytes: usize,
}

static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct EditorBuffer {
    pub lines: Vec<String>,
    pub selections: Vec<Selection>,
    pub bookmarks: Vec<usize>, // 0-based 行號
    undo_stack: VecDeque<Edit>,
    redo_stack: Vec<Edit>,
    undo_bytes: usize,
    next_edit_id: u64,
    /// 存檔當下復原堆疊頂端的紀錄 id（0 = 空堆疊）；用來判斷復原/重做後是否回到已存檔的內容
    saved_id: u64,
    pub modified: bool,
    pub file_path: Option<String>,
    pub line_ending: &'static str, // "\n" or "\r\n"
    /// 開檔時偵測到的編碼；存檔時以同一編碼寫回
    pub encoding: FileEncoding,
    /// 分頁的唯一識別（不隨分頁順序改變），供 UI 快取使用
    pub id: u64,
    revision: u64,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    /// 復原紀錄的記憶體上限：超過時丟棄最舊的紀錄
    pub const MAX_UNDO_BYTES: usize = 128 * 1024 * 1024;
    const MAX_UNDO_EDITS: usize = 100_000;
    /// 連續逐字輸入的間隔在此之內，復原時合併為一步
    const TYPING_GROUP_GAP: Duration = Duration::from_millis(700);

    pub fn new() -> Self {
        Self::from_lines(vec![String::new()], "\n")
    }

    fn from_lines(lines: Vec<String>, line_ending: &'static str) -> Self {
        Self {
            lines,
            selections: vec![Selection::point(Position { line: 0, col: 0 })],
            bookmarks: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            undo_bytes: 0,
            next_edit_id: 1,
            saved_id: 0,
            modified: false,
            file_path: None,
            line_ending,
            encoding: FileEncoding::Utf8,
            id: NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
            revision: 0,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let normalized = text.replace("\r\n", "\n");
        let lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
        Self::from_lines(lines, ending)
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

    /// 每次內容變動（編輯、復原、重做）都會改變；供 UI 判斷快取是否過期
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 復原紀錄目前佔用的大約位元組數
    pub fn undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    /// 內容已寫入磁碟：之後復原/重做回到這個狀態時視為未修改
    pub fn mark_saved(&mut self) {
        self.saved_id = self.top_edit_id();
        self.modified = false;
    }

    /// 變更存檔編碼（內容不變，但需要重新存檔才會生效）
    pub fn set_encoding(&mut self, encoding: FileEncoding) {
        if self.encoding != encoding {
            self.encoding = encoding;
            self.saved_id = u64::MAX;
            self.modified = true;
        }
    }

    /// 以磁碟上的新內容取代整份文件（檔案被其他程式修改後重新載入）：
    /// 清空復原紀錄並視為未修改；分頁 id、書籤與游標位置（在範圍內的部分）保留
    pub fn reload_text(&mut self, text: &str) {
        let fresh = Self::from_text(text);
        self.lines = fresh.lines;
        self.line_ending = fresh.line_ending;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.undo_bytes = 0;
        self.saved_id = 0;
        self.modified = false;
        let n = self.lines.len();
        self.bookmarks.retain(|&l| l < n);
        self.clamp_cursors();
        self.revision += 1;
    }

    /// 以整份文字取代緩衝區（指令、Markdown 區塊編輯等），可復原
    pub fn replace_all_text(&mut self, text: &str) {
        self.apply_text(text, false);
    }

    /// 編輯區逐字輸入後的整份文字；時間相近的連續輸入在復原時合併為一步
    pub fn apply_typed_text(&mut self, text: &str) {
        self.apply_text(text, true);
    }

    fn apply_text(&mut self, text: &str, typing: bool) {
        let normalized = text.replace("\r\n", "\n");
        let new_lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
        if new_lines == self.lines {
            return;
        }
        let old = std::mem::replace(&mut self.lines, new_lines);
        self.push_edit(&old, typing);
        self.clamp_cursors();
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line < self.lines.len() {
            self.lines[line].chars().count()
        } else {
            0
        }
    }

    fn top_edit_id(&self) -> u64 {
        self.undo_stack.back().map_or(0, |e| e.id)
    }

    /// 記錄一筆復原紀錄：比對 `old` 與目前的 `self.lines`，只保存頭尾相同部分以外的行
    fn push_edit(&mut self, old: &[String], typing: bool) {
        let new = &self.lines;
        let max_common = old.len().min(new.len());
        let prefix = old.iter().zip(new.iter()).take_while(|(a, b)| a == b).count();
        if prefix == old.len() && prefix == new.len() {
            return;
        }
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(max_common - prefix)
            .take_while(|(a, b)| a == b)
            .count();
        let removed = old[prefix..old.len() - suffix].to_vec();
        let inserted = new[prefix..new.len() - suffix].to_vec();
        let bytes = removed.iter().chain(inserted.iter()).map(|l| l.len() + 24).sum::<usize>() + 96;
        let id = self.next_edit_id;
        self.next_edit_id += 1;
        self.undo_stack.push_back(Edit { id, start: prefix, removed, inserted, typing, at: Instant::now(), bytes });
        self.undo_bytes += bytes;
        // 至少保留最新一筆，其餘超過上限就從最舊的丟
        while self.undo_stack.len() > 1
            && (self.undo_bytes > Self::MAX_UNDO_BYTES || self.undo_stack.len() > Self::MAX_UNDO_EDITS)
        {
            if let Some(dropped) = self.undo_stack.pop_front() {
                self.undo_bytes -= dropped.bytes;
            }
        }
        self.redo_stack.clear();
        self.modified = true;
        self.revision += 1;
    }

    /// 把 `start` 起的 `from` 那幾行換成 `to`，並把游標放到變動結尾
    fn swap_lines(&mut self, start: usize, from: &[String], to: &[String]) {
        self.lines.splice(start..start + from.len(), to.iter().cloned());
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let pos = match to.last() {
            None => {
                let line = start.saturating_sub(1);
                Position { line, col: self.line_len(line) }
            }
            Some(last) => {
                let len = last.chars().count();
                let tail = from.last().map_or(0, |o| {
                    last.chars().rev().zip(o.chars().rev()).take_while(|(a, b)| a == b).count()
                });
                let mut col = len - tail.min(len);
                if from.len() == 1 && to.len() == 1 {
                    let head = last.chars().zip(from[0].chars()).take_while(|(a, b)| a == b).count();
                    col = col.max(head);
                }
                Position { line: start + to.len() - 1, col }
            }
        };
        self.selections = vec![Selection::point(pos)];
        self.clamp_cursors();
        self.revision += 1;
    }

    pub fn undo(&mut self) -> bool {
        let mut any = false;
        while let Some(edit) = self.undo_stack.pop_back() {
            self.undo_bytes -= edit.bytes;
            self.swap_lines(edit.start, &edit.inserted, &edit.removed);
            let keep_going = edit.typing
                && self.undo_stack.back().map_or(false, |prev| {
                    prev.typing && edit.at.duration_since(prev.at) < Self::TYPING_GROUP_GAP
                });
            self.redo_stack.push(edit);
            any = true;
            if !keep_going {
                break;
            }
        }
        if any {
            self.modified = self.top_edit_id() != self.saved_id;
        }
        any
    }

    pub fn redo(&mut self) -> bool {
        let mut any = false;
        while let Some(edit) = self.redo_stack.pop() {
            self.swap_lines(edit.start, &edit.removed, &edit.inserted);
            let keep_going = edit.typing
                && self.redo_stack.last().map_or(false, |next| {
                    next.typing && next.at.duration_since(edit.at) < Self::TYPING_GROUP_GAP
                });
            self.undo_bytes += edit.bytes;
            self.undo_stack.push_back(edit);
            any = true;
            if !keep_going {
                break;
            }
        }
        if any {
            self.modified = self.top_edit_id() != self.saved_id;
        }
        any
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
        let before = self.lines.clone();

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
        self.push_edit(&before, false);
    }

    /// 刪除當前行 (Ctrl+Shift+L)
    pub fn delete_current_line(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        let before = self.lines.clone();
        let cur_line = self.selections[0].head.line;
        if self.lines.len() > 1 {
            self.lines.remove(cur_line);
        } else {
            self.lines[0].clear();
        }
        self.clamp_cursors();
        self.push_edit(&before, false);
    }

    /// 複製當前行至下一行 (Ctrl+D)
    pub fn duplicate_current_line(&mut self) {
        let before = self.lines.clone();
        let cur_line = self.selections[0].head.line;
        let line_content = self.lines[cur_line].clone();
        self.lines.insert(cur_line + 1, line_content);
        self.selections[0].head.line += 1;
        self.selections[0].anchor = self.selections[0].head;
        self.push_edit(&before, false);
    }

    /// 上移當前行 (Alt+Up)
    pub fn move_line_up(&mut self) {
        let cur_line = self.selections[0].head.line;
        if cur_line > 0 {
            let before = self.lines.clone();
            self.lines.swap(cur_line, cur_line - 1);
            self.selections[0].head.line -= 1;
            self.selections[0].anchor = self.selections[0].head;
            self.push_edit(&before, false);
        }
    }

    /// 下移當前行 (Alt+Down)
    pub fn move_line_down(&mut self) {
        let cur_line = self.selections[0].head.line;
        if cur_line + 1 < self.lines.len() {
            let before = self.lines.clone();
            self.lines.swap(cur_line, cur_line + 1);
            self.selections[0].head.line += 1;
            self.selections[0].anchor = self.selections[0].head;
            self.push_edit(&before, false);
        }
    }

    /// 切換註解（依註解符號如 "// " 或 "# "）
    pub fn toggle_comment(&mut self, prefix: &str) {
        let before = self.lines.clone();
        let cur_line = self.selections[0].head.line;
        let line = &self.lines[cur_line];
        let trimmed = line.trim_start();
        if trimmed.starts_with(prefix) {
            let offset = line.len() - trimmed.len();
            let mut new_line = line[0..offset].to_string();
            // 加註解時會補一個空白，取消時一併移除
            let rest = trimmed.strip_prefix(prefix).unwrap_or(trimmed);
            new_line.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            self.lines[cur_line] = new_line;
        } else {
            let indent = line.len() - trimmed.len();
            let mut new_line = line[0..indent].to_string();
            new_line.push_str(prefix);
            new_line.push(' ');
            new_line.push_str(trimmed);
            self.lines[cur_line] = new_line;
        }
        self.push_edit(&before, false);
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
