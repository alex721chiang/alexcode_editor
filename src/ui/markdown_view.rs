//! Typora 風格 Markdown 即時預覽與所見即所得編輯器
//!
//! 支援三種檢視模式：
//! - **Typora 模式 (`MdViewMode::Typora`)**：直觀的排版閱讀體驗；點擊任意段落區塊即可原位編輯 Markdown 原始碼，
//!   按 `Esc` 或點擊其他區塊立即回到精美排版；待辦清單 (`- [ ]` / `- [x]`) 可直接點擊勾選！
//! - **雙欄同步預覽 (`MdViewMode::Split`)**：左側程式碼編輯器 + 右側即時渲染預覽。
//! - **原始碼模式 (`MdViewMode::Source`)**：純 Markdown 原始碼編輯。
//! - **大綱側邊欄 (Outline / TOC)**：自動擷取 `# H1` ~ `###### H6` 標題階層，點擊快速跳轉章節。

use super::highlight::{highlight, Lang};
use super::style::{hex, with_alpha, Colors, ORBITRON};
use crate::editor::EditorBuffer;
use crate::theme::Palette;
use eframe::egui::{
    self, Align, Color32, CursorIcon, FontFamily, FontId, Key, Layout, Margin, Rect, RichText, Rounding,
    Sense, Stroke, Vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Markdown 檢視模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdViewMode {
    /// Typora 風格：富文本閱讀 + 點擊區塊原位編輯
    Typora,
    /// 左右雙欄：左編輯器、右即時預覽
    Split,
    /// 純原始碼編輯器
    Source,
}

impl MdViewMode {
    pub fn next(self) -> Self {
        match self {
            MdViewMode::Typora => MdViewMode::Split,
            MdViewMode::Split => MdViewMode::Source,
            MdViewMode::Source => MdViewMode::Typora,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MdViewMode::Typora => "📖 Typora 閱讀",
            MdViewMode::Split => "◫ 雙欄預覽",
            MdViewMode::Source => "‹/› 原始碼",
        }
    }
}

/// 提示框種類（GitHub / Obsidian / Typora 風格 `> [!NOTE]` 等）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    fn from_tag(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "[!NOTE]" | "[!INFO]" => Some(AlertKind::Note),
            "[!TIP]" | "[!HINT]" | "[!SUCCESS]" => Some(AlertKind::Tip),
            "[!IMPORTANT]" => Some(AlertKind::Important),
            "[!WARNING]" | "[!WARN]" => Some(AlertKind::Warning),
            "[!CAUTION]" | "[!DANGER]" | "[!ERROR]" => Some(AlertKind::Caution),
            _ => None,
        }
    }

    fn badge_and_color(self, c: &Colors, pal: &Palette) -> (&'static str, Color32) {
        match self {
            AlertKind::Note => ("ℹ  NOTE", c.accent),
            AlertKind::Tip => ("💡 TIP", hex(pal.syn_string)),
            AlertKind::Important => ("📌 IMPORTANT", hex(pal.syn_num)),
            AlertKind::Warning => ("⚠  WARNING", c.warn),
            AlertKind::Caution => ("🔥 CAUTION", c.accent2),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ListMarker {
    Bullet,
    Ordered(String),
    Task(bool),
}

#[derive(Debug, Clone)]
pub enum MdBlockKind {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    CodeBlock {
        lang: String,
        code: String,
    },
    Alert {
        kind: AlertKind,
        title: String,
        lines: Vec<String>,
    },
    BlockQuote {
        lines: Vec<String>,
    },
    ListItem {
        indent: usize,
        marker: ListMarker,
        content: String,
        line_idx: usize,
    },
    Table {
        headers: Vec<String>,
        aligns: Vec<Align>,
        rows: Vec<Vec<String>>,
    },
    HorizontalRule,
    Image {
        alt: String,
        url: String,
    },
}

#[derive(Debug, Clone)]
pub struct MdBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: MdBlockKind,
}

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub line: usize,
}

/// 行內樣式狀態
#[derive(Debug, Clone, Default)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    highlight: bool,
    link_target: Option<(String, bool)>, // (target, is_wiki)
}

#[derive(Debug, Clone)]
struct InlineSegment {
    text: String,
    style: InlineStyle,
}

/// Typora 檢視器與原位編輯狀態
pub struct MarkdownViewState {
    pub mode: MdViewMode,
    pub show_outline: bool,
    /// 目前正在原位編輯的區塊 (start_line, end_line)
    pub editing_block: Option<(usize, usize)>,
    pub editing_text: String,
    pub request_edit_focus: bool,
    /// 大綱點擊後請求捲動到的目標行號
    pub scroll_to_line: Option<usize>,
    /// 已載入的本地圖片快取
    image_cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl Default for MarkdownViewState {
    fn default() -> Self {
        Self {
            mode: MdViewMode::Typora,
            show_outline: true,
            editing_block: None,
            editing_text: String::new(),
            request_edit_focus: false,
            scroll_to_line: None,
            image_cache: HashMap::new(),
        }
    }
}

/// 檢視器回傳給主程式的互動事件
pub enum MdAction {
    None,
    OpenLink { target: String, wiki: bool },
    JumpToSourceLine(usize),
}

impl MarkdownViewState {
    /// 若正在編輯某區塊，將變更寫回 `EditorBuffer`
    pub fn commit_active_edit(&mut self, buf: &mut EditorBuffer) {
        if let Some((start, end)) = self.editing_block.take() {
            let old_slice = if start <= end && end < buf.lines.len() {
                buf.lines[start..=end].join("\n")
            } else {
                String::new()
            };
            if self.editing_text != old_slice {
                let mut all = buf.lines.clone();
                let new_lines: Vec<String> = self
                    .editing_text
                    .replace("\r\n", "\n")
                    .split('\n')
                    .map(|s| s.to_string())
                    .collect();
                if start <= end && end < all.len() {
                    all.splice(start..=end, new_lines);
                } else {
                    all.extend(new_lines);
                }
                buf.replace_all_text(&all.join("\n"));
            }
            self.editing_text.clear();
        }
    }

