//! 純邏輯 VT100/ANSI 終端機解析器與螢幕模型 (VtParser)
//!
//! 支援：
//! - 可列印字元、CR (\r)、LF (\n)、BS (\b)、TAB (\t 對齊 8 的倍數)
//! - EOL 自動換行
//! - 頂端捲動與 Scrollback 歷史紀錄 (上限 5000 行)
//! - CSI 游標移動 (CUU, CUD, CUF, CUB, CHA, CUP)
//! - 清除操作 (ED 2J 全螢幕清除, EL K 清除到行尾)
//! - SGR 樣式設定（重置、粗體、標準前背景色 30..37/40..47、亮色前背景色 90..97/100..107）
//! - OSC 標題設定序列略過
//! - 跨 Feed 切割與容忍未完整逸出序列
//! - UTF-8 串流多位元組解碼（支援中文、4 位元組 Emoji）

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: i8, // -1 = 預設; 0..15 = ANSI 顏色索引
    pub bg: i8, // -1 = 預設; 0..15 = ANSI 顏色索引
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: -1,
            bg: -1,
            bold: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Esc,
    Csi,
    Osc,
}

pub struct VtParser {
    rows: usize,
    cols: usize,
    cx: usize,
    cy: usize,
    grid: Vec<Vec<Cell>>,
    scrollback: Vec<String>,
    cur: Cell,
    state: State,
    csi_buf: Vec<u8>,
    utf8_buf: Vec<u8>,
    utf8_need: usize,
}

impl VtParser {
    pub const MAX_SCROLLBACK: usize = 5000;

    pub fn new(rows: usize, cols: usize) -> Self {
        let r = rows.max(1);
        let c = cols.max(1);
        let grid = vec![vec![Cell::default(); c]; r];
        Self {
            rows: r,
            cols: c,
            cx: 0,
            cy: 0,
            grid,
            scrollback: Vec::new(),
            cur: Cell::default(),
            state: State::Ground,
            csi_buf: Vec::new(),
            utf8_buf: Vec::new(),
            utf8_need: 0,
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn cursor_row(&self) -> usize {
        self.cy
    }

    pub fn cursor_col(&self) -> usize {
        self.cx
    }

    pub fn cell_at(&self, row: usize, col: usize) -> Cell {
        if row < self.rows && col < self.cols {
            self.grid[row][col]
        } else {
            Cell::default()
        }
    }

    pub fn line_text(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }
        let mut s: String = self.grid[row].iter().map(|c| c.ch).collect();
        while s.ends_with(' ') {
            s.pop();
        }
        s
    }

    pub fn scrollback_count(&self) -> usize {
        self.scrollback.len()
    }

    pub fn scrollback_line(&self, i: usize) -> String {
        if i < self.scrollback.len() {
            self.scrollback[i].clone()
        } else {
            String::new()
        }
    }

    fn scroll_up(&mut self) {
        let mut top: String = self.grid[0].iter().map(|c| c.ch).collect();
        while top.ends_with(' ') {
            top.pop();
        }
        self.scrollback.push(top);
        if self.scrollback.len() > Self::MAX_SCROLLBACK {
            let overflow = self.scrollback.len() - Self::MAX_SCROLLBACK;
            self.scrollback.drain(0..overflow);
        }

        for r in 0..self.rows - 1 {
            self.grid[r] = self.grid[r + 1].clone();
        }
        self.grid[self.rows - 1] = vec![Cell::default(); self.cols];
    }

    fn newline(&mut self) {
        if self.cy >= self.rows - 1 {
            self.scroll_up();
        } else {
            self.cy += 1;
        }
    }

    fn put_char(&mut self, ch: char) {
        if self.cx >= self.cols {
            self.cx = 0;
            self.newline();
        }
        let cell = &mut self.grid[self.cy][self.cx];
        cell.ch = ch;
        cell.fg = self.cur.fg;
        cell.bg = self.cur.bg;
        cell.bold = self.cur.bold;
        self.cx += 1;
    }

    fn emit_utf8(&mut self, bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        for ch in s.chars() {
            self.put_char(ch);
        }
    }

    fn flush_utf8(&mut self) {
        if self.utf8_need > 0 {
            self.put_char('\u{FFFD}');
            self.utf8_buf.clear();
            self.utf8_need = 0;
        }
    }

    fn feed_text_byte(&mut self, c: u8) {
        if self.utf8_need > 0 {
            if (c & 0xC0) == 0x80 {
                self.utf8_buf.push(c);
                self.utf8_need -= 1;
                if self.utf8_need == 0 {
                    let buf = std::mem::take(&mut self.utf8_buf);
                    self.emit_utf8(&buf);
                }
            } else {
                self.put_char('\u{FFFD}');
                self.utf8_buf.clear();
                self.utf8_need = 0;
                self.feed_text_byte(c);
            }
            return;
        }

        if c < 0x80 {
            self.put_char(c as char);
            return;
        }

        let len = if (c & 0xE0) == 0xC0 {
            2
        } else if (c & 0xF0) == 0xE0 {
            3
        } else if (c & 0xF8) == 0xF0 {
            4
        } else {
            0
        };

        if len == 0 {
            self.put_char('\u{FFFD}');
            return;
        }

        self.utf8_buf.clear();
        self.utf8_buf.push(c);
        self.utf8_need = len - 1;
    }

    fn param_or(&self, idx: usize, def: i32) -> i32 {
        let s = String::from_utf8_lossy(&self.csi_buf);
        let parts: Vec<&str> = s.split(';').collect();
        if idx >= parts.len() {
            return def;
        }
        parts[idx].trim().parse::<i32>().unwrap_or(def)
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            match self.state {
                State::Ground => {
                    if b == 0x1b {
                        self.flush_utf8();
                        self.state = State::Esc;
                    } else if b == b'\r' {
                        self.flush_utf8();
                        self.cx = 0;
                    } else if b == b'\n' {
                        self.flush_utf8();
                        self.newline();
                    } else if b == 0x08 {
                        self.flush_utf8();
                        if self.cx > 0 {
                            self.cx -= 1;
                        }
                    } else if b == b'\t' {
                        self.flush_utf8();
                        self.cx = (((self.cx / 8) + 1) * 8).min(self.cols - 1);
                    } else if b == 0x07 {
                        self.flush_utf8();
                    } else if b >= 0x20 {
                        self.feed_text_byte(b);
                    }
                }
                State::Esc => {
                    if b == b'[' {
                        self.state = State::Csi;
                        self.csi_buf.clear();
                    } else if b == b']' {
                        self.state = State::Osc;
                    } else {
                        self.state = State::Ground;
                    }
                }
                State::Csi => {
                    if (0x40..=0x7e).contains(&b) {
                        self.exec_csi(b as char);
                        self.state = State::Ground;
                        self.csi_buf.clear();
                    } else {
                        self.csi_buf.push(b);
                    }
                }
                State::Osc => {
                    if b == 0x07 || b == 0x1b {
                        self.state = State::Ground;
                    }
                }
            }
        }
    }