    fn begin_block_edit(&mut self, buf: &EditorBuffer, start: usize, end: usize) {
        if start <= end && end < buf.lines.len() {
            self.editing_block = Some((start, end));
            self.editing_text = buf.lines[start..=end].join("\n");
            self.request_edit_focus = true;
        }
    }
}

// =====================================================================
//  Markdown 區塊與行內解析
// =====================================================================

/// 擷取大綱 (TOC) 標題清單
pub fn extract_toc(lines: &[String]) -> Vec<TocEntry> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((lvl, title)) = parse_atx_heading(t) {
            out.push(TocEntry {
                level: lvl,
                title: strip_inline_markers(&title),
                line: idx,
            });
        }
    }
    out
}

fn parse_atx_heading(trimmed: &str) -> Option<(u8, String)> {
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) {
        let rest = &trimmed[hashes..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
            let clean = rest.trim().trim_end_matches('#').trim().to_string();
            return Some((hashes as u8, clean));
        }
    }
    None
}

fn is_hr(trimmed: &str) -> bool {
    if trimmed.len() < 3 {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() < 3 {
        return false;
    }
    let first = chars[0];
    (first == '-' || first == '*' || first == '_') && chars.iter().all(|&c| c == first)
}

fn is_table_separator(line: &str) -> Option<Vec<Align>> {
    let t = line.trim();
    if !t.contains('|') || !t.contains('-') {
        return None;
    }
    let cells = split_table_row(t);
    if cells.is_empty() {
        return None;
    }
    let mut aligns = Vec::with_capacity(cells.len());
    for cell in cells {
        let c = cell.trim();
        if c.is_empty() {
            return None;
        }
        let body = c.trim_start_matches(':').trim_end_matches(':');
        if body.len() < 2 || !body.chars().all(|ch| ch == '-') {
            return None;
        }
        let left = c.starts_with(':');
        let right = c.ends_with(':');
        aligns.push(match (left, right) {
            (true, true) => Align::Center,
            (false, true) => Align::Max,
            _ => Align::Min,
        });
    }
    Some(aligns)
}

fn split_table_row(line: &str) -> Vec<String> {
    let mut t = line.trim();
    if let Some(s) = t.strip_prefix('|') {
        t = s;
    }
    if let Some(s) = t.strip_suffix('|') {
        t = s;
    }
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_list_line(line: &str) -> Option<(usize, ListMarker, String)> {
    let indent_spaces: usize = line
        .chars()
        .take_while(|&c| c == ' ' || c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum();
    let indent = indent_spaces / 2;
    let t = line.trim_start();

    // 無序清單或待辦清單
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let r = rest.trim_start();
            if let Some(after_box) = r.strip_prefix("[ ] ").or_else(|| r.strip_prefix("[] ")) {
                return Some((indent, ListMarker::Task(false), after_box.to_string()));
            }
            if let Some(after_box) = r.strip_prefix("[x] ").or_else(|| r.strip_prefix("[X] ")) {
                return Some((indent, ListMarker::Task(true), after_box.to_string()));
            }
            if r == "[ ]" {
                return Some((indent, ListMarker::Task(false), String::new()));
            }
            if r == "[x]" || r == "[X]" {
                return Some((indent, ListMarker::Task(true), String::new()));
            }
            return Some((indent, ListMarker::Bullet, rest.to_string()));
        }
    }

    // 有序清單 `1. text`
    let digits = t.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 && digits <= 6 {
        let after = &t[digits..];
        if let Some(rest) = after.strip_prefix(". ").or_else(|| after.strip_prefix(") ")) {
            let num = format!("{}.", &t[..digits]);
            return Some((indent, ListMarker::Ordered(num), rest.to_string()));
        }
    }
    None
}

fn parse_standalone_image(trimmed: &str) -> Option<(String, String)> {
    let rest = trimmed.strip_prefix("![")?;
    let close_bracket = rest.find("](")?;
    let alt = rest[..close_bracket].to_string();
    let after = &rest[close_bracket + 2..];
    let close_paren = after.rfind(')')?;
    if after[close_paren + 1..].trim().is_empty() {
        let url = after[..close_paren].trim().to_string();
        if !url.is_empty() {
            return Some((alt, url));
        }
    }
    None
}

/// 將行陣列解析為結構化的 Typora 區塊列表
pub fn parse_markdown_blocks(lines: &[String]) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let n = lines.len();
    let mut i = 0;

    while i < n {
        let line = &lines[i];
        let trimmed = line.trim();

        // 空白行略過（交由區塊間距表現）
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // 1. 圍欄式程式碼區塊 ``` 或 ~~~
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fence = if trimmed.starts_with("```") { "```" } else { "~~~" };
            let lang = trimmed[3..].trim().to_string();
            let start = i;
            i += 1;
            let mut code_lines = Vec::new();
            while i < n {
                if lines[i].trim().starts_with(fence) {
                    i += 1;
                    break;
                }
                code_lines.push(lines[i].as_str());
                i += 1;
            }
            let end = (i - 1).max(start);
            blocks.push(MdBlock {
                start_line: start,
                end_line: end,
                kind: MdBlockKind::CodeBlock {
                    lang,
                    code: code_lines.join("\n"),
                },
            });
            continue;
        }

        // 2. ATX 標題 (# ~ ######)
        if let Some((level, text)) = parse_atx_heading(trimmed) {
            blocks.push(MdBlock {
                start_line: i,
                end_line: i,
                kind: MdBlockKind::Heading { level, text },
            });
            i += 1;
            continue;
        }

        // 3. 水平分隔線
        if is_hr(trimmed) {
            blocks.push(MdBlock {
                start_line: i,
                end_line: i,
                kind: MdBlockKind::HorizontalRule,
            });
            i += 1;
            continue;
        }

        // 4. 獨立圖片行 ![alt](url)
        if let Some((alt, url)) = parse_standalone_image(trimmed) {
            blocks.push(MdBlock {
                start_line: i,
                end_line: i,
                kind: MdBlockKind::Image { alt, url },
            });
            i += 1;
            continue;
        }

        // 5. 引用區塊 / GitHub Alert (`> ...`)
        if trimmed.starts_with('>') {
            let start = i;
            let mut quote_lines = Vec::new();
            while i < n && lines[i].trim().starts_with('>') {
                let raw = lines[i].trim();
                let stripped = raw.strip_prefix("> ").or_else(|| raw.strip_prefix('>')).unwrap_or("");
                quote_lines.push(stripped.to_string());
                i += 1;
            }
            let end = i - 1;
            let first = quote_lines.first().map(|s| s.trim()).unwrap_or("");
            let mut words = first.splitn(2, char::is_whitespace);
            let tag = words.next().unwrap_or("");
            if let Some(alert_kind) = AlertKind::from_tag(tag) {
                let custom_title = words.next().unwrap_or("").trim().to_string();
                let body: Vec<String> = quote_lines.into_iter().skip(1).collect();
                blocks.push(MdBlock {
                    start_line: start,
                    end_line: end,
                    kind: MdBlockKind::Alert {
                        kind: alert_kind,
                        title: custom_title,
                        lines: body,
                    },
                });
            } else {
                blocks.push(MdBlock {
                    start_line: start,
                    end_line: end,
                    kind: MdBlockKind::BlockQuote { lines: quote_lines },
                });
            }
            continue;
        }

        // 6. 表格（目前行含 `|`，且下一行為 `| --- | --- |`）
        if trimmed.contains('|') && i + 1 < n {
            if let Some(aligns) = is_table_separator(&lines[i + 1]) {
                let start = i;
                let headers = split_table_row(trimmed);
                i += 2;
                let mut rows = Vec::new();
                while i < n {
                    let row_t = lines[i].trim();
                    if row_t.is_empty() || !row_t.contains('|') {
                        break;
                    }
                    rows.push(split_table_row(row_t));
                    i += 1;
                }
                let end = i - 1;
                blocks.push(MdBlock {
                    start_line: start,
                    end_line: end,
                    kind: MdBlockKind::Table { headers, aligns, rows },
                });
                continue;
            }
        }

        // 7. 清單項目 / 待辦事項
        if let Some((indent, marker, content)) = parse_list_line(line) {
            blocks.push(MdBlock {
                start_line: i,
                end_line: i,
                kind: MdBlockKind::ListItem {
                    indent,
                    marker,
                    content,
                    line_idx: i,
                },
            });
            i += 1;
            continue;
        }

        // 8. 一般段落（合併連續非特殊行）
        let start = i;
        let mut para = Vec::new();
        while i < n {
            let l = &lines[i];
            let t = l.trim();
            if t.is_empty()
                || t.starts_with('#')
                || t.starts_with("```")
                || t.starts_with("~~~")
                || t.starts_with('>')
                || is_hr(t)
                || parse_list_line(l).is_some()
                || parse_standalone_image(t).is_some()
                || (t.contains('|') && i + 1 < n && is_table_separator(&lines[i + 1]).is_some())
            {
                break;
            }
            para.push(t);
            i += 1;
        }
        let end = (i - 1).max(start);
        blocks.push(MdBlock {
            start_line: start,
            end_line: end,
            kind: MdBlockKind::Paragraph {
                text: para.join(" "),
            },
        });
    }

    blocks
}

/// 去除標題中的粗體/行內碼符號以供大綱顯示
fn strip_inline_markers(s: &str) -> String {
    s.replace("**", "")
        .replace("__", "")
        .replace('`', "")
        .replace("==", "")
        .replace("~~", "")
}

/// 解析行內 Markdown 語法：`***bold italic***`、`**bold**`、`*italic*`、`~~strike~~`、`` `code` ``、`==highlight==`、`[[wiki]]`、`[text](url)`
fn parse_inline(text: &str) -> Vec<InlineSegment> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut style = InlineStyle::default();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let len = chars.len();
    let mut idx = 0;

    let flush = |out: &mut Vec<InlineSegment>, cur: &mut String, style: &InlineStyle| {
        if !cur.is_empty() {
            out.push(InlineSegment {
                text: std::mem::take(cur),
                style: style.clone(),
            });
        }
    };

    while idx < len {
        let byte_pos = chars[idx].0;
        let rest = &text[byte_pos..];

        // 行內程式碼 `...`
        if rest.starts_with('`') {
            if let Some(end_off) = rest[1..].find('`') {
                flush(&mut out, &mut cur, &style);
                let code_str = &rest[1..1 + end_off];
                let mut code_style = style.clone();
                code_style.code = true;
                out.push(InlineSegment {
                    text: code_str.to_string(),
                    style: code_style,
                });
                let total_bytes = 1 + end_off + 1;
                while idx < len && chars[idx].0 < byte_pos + total_bytes {
                    idx += 1;
                }
                continue;
            }
        }

        // WikiLink: [[target|alias]] 或 [[target]]
        if rest.starts_with("[[") {
            if let Some(end_off) = rest[2..].find("]]") {
                flush(&mut out, &mut cur, &style);
                let inner = &rest[2..2 + end_off];
                let (target, display) = if let Some((t, alias)) = inner.split_once('|') {
                    (t.trim(), alias.trim())
                } else {
                    (inner.trim(), inner.trim())
                };
                let mut lk_style = style.clone();
                lk_style.link_target = Some((target.to_string(), true));
                out.push(InlineSegment {
                    text: format!("🔗 {}", display),
                    style: lk_style,
                });
                let total_bytes = 2 + end_off + 2;
                while idx < len && chars[idx].0 < byte_pos + total_bytes {
                    idx += 1;
                }
                continue;
            }
        }

        // 標準連結: [text](url)
        if rest.starts_with('[') && !rest.starts_with("[[") {
            if let Some(mid) = rest[1..].find("](") {
                let after_mid = &rest[1 + mid + 2..];
                if let Some(close_p) = after_mid.find(')') {
                    flush(&mut out, &mut cur, &style);
                    let label = &rest[1..1 + mid];
                    let url = after_mid[..close_p].trim();
                    let mut lk_style = style.clone();
                    lk_style.link_target = Some((url.to_string(), false));
                    out.push(InlineSegment {
                        text: label.to_string(),
                        style: lk_style,
                    });
                    let total_bytes = 1 + mid + 2 + close_p + 1;
                    while idx < len && chars[idx].0 < byte_pos + total_bytes {
                        idx += 1;
                    }
                    continue;
                }
            }
        }

        // 粗斜體 ***
        if rest.starts_with("***") {
            flush(&mut out, &mut cur, &style);
            style.bold = !style.bold;
            style.italic = !style.italic;
            idx += 3;
            continue;
        }
        // 粗體 ** 或 __
        if rest.starts_with("**") || rest.starts_with("__") {
            flush(&mut out, &mut cur, &style);
            style.bold = !style.bold;
            idx += 2;
            continue;
        }
        // 刪除線 ~~
        if rest.starts_with("~~") {
            flush(&mut out, &mut cur, &style);
            style.strike = !style.strike;
            idx += 2;
            continue;
        }
        // 螢光筆標記 ==
        if rest.starts_with("==") {
            flush(&mut out, &mut cur, &style);
            style.highlight = !style.highlight;
            idx += 2;
            continue;
        }
        // 斜體 *
        if rest.starts_with('*') {
            flush(&mut out, &mut cur, &style);
            style.italic = !style.italic;
            idx += 1;
            continue;
        }

        cur.push(chars[idx].1);
        idx += 1;
    }

    flush(&mut out, &mut cur, &style);
    out
}