    fn exec_csi(&mut self, final_ch: char) {
        if !self.csi_buf.is_empty() && self.csi_buf[0] == b'?' {
            return; // 忽略私有模式序列（如 ?25l）
        }

        match final_ch {
            'A' => {
                let count = self.param_or(0, 1) as usize;
                self.cy = self.cy.saturating_sub(count);
            }
            'B' => {
                let count = self.param_or(0, 1) as usize;
                self.cy = (self.cy + count).min(self.rows - 1);
            }
            'C' => {
                let count = self.param_or(0, 1) as usize;
                self.cx = (self.cx + count).min(self.cols - 1);
            }
            'D' => {
                let count = self.param_or(0, 1) as usize;
                self.cx = self.cx.saturating_sub(count);
            }
            'G' => {
                let col = self.param_or(0, 1);
                self.cx = (col - 1).clamp(0, (self.cols - 1) as i32) as usize;
            }
            'd' => {
                let row = self.param_or(0, 1);
                self.cy = (row - 1).clamp(0, (self.rows - 1) as i32) as usize;
            }
            'H' | 'f' => {
                let row = self.param_or(0, 1);
                let col = self.param_or(1, 1);
                self.cy = (row - 1).clamp(0, (self.rows - 1) as i32) as usize;
                self.cx = (col - 1).clamp(0, (self.cols - 1) as i32) as usize;
            }
            'J' => {
                let mode = self.param_or(0, 0);
                if mode == 2 {
                    for r in 0..self.rows {
                        for c in 0..self.cols {
                            self.grid[r][c] = Cell::default();
                        }
                    }
                }
            }
            'K' => {
                let mode = self.param_or(0, 0);
                let (from, to) = if mode == 0 {
                    (self.cx, self.cols - 1)
                } else if mode == 1 {
                    (0, self.cx)
                } else {
                    (0, self.cols - 1)
                };
                for c in from..=to {
                    self.grid[self.cy][c] = Cell::default();
                }
            }
            'm' => {
                let s = String::from_utf8_lossy(&self.csi_buf);
                let mut params = Vec::new();
                for p in s.split(';') {
                    if p.is_empty() {
                        params.push(0);
                    } else if let Ok(val) = p.trim().parse::<i32>() {
                        params.push(val);
                    }
                }
                if params.is_empty() {
                    params.push(0);
                }
                self.apply_sgr(&params);
            }
            _ => {}
        }
    }

    fn apply_sgr(&mut self, params: &[i32]) {
        for &p in params {
            if p == 0 {
                self.cur = Cell::default();
            } else if p == 1 {
                self.cur.bold = true;
            } else if p == 22 {
                self.cur.bold = false;
            } else if (30..=37).contains(&p) {
                self.cur.fg = (p - 30) as i8;
            } else if p == 39 {
                self.cur.fg = -1;
            } else if (40..=47).contains(&p) {
                self.cur.bg = (p - 40) as i8;
            } else if p == 49 {
                self.cur.bg = -1;
            } else if (90..=97).contains(&p) {
                self.cur.fg = (8 + (p - 90)) as i8;
            } else if (100..=107).contains(&p) {
                self.cur.bg = (8 + (p - 100)) as i8;
            }
        }
    }
}