// =====================================================================
//  UI 渲染：行內與區塊繪製
// =====================================================================

fn render_inline_text(
    ui: &mut egui::Ui,
    text: &str,
    base_size: f32,
    base_color: Color32,
    force_bold: bool,
    force_strike: bool,
    c: &Colors,
    pal: &Palette,
    find_query: &str,
    action: &mut MdAction,
) {
    let segments = parse_inline(text);
    let syn_code_fg = hex(pal.syn_string);
    let syn_kw = hex(pal.syn_kw);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        ui.spacing_mut().item_spacing.y = 4.0;

        for seg in segments {
            let is_hit = !find_query.is_empty() && seg.text.to_lowercase().contains(&find_query.to_lowercase());
            if let Some((target, is_wiki)) = seg.style.link_target {
                let mut rt = RichText::new(&seg.text)
                    .size(base_size)
                    .color(if is_wiki { hex(pal.syn_type) } else { c.accent })
                    .underline();
                if seg.style.bold || force_bold {
                    rt = rt.strong();
                }
                if is_wiki || is_hit {
                    rt = rt.background_color(if is_hit { c.search_bg } else { c.accent_faint });
                }
                let resp = ui
                    .add(egui::Label::new(rt).sense(Sense::click()))
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text(format!("點擊開啟連結：{}", target));
                if resp.clicked() {
                    *action = MdAction::OpenLink { target, wiki: is_wiki };
                }
                continue;
            }

            if seg.style.code {
                let rt = RichText::new(format!(" {} ", seg.text))
                    .font(FontId::monospace((base_size - 1.0).max(10.0)))
                    .color(syn_code_fg)
                    .background_color(if is_hit { c.search_bg } else { c.panel });
                ui.add(egui::Label::new(rt));
                continue;
            }

            let mut col = if seg.style.bold && !force_bold {
                syn_kw
            } else {
                base_color
            };
            if is_hit {
                col = c.search_fg;
            }

            let mut rt = RichText::new(&seg.text).size(base_size).color(col);
            if seg.style.bold || force_bold {
                rt = rt.strong();
            }
            if seg.style.italic {
                rt = rt.italics();
            }
            if seg.style.strike || force_strike {
                rt = rt.strikethrough();
            }
            if seg.style.highlight {
                rt = rt
                    .background_color(with_alpha(c.warn, if c.light { 95 } else { 65 }))
                    .color(if c.light { Color32::BLACK } else { c.warn });
            } else if is_hit {
                rt = rt.background_color(c.search_bg);
            }
            ui.add(egui::Label::new(rt));
        }
    });
}

/// 繪製 Typora 頂部工具條（檢視模式切換、大綱開關、快速語法插入、字數與閱讀時間統計）
pub fn render_markdown_toolbar(
    ui: &mut egui::Ui,
    state: &mut MarkdownViewState,
    buf: &mut EditorBuffer,
    c: &Colors,
) {
    let frame = egui::Frame::none()
        .fill(with_alpha(c.panel, 225))
        .inner_margin(Margin::symmetric(12.0, 5.0));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // 1. 大綱按鈕
            let toc_btn = egui::SelectableLabel::new(
                state.show_outline,
                RichText::new("📑 大綱").size(12.5).color(if state.show_outline {
                    c.accent
                } else {
                    c.fg_dim
                }),
            );
            if ui.add(toc_btn).on_hover_text("顯示 / 隱藏章節大綱側邊欄 (TOC)").clicked() {
                state.show_outline = !state.show_outline;
            }

            ui.separator();

            // 2. 三種模式切換膠囊按鈕
            for m in [MdViewMode::Typora, MdViewMode::Split, MdViewMode::Source] {
                let active = state.mode == m;
                let lbl = egui::SelectableLabel::new(
                    active,
                    RichText::new(m.label())
                        .size(12.5)
                        .color(if active { c.accent } else { c.fg_dim }),
                );
                if ui
                    .add(lbl)
                    .on_hover_text("切換 Markdown 檢視模式 (快捷鍵 Ctrl+Shift+M)")
                    .clicked()
                {
                    if state.mode == MdViewMode::Typora && m != MdViewMode::Typora {
                        state.commit_active_edit(buf);
                    }
                    state.mode = m;
                }
            }

            ui.separator();

            // 3. 快速格式插入按鈕
            let mut insert_snippet: Option<&str> = None;
            ui.spacing_mut().item_spacing.x = 3.0;
            if ui.small_button(RichText::new("H1").strong()).on_hover_text("插入一級標題").clicked() {
                insert_snippet = Some("\n# 標題\n");
            }
            if ui.small_button(RichText::new("H2").strong()).on_hover_text("插入二級標題").clicked() {
                insert_snippet = Some("\n## 副標題\n");
            }
            if ui.small_button(RichText::new("B").strong()).on_hover_text("插入粗體").clicked() {
                insert_snippet = Some("**粗體文字**");
            }
            if ui.small_button(RichText::new("I").italics()).on_hover_text("插入斜體").clicked() {
                insert_snippet = Some("*斜體文字*");
            }
            if ui.small_button("==").on_hover_text("插入螢光筆標記").clicked() {
                insert_snippet = Some("==重點標記==");
            }
            if ui.small_button("`<>`").on_hover_text("插入程式碼區塊").clicked() {
                insert_snippet = Some("\n```rust\nprintln!(\"Hello Markdown!\");\n```\n");
            }
            if ui.small_button("☑").on_hover_text("插入待辦清單").clicked() {
                insert_snippet = Some("\n- [ ] 待辦事項\n- [x] 已完成事項");
            }
            if ui.small_button("❝").on_hover_text("插入提示區塊 (Callout)").clicked() {
                insert_snippet = Some("\n> [!TIP]\n> 在這裡輸入提示內容\n");
            }
            if ui.small_button("▦").on_hover_text("插入表格").clicked() {
                insert_snippet = Some("\n| 欄位 A | 欄位 B | 欄位 C |\n| :--- | :---: | ---: |\n| 內容 1 | 內容 2 | 100 |\n");
            }

            if let Some(snip) = insert_snippet {
                if state.editing_block.is_some() {
                    state.editing_text.push_str(snip);
                } else {
                    let mut text = buf.edit_text();
                    text.push_str(snip);
                    buf.replace_all_text(&text);
                }
            }

            // 4. 右側字數與閱讀時間統計（Typora 風格）
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let char_count: usize = buf.lines.iter().map(|l| l.chars().count()).sum();
                let read_min = (char_count / 350).max(1);
                ui.label(
                    RichText::new(format!("{} 字 · 約 {} 分鐘閱讀", char_count, read_min))
                        .size(11.5)
                        .color(c.fg_dim2),
                );
                if state.mode == MdViewMode::Typora {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("💡 點擊任一段落即可原位編輯，Esc 返回預覽")
                            .size(11.5)
                            .color(c.accent_dim),
                    );
                }
            });
        });
    });
    let r = ui.max_rect();
    ui.painter().hline(r.x_range(), r.bottom(), Stroke::new(1.0_f32, c.border));
}

/// 繪製左側 Typora 大綱 (Outline) 面板
pub fn render_outline_panel(
    ui: &mut egui::Ui,
    lines: &[String],
    c: &Colors,
) -> Option<usize> {
    let mut clicked_line = None;
    let toc = extract_toc(lines);

    egui::Frame::none()
        .fill(c.panel)
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("OUTLINE  大綱")
                        .font(FontId::new(11.0, FontFamily::Name(ORBITRON.into())))
                        .color(c.accent)
                        .strong(),
                );
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            if toc.is_empty() {
                ui.label(
                    RichText::new("尚無標題（使用 # ~ ###### 建立章節）")
                        .size(12.0)
                        .color(c.fg_dim2),
                );
                return;
            }

            egui::ScrollArea::vertical()
                .id_source("md_outline_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in &toc {
                        let indent = (entry.level.saturating_sub(1) as f32) * 12.0;
                        ui.horizontal(|ui| {
                            ui.add_space(indent);
                            let col = match entry.level {
                                1 => c.accent,
                                2 => c.fg,
                                3 => c.fg_dim,
                                _ => c.fg_dim2,
                            };
                            let prefix = match entry.level {
                                1 => "▌ ",
                                2 => "• ",
                                _ => "· ",
                            };
                            let mut rt = RichText::new(format!("{}{}", prefix, entry.title))
                                .size(if entry.level == 1 { 13.0 } else { 12.2 })
                                .color(col);
                            if entry.level == 1 {
                                rt = rt.strong();
                            }
                            let resp = ui
                                .add(egui::Label::new(rt).wrap(false).sense(Sense::click()))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .on_hover_text(format!("跳至第 {} 行", entry.line + 1));
                            if resp.clicked() {
                                clicked_line = Some(entry.line);
                            }
                        });
                        ui.add_space(2.0);
                    }
                });
        });

    clicked_line
}

/// 繪製 Typora 富文本文件預覽（支援點擊原位編輯、互動式勾選框、程式碼高亮、表格、提示框與圖片）
pub fn render_typora_document(
    ui: &mut egui::Ui,
    state: &mut MarkdownViewState,
    buf: &mut EditorBuffer,
    base_font_size: f32,
    c: &Colors,
    pal: &Palette,
    find_query: &str,
    project_root: &Path,
    allow_inline_edit: bool,
) -> MdAction {
    let mut action = MdAction::None;

    // 按下 Esc 立即結束區塊原位編輯並回到精美預覽
    if state.editing_block.is_some() && ui.input(|i| i.key_pressed(Key::Escape)) {
        state.commit_active_edit(buf);
    }

    let blocks = parse_markdown_blocks(&buf.lines);
    let scroll_target = state.scroll_to_line.take();
    let mut toggle_task_line: Option<usize> = None;
    let mut start_edit_range: Option<(usize, usize)> = None;
    let mut commit_now = false;

    egui::ScrollArea::vertical()
        .id_source(("typora_doc_scroll", allow_inline_edit))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_w = ui.available_width();
            // Typora 標誌性的居中舒適閱讀版心（自動留白）
            let max_doc_w = 820.0_f32;
            let side_pad = ((avail_w - max_doc_w) * 0.5).max(24.0);

            egui::Frame::none()
                .inner_margin(Margin {
                    left: side_pad,
                    right: side_pad,
                    top: 22.0,
                    bottom: 60.0,
                })
                .show(ui, |ui| {
                    ui.set_width((avail_w - side_pad * 2.0).max(240.0));

                    if blocks.is_empty() && state.editing_block.is_none() {
                        let resp = ui.add_sized(
                            [ui.available_width(), 120.0],
                            egui::Label::new(
                                RichText::new("空白的 Markdown 文件 — 點擊此處開始撰寫…")
                                    .size(base_font_size)
                                    .color(c.fg_dim2),
                            )
                            .sense(Sense::click()),
                        );
                        if allow_inline_edit && resp.clicked() {
                            start_edit_range = Some((0, 0));
                        }
                    }

                    for block in &blocks {
                        let is_editing_this = allow_inline_edit
                            && state.editing_block == Some((block.start_line, block.end_line));

                        // 若此區塊正在原位編輯：顯示帶語法高亮的 Markdown 編輯框
                        if is_editing_this {
                            let edit_frame = egui::Frame::none()
                                .fill(c.current_line)
                                .stroke(Stroke::new(1.5_f32, c.accent))
                                .rounding(Rounding::same(6.0))
                                .inner_margin(Margin::symmetric(12.0, 8.0));

                            edit_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "✎ 原位編輯 Markdown (第 {}~{} 行)",
                                            block.start_line + 1,
                                            block.end_line + 1
                                        ))
                                        .size(11.0)
                                        .color(c.accent),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .small_button(RichText::new("✓ 完成 (Esc)").color(c.accent))
                                            .clicked()
                                        {
                                            commit_now = true;
                                        }
                                    });
                                });
                                ui.add_space(4.0);

                                let pal_c = pal.clone();
                                let font_c = FontId::monospace(base_font_size);
                                let mut layouter = move |ui: &egui::Ui, s: &str, wrap_w: f32| {
                                    let mut job = highlight(s, Lang::Markdown, &pal_c, font_c.clone(), "");
                                    job.wrap.max_width = wrap_w;
                                    ui.fonts(|f| f.layout_job(job))
                                };

                                let rows = state.editing_text.lines().count().max(2);
                                let te_resp = ui.add(
                                    egui::TextEdit::multiline(&mut state.editing_text)
                                        .font(FontId::monospace(base_font_size))
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(rows)
                                        .layouter(&mut layouter),
                                );
                                if state.request_edit_focus {
                                    te_resp.request_focus();
                                    state.request_edit_focus = false;
                                }
                                let ctrl_enter = ui.input(|i| i.modifiers.command && i.key_pressed(Key::Enter));
                                if ctrl_enter || (te_resp.lost_focus() && !ui.input(|i| i.pointer.any_down())) {
                                    commit_now = true;
                                }
                            });
                            ui.add_space(10.0);
                            continue;
                        }

                        // 一般渲染狀態
                        let block_resp = render_single_block(
                            ui,
                            block,
                            base_font_size,
                            c,
                            pal,
                            find_query,
                            project_root,
                            buf.file_path.as_deref(),
                            &mut state.image_cache,
                            &mut toggle_task_line,
                            &mut action,
                        );

                        // 大綱跳轉捲動定位
                        if let Some(target_line) = scroll_target {
                            if target_line >= block.start_line && target_line <= block.end_line {
                                ui.scroll_to_rect(block_resp.rect, Some(Align::TOP));
                            }
                        }

                        // Typora 點擊任意段落進入原位編輯
                        if allow_inline_edit && block_resp.clicked() && toggle_task_line.is_none() {
                            start_edit_range = Some((block.start_line, block.end_line));
                        }
                    }
                });
        });

    if commit_now {
        state.commit_active_edit(buf);
    }
    if let Some(line_idx) = toggle_task_line {
        toggle_markdown_checkbox(buf, line_idx);
    } else if let Some((s, e)) = start_edit_range {
        state.commit_active_edit(buf);
        state.begin_block_edit(buf, s, e);
    }

    action
}

fn toggle_markdown_checkbox(buf: &mut EditorBuffer, line_idx: usize) {
    if line_idx >= buf.lines.len() {
        return;
    }
    let mut lines = buf.lines.clone();
    let line = &lines[line_idx];
    let updated = if line.contains("[ ]") {
        line.replacen("[ ]", "[x]", 1)
    } else if line.contains("[]") {
        line.replacen("[]", "[x]", 1)
    } else if line.contains("[x]") {
        line.replacen("[x]", "[ ]", 1)
    } else if line.contains("[X]") {
        line.replacen("[X]", "[ ]", 1)
    } else {
        line.clone()
    };
    lines[line_idx] = updated;
    buf.replace_all_text(&lines.join("\n"));
}

#[allow(clippy::too_many_arguments)]
fn render_single_block(
    ui: &mut egui::Ui,
    block: &MdBlock,
    base_size: f32,
    c: &Colors,
    pal: &Palette,
    find_query: &str,
    project_root: &Path,
    file_path: Option<&str>,
    image_cache: &mut HashMap<String, Option<egui::TextureHandle>>,
    toggle_task_line: &mut Option<usize>,
    action: &mut MdAction,
) -> egui::Response {
    let inner = ui.scope(|ui| {
        match &block.kind {
            MdBlockKind::Heading { level, text } => {
                let top_space = match level {
                    1 => 14.0,
                    2 => 12.0,
                    3 => 8.0,
                    _ => 6.0,
                };
                ui.add_space(top_space);
                let scale = match level {
                    1 => 1.95,
                    2 => 1.58,
                    3 => 1.32,
                    4 => 1.15,
                    5 => 1.05,
                    _ => 0.96,
                };
                let col = match level {
                    1 => c.accent,
                    2 => c.fg,
                    3 => hex(pal.syn_func),
                    _ => c.fg_dim,
                };
                ui.horizontal(|ui| {
                    render_inline_text(
                        ui,
                        text,
                        base_size * scale,
                        col,
                        true,
                        false,
                        c,
                        pal,
                        find_query,
                        action,
                    );
                    ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                        ui.label(
                            RichText::new(format!("H{}", level))
                                .font(FontId::new(9.5, FontFamily::Name(ORBITRON.into())))
                                .color(with_alpha(c.faint, 140)),
                        );
                    });
                });
                if *level == 1 {
                    ui.add_space(4.0);
                    let r = ui.max_rect();
                    ui.painter()
                        .hline(r.x_range(), ui.cursor().top(), Stroke::new(1.5_f32, c.glow));
                    ui.add_space(8.0);
                } else if *level == 2 {
                    ui.add_space(3.0);
                    let r = ui.max_rect();
                    ui.painter()
                        .hline(r.x_range(), ui.cursor().top(), Stroke::new(1.0_f32, c.border));
                    ui.add_space(6.0);
                } else {
                    ui.add_space(4.0);
                }
            }

            MdBlockKind::Paragraph { text } => {
                render_inline_text(ui, text, base_size, c.fg, false, false, c, pal, find_query, action);
                ui.add_space(8.0);
            }

            MdBlockKind::HorizontalRule => {
                ui.add_space(10.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), Sense::hover());
                ui.painter()
                    .hline(rect.x_range(), rect.center().y, Stroke::new(1.5_f32, c.glow));
                ui.painter().circle_filled(rect.center(), 3.0, c.accent);
                ui.add_space(10.0);
            }

            MdBlockKind::ListItem {
                indent,
                marker,
                content,
                line_idx,
            } => {
                ui.horizontal_top(|ui| {
                    ui.add_space((*indent as f32) * 20.0);
                    match marker {
                        ListMarker::Bullet => {
                            let bullet = match indent % 3 {
                                0 => "•",
                                1 => "◦",
                                _ => "▪",
                            };
                            ui.label(RichText::new(format!("{} ", bullet)).size(base_size).color(c.accent).strong());
                            render_inline_text(
                                ui, content, base_size, c.fg, false, false, c, pal, find_query, action,
                            );
                        }
                        ListMarker::Ordered(num) => {
                            ui.label(RichText::new(format!("{} ", num)).size(base_size).color(c.accent).strong());
                            render_inline_text(
                                ui, content, base_size, c.fg, false, false, c, pal, find_query, action,
                            );
                        }
                        ListMarker::Task(checked) => {
                            let mut val = *checked;
                            if ui
                                .checkbox(&mut val, "")
                                .on_hover_text("點擊切換待辦完成狀態")
                                .changed()
                            {
                                *toggle_task_line = Some(*line_idx);
                            }
                            let text_col = if *checked { c.fg_dim2 } else { c.fg };
                            render_inline_text(
                                ui,
                                content,
                                base_size,
                                text_col,
                                false,
                                *checked,
                                c,
                                pal,
                                find_query,
                                action,
                            );
                        }
                    }
                });
                ui.add_space(4.0);
            }

            MdBlockKind::BlockQuote { lines } => {
                let frame = egui::Frame::none()
                    .fill(with_alpha(c.panel, 180))
                    .rounding(Rounding::same(5.0))
                    .inner_margin(Margin {
                        left: 14.0,
                        right: 12.0,
                        top: 8.0,
                        bottom: 8.0,
                    });
                let resp = frame.show(ui, |ui| {
                    for l in lines {
                        render_inline_text(ui, l, base_size, c.fg_dim, false, false, c, pal, find_query, action);
                    }
                });
                let r = resp.response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_size(r.left_top(), Vec2::new(3.5, r.height())),
                    2.0,
                    c.accent,
                );
                ui.add_space(8.0);
            }

            MdBlockKind::Alert { kind, title, lines } => {
                let (badge, col) = kind.badge_and_color(c, pal);
                let frame = egui::Frame::none()
                    .fill(with_alpha(col, if c.light { 24 } else { 20 }))
                    .stroke(Stroke::new(1.0_f32, with_alpha(col, 80)))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin {
                        left: 16.0,
                        right: 12.0,
                        top: 10.0,
                        bottom: 10.0,
                    });
                let resp = frame.show(ui, |ui| {
                    let heading = if title.is_empty() {
                        badge.to_string()
                    } else {
                        format!("{} — {}", badge, title)
                    };
                    ui.label(RichText::new(heading).size(base_size * 0.96).color(col).strong());
                    ui.add_space(4.0);
                    for l in lines {
                        render_inline_text(ui, l, base_size, c.fg, false, false, c, pal, find_query, action);
                    }
                });
                let r = resp.response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_size(r.left_top(), Vec2::new(4.0, r.height())),
                    2.0,
                    col,
                );
                ui.add_space(10.0);
            }

            MdBlockKind::CodeBlock { lang, code } => {
                let frame = egui::Frame::none()
                    .fill(c.panel)
                    .stroke(Stroke::new(1.0_f32, c.border))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(12.0, 8.0));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let tag = if lang.is_empty() { "CODE" } else { lang.as_str() };
                        ui.label(
                            RichText::new(tag.to_uppercase())
                                .font(FontId::new(10.0, FontFamily::Name(ORBITRON.into())))
                                .color(c.accent),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("📋 複製").on_hover_text("複製程式碼到剪貼簿").clicked() {
                                ui.output_mut(|o| o.copied_text = code.clone());
                            }
                        });
                    });
                    ui.separator();
                    let hl_lang = Lang::from_path(&format!("snippet.{}", lang));
                    let job = highlight(
                        code,
                        hl_lang,
                        pal,
                        FontId::monospace((base_size - 0.5).max(11.0)),
                        find_query,
                    );
                    let galley = ui.fonts(|f| f.layout_job(job));
                    egui::ScrollArea::horizontal()
                        .id_source(("md_code_block", block.start_line))
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(galley).wrap(false));
                        });
                });
                ui.add_space(10.0);
            }

            MdBlockKind::Table { headers, aligns, rows } => {
                let cols = headers.len().max(1);
                let col_w = ((ui.available_width() - 24.0) / cols as f32).max(80.0);

                egui::Frame::none()
                    .fill(c.panel)
                    .stroke(Stroke::new(1.0_f32, c.border))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(6.0))
                    .show(ui, |ui| {
                        egui::Grid::new(("md_table", block.start_line))
                            .striped(true)
                            .min_col_width(col_w)
                            .spacing(egui::vec2(12.0, 6.0))
                            .show(ui, |ui| {
                                for (ci, h) in headers.iter().enumerate() {
                                    let align = aligns.get(ci).copied().unwrap_or(Align::Min);
                                    ui.with_layout(Layout::top_down(align), |ui| {
                                        render_inline_text(
                                            ui, h, base_size, c.accent, true, false, c, pal, find_query, action,
                                        );
                                    });
                                }
                                ui.end_row();

                                for row in rows {
                                    for ci in 0..cols {
                                        let cell = row.get(ci).map(|s| s.as_str()).unwrap_or("");
                                        let align = aligns.get(ci).copied().unwrap_or(Align::Min);
                                        ui.with_layout(Layout::top_down(align), |ui| {
                                            render_inline_text(
                                                ui, cell, base_size, c.fg, false, false, c, pal, find_query, action,
                                            );
                                        });
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(10.0);
            }

            MdBlockKind::Image { alt, url } => {
                let resolved = resolve_image_path(url, file_path, project_root);
                let key = resolved.to_string_lossy().to_string();
                let tex = image_cache
                    .entry(key)
                    .or_insert_with(|| {
                        let bytes = std::fs::read(&resolved).ok()?;
                        let img = image::load_from_memory(&bytes).ok()?;
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                        Some(ui.ctx().load_texture(url, ci, egui::TextureOptions::LINEAR))
                    })
                    .clone();

                egui::Frame::none()
                    .fill(c.panel)
                    .stroke(Stroke::new(1.0_f32, c.border))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(t) = tex {
                                let max_w = ui.available_width().min(680.0);
                                let sz = t.size_vec2();
                                let scale = if sz.x > max_w { max_w / sz.x } else { 1.0 };
                                ui.add(egui::Image::new(&t).fit_to_exact_size(sz * scale));
                            } else {
                                ui.label(RichText::new(format!("🖼 圖片：{} ({})", alt, url)).color(c.accent));
                            }
                            if !alt.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new(alt).size(11.5).italics().color(c.fg_dim2));
                            }
                        });
                    });
                ui.add_space(10.0);
            }
        }
    });

    // 為整個區塊建立點擊感應與懸停淡淡提示框
    let rect = inner.response.rect.expand2(egui::vec2(6.0, 2.0));
    let id = ui.id().with(("typora_block", block.start_line, block.end_line));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0_f32, with_alpha(c.accent, 65)));
    }
    resp
}

fn resolve_image_path(url: &str, file_path: Option<&str>, project_root: &Path) -> PathBuf {
    let p = Path::new(url);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Some(fp) = file_path {
        if let Some(parent) = Path::new(fp).parent() {
            let candidate = parent.join(p);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    project_root.join(p)
}
