//! AlexCode 主應用程式介面與事件迴圈 (Egui / Eframe)
//!
//! 介面配置對應原版 Qt `MainWindow`：
//! - 選單列（龍形 Logo + Orbitron 字標）
//! - 主工具列（霓虹線條圖示：新增 / 開啟 / 儲存 | 復原 / 重做 | 搜尋）+ ⚡ FILTER 篩選工具列
//! - 平面分頁列（強調色底線指示）
//! - 程式碼編輯區（行號 gutter、目前行高亮、語法高亮、搜尋命中底色）+ Neon HUD 角標
//! - 底部 Dock（終端機 / Markdown 關聯圖，左側強調色識別條）
//! - 狀態列（漸層 + 光暈上邊框）

use super::highlight::{find_matches, highlight, Lang};
use super::icons::{paint_icon, tool_button, IconKind};
use super::markdown_view::{
    render_markdown_toolbar, render_outline_panel, render_typora_document, MarkdownViewState, MdAction, MdViewMode,
};
use super::style::{self, hgradient, paint_hud_corners, vgradient, with_alpha, Colors, ORBITRON};
use crate::core::{available_shells, open_terminal, reveal_in_file_manager, EditorConfig, ShellKind};
use crate::editor::{detect_and_decode, EditorBuffer, FileEncoding, Position, Selection};
use crate::filter::{FilterEngine, FilterLogic};
use crate::markdown::{compute_graph_layout, MarkdownLinkIndex};
use crate::plugins::PluginManager;
use crate::search::{command_match_score, search_in_files, FileMatch, SearchStats};
use crate::snippets::SnippetsController;
use crate::terminal::VtParser;
use crate::theme::ThemeManager;
use crossbeam_channel::{unbounded, Receiver};
use eframe::egui::{
    self, text::CCursor, text::CCursorRange, Align, Align2, Color32, FontFamily, FontId, Key, KeyboardShortcut,
    Layout, Modifiers, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const WELCOME: &str = r##"// ========================================================
//  AlexCode — Neon Edition (v5.3 Rust Port)
//  A fast Notepad++ style code editor rewritten in Rust
// ========================================================

fn main() {
    println!("Welcome to AlexCode in Rust!");
    // Shortcuts:
    // Ctrl+N: New File | Ctrl+O: Open File | Ctrl+S: Save
    // Ctrl+Shift+P: Command Palette | Ctrl+Shift+F: Find in Files
    // Ctrl+`: Terminal | Ctrl+F: Find | Ctrl+G: Go to Line
    let neon = ["#00e5ff", "#ff2d95", "#ffd166"];
    for (i, color) in neon.iter().enumerate() {
        println!("accent {} = {}", i + 1, color);
    }
}
"##;

const WELCOME_MD: &str = r##"# AlexCode — Typora 風格 Markdown 閱讀與即時編輯

歡迎使用 **AlexCode Neon Edition** 的 **Typora 閱讀模式**！在這裡你可以像閱讀精美排版文章一樣直觀瀏覽 `.md` 檔案，同時**點擊任意段落即可直接原位修改 Markdown 原始碼**（按 `Esc` 立即回到排版預覽）。

> [!TIP] 快速操作秘笈
> - **點擊任一段落**：立即展開該區塊的 Markdown 原始碼進行原位編輯，按 `Esc` 或點擊空白處完成編輯。
> - **模式切換 (`Ctrl+Shift+M`)**：隨時在 **📖 Typora 閱讀**、**◫ 雙欄預覽** 與 **‹/› 原始碼** 三種模式間切換。
> - **左側大綱 (Outline)**：自動擷取 `# H1 ~ ###### H6` 標題階層，點擊即可快速跳轉章節。

---

## 1. 互動式待辦清單 (Task List)

不用進入原始碼，直接點擊下方勾選框即可切換完成狀態：

- [x] 移植原版 C++ `NeonIcons` 向量圖示與黃金龍形 Logo
- [x] 支援 **Typora 風格** 所見即所得 Markdown 閱讀與點擊原位編輯
- [ ] 試著點擊這個勾選框，或點擊這行文字直接修改內容！
- [ ] 支援 Obsidian 雙中括號筆記連結，例如點擊 [[welcome]]

## 2. 行內語法與螢光筆標記

支援 **粗體 (Bold)**、*斜體 (Italic)*、***粗斜體***、~~刪除線~~、`行內程式碼 (inline code)`、==Typora 螢光筆重點標記==，以及外部連結 [AlexCode GitHub 專案](https://github.com/alex721chiang/alexcode_editor)。

## 3. 語法高亮程式碼區塊 (Fenced Code Blocks)

```rust
// 支援語法高亮與右上角「一鍵複製」按鈕
fn render_markdown(doc: &str) -> bool {
    let themes = ["Neon Grid", "Neon HUD", "Paper Light", "Matrix"];
    println!("Active themes: {:?}", themes);
    true
}
```

## 4. 結構化資料表格 (Tables)

| 功能模組 | 快捷鍵 | 狀態 |
| :--- | :---: | ---: |
| Typora 閱讀 / 雙欄 / 原始碼切換 | `Ctrl+Shift+M` | 已啟用 |
| 新增 Markdown 筆記 | `Ctrl+Alt+N` | 已啟用 |
| 命令面板 (Command Palette) | `Ctrl+Shift+P` | 已啟用 |
| 全專案搜尋 (Find in Files) | `Ctrl+Shift+F` | 已啟用 |

> [!NOTE] 支援多種主題配色
> 無論是深邃科技的 **Neon Grid / Neon HUD**、護眼的 **Paper Light** 或駭客風 **Matrix**，Markdown 排版都會自動適應主題色票！
"##;

/// 可由選單、工具列、快捷鍵與命令面板共用的動作
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    NewFile,
    NewMarkdown,
    Open,
    Save,
    SaveAs,
    CloseTab,
    RevealInFolder,
    OpenShell(ShellKind),
    CopyPath,
    Undo,
    Redo,
    DuplicateLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,
    ToggleComment,
    ToggleBookmark,
    Find,
    GotoLine,
    FindInFiles,
    CommandPalette,
    ToggleTerminal,
    ToggleGraph,
    ToggleFilterBar,
    CycleMarkdownMode,
    ToggleMarkdownOutline,
    Theme(&'static str),
    SortAsc,
    SortDesc,
    Dedup,
    HalfWidth,
    ZoomIn,
    ZoomOut,
    About,
    Plugin(String),
}

pub struct AlexCodeApp {
    pub theme_mgr: ThemeManager,
    pub config: EditorConfig,
    pub tabs: Vec<EditorBuffer>,
    pub active_tab_idx: usize,
    pub filter_engine: FilterEngine,
    pub filter_query: String,
    pub filter_logic: FilterLogic,
    pub filter_fuzzy: bool,
    /// Log 篩選工具列是否顯示（原版常駐於主工具列右側）
    pub filter_active: bool,
    pub terminal: VtParser,
    pub term_input: String,
    pub show_terminal: bool,
    pub show_graph: bool,
    pub show_command_palette: bool,
    pub show_find_in_files: bool,
    pub show_diff_dialog: bool,
    pub command_query: String,
    pub find_query: String,
    pub find_results: Vec<FileMatch>,
    pub plugin_mgr: PluginManager,
    pub snippets_ctrl: SnippetsController,
    pub status_message: String,
    pub link_index: MarkdownLinkIndex,
    pub md_view: MarkdownViewState,

    // ---- GUI 狀態 ----
    logo: Option<egui::TextureHandle>,
    applied_theme: String,
    font_size: f32,
    show_find_bar: bool,
    find_bar_text: String,
    find_bar_focus: bool,
    find_cursor: usize,
    show_goto: bool,
    goto_text: String,
    show_about: bool,
    palette_focus: bool,
    pending_select: Option<(usize, usize)>,
    scroll_to_cursor: bool,
    project_root: PathBuf,
    term_rx: Option<Receiver<Vec<u8>>>,
    term_cwd: PathBuf,

    /// 等待使用者決定「儲存 / 不儲存 / 取消」的關閉要求
    pending_close: Option<PendingClose>,
    /// 已確認可以結束程式（不再攔截視窗關閉）
    allow_exit: bool,
    /// 上一幀程式碼編輯區是否擁有鍵盤焦點（決定 Ctrl+Z 是否由文件的復原紀錄處理）
    editor_focused: bool,
    /// 目前分頁以 `\n` 連接的全文快取：鍵為 (分頁 id, 內容版本)
    text_cache: ((u64, u64), String),
    /// 編輯區排版結果（含語法高亮）快取：鍵為全文 / 語言 / 主題 / 字型 / 搜尋字串的雜湊。
    /// 一併記住排版當時的字型圖集：egui 重建圖集（縮放改變、圖集快滿）後舊排版的字形座標即失效。
    highlight_cache: Option<(u64, Arc<egui::Galley>, FontAtlas)>,
    /// 尋找列命中位置快取
    find_cache: ((u64, u64, String), Vec<(usize, usize)>),
    /// Log 篩選命中的行號快取
    filter_cache: (Option<(u64, u64, String, bool, bool)>, Vec<usize>),
    /// 背景執行中的全專案搜尋
    find_job: Option<FindJob>,
    last_title: String,
    pending_copy: Option<String>,
}

type FontAtlas = Arc<egui::mutex::Mutex<egui::epaint::TextureAtlas>>;

struct PendingClose {
    /// 要關閉的分頁 id
    ids: Vec<u64>,
    /// 關閉後結束程式
    exit: bool,
}

struct FindJob {
    rx: Receiver<(Vec<FileMatch>, SearchStats)>,
    cancel: Arc<AtomicBool>,
}

/// 分頁右鍵選單的動作
enum TabAction {
    Reveal(usize),
    Shell(usize, ShellKind),
    CopyPath(usize),
    CopyName(usize),
    Close(usize),
    CloseOthers(usize),
    CloseRight(usize),
    CloseAll,
}

/// 超過此大小的文件不做語法高亮（仍可編輯與搜尋）
const HIGHLIGHT_MAX_BYTES: usize = 2 * 1024 * 1024;

impl Default for AlexCodeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AlexCodeApp {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            theme_mgr: ThemeManager::new(),
            config: EditorConfig::default(),
            tabs: Vec::new(),
            active_tab_idx: 0,
            filter_engine: FilterEngine::new(),
            filter_query: String::new(),
            filter_logic: FilterLogic::Or,
            filter_fuzzy: false,
            filter_active: true,
            terminal: VtParser::new(200, 160),
            term_input: String::new(),
            show_terminal: false,
            show_graph: false,
            show_command_palette: false,
            show_find_in_files: false,
            show_diff_dialog: false,
            command_query: String::new(),
            find_query: String::new(),
            find_results: Vec::new(),
            plugin_mgr: PluginManager::new(),
            snippets_ctrl: SnippetsController::new(),
            status_message: "Ready".to_string(),
            link_index: MarkdownLinkIndex::new(),
            md_view: MarkdownViewState::default(),
            logo: None,
            applied_theme: String::new(),
            font_size: 14.0,
            show_find_bar: false,
            find_bar_text: String::new(),
            find_bar_focus: false,
            find_cursor: 0,
            show_goto: false,
            goto_text: String::new(),
            show_about: false,
            palette_focus: bool::default(),
            pending_select: None,
            scroll_to_cursor: false,
            project_root: cwd.clone(),
            term_rx: None,
            term_cwd: cwd,
            pending_close: None,
            allow_exit: false,
            editor_focused: false,
            text_cache: ((0, 0), String::new()),
            highlight_cache: None,
            find_cache: ((0, 0, String::new()), Vec::new()),
            filter_cache: (None, Vec::new()),
            find_job: None,
            last_title: String::new(),
            pending_copy: None,
        };

        // 開啟歡迎頁面與 Typora Markdown 導覽頁
        let mut buf = EditorBuffer::from_text(WELCOME);
        buf.file_path = Some("welcome.rs".to_string());
        app.tabs.push(buf);

        let mut md_buf = EditorBuffer::from_text(WELCOME_MD);
        md_buf.file_path = Some("guide.md".to_string());
        app.tabs.push(md_buf);
        app.active_tab_idx = 1; // 預設展示 Typora Markdown 閱讀分頁
        app
    }

    /// 由 eframe 建立：設定字型（含中文 fallback 與 Orbitron）後回傳 App
    pub fn with_context(cc: &eframe::CreationContext<'_>) -> Self {
        style::setup_fonts(&cc.egui_ctx);
        let mut app = Self::new();
        // 命令列參數：直接開啟檔案
        for arg in std::env::args().skip(1) {
            let p = PathBuf::from(&arg);
            if p.is_file() {
                app.open_path(&p);
            }
        }
        app
    }

    pub fn active_buffer(&self) -> Option<&EditorBuffer> {
        self.tabs.get(self.active_tab_idx)
    }

    pub fn active_buffer_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.tabs.get_mut(self.active_tab_idx)
    }

    pub fn commit_md_edit(&mut self) {
        if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
            self.md_view.commit_active_edit(buf);
        }
    }

    pub fn new_tab(&mut self) {
        self.commit_md_edit();
        let mut buf = EditorBuffer::new();
        buf.file_path = Some(format!("untitled_{}.txt", self.tabs.len() + 1));
        self.tabs.push(buf);
        self.active_tab_idx = self.tabs.len() - 1;
    }

    pub fn new_markdown_tab(&mut self) {
        self.commit_md_edit();
        let idx = self.tabs.len() + 1;
        let mut buf = EditorBuffer::from_text(&format!("# 新筆記 {}\n\n在此點擊段落即可開始撰寫 Markdown…\n", idx));
        buf.file_path = Some(format!("note_{}.md", idx));
        self.tabs.push(buf);
        self.active_tab_idx = self.tabs.len() - 1;
        self.md_view.mode = MdViewMode::Typora;
    }

    /// 關閉分頁；有未儲存的變更時先詢問
    pub fn close_tab(&mut self, idx: usize) {
        if let Some(id) = self.tabs.get(idx).map(|b| b.id) {
            self.request_close(vec![id], false);
        }
    }

    /// 要求關閉一批分頁（`exit` 為 true 時關閉後結束程式）。
    /// 其中有未儲存的變更就先跳出「儲存 / 不儲存 / 取消」，否則直接關閉。
    fn request_close(&mut self, ids: Vec<u64>, exit: bool) {
        self.commit_md_edit();
        if self.tabs.iter().any(|b| b.modified && ids.contains(&b.id)) {
            self.pending_close = Some(PendingClose { ids, exit });
        } else {
            self.finish_close(&ids, exit);
        }
    }

    fn finish_close(&mut self, ids: &[u64], exit: bool) {
        if exit {
            self.allow_exit = true; // update() 會送出視窗關閉命令
            return;
        }
        let active_id = self.tabs.get(self.active_tab_idx).map(|b| b.id);
        let old_idx = self.active_tab_idx;
        self.tabs.retain(|b| !ids.contains(&b.id));
        self.active_tab_idx = active_id
            .and_then(|id| self.tabs.iter().position(|b| b.id == id))
            .unwrap_or_else(|| old_idx.min(self.tabs.len().saturating_sub(1)));
        if self.tabs.is_empty() {
            self.new_tab();
        }
    }

    /// 分頁對應的資料夾：已存檔的檔案取其所在資料夾，否則用目前專案資料夾
    fn tab_dir(&self, idx: usize) -> PathBuf {
        self.tabs
            .get(idx)
            .and_then(|b| b.file_path.as_deref())
            .map(Path::new)
            .filter(|p| p.is_absolute())
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.project_root.clone())
    }

    fn reveal_tab(&mut self, idx: usize) {
        let file = self
            .tabs
            .get(idx)
            .and_then(|b| b.file_path.clone())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && p.exists());
        let target = file.unwrap_or_else(|| self.tab_dir(idx));
        self.status_message = match reveal_in_file_manager(&target) {
            Ok(_) => format!("已在檔案總管顯示：{}", target.display()),
            Err(e) => format!("無法開啟檔案總管：{}", e),
        };
    }

    fn open_shell_for_tab(&mut self, idx: usize, kind: ShellKind) {
        let dir = self.tab_dir(idx);
        self.status_message = match open_terminal(&dir, kind) {
            Ok(_) => format!("已在 {} 開啟終端機", dir.display()),
            Err(e) => format!("無法開啟終端機：{}", e),
        };
    }

    fn run_tab_action(&mut self, ctx: &egui::Context, action: TabAction) {
        let ids_where = |tabs: &[EditorBuffer], keep: &dyn Fn(usize) -> bool| -> Vec<u64> {
            tabs.iter().enumerate().filter(|(i, _)| keep(*i)).map(|(_, b)| b.id).collect()
        };
        match action {
            TabAction::Reveal(i) => self.reveal_tab(i),
            TabAction::Shell(i, kind) => self.open_shell_for_tab(i, kind),
            TabAction::CopyPath(i) => {
                if let Some(p) = self.tabs.get(i).and_then(|b| b.file_path.clone()) {
                    ctx.copy_text(p.clone());
                    self.status_message = format!("已複製路徑：{}", p);
                }
            }
            TabAction::CopyName(i) => {
                if let Some(b) = self.tabs.get(i) {
                    let name = Self::tab_title(b);
                    ctx.copy_text(name.clone());
                    self.status_message = format!("已複製檔名：{}", name);
                }
            }
            TabAction::Close(i) => self.close_tab(i),
            TabAction::CloseOthers(i) => {
                let ids = ids_where(&self.tabs, &|k| k != i);
                self.request_close(ids, false);
            }
            TabAction::CloseRight(i) => {
                let ids = ids_where(&self.tabs, &|k| k > i);
                self.request_close(ids, false);
            }
            TabAction::CloseAll => {
                let ids = ids_where(&self.tabs, &|_| true);
                self.request_close(ids, false);
            }
        }
    }

    /// 確保 `text_cache` 是目前分頁的最新全文（內容沒變就不重新串接）
    fn sync_text_cache(&mut self) {
        if let Some(b) = self.tabs.get(self.active_tab_idx) {
            let key = (b.id, b.revision());
            if self.text_cache.0 != key {
                self.text_cache = (key, b.edit_text());
            }
        }
    }

    /// 確保 `filter_cache` 是目前分頁、目前篩選條件下命中的行號
    fn sync_filter_hits(&mut self) {
        let Some(b) = self.tabs.get(self.active_tab_idx) else { return };
        let key = (b.id, b.revision(), self.filter_query.clone(), self.filter_logic == FilterLogic::Or, self.filter_fuzzy);
        if self.filter_cache.0.as_ref() != Some(&key) {
            let hits = b
                .lines
                .iter()
                .enumerate()
                .filter(|(_, l)| self.filter_engine.match_compiled(l))
                .map(|(i, _)| i)
                .collect();
            self.filter_cache = (Some(key), hits);
        }
    }

    fn tab_title(buf: &EditorBuffer) -> String {
        let p = buf.file_path.clone().unwrap_or_else(|| "Untitled".to_string());
        Path::new(&p)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(p)
    }

    fn tab_lang(&self, idx: usize) -> Lang {
        self.tabs
            .get(idx)
            .and_then(|b| b.file_path.as_deref())
            .map(Lang::from_path)
            .unwrap_or(Lang::Plain)
    }

    pub fn open_path(&mut self, path: &Path) {
        self.commit_md_edit();
        let full = path.to_string_lossy().to_string();
        if let Some(i) = self.tabs.iter().position(|b| b.file_path.as_deref() == Some(full.as_str())) {
            self.active_tab_idx = i;
            return;
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                // 偵測編碼（UTF-8 / BOM / Big5 / GBK / Shift_JIS …）並記在分頁上，存檔時以同一編碼寫回
                let (text, encoding) = detect_and_decode(&bytes);
                let mut buf = EditorBuffer::from_text(&text);
                buf.encoding = encoding;
                buf.file_path = Some(full.clone());
                let is_md = Lang::from_path(&full) == Lang::Markdown;
                // 若目前只有未修改的歡迎頁 / 空白頁，直接取代
                let replace = self.tabs.len() == 1
                    && !self.tabs[0].modified
                    && self.tabs[0]
                        .file_path
                        .as_deref()
                        .map_or(true, |p| !Path::new(p).is_absolute());
                if replace {
                    self.tabs[0] = buf;
                    self.active_tab_idx = 0;
                } else {
                    self.tabs.push(buf);
                    self.active_tab_idx = self.tabs.len() - 1;
                }
                if is_md {
                    self.md_view.mode = MdViewMode::Typora;
                }
                if let Some(parent) = path.parent() {
                    self.project_root = parent.to_path_buf();
                }
                self.status_message = format!("已開啟 {}", full);
            }
            Err(e) => self.status_message = format!("開啟失敗：{} ({})", full, e),
        }
    }

    fn open_md_link(&mut self, ctx: &egui::Context, target: &str, wiki: bool) {
        let t = target.trim();
        if t.starts_with("http://") || t.starts_with("https://") || t.starts_with("mailto:") {
            ctx.open_url(egui::OpenUrl::new_tab(t));
            self.status_message = format!("已在瀏覽器開啟：{}", t);
            return;
        }
        // 1. 先檢查已開啟的分頁是否符合檔名或 stem
        let key = crate::markdown::normalize_key(t);
        if let Some(idx) = self.tabs.iter().position(|b| {
            b.file_path
                .as_deref()
                .map_or(false, |fp| crate::markdown::normalize_key(fp) == key)
        }) {
            self.commit_md_edit();
            self.active_tab_idx = idx;
            self.status_message = format!("已切換至分頁：{}", t);
            return;
        }
        // 2. 在目前檔案目錄或專案根目錄尋找對應檔案
        let base_dir = self
            .active_buffer()
            .and_then(|b| b.file_path.as_deref())
            .and_then(|fp| Path::new(fp).parent())
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.project_root.clone());

        let candidates = if wiki && !t.to_lowercase().ends_with(".md") {
            vec![base_dir.join(format!("{}.md", t)), self.project_root.join(format!("{}.md", t)), base_dir.join(t)]
        } else {
            vec![base_dir.join(t), self.project_root.join(t)]
        };
        for cand in candidates {
            if cand.is_file() {
                self.open_path(&cand);
                return;
            }
        }
        // 3. 若為 WikiLink 且尚無該筆記，自動建立新 Markdown 筆記分頁
        if wiki {
            self.commit_md_edit();
            let filename = if t.to_lowercase().ends_with(".md") {
                t.to_string()
            } else {
                format!("{}.md", t)
            };
            let mut buf = EditorBuffer::from_text(&format!("# {}\n\n", t));
            buf.file_path = Some(filename.clone());
            self.tabs.push(buf);
            self.active_tab_idx = self.tabs.len() - 1;
            self.md_view.mode = MdViewMode::Typora;
            self.status_message = format!("已建立新筆記分頁：{}", filename);
        } else {
            self.status_message = format!("找不到連結目標：{}", t);
        }
    }

    fn save_active(&mut self, force_dialog: bool) {
        self.commit_md_edit();
        self.save_tab(self.active_tab_idx, force_dialog);
    }

    /// 儲存指定分頁；回傳是否真的寫入成功（使用者取消對話框或寫入失敗都回傳 false）
    fn save_tab(&mut self, idx: usize, force_dialog: bool) -> bool {
        let Some(buf) = self.tabs.get(idx) else { return false };
        let current = buf.file_path.clone().unwrap_or_default();
        let path = if !force_dialog && Path::new(&current).is_absolute() {
            Some(PathBuf::from(&current))
        } else {
            rfd::FileDialog::new()
                .set_file_name(Self::tab_title(buf))
                .set_directory(&self.project_root)
                .save_file()
        };
        let Some(path) = path else { return false };
        let encoding = self.tabs[idx].encoding;
        // 文字含有此編碼無法表示的字元時拒絕寫入，避免靜默換成替代字元
        let bytes = match encoding.encode(&self.tabs[idx].text()) {
            Ok(b) => b,
            Err(ch) => {
                self.status_message = format!(
                    "未儲存：字元「{}」無法以 {} 表示。請點狀態列的編碼改成 UTF-8 後再儲存",
                    ch,
                    encoding.label()
                );
                return false;
            }
        };
        match std::fs::write(&path, bytes) {
            Ok(_) => {
                let b = &mut self.tabs[idx];
                b.file_path = Some(path.to_string_lossy().to_string());
                b.mark_saved();
                self.status_message = format!("已儲存 {}（{}）", path.display(), encoding.label());
                true
            }
            Err(e) => {
                self.status_message = format!("儲存失敗：{}", e);
                false
            }
        }
    }

    /// 以指定編碼重新讀取磁碟上的檔案（自動偵測猜錯時使用）
    fn reload_with_encoding(&mut self, idx: usize, encoding: FileEncoding) {
        let Some(buf) = self.tabs.get(idx) else { return };
        let Some(path) = buf.file_path.clone().filter(|p| Path::new(p).is_absolute()) else {
            self.status_message = "此分頁尚未存檔，無法重新讀取".to_string();
            return;
        };
        if buf.modified {
            self.status_message = "有未儲存的變更，請先儲存或復原後再以其他編碼重新讀取".to_string();
            return;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                let (text, bad) = encoding.decode(&bytes);
                let mut nb = EditorBuffer::from_text(&text);
                nb.encoding = encoding;
                nb.file_path = Some(path);
                self.tabs[idx] = nb;
                self.status_message = if bad {
                    format!("已用 {} 重新讀取，但有無法解碼的位元組（顯示為 �）；這個編碼可能不對，請勿存檔", encoding.label())
                } else {
                    format!("已用 {} 重新讀取", encoding.label())
                };
            }
            Err(e) => self.status_message = format!("重新讀取失敗：{}", e),
        }
    }

    /// (行, 列) → TextEdit 字元索引
    fn char_index_of(buf: &EditorBuffer, line: usize, col: usize) -> usize {
        let mut idx = 0;
        for l in buf.lines.iter().take(line) {
            idx += l.chars().count() + 1;
        }
        idx + col.min(buf.line_len(line))
    }

    fn sync_cursor_to_editor(&mut self) {
        if let Some(buf) = self.tabs.get(self.active_tab_idx) {
            let h = buf.selections[0].head;
            let ci = Self::char_index_of(buf, h.line, h.col);
            self.pending_select = Some((ci, ci));
        }
    }

    fn replace_text_with<F: Fn(&str) -> String>(&mut self, f: F) {
        if let Some(buf) = self.active_buffer_mut() {
            let res = f(&buf.edit_text());
            buf.replace_all_text(&res);
        }
    }

    fn all_commands(&self) -> Vec<(String, Cmd)> {
        let mut v: Vec<(String, Cmd)> = vec![
            ("檔案：新增檔案 (New File)".into(), Cmd::NewFile),
            ("檔案：新增 Markdown 筆記 (New Markdown)".into(), Cmd::NewMarkdown),
            ("檔案：開啟檔案 (Open File)".into(), Cmd::Open),
            ("檔案：儲存 (Save)".into(), Cmd::Save),
            ("檔案：另存新檔 (Save As)".into(), Cmd::SaveAs),
            ("檔案：關閉分頁 (Close Tab)".into(), Cmd::CloseTab),
            ("檔案：開啟檔案所在資料夾 (Reveal in Folder)".into(), Cmd::RevealInFolder),
            ("檔案：複製完整路徑 (Copy Path)".into(), Cmd::CopyPath),
            ("編輯：複製當前行 (Duplicate Line)".into(), Cmd::DuplicateLine),
            ("編輯：刪除當前行 (Delete Line)".into(), Cmd::DeleteLine),
            ("編輯：切換註解 (Toggle Comment)".into(), Cmd::ToggleComment),
            ("編輯：切換書籤 (Toggle Bookmark)".into(), Cmd::ToggleBookmark),
            ("搜尋：尋找 (Find)".into(), Cmd::Find),
            ("搜尋：跳至行 (Go to Line)".into(), Cmd::GotoLine),
            ("搜尋：全專案搜尋 (Find in Files)".into(), Cmd::FindInFiles),
            ("檢視：切換 Typora Markdown 模式 (Typora / 雙欄 / 原始碼)".into(), Cmd::CycleMarkdownMode),
            ("檢視：切換 Markdown 大綱側邊欄 (Outline)".into(), Cmd::ToggleMarkdownOutline),
            ("檢視：終端機 (Terminal)".into(), Cmd::ToggleTerminal),
            ("檢視：Markdown 關聯圖 (Graph)".into(), Cmd::ToggleGraph),
            ("檢視：Log 篩選工具列 (Filter Bar)".into(), Cmd::ToggleFilterBar),
            ("檢視：放大字型 (Zoom In)".into(), Cmd::ZoomIn),
            ("檢視：縮小字型 (Zoom Out)".into(), Cmd::ZoomOut),
            ("工具：行排序 遞增 (Sort Asc)".into(), Cmd::SortAsc),
            ("工具：行排序 遞減 (Sort Desc)".into(), Cmd::SortDesc),
            ("工具：去除重複行 (Deduplicate)".into(), Cmd::Dedup),
            ("工具：全形轉半形 (To Half-width)".into(), Cmd::HalfWidth),
            ("說明：關於 AlexCode (About)".into(), Cmd::About),
        ];
        for (kind, label) in available_shells() {
            v.push((format!("檔案：{}", label), Cmd::OpenShell(*kind)));
        }
        for name in ThemeManager::theme_names() {
            v.push((format!("主題：{}", name), Cmd::Theme(name)));
        }
        for name in self.plugin_mgr.command_names() {
            v.push((format!("外掛：{}", name), Cmd::Plugin(name.clone())));
        }
        v
    }

    pub fn run_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::NewFile => self.new_tab(),
            Cmd::NewMarkdown => self.new_markdown_tab(),
            Cmd::Open => {
                if let Some(files) = rfd::FileDialog::new().set_directory(&self.project_root).pick_files() {
                    for f in files {
                        self.open_path(&f);
                    }
                }
            }
            Cmd::Save => self.save_active(false),
            Cmd::SaveAs => self.save_active(true),
            Cmd::CloseTab => self.close_tab(self.active_tab_idx),
            Cmd::RevealInFolder => self.reveal_tab(self.active_tab_idx),
            Cmd::OpenShell(kind) => self.open_shell_for_tab(self.active_tab_idx, kind),
            Cmd::CopyPath => {
                // 剪貼簿需要 egui Context：交給下一幀的 update() 處理
                self.pending_copy = self.active_buffer().and_then(|b| b.file_path.clone());
            }
            Cmd::Undo => {
                self.commit_md_edit();
                if self.active_buffer_mut().map_or(false, |b| b.undo()) {
                    self.sync_cursor_to_editor();
                    self.scroll_to_cursor = true;
                }
            }
            Cmd::Redo => {
                self.commit_md_edit();
                if self.active_buffer_mut().map_or(false, |b| b.redo()) {
                    self.sync_cursor_to_editor();
                    self.scroll_to_cursor = true;
                }
            }
            Cmd::DuplicateLine => {
                if let Some(b) = self.active_buffer_mut() {
                    b.duplicate_current_line();
                }
                self.sync_cursor_to_editor();
            }
            Cmd::DeleteLine => {
                if let Some(b) = self.active_buffer_mut() {
                    b.delete_current_line();
                }
                self.sync_cursor_to_editor();
            }
            Cmd::MoveLineUp => {
                if let Some(b) = self.active_buffer_mut() {
                    b.move_line_up();
                }
                self.sync_cursor_to_editor();
            }
            Cmd::MoveLineDown => {
                if let Some(b) = self.active_buffer_mut() {
                    b.move_line_down();
                }
                self.sync_cursor_to_editor();
            }
            Cmd::ToggleComment => {
                let prefix = self.tab_lang(self.active_tab_idx).comment_prefix();
                if let Some(b) = self.active_buffer_mut() {
                    b.toggle_comment(prefix);
                }
            }
            Cmd::ToggleBookmark => {
                if let Some(b) = self.active_buffer_mut() {
                    let l = b.selections[0].head.line;
                    b.toggle_bookmark(l);
                }
            }
            Cmd::Find => {
                self.show_find_bar = true;
                self.find_bar_focus = true;
            }
            Cmd::GotoLine => {
                self.show_goto = true;
                self.goto_text.clear();
            }
            Cmd::FindInFiles => self.show_find_in_files = true,
            Cmd::CommandPalette => {
                self.show_command_palette = true;
                self.palette_focus = true;
                self.command_query.clear();
            }
            Cmd::ToggleTerminal => self.show_terminal = !self.show_terminal,
            Cmd::ToggleGraph => {
                self.show_graph = !self.show_graph;
                if self.show_graph {
                    self.rebuild_link_index();
                }
            }
            Cmd::ToggleFilterBar => self.filter_active = !self.filter_active,
            Cmd::CycleMarkdownMode => {
                self.commit_md_edit();
                if self.tab_lang(self.active_tab_idx) != Lang::Markdown {
                    // 若目前不是 .md 分頁，尋求已開啟的 .md 分頁或新增一個 Markdown 筆記
                    if let Some(md_idx) = (0..self.tabs.len()).find(|&i| self.tab_lang(i) == Lang::Markdown) {
                        self.active_tab_idx = md_idx;
                        self.md_view.mode = MdViewMode::Typora;
                    } else {
                        self.new_markdown_tab();
                    }
                } else {
                    self.md_view.mode = self.md_view.mode.next();
                }
                self.status_message = format!("Markdown 模式：{}", self.md_view.mode.label());
            }
            Cmd::ToggleMarkdownOutline => {
                self.md_view.show_outline = !self.md_view.show_outline;
            }
            Cmd::Theme(name) => self.theme_mgr.set_theme(name),
            Cmd::SortAsc => self.replace_text_with(|t| crate::editor::sort_lines(t, false)),
            Cmd::SortDesc => self.replace_text_with(|t| crate::editor::sort_lines(t, true)),
            Cmd::Dedup => self.replace_text_with(crate::editor::remove_duplicate_lines),
            Cmd::HalfWidth => self.replace_text_with(crate::editor::to_half_width),
            Cmd::ZoomIn => self.font_size = (self.font_size + 1.0).min(36.0),
            Cmd::ZoomOut => self.font_size = (self.font_size - 1.0).max(8.0),
            Cmd::About => self.show_about = true,
            Cmd::Plugin(name) => {
                if let Some(res) = self.plugin_mgr.run_command(&name) {
                    self.status_message = format!("執行指令成功: {}", res);
                }
            }
        }
    }

    /// 掃描專案資料夾與已開啟分頁中的 Markdown，建立 [[wiki link]] 關聯索引
    fn rebuild_link_index(&mut self) {
        let mut map: HashMap<String, String> = HashMap::new();
        fn walk(dir: &Path, depth: usize, map: &mut HashMap<String, String>) {
            if depth > 4 || map.len() > 400 {
                return;
            }
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if p.is_dir() {
                    if !name.starts_with('.') && name != "target" && name != "node_modules" {
                        walk(&p, depth + 1, map);
                    }
                } else if name.to_ascii_lowercase().ends_with(".md") {
                    if let Ok(s) = std::fs::read_to_string(&p) {
                        let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(name);
                        map.insert(stem, s);
                    }
                }
            }
        }
        walk(&self.project_root, 0, &mut map);
        for b in &self.tabs {
            if let Some(p) = &b.file_path {
                if p.to_ascii_lowercase().ends_with(".md") {
                    let stem = Path::new(p).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    map.insert(stem, b.edit_text());
                }
            }
        }
        self.link_index = MarkdownLinkIndex::new();
        self.link_index.build_from_contents(&map);
    }

    fn run_terminal_command(&mut self, cmd: String) {
        let prompt = format!("\x1b[36m{}\x1b[0m> {}\r\n", self.term_cwd.display(), cmd);
        self.terminal.feed(prompt.as_bytes());
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        if trimmed == "cls" || trimmed == "clear" {
            self.terminal = VtParser::new(200, 160);
            return;
        }
        if let Some(dir) = trimmed.strip_prefix("cd ").or_else(|| trimmed.strip_prefix("cd\t")) {
            let target = self.term_cwd.join(dir.trim().trim_matches('"'));
            match target.canonicalize() {
                Ok(p) if p.is_dir() => self.term_cwd = p,
                _ => self.terminal.feed(b"\x1b[31mThe system cannot find the path specified.\x1b[0m\r\n"),
            }
            return;
        }
        let (tx, rx) = unbounded::<Vec<u8>>();
        let cwd = self.term_cwd.clone();
        std::thread::spawn(move || {
            #[cfg(windows)]
            let output = {
                use std::os::windows::process::CommandExt;
                std::process::Command::new("cmd")
                    .args(["/C", &format!("chcp 65001>nul & {}", cmd)])
                    .current_dir(&cwd)
                    .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                    .output()
            };
            #[cfg(not(windows))]
            let output = std::process::Command::new("sh").args(["-c", &cmd]).current_dir(&cwd).output();
            let mut data = match output {
                Ok(o) => {
                    let mut d = o.stdout;
                    if !o.stderr.is_empty() {
                        d.extend_from_slice(b"\x1b[31m");
                        d.extend_from_slice(&o.stderr);
                        d.extend_from_slice(b"\x1b[0m");
                    }
                    d
                }
                Err(e) => format!("\x1b[31m{}\x1b[0m\n", e).into_bytes(),
            };
            // 正規化換行為 CRLF 以符合 VT 終端行為
            let s = String::from_utf8_lossy(&data).replace("\r\n", "\n").replace('\n', "\r\n");
            data = s.into_bytes();
            let _ = tx.send(data);
        });
        self.term_rx = Some(rx);
    }

    // =====================================================================
    //  UI 區塊
    // =====================================================================

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let ca = Modifiers::COMMAND | Modifiers::ALT;
        let shortcuts: [(Modifiers, Key, Cmd); 20] = [
            (cs, Key::P, Cmd::CommandPalette),
            (cs, Key::F, Cmd::FindInFiles),
            (cs, Key::S, Cmd::SaveAs),
            (cs, Key::L, Cmd::DeleteLine),
            (cs, Key::M, Cmd::CycleMarkdownMode),
            (ca, Key::N, Cmd::NewMarkdown),
            (Modifiers::COMMAND, Key::N, Cmd::NewFile),
            (Modifiers::COMMAND, Key::O, Cmd::Open),
            (Modifiers::COMMAND, Key::S, Cmd::Save),
            (Modifiers::COMMAND, Key::W, Cmd::CloseTab),
            (Modifiers::COMMAND, Key::F, Cmd::Find),
            (Modifiers::COMMAND, Key::G, Cmd::GotoLine),
            (Modifiers::COMMAND, Key::D, Cmd::DuplicateLine),
            (Modifiers::COMMAND, Key::Slash, Cmd::ToggleComment),
            (Modifiers::COMMAND, Key::Backtick, Cmd::ToggleTerminal),
            (Modifiers::COMMAND, Key::Plus, Cmd::ZoomIn),
            (Modifiers::COMMAND, Key::Equals, Cmd::ZoomIn),
            (Modifiers::COMMAND, Key::Minus, Cmd::ZoomOut),
            (Modifiers::ALT, Key::ArrowUp, Cmd::MoveLineUp),
            (Modifiers::ALT, Key::ArrowDown, Cmd::MoveLineDown),
        ];
        for (m, k, cmd) in shortcuts {
            if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k))) {
                self.run_cmd(cmd);
            }
        }
        // 復原 / 重做只有一套紀錄（EditorBuffer）：程式碼編輯區有焦點時攔下快捷鍵，
        // 不讓 egui TextEdit 內建的另一套復原紀錄處理。其他輸入框（尋找、篩選…）維持各自的復原。
        if self.editor_focused {
            let redo = ctx.input_mut(|i| {
                i.consume_shortcut(&KeyboardShortcut::new(cs, Key::Z))
                    || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Y))
            });
            if redo {
                self.run_cmd(Cmd::Redo);
            } else if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Z))) {
                self.run_cmd(Cmd::Undo);
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F2))) {
            self.run_cmd(Cmd::ToggleBookmark);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.commit_md_edit();
            self.show_find_bar = false;
            self.show_goto = false;
            self.show_command_palette = false;
            self.pending_close = None;
        }
        // 拖放檔案開啟
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for p in dropped {
            self.open_path(&p);
        }
    }

    /// 帶漸層底與光暈邊線的面板 Frame
    fn bar_frame(c: &Colors) -> egui::Frame {
        egui::Frame::none().fill(c.panel).inner_margin(egui::Margin::symmetric(6.0, 3.0))
    }

    fn paint_bar_bg(ui: &egui::Ui, top: Color32, bottom: Color32, line: Option<(bool, Color32)>) {
        let rect = ui.max_rect().expand2(egui::vec2(6.0, 3.0));
        ui.painter().add(vgradient(rect, top, bottom));
        if let Some((at_bottom, col)) = line {
            let y = if at_bottom { rect.bottom() - 0.5 } else { rect.top() + 0.5 };
            ui.painter().hline(rect.x_range(), y, Stroke::new(1.0_f32, col));
        }
    }

    fn menu_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        egui::TopBottomPanel::top("menu_bar")
            .frame(Self::bar_frame(c))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let line = if c.hud { c.accent } else { c.glow };
                Self::paint_bar_bg(ui, c.panel, c.bg, Some((true, line)));
                egui::menu::bar(ui, |ui| {
                    // 龍形 Logo + Orbitron 字標
                    if let Some(tex) = &self.logo {
                        let r = ui
                            .add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(20.0, 20.0)).sense(Sense::click()))
                            .on_hover_text("關於 AlexCode");
                        if r.clicked() {
                            self.show_about = true;
                        }
                    }
                    ui.label(
                        RichText::new("ALEXCODE")
                            .font(FontId::new(13.0, FontFamily::Name(ORBITRON.into())))
                            .color(c.accent)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    let mut cmd: Option<Cmd> = None;
                    fn mi(ui: &mut egui::Ui, label: &str, sc: &str) -> bool {
                        let btn = egui::Button::new(label).shortcut_text(sc);
                        if ui.add(btn).clicked() {
                            ui.close_menu();
                            true
                        } else {
                            false
                        }
                    }

                    ui.menu_button("檔案 (File)", |ui| {
                        ui.set_min_width(240.0);
                        if mi(ui, "新增檔案 (New)", "Ctrl+N") { cmd = Some(Cmd::NewFile); }
                        if mi(ui, "新增 Markdown 筆記 (New MD)", "") { cmd = Some(Cmd::NewMarkdown); }
                        if mi(ui, "開啟檔案 (Open)…", "Ctrl+O") { cmd = Some(Cmd::Open); }
                        ui.separator();
                        if mi(ui, "儲存 (Save)", "Ctrl+S") { cmd = Some(Cmd::Save); }
                        if mi(ui, "另存新檔 (Save As)…", "Ctrl+Shift+S") { cmd = Some(Cmd::SaveAs); }
                        ui.separator();
                        if mi(ui, "開啟檔案所在資料夾", "") { cmd = Some(Cmd::RevealInFolder); }
                        for (kind, label) in available_shells() {
                            if mi(ui, label, "") { cmd = Some(Cmd::OpenShell(*kind)); }
                        }
                        if mi(ui, "複製完整路徑", "") { cmd = Some(Cmd::CopyPath); }
                        ui.separator();
                        if mi(ui, "關閉分頁 (Close)", "Ctrl+W") { cmd = Some(Cmd::CloseTab); }
                        if ui.button("結束 (Exit)").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button("編輯 (Edit)", |ui| {
                        ui.set_min_width(240.0);
                        if mi(ui, "復原 (Undo)", "Ctrl+Z") { cmd = Some(Cmd::Undo); }
                        if mi(ui, "重做 (Redo)", "Ctrl+Y") { cmd = Some(Cmd::Redo); }
                        ui.separator();
                        if mi(ui, "複製當前行", "Ctrl+D") { cmd = Some(Cmd::DuplicateLine); }
                        if mi(ui, "刪除當前行", "Ctrl+Shift+L") { cmd = Some(Cmd::DeleteLine); }
                        if mi(ui, "向上移動行", "Alt+Up") { cmd = Some(Cmd::MoveLineUp); }
                        if mi(ui, "向下移動行", "Alt+Down") { cmd = Some(Cmd::MoveLineDown); }
                        ui.separator();
                        if mi(ui, "切換註解", "Ctrl+/") { cmd = Some(Cmd::ToggleComment); }
                        if mi(ui, "切換書籤", "Ctrl+F2") { cmd = Some(Cmd::ToggleBookmark); }
                    });

                    ui.menu_button("搜尋 (Search)", |ui| {
                        ui.set_min_width(260.0);
                        if mi(ui, "尋找 (Find)", "Ctrl+F") { cmd = Some(Cmd::Find); }
                        if mi(ui, "跳至行 (Go to Line)", "Ctrl+G") { cmd = Some(Cmd::GotoLine); }
                        ui.separator();
                        if mi(ui, "命令面板 (Command Palette)", "Ctrl+Shift+P") { cmd = Some(Cmd::CommandPalette); }
                        if mi(ui, "全專案搜尋 (Find in Files)", "Ctrl+Shift+F") { cmd = Some(Cmd::FindInFiles); }
                    });

                    ui.menu_button("檢視 (View)", |ui| {
                        ui.set_min_width(260.0);
                        if mi(ui, "切換 Typora Markdown 模式", "Ctrl+Shift+M") {
                            cmd = Some(Cmd::CycleMarkdownMode);
                        }
                        if ui.checkbox(&mut self.md_view.show_outline, "Markdown 大綱側邊欄 (Outline)").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.show_terminal, "互動終端機 (Terminal)").clicked() {
                            ui.close_menu();
                        }
                        let mut g = self.show_graph;
                        if ui.checkbox(&mut g, "Markdown 筆記關聯圖 (Graph)").clicked() {
                            cmd = Some(Cmd::ToggleGraph);
                            ui.close_menu();
                        }
                        if ui.checkbox(&mut self.filter_active, "Log 行篩選工具列 (Filter)").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if mi(ui, "放大字型 (Zoom In)", "Ctrl++") { cmd = Some(Cmd::ZoomIn); }
                        if mi(ui, "縮小字型 (Zoom Out)", "Ctrl+-") { cmd = Some(Cmd::ZoomOut); }
                    });

                    ui.menu_button("主題 (Theme)", |ui| {
                        for name in ThemeManager::theme_names() {
                            let is_current = self.theme_mgr.current_theme_name() == *name;
                            let pal = crate::theme::builtin_palettes();
                            let p = &pal[name];
                            ui.horizontal(|ui| {
                                // 色票預覽
                                let (r, _) = ui.allocate_exact_size(egui::vec2(36.0, 14.0), Sense::hover());
                                let w = r.width() / 3.0;
                                for (k, col) in [p.bg, p.accent, p.accent2].iter().enumerate() {
                                    let rr = Rect::from_min_size(
                                        Pos2::new(r.left() + w * k as f32, r.top()),
                                        egui::vec2(w, r.height()),
                                    );
                                    ui.painter().rect_filled(rr, 2.0, style::hex(col));
                                }
                                if ui.selectable_label(is_current, *name).clicked() {
                                    cmd = Some(Cmd::Theme(name));
                                    ui.close_menu();
                                }
                            });
                        }
                    });

                    ui.menu_button("工具 (Tools)", |ui| {
                        ui.set_min_width(220.0);
                        if mi(ui, "行排序（遞增）", "") { cmd = Some(Cmd::SortAsc); }
                        if mi(ui, "行排序（遞減）", "") { cmd = Some(Cmd::SortDesc); }
                        if mi(ui, "去除重複行 (Deduplicate)", "") { cmd = Some(Cmd::Dedup); }
                        if mi(ui, "全形轉半形 (To Half-width)", "") { cmd = Some(Cmd::HalfWidth); }
                    });

                    ui.menu_button("說明 (Help)", |ui| {
                        if mi(ui, "關於 AlexCode (About)", "") { cmd = Some(Cmd::About); }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if c.hud {
                            ui.label(
                                RichText::new("◢ ALEXCODE NEON HUD ◣")
                                    .font(FontId::new(11.0, FontFamily::Name(ORBITRON.into())))
                                    .color(c.accent),
                            );
                        } else {
                            ui.label(
                                RichText::new("v5.3")
                                    .font(FontId::new(11.0, FontFamily::Name(ORBITRON.into())))
                                    .color(c.fg_dim2),
                            );
                        }
                    });

                    if let Some(cmd) = cmd {
                        self.run_cmd(cmd);
                    }
                });
            });
    }

    fn tool_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        egui::TopBottomPanel::top("tool_bar")
            .frame(Self::bar_frame(c).inner_margin(egui::Margin::symmetric(6.0, 4.0)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let line = if c.hud { c.glow } else { c.border };
                Self::paint_bar_bg(ui, c.panel, c.bg, Some((true, line)));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let mut cmd = None;
                    let mut tb = |ui: &mut egui::Ui, k: IconKind, tip: &str, c_: Cmd| {
                        if tool_button(ui, k, tip, c.fg, c.accent, c.accent_faint, c.glow).clicked() {
                            cmd = Some(c_);
                        }
                    };
                    tb(ui, IconKind::New, "新增檔案 (Ctrl+N)", Cmd::NewFile);
                    tb(ui, IconKind::Open, "開啟檔案 (Ctrl+O)", Cmd::Open);
                    tb(ui, IconKind::Save, "儲存 (Ctrl+S)", Cmd::Save);
                    Self::tool_sep(ui, c);
                    tb(ui, IconKind::Undo, "復原 (Ctrl+Z)", Cmd::Undo);
                    tb(ui, IconKind::Redo, "重做 (Ctrl+Y)", Cmd::Redo);
                    Self::tool_sep(ui, c);
                    tb(ui, IconKind::Find, "尋找 (Ctrl+F)", Cmd::Find);
                    tb(ui, IconKind::FindInFiles, "全專案搜尋 (Ctrl+Shift+F)", Cmd::FindInFiles);
                    Self::tool_sep(ui, c);
                    tb(ui, IconKind::Palette, "命令面板 (Ctrl+Shift+P)", Cmd::CommandPalette);
                    tb(ui, IconKind::Terminal, "終端機 (Ctrl+`)", Cmd::ToggleTerminal);
                    tb(ui, IconKind::Graph, "Markdown 關聯圖", Cmd::ToggleGraph);
                    tb(
                        ui,
                        IconKind::Markdown,
                        "Typora Markdown 閱讀/雙欄/原始碼切換 (Ctrl+Shift+M)",
                        Cmd::CycleMarkdownMode,
                    );
                    if let Some(cmd) = cmd {
                        self.run_cmd(cmd);
                    }

                    if self.filter_active {
                        Self::tool_sep(ui, c);
                        self.filter_widgets(ui, c);
                    }
                });
            });
    }

    fn tool_sep(ui: &mut egui::Ui, c: &Colors) {
        ui.add_space(4.0);
        let (r, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, c.border);
        ui.add_space(4.0);
    }

    /// ⚡ FILTER 工具列（原版：關鍵字 || / && / ! 與 OR/AND + Fuzzy）
    fn filter_widgets(&mut self, ui: &mut egui::Ui, c: &Colors) {
        ui.label(RichText::new(" ⚡ FILTER ").color(c.accent2).strong());
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.filter_query)
                .hint_text("error && !heartbeat || fatal")
                .desired_width(260.0),
        );
        let mut recompile = resp.changed();
        egui::ComboBox::from_id_source("filter_logic")
            .width(56.0)
            .selected_text(if self.filter_logic == FilterLogic::Or { "OR" } else { "AND" })
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut self.filter_logic, FilterLogic::Or, "OR").clicked() {
                    recompile = true;
                }
                if ui.selectable_value(&mut self.filter_logic, FilterLogic::And, "AND").clicked() {
                    recompile = true;
                }
            });
        if ui
            .checkbox(&mut self.filter_fuzzy, "Fuzzy")
            .on_hover_text("模糊：子序列比對，例如 mwin 可比對 MainWindow")
            .changed()
        {
            recompile = true;
        }
        if recompile {
            self.filter_engine.set_logic(self.filter_logic);
            self.filter_engine.set_fuzzy(self.filter_fuzzy);
            self.filter_engine.compile(&self.filter_query);
        }
        if !self.filter_query.trim().is_empty() {
            self.sync_filter_hits();
            if ui.button("提取 Tab").on_hover_text("將篩選結果開成新分頁").clicked() {
                if let Some(buf) = self.tabs.get(self.active_tab_idx) {
                    let lines: Vec<&str> = self.filter_cache.1.iter().filter_map(|&i| buf.lines.get(i)).map(|s| s.as_str()).collect();
                    let mut nb = EditorBuffer::from_text(&lines.join("\n"));
                    nb.file_path = Some(format!("filtered_{}.log", self.tabs.len() + 1));
                    self.tabs.push(nb);
                    self.active_tab_idx = self.tabs.len() - 1;
                    self.filter_query.clear();
                    self.filter_engine.compile("");
                }
            }
            {
                self.sync_filter_hits();
                let total = self.tabs.get(self.active_tab_idx).map_or(0, |b| b.lines.len());
                ui.label(RichText::new(format!("{} / {} 行", self.filter_cache.1.len(), total)).color(c.accent));
            }
        }
    }

    fn tab_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame::none().fill(c.panel).inner_margin(egui::Margin { left: 4.0, right: 4.0, top: 2.0, bottom: 0.0 }))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let full = ui.max_rect().expand2(egui::vec2(4.0, 1.0));
                ui.painter().add(vgradient(full, c.panel, c.bg));
                let mut close_idx = None;
                let mut tab_action: Option<TabAction> = None;
                let tab_count = self.tabs.len();
                egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for i in 0..self.tabs.len() {
                            let is_active = i == self.active_tab_idx;
                            let title = Self::tab_title(&self.tabs[i]);
                            let modified = self.tabs[i].modified;
                            let font = FontId::proportional(13.0);
                            let galley = ui.painter().layout_no_wrap(title.clone(), font.clone(), c.fg);
                            let pad = egui::vec2(16.0, 7.0);
                            let close_w = 18.0;
                            let size = egui::vec2(galley.size().x + pad.x * 2.0 + close_w, galley.size().y + pad.y * 2.0);
                            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                            let hovered = resp.hovered();
                            let p = ui.painter();
                            if is_active {
                                let grad = Rect::from_min_max(
                                    Pos2::new(rect.left(), rect.top() + rect.height() * 0.5),
                                    rect.right_bottom(),
                                );
                                p.add(vgradient(grad, Color32::TRANSPARENT, c.accent_faint));
                                p.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0_f32, c.accent));
                            } else if hovered {
                                p.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0_f32, c.glow));
                            }
                            let tcol = if is_active {
                                c.accent
                            } else if hovered {
                                c.fg_list
                            } else {
                                c.fg_dim2
                            };
                            p.text(
                                Pos2::new(rect.left() + pad.x, rect.center().y),
                                Align2::LEFT_CENTER,
                                &title,
                                font,
                                tcol,
                            );
                            // 關閉鈕 / 修改圓點
                            let cr = Rect::from_center_size(
                                Pos2::new(rect.right() - pad.x * 0.5 - close_w * 0.5 + 4.0, rect.center().y),
                                egui::vec2(16.0, 16.0),
                            );
                            let pointer_in_close = ui
                                .input(|inp| inp.pointer.hover_pos())
                                .map_or(false, |pp| cr.contains(pp));
                            if hovered || is_active {
                                if pointer_in_close {
                                    p.rect_filled(cr, 3.0, c.hover);
                                }
                                let col = if pointer_in_close { c.accent2 } else { c.fg_dim2 };
                                let s = 3.5;
                                let st = Stroke::new(1.4_f32, col);
                                p.line_segment([cr.center() + egui::vec2(-s, -s), cr.center() + egui::vec2(s, s)], st);
                                p.line_segment([cr.center() + egui::vec2(-s, s), cr.center() + egui::vec2(s, -s)], st);
                            } else if modified {
                                p.circle_filled(cr.center(), 3.5, c.accent2);
                            }
                            if modified && (hovered || is_active) && !pointer_in_close {
                                p.circle_filled(Pos2::new(rect.left() + 7.0, rect.top() + 7.0), 2.5, c.accent2);
                            }
                            if resp.clicked() {
                                if pointer_in_close {
                                    close_idx = Some(i);
                                } else {
                                    self.commit_md_edit();
                                    self.active_tab_idx = i;
                                }
                            }
                            if resp.middle_clicked() {
                                close_idx = Some(i);
                            }
                            let on_disk = self.tabs[i].file_path.as_deref().map_or(false, |p| Path::new(p).is_absolute());
                            let resp = resp.on_hover_text(self.tabs[i].file_path.clone().unwrap_or_default());
                            // 右鍵選單：檔案總管 / 終端機 / 複製路徑 / 關閉
                            resp.context_menu(|ui| {
                                ui.set_min_width(250.0);
                                let mut pick = |ui: &mut egui::Ui, enabled: bool, label: &str, a: TabAction| {
                                    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                        tab_action = Some(a);
                                        ui.close_menu();
                                    }
                                };
                                pick(ui, true, "開啟檔案所在資料夾", TabAction::Reveal(i));
                                for (kind, label) in available_shells() {
                                    pick(ui, true, label, TabAction::Shell(i, *kind));
                                }
                                ui.separator();
                                pick(ui, on_disk, "複製完整路徑", TabAction::CopyPath(i));
                                pick(ui, true, "複製檔名", TabAction::CopyName(i));
                                ui.separator();
                                pick(ui, true, "關閉", TabAction::Close(i));
                                pick(ui, tab_count > 1, "關閉其他分頁", TabAction::CloseOthers(i));
                                pick(ui, i + 1 < tab_count, "關閉右側分頁", TabAction::CloseRight(i));
                                pick(ui, true, "全部關閉", TabAction::CloseAll);
                            });
                        }
                    });
                });
                if let Some(i) = close_idx {
                    self.close_tab(i);
                }
                if let Some(a) = tab_action {
                    self.run_tab_action(ctx, a);
                }
                // 分頁列與編輯區的分隔線
                let r = ui.max_rect().expand2(egui::vec2(4.0, 0.0));
                ui.painter().hline(r.x_range(), r.bottom(), Stroke::new(1.0_f32, c.border));
            });
    }

    fn find_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_find_bar {
            return;
        }
        // 全文與命中位置都只在內容或搜尋字串改變時重算
        self.sync_text_cache();
        let text = std::mem::take(&mut self.text_cache.1);
        let fkey = (self.text_cache.0 .0, self.text_cache.0 .1, self.find_bar_text.clone());
        if self.find_cache.0 != fkey {
            self.find_cache = (fkey, find_matches(&text, &self.find_bar_text));
        }
        let matches = std::mem::take(&mut self.find_cache.1);
        egui::TopBottomPanel::top("find_bar")
            .frame(egui::Frame::none().fill(c.panel).inner_margin(egui::Margin::symmetric(10.0, 5.0)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.painter().hline(
                    ui.max_rect().expand(10.0).x_range(),
                    ui.max_rect().bottom() + 5.0,
                    Stroke::new(1.0_f32, c.border),
                );
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                    paint_icon(ui.painter(), r, IconKind::Find, c.fg, c.accent);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.find_bar_text)
                            .hint_text("尋找…")
                            .desired_width(280.0),
                    );
                    if self.find_bar_focus {
                        resp.request_focus();
                        self.find_bar_focus = false;
                    }
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let shift = ui.input(|i| i.modifiers.shift);
                    let mut go: Option<bool> = None;
                    if enter {
                        go = Some(!shift);
                        resp.request_focus();
                    }
                    if ui.button("▲").on_hover_text("上一個 (Shift+Enter)").clicked() {
                        go = Some(false);
                    }
                    if ui.button("▼").on_hover_text("下一個 (Enter)").clicked() {
                        go = Some(true);
                    }
                    if let Some(forward) = go {
                        if !matches.is_empty() {
                            let n = matches.len();
                            self.find_cursor = if forward {
                                (self.find_cursor + 1) % n
                            } else {
                                (self.find_cursor + n - 1) % n
                            };
                            let (a, b) = matches[self.find_cursor.min(n - 1)];
                            let ca = text[..a].chars().count();
                            let cb = ca + text[a..b].chars().count();
                            self.pending_select = Some((ca, cb));
                            self.scroll_to_cursor = true;
                        }
                    }
                    let label = if self.find_bar_text.is_empty() {
                        String::new()
                    } else if matches.is_empty() {
                        "無結果".to_string()
                    } else {
                        format!("{} / {}", self.find_cursor.min(matches.len() - 1) + 1, matches.len())
                    };
                    ui.label(RichText::new(label).color(if matches.is_empty() { c.accent2 } else { c.accent }));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            self.show_find_bar = false;
                        }
                    });
                });
            });
        self.text_cache.1 = text;
        self.find_cache.1 = matches;
    }

    fn status_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(c.bg).inner_margin(egui::Margin::symmetric(4.0, 3.0)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let full = ui.max_rect().expand2(egui::vec2(4.0, 3.0));
                ui.painter().add(vgradient(full, c.bg, c.panel));
                let line = if c.hud { c.accent } else { c.glow };
                ui.painter().hline(full.x_range(), full.top() + 0.5, Stroke::new(1.0_f32, line));

                let (line_no, col, eol, lines) = if let Some(buf) = self.active_buffer() {
                    let sel = buf.selections.first().copied().unwrap_or(Selection::point(Position { line: 0, col: 0 }));
                    (
                        sel.head.line + 1,
                        sel.head.col + 1,
                        if buf.line_ending == "\r\n" { "CRLF" } else { "LF" },
                        buf.lines.len(),
                    )
                } else {
                    (1, 1, "LF", 0)
                };
                let lang = self.tab_lang(self.active_tab_idx);
                let encoding = self.active_buffer().map_or(FileEncoding::Utf8, |b| b.encoding);
                // (新編碼, 是否為「重新讀取」)
                let mut enc_choice: Option<(FileEncoding, bool)> = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let seg = |ui: &mut egui::Ui, t: String, col: Color32| {
                        let r = ui.add(egui::Label::new(RichText::new(t).size(12.0).color(col)).wrap(false));
                        let _ = r;
                        ui.add_space(10.0);
                        let (sr, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), Sense::hover());
                        ui.painter().rect_filled(sr, 0.0, c.border);
                        ui.add_space(10.0);
                    };
                    ui.add_space(6.0);
                    seg(ui, format!("Ln {}, Col {}", line_no, col), c.fg_dim2);
                    seg(ui, format!("{} 行", lines), c.fg_dim2);
                    // 編碼：點擊可改存檔編碼，或以其他編碼重新讀取
                    ui.menu_button(RichText::new(encoding.label()).size(12.0).color(c.fg_dim2), |ui| {
                        ui.set_min_width(200.0);
                        ui.label(RichText::new("存檔時使用的編碼").size(11.5).color(c.fg_dim2));
                        for e in FileEncoding::ALL {
                            if ui.selectable_label(e == encoding, e.label()).clicked() {
                                enc_choice = Some((e, false));
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.menu_button("以其他編碼重新讀取", |ui| {
                            for e in FileEncoding::ALL {
                                if ui.button(e.label()).clicked() {
                                    enc_choice = Some((e, true));
                                    ui.close_menu();
                                }
                            }
                        });
                    })
                    .response
                    .on_hover_text("檔案編碼（點擊變更）");
                    ui.add_space(10.0);
                    let (sr, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), Sense::hover());
                    ui.painter().rect_filled(sr, 0.0, c.border);
                    ui.add_space(10.0);
                    seg(ui, eol.into(), c.fg_dim2);
                    seg(ui, lang.display_name().into(), c.fg_dim2);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(c.fg_dim));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(self.theme_mgr.current_theme_name().to_uppercase())
                                .font(FontId::new(10.0, FontFamily::Name(ORBITRON.into())))
                                .color(c.accent),
                        );
                        let (dot, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.5, c.accent);
                        ui.painter().circle_stroke(dot.center(), 5.5, Stroke::new(1.0_f32, c.glow));
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{}%", (self.font_size / 14.0 * 100.0).round())).size(12.0).color(c.fg_dim2));
                    });
                });
                if let Some((e, reload)) = enc_choice {
                    let idx = self.active_tab_idx;
                    if reload {
                        self.commit_md_edit();
                        self.reload_with_encoding(idx, e);
                    } else if let Some(b) = self.tabs.get_mut(idx) {
                        b.set_encoding(e);
                        self.status_message = format!("存檔編碼已改為 {}（儲存後生效）", e.label());
                    }
                }
            });
    }

    /// Dock 標題（左側強調色識別條 + 水平漸層，對應原版 QDockWidget::title）
    fn dock_title(ui: &mut egui::Ui, c: &Colors, title: &str) -> bool {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::hover());
        let p = ui.painter();
        let split = rect.left() + rect.width() * 0.35;
        p.add(hgradient(Rect::from_min_max(rect.left_top(), Pos2::new(split, rect.bottom())), c.accent_faint, c.panel));
        p.rect_filled(Rect::from_min_max(Pos2::new(split, rect.top()), rect.right_bottom()), 0.0, c.panel);
        p.rect_filled(
            Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
            0.0,
            if c.hud { c.accent2 } else { c.accent },
        );
        p.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0_f32, c.border));
        p.text(
            Pos2::new(rect.left() + 12.0, rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::new(11.5, FontFamily::Name(ORBITRON.into())),
            c.accent,
        );
        let cr = Rect::from_center_size(Pos2::new(rect.right() - 14.0, rect.center().y), egui::vec2(18.0, 18.0));
        let resp = ui.interact(cr, ui.id().with(title), Sense::click());
        let col = if resp.hovered() { c.accent2 } else { c.fg_dim2 };
        if resp.hovered() {
            ui.painter().rect_filled(cr, 3.0, c.hover);
        }
        let s = 4.0;
        let st = Stroke::new(1.4_f32, col);
        ui.painter().line_segment([cr.center() + egui::vec2(-s, -s), cr.center() + egui::vec2(s, s)], st);
        ui.painter().line_segment([cr.center() + egui::vec2(-s, s), cr.center() + egui::vec2(s, -s)], st);
        resp.clicked()
    }

    fn terminal_dock(&mut self, ctx: &egui::Context, c: &Colors) {
        if let Some(rx) = &self.term_rx {
            if let Ok(data) = rx.try_recv() {
                self.terminal.feed(&data);
                self.term_rx = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if !self.show_terminal {
            return;
        }
        egui::TopBottomPanel::bottom("terminal_dock")
            .resizable(true)
            .default_height(200.0)
            .frame(egui::Frame::none().fill(c.bg).stroke(Stroke::new(1.0_f32, c.border)))
            .show(ctx, |ui| {
                if Self::dock_title(ui, c, "TERMINAL  整合終端機") {
                    self.show_terminal = false;
                }
                egui::Frame::none().inner_margin(egui::Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
                    let input_h = 28.0;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(ui.available_height() - input_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            let font = FontId::monospace(13.0);
                            for i in 0..self.terminal.scrollback_count() {
                                ui.label(RichText::new(self.terminal.scrollback_line(i)).font(font.clone()).color(c.fg_dim));
                            }
                            let last = (0..self.terminal.rows())
                                .rev()
                                .find(|&r| !self.terminal.line_text(r).trim().is_empty())
                                .map_or(0, |r| r + 1);
                            for r in 0..last.max(self.terminal.cursor_row()) {
                                let line = self.terminal.line_text(r);
                                ui.label(RichText::new(line).font(font.clone()).color(c.fg));
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("❯").color(c.accent).strong());
                        let busy = self.term_rx.is_some();
                        let resp = ui.add_enabled(
                            !busy,
                            egui::TextEdit::singleline(&mut self.term_input)
                                .font(FontId::monospace(13.0))
                                .hint_text(if busy { "執行中…" } else { "輸入指令後按 Enter（例如 dir、git status、cargo --version）" })
                                .desired_width(f32::INFINITY),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            let cmd = std::mem::take(&mut self.term_input);
                            self.run_terminal_command(cmd);
                            resp.request_focus();
                        }
                    });
                });
            });
    }

    fn graph_dock(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_graph {
            return;
        }
        egui::TopBottomPanel::bottom("graph_dock")
            .resizable(true)
            .default_height(240.0)
            .frame(egui::Frame::none().fill(c.bg).stroke(Stroke::new(1.0_f32, c.border)))
            .show(ctx, |ui| {
                if Self::dock_title(ui, c, "GRAPH  Markdown 筆記關聯圖") {
                    self.show_graph = false;
                }
                let files = self.link_index.files().to_vec();
                let mut edges = Vec::new();
                for (f, t) in self.link_index.edges() {
                    if let (Some(u), Some(v)) = (files.iter().position(|x| x == &f), files.iter().position(|x| x == &t)) {
                        edges.push((u, v));
                    }
                }
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("節點 {}  ·  關聯 {}  ·  {}", files.len(), edges.len(), self.project_root.display()))
                            .size(12.0)
                            .color(c.fg_dim2),
                    );
                    if ui.small_button("重新整理").clicked() {
                        self.rebuild_link_index();
                    }
                });
                let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                let painter = ui.painter_at(rect);
                // 背景網格（Neon Grid 風格）
                let step = 28.0;
                let grid = with_alpha(c.border, 120);
                let mut x = rect.left();
                while x < rect.right() {
                    painter.vline(x, rect.y_range(), Stroke::new(1.0_f32, grid));
                    x += step;
                }
                let mut y = rect.top();
                while y < rect.bottom() {
                    painter.hline(rect.x_range(), y, Stroke::new(1.0_f32, grid));
                    y += step;
                }
                if files.is_empty() {
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "此資料夾沒有 Markdown 筆記（開啟 .md 檔案或含 [[連結]] 的資料夾）",
                        FontId::proportional(13.0),
                        c.fg_dim2,
                    );
                    return;
                }
                let pts = compute_graph_layout(files.len(), &edges, rect.width() as f64, rect.height() as f64, 60);
                let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for p in &pts {
                    minx = minx.min(p.x as f32);
                    miny = miny.min(p.y as f32);
                    maxx = maxx.max(p.x as f32);
                    maxy = maxy.max(p.y as f32);
                }
                let inner = rect.shrink(40.0);
                let map = |px: f32, py: f32| -> Pos2 {
                    let nx = if maxx > minx { (px - minx) / (maxx - minx) } else { 0.5 };
                    let ny = if maxy > miny { (py - miny) / (maxy - miny) } else { 0.5 };
                    Pos2::new(inner.left() + nx * inner.width(), inner.top() + ny * inner.height())
                };
                let pos: Vec<Pos2> = pts.iter().map(|p| map(p.x as f32, p.y as f32)).collect();
                for &(u, v) in &edges {
                    painter.line_segment([pos[u], pos[v]], Stroke::new(1.2_f32, c.glow));
                }
                let hover = ui.input(|i| i.pointer.hover_pos());
                for (i, p) in pos.iter().enumerate() {
                    let deg = edges.iter().filter(|&&(a, b)| a == i || b == i).count();
                    let r = 4.0 + (deg as f32).sqrt() * 2.0;
                    let hot = hover.map_or(false, |h| h.distance(*p) < r + 4.0);
                    painter.circle_filled(*p, r + 4.0, with_alpha(c.accent, if hot { 70 } else { 30 }));
                    painter.circle_filled(*p, r, if hot { c.accent2 } else { c.accent });
                    painter.text(
                        *p + egui::vec2(0.0, r + 4.0),
                        Align2::CENTER_TOP,
                        &files[i],
                        FontId::proportional(11.5),
                        if hot { c.fg } else { c.fg_dim },
                    );
                }
            });
    }

    fn editor_area(&mut self, ctx: &egui::Context, c: &Colors) {
        let pal = self.theme_mgr.current_palette().clone();
        let is_md = self.tab_lang(self.active_tab_idx) == Lang::Markdown;
        let filtering = self.filter_active && !self.filter_query.trim().is_empty();

        if !filtering && is_md {
            // Markdown 專屬頂部工具列（Typora 閱讀 / 雙欄 / 原始碼 切換 + 快速格式插入）
            egui::TopBottomPanel::top("md_typora_toolbar")
                .frame(egui::Frame::none().fill(c.panel))
                .show_separator_line(false)
                .show(ctx, |ui| {
                    if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
                        render_markdown_toolbar(ui, &mut self.md_view, buf, c);
                    }
                });

            // 左側 Typora 章節大綱 (Outline) 面板
            if self.md_view.show_outline {
                egui::SidePanel::left("md_outline_panel")
                    .resizable(true)
                    .default_width(220.0)
                    .min_width(150.0)
                    .max_width(380.0)
                    .frame(egui::Frame::none().fill(c.panel).stroke(Stroke::new(1.0_f32, c.border)))
                    .show(ctx, |ui| {
                        if let Some(buf) = self.tabs.get(self.active_tab_idx) {
                            if let Some(line) = render_outline_panel(ui, &buf.lines, c) {
                                self.md_view.scroll_to_line = Some(line);
                                self.tabs[self.active_tab_idx].selections =
                                    vec![Selection::point(Position { line, col: 0 })];
                                self.scroll_to_cursor = true;
                            }
                        }
                    });
            }
        }

        self.editor_focused = false; // 由 code_editor 在本幀重新設定
        let mut md_action = MdAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(c.bg))
            .show(ctx, |ui| {
                let full_rect = ui.max_rect();
                if filtering {
                    self.filtered_view(ui, c, &pal);
                } else if is_md {
                    let find = if self.show_find_bar {
                        self.find_bar_text.clone()
                    } else {
                        String::new()
                    };
                    let root = self.project_root.clone();
                    let fsize = self.font_size;
                    match self.md_view.mode {
                        MdViewMode::Typora => {
                            if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
                                md_action = render_typora_document(
                                    ui,
                                    &mut self.md_view,
                                    buf,
                                    fsize,
                                    c,
                                    &pal,
                                    &find,
                                    &root,
                                    true,
                                );
                            }
                        }
                        MdViewMode::Split => {
                            let avail = ui.available_size();
                            let half_w = ((avail.x - 10.0) * 0.5).max(200.0);
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(half_w, avail.y),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        self.code_editor(ui, c, &pal);
                                    },
                                );
                                let (sep, _) =
                                    ui.allocate_exact_size(egui::vec2(2.0, avail.y), Sense::hover());
                                ui.painter().rect_filled(sep, 0.0, c.border);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(avail.x - half_w - 8.0, avail.y),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
                                            md_action = render_typora_document(
                                                ui,
                                                &mut self.md_view,
                                                buf,
                                                fsize,
                                                c,
                                                &pal,
                                                &find,
                                                &root,
                                                false,
                                            );
                                        }
                                    },
                                );
                            });
                        }
                        MdViewMode::Source => {
                            self.code_editor(ui, c, &pal);
                        }
                    }
                } else {
                    self.code_editor(ui, c, &pal);
                }
                if c.hud {
                    paint_hud_corners(&ui.painter_at(full_rect), full_rect, c.accent);
                }
            });

        if let MdAction::OpenLink { target, wiki } = md_action {
            self.open_md_link(ctx, &target, wiki);
        }
    }

    fn code_editor(&mut self, ui: &mut egui::Ui, c: &Colors, pal: &crate::theme::Palette) {
        let idx = self.active_tab_idx;
        if idx >= self.tabs.len() {
            return;
        }
        let lang = self.tab_lang(idx);
        let font = FontId::monospace(self.font_size);
        let find = if self.show_find_bar { self.find_bar_text.clone() } else { String::new() };
        // 全文只在內容改變時重新串接（不再每一幀 join 所有行）
        self.sync_text_cache();
        let mut text = std::mem::take(&mut self.text_cache.1);
        let line_count = self.tabs[idx].lines.len();
        let bookmarks = self.tabs[idx].bookmarks.clone();
        let path_key = self.tabs[idx].file_path.clone().unwrap_or_default();
        let editor_id = egui::Id::new(("alexcode-editor", idx, path_key));

        // 套用待定的游標選取（尋找 / 跳至行 / 行操作）
        if let Some((a, b)) = self.pending_select.take() {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
            state.store(ui.ctx(), editor_id);
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }

        let row_h = ui.fonts(|f| f.row_height(&font));
        let char_w = ui.fonts(|f| f.glyph_width(&font, '0'));
        let digits = line_count.to_string().len().max(3);
        let gutter_w = digits as f32 * char_w + 30.0;

        // 語法高亮與排版只在輸入（文字 / 語言 / 主題 / 字型 / 搜尋字串）改變時重算；
        // 其餘每一幀只花一次全文雜湊。過大的文件不做語法高亮。
        let pal_c = pal.clone();
        let font_c = font.clone();
        let theme = self.theme_mgr.current_theme_name().to_string();
        let mut hl_cache = self.highlight_cache.take();
        let mut layouter = |ui: &egui::Ui, s: &str, _wrap: f32| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut h);
            lang.display_name().hash(&mut h);
            theme.hash(&mut h);
            font_c.size.to_bits().hash(&mut h);
            find.hash(&mut h);
            let key = h.finish();
            let atlas = ui.fonts(|f| f.texture_atlas());
            match &hl_cache {
                Some((k, galley, a)) if *k == key && Arc::ptr_eq(a, &atlas) => galley.clone(),
                _ => {
                    let l = if s.len() > HIGHLIGHT_MAX_BYTES { Lang::Plain } else { lang };
                    let job = highlight(s, l, &pal_c, font_c.clone(), &find);
                    let galley = ui.fonts(|f| f.layout_job(job));
                    hl_cache = Some((key, galley.clone(), atlas));
                    galley
                }
            }
        };

        let mut new_cursor: Option<(usize, usize)> = None;
        let mut changed = false;
        let scroll_to_cursor = std::mem::take(&mut self.scroll_to_cursor);

        egui::ScrollArea::both()
            .id_source(("editor-scroll", idx))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let min_h = ui.available_height().max(line_count as f32 * row_h + 40.0);
                    let (gutter_rect, gutter_resp) =
                        ui.allocate_exact_size(egui::vec2(gutter_w, min_h), Sense::click());
                    let line_bg_idx = ui.painter().add(Shape::Noop);
                    let gutter_idx = ui.painter().add(Shape::Noop);

                    let output = egui::TextEdit::multiline(&mut text)
                        .id(editor_id)
                        .font(font.clone())
                        .code_editor()
                        .frame(false)
                        .margin(egui::vec2(10.0, 4.0))
                        .desired_width(f32::INFINITY)
                        .desired_rows((min_h / row_h) as usize)
                        .lock_focus(true)
                        .layouter(&mut layouter)
                        .show(ui);

                    changed = output.response.changed();
                    self.editor_focused = output.response.has_focus();
                    let galley = &output.galley;
                    let gpos = output.galley_pos;

                    // 目前游標所在行
                    let cur_row = output.cursor_range.map(|cr| (cr.primary.rcursor.row, cr.primary.rcursor.column));
                    if let Some((r, col)) = cur_row {
                        new_cursor = Some((r, col));
                    }
                    let active_line = cur_row
                        .map(|(r, _)| r)
                        .unwrap_or_else(|| self.tabs[idx].selections[0].head.line);

                    // 目前行底色（畫在文字下方）
                    let clip = ui.clip_rect();
                    if let Some(row) = galley.rows.get(active_line) {
                        let y0 = gpos.y + row.rect.top();
                        let r = Rect::from_min_max(
                            Pos2::new(gutter_rect.right(), y0),
                            Pos2::new(clip.right().max(output.response.rect.right()), y0 + row.rect.height().max(row_h)),
                        );
                        ui.painter().set(
                            line_bg_idx,
                            Shape::rect_filled(r, 0.0, c.current_line),
                        );
                    }

                    // 行號 gutter
                    let mut shapes = vec![
                        Shape::rect_filled(gutter_rect, 0.0, c.panel),
                        Shape::line_segment(
                            [gutter_rect.right_top(), gutter_rect.right_bottom()],
                            Stroke::new(1.0_f32, c.border),
                        ),
                    ];
                    let num_font = FontId::monospace((self.font_size - 1.0).max(8.0));
                    ui.fonts(|f| {
                        for (i, row) in galley.rows.iter().enumerate() {
                            let y = gpos.y + row.rect.top();
                            if y + row_h < clip.top() || y > clip.bottom() {
                                continue;
                            }
                            let active = i == active_line;
                            let col = if active { c.accent } else { c.faint };
                            let g = f.layout_no_wrap(format!("{}", i + 1), num_font.clone(), col);
                            let pos = Pos2::new(
                                gutter_rect.right() - 10.0 - g.size().x,
                                y + (row.rect.height().max(row_h) - g.size().y) * 0.5,
                            );
                            shapes.push(Shape::galley(pos, g, col));
                            if active {
                                shapes.push(Shape::rect_filled(
                                    Rect::from_min_size(Pos2::new(gutter_rect.left(), y), egui::vec2(2.0, row_h)),
                                    0.0,
                                    c.accent,
                                ));
                            }
                            if bookmarks.contains(&i) {
                                shapes.push(Shape::circle_filled(
                                    Pos2::new(gutter_rect.left() + 9.0, y + row_h * 0.5),
                                    3.5,
                                    c.accent2,
                                ));
                            }
                        }
                    });
                    ui.painter().set(gutter_idx, Shape::Vec(shapes));

                    // 點擊 gutter 切換書籤
                    if gutter_resp.clicked() {
                        if let Some(pp) = gutter_resp.interact_pointer_pos() {
                            let line = ((pp.y - gpos.y) / row_h).floor().max(0.0) as usize;
                            if line < line_count {
                                self.tabs[idx].toggle_bookmark(line);
                            }
                        }
                    }

                    if scroll_to_cursor {
                        if let Some(cr) = output.cursor_range {
                            let r = galley.pos_from_cursor(&cr.primary).translate(gpos.to_vec2());
                            ui.scroll_to_rect(r.expand2(egui::vec2(40.0, row_h * 2.0)), Some(Align::Center));
                        }
                    }
                });
            });

        self.highlight_cache = hl_cache;
        if changed {
            self.tabs[idx].apply_typed_text(&text);
        }
        // 把全文放回快取（貼上的內容含 \r 時緩衝區會正規化換行，快取就不能沿用）
        if !text.contains('\r') {
            self.text_cache = ((self.tabs[idx].id, self.tabs[idx].revision()), text);
        } else {
            self.text_cache = ((0, 0), String::new());
        }
        if let Some((line, col)) = new_cursor {
            let buf = &mut self.tabs[idx];
            let p = Position { line: line.min(buf.lines.len().saturating_sub(1)), col };
            buf.selections = vec![Selection::point(p)];
        }
    }

    /// Log 篩選結果檢視（僅顯示符合的行，保留原始行號）
    fn filtered_view(&mut self, ui: &mut egui::Ui, c: &Colors, pal: &crate::theme::Palette) {
        let idx = self.active_tab_idx;
        // 命中的行號有快取；顯示時直接借用緩衝區的行，不複製
        self.sync_filter_hits();
        let hits = std::mem::take(&mut self.filter_cache.1);
        let Some(buf) = self.tabs.get(idx) else { return };
        let lang = self.tab_lang(idx);
        let font = FontId::monospace(self.font_size);
        let needle = self.filter_engine.positive_keywords().into_iter().next().unwrap_or_default();
        let digits = buf.lines.len().to_string().len().max(3);
        let row_h = ui.fonts(|f| f.row_height(&font));
        let mut jump: Option<usize> = None;

        egui::ScrollArea::both().auto_shrink([false, false]).show_rows(ui, row_h, hits.len(), |ui, range| {
            for ln in &hits[range] {
                let Some(line) = buf.lines.get(*ln) else { continue };
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(400.0), row_h), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, 0.0, c.current_line);
                }
                let gw = digits as f32 * 9.0 + 26.0;
                ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(gw, row_h)), 0.0, c.panel);
                ui.painter().text(
                    Pos2::new(rect.left() + gw - 10.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{}", ln + 1),
                    FontId::monospace(self.font_size - 1.0),
                    c.accent2,
                );
                let job = highlight(line, lang, pal, font.clone(), &needle);
                let g = ui.fonts(|f| f.layout_job(job));
                ui.painter().galley(Pos2::new(rect.left() + gw + 10.0, rect.top()), g, c.fg);
                if resp.double_clicked() {
                    jump = Some(*ln);
                }
                resp.on_hover_text("雙擊跳至此行");
            }
        });
        self.filter_cache.1 = hits;
        if let Some(ln) = jump {
            self.filter_query.clear();
            self.filter_engine.compile("");
            let ci = Self::char_index_of(&self.tabs[idx], ln, 0);
            self.pending_select = Some((ci, ci));
            self.scroll_to_cursor = true;
        }
    }

    fn popup_windows(&mut self, ctx: &egui::Context, c: &Colors) {
        // 命令面板
        if self.show_command_palette {
            let mut open = true;
            let mut run: Option<Cmd> = None;
            egui::Window::new(RichText::new("COMMAND PALETTE").font(FontId::new(12.0, FontFamily::Name(ORBITRON.into()))).color(c.accent))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .fixed_size([520.0, 360.0])
                .anchor(Align2::CENTER_TOP, [0.0, 80.0])
                .show(ctx, |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.command_query)
                            .hint_text("> 輸入指令名稱…")
                            .desired_width(f32::INFINITY),
                    );
                    if self.palette_focus {
                        resp.request_focus();
                        self.palette_focus = false;
                    }
                    ui.add_space(4.0);
                    let mut cmds: Vec<(i32, String, Cmd)> = self
                        .all_commands()
                        .into_iter()
                        .filter_map(|(n, cmd)| {
                            let s = command_match_score(&n, &self.command_query);
                            (s >= 0).then_some((s, n, cmd))
                        })
                        .collect();
                    cmds.sort_by(|a, b| b.0.cmp(&a.0));
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        if let Some((_, _, cmd)) = cmds.first() {
                            run = Some(cmd.clone());
                        }
                    }
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for (i, (_, name, cmd)) in cmds.iter().enumerate() {
                            let (cat, rest) = name.split_once('：').unwrap_or(("", name.as_str()));
                            let r = ui.add_sized(
                                [ui.available_width(), 26.0],
                                egui::SelectableLabel::new(
                                    i == 0,
                                    RichText::new(format!("{}  {}", cat, rest)).color(if i == 0 { c.accent } else { c.fg_list }),
                                ),
                            );
                            if r.clicked() {
                                run = Some(cmd.clone());
                            }
                        }
                    });
                });
            if let Some(cmd) = run {
                self.show_command_palette = false;
                self.run_cmd(cmd);
            } else if !open {
                self.show_command_palette = false;
            }
        }

        // 跳至行
        if self.show_goto {
            let mut open = true;
            egui::Window::new(RichText::new("GO TO LINE").font(FontId::new(12.0, FontFamily::Name(ORBITRON.into()))).color(c.accent))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_TOP, [0.0, 80.0])
                .show(ctx, |ui| {
                    let total = self.active_buffer().map_or(1, |b| b.lines.len());
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.goto_text)
                            .hint_text(format!("行號 (1 - {})", total))
                            .desired_width(240.0),
                    );
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        if let Ok(n) = self.goto_text.trim().parse::<usize>() {
                            let line = n.clamp(1, total) - 1;
                            if let Some(buf) = self.active_buffer() {
                                let ci = Self::char_index_of(buf, line, 0);
                                self.pending_select = Some((ci, ci));
                                self.scroll_to_cursor = true;
                            }
                        }
                        self.show_goto = false;
                    }
                });
            if !open {
                self.show_goto = false;
            }
        }

        // 全專案搜尋：收取背景執行緒的結果
        if let Some(job) = &self.find_job {
            if let Ok((matches, stats)) = job.rx.try_recv() {
                self.status_message = if stats.canceled {
                    format!("搜尋已取消（已找到 {} 筆）", matches.len())
                } else if stats.truncated {
                    format!("找到 {} 筆結果（已達上限，僅顯示前 {} 筆）", matches.len(), matches.len())
                } else {
                    format!("找到 {} 筆結果（掃描 {} 個檔案）", matches.len(), stats.files_scanned)
                };
                self.find_results = matches;
                self.find_job = None;
            }
        }
        if self.show_find_in_files {
            let mut open = true;
            let mut to_open: Option<(String, usize)> = None;
            egui::Window::new(RichText::new("FIND IN FILES").font(FontId::new(12.0, FontFamily::Name(ORBITRON.into()))).color(c.accent))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([620.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                        paint_icon(ui.painter(), r, IconKind::FindInFiles, c.fg, c.accent);
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.find_query)
                                .hint_text("搜尋文字…")
                                .desired_width(ui.available_width() - 90.0),
                        );
                        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                        if (ui.button("搜尋").clicked() || enter) && !self.find_query.is_empty() {
                            // 在背景執行緒搜尋：大型資料夾 / 網路磁碟不會凍結視窗
                            if let Some(job) = self.find_job.take() {
                                job.cancel.store(true, Ordering::Relaxed);
                            }
                            let cancel = Arc::new(AtomicBool::new(false));
                            let (tx, rx) = unbounded();
                            let (root, query, flag) = (self.project_root.clone(), self.find_query.clone(), cancel.clone());
                            let repaint = ui.ctx().clone();
                            std::thread::spawn(move || {
                                let _ = tx.send(search_in_files(&root, &[], &query, &flag, 500));
                                repaint.request_repaint();
                            });
                            self.find_job = Some(FindJob { rx, cancel });
                            self.find_results.clear();
                            self.status_message = "搜尋中…".to_string();
                        }
                    });
                    if self.find_job.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("搜尋中…").color(c.fg_dim));
                            if ui.small_button("取消").clicked() {
                                if let Some(job) = &self.find_job {
                                    job.cancel.store(true, Ordering::Relaxed);
                                }
                            }
                        });
                    }
                    ui.label(RichText::new(format!("範圍：{}", self.project_root.display())).size(11.5).color(c.fg_dim2));
                    if ui.small_button("變更資料夾…").clicked() {
                        if let Some(d) = rfd::FileDialog::new().set_directory(&self.project_root).pick_folder() {
                            self.project_root = d;
                        }
                    }
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        for m in &self.find_results {
                            let name = Path::new(&m.file_path)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| m.file_path.clone());
                            ui.horizontal(|ui| {
                                let r = ui.add(
                                    egui::Label::new(RichText::new(format!("{}:{}", name, m.line)).color(c.accent))
                                        .sense(Sense::click()),
                                );
                                ui.label(RichText::new(m.snippet.trim()).monospace().color(c.fg_dim));
                                if r.on_hover_text(&m.file_path).clicked() {
                                    to_open = Some((m.file_path.clone(), m.line));
                                }
                            });
                        }
                    });
                });
            if let Some((p, line)) = to_open {
                let full = if Path::new(&p).is_absolute() { PathBuf::from(&p) } else { self.project_root.join(&p) };
                self.open_path(&full);
                if let Some(buf) = self.active_buffer() {
                    let ci = Self::char_index_of(buf, line.saturating_sub(1), 0);
                    self.pending_select = Some((ci, ci));
                    self.scroll_to_cursor = true;
                }
            }
            if !open {
                self.show_find_in_files = false;
                if let Some(job) = &self.find_job {
                    job.cancel.store(true, Ordering::Relaxed);
                }
            }
        }

        self.unsaved_dialog(ctx, c);

        // 關於
        if self.show_about {
            let mut open = true;
            egui::Window::new("About")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([360.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        if let Some(tex) = &self.logo {
                            ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(128.0, 128.0)));
                        }
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("ALEXCODE")
                                .font(FontId::new(26.0, FontFamily::Name(ORBITRON.into())))
                                .color(c.accent),
                        );
                        ui.label(
                            RichText::new("NEON EDITION · v5.3 RUST PORT")
                                .font(FontId::new(10.0, FontFamily::Name(ORBITRON.into())))
                                .color(c.accent2),
                        );
                        ui.add_space(8.0);
                        ui.label(RichText::new("快速、未來感的 Notepad++ 風格程式碼編輯器").color(c.fg_dim));
                        ui.label(RichText::new("以 Rust + egui 重寫").color(c.fg_dim2));
                        ui.add_space(4.0);
                        ui.hyperlink_to("github.com/alex721chiang/alexcode_editor", "https://github.com/alex721chiang/alexcode_editor");
                        ui.add_space(10.0);
                        if ui.button("  確定  ").clicked() {
                            open = false;
                        }
                        ui.add_space(6.0);
                    });
                });
            if !open {
                self.show_about = false;
            }
        }
    }
}

impl AlexCodeApp {
    /// 「尚未儲存的變更」對話框：儲存 / 不儲存 / 取消
    fn unsaved_dialog(&mut self, ctx: &egui::Context, c: &Colors) {
        let Some(pending) = &self.pending_close else { return };
        let dirty: Vec<(u64, String)> = self
            .tabs
            .iter()
            .filter(|b| b.modified && pending.ids.contains(&b.id))
            .map(|b| (b.id, Self::tab_title(b)))
            .collect();
        let exit = pending.exit;
        if dirty.is_empty() {
            let p = self.pending_close.take().unwrap();
            self.finish_close(&p.ids, p.exit);
            return;
        }
        // 蓋住主畫面，對話框處理完之前不能操作後面的內容
        egui::Area::new(egui::Id::new("unsaved-dim"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let r = ctx.screen_rect();
                ui.allocate_response(r.size(), Sense::click());
                ui.painter().rect_filled(r, 0.0, Color32::from_black_alpha(140));
            });
        // 0 = 取消, 1 = 儲存, 2 = 不儲存
        let mut choice: Option<u8> = None;
        // 對話框開著時，後面的輸入框不該繼續收到鍵盤輸入
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        egui::Area::new(egui::Id::new("unsaved-dialog"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.style()).show(ui, |ui| {
                ui.set_min_width(380.0);
                ui.label(RichText::new("尚未儲存的變更").color(c.accent).strong());
                ui.add_space(6.0);
                ui.label(if exit {
                    "結束前，下列檔案有尚未儲存的變更："
                } else {
                    "下列檔案有尚未儲存的變更："
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (_, name) in &dirty {
                        ui.label(RichText::new(format!("  ●  {}", name)).color(c.accent2));
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let save = if dirty.len() > 1 { "全部儲存" } else { "儲存" };
                    if ui.button(RichText::new(save).color(c.accent).strong()).clicked() {
                        choice = Some(1);
                    }
                    if ui.button("不儲存").clicked() {
                        choice = Some(2);
                    }
                    if ui.button("取消").clicked() {
                        choice = Some(0);
                    }
                });
                });
            });
        match choice {
            Some(0) => self.pending_close = None,
            Some(1) => {
                let p = self.pending_close.take().unwrap();
                for (id, name) in &dirty {
                    let saved = self.tabs.iter().position(|b| b.id == *id).map_or(false, |i| self.save_tab(i, false));
                    if !saved {
                        // 任何一個沒存成功（取消另存對話框、寫入失敗、編碼無法表示）就整個取消，不關閉也不結束
                        if !self.status_message.starts_with("未儲存") && !self.status_message.starts_with("儲存失敗") {
                            self.status_message = format!("「{}」尚未儲存，已取消關閉", name);
                        }
                        return;
                    }
                }
                self.finish_close(&p.ids, p.exit);
            }
            Some(_) => {
                let p = self.pending_close.take().unwrap();
                self.finish_close(&p.ids, p.exit);
            }
            None => {}
        }
    }
}

impl eframe::App for AlexCodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame(ctx);
    }
}

impl AlexCodeApp {
    /// 一幀的完整 UI（與 eframe 無關，測試可用無視窗的 `egui::Context` 直接驅動）
    pub fn ui_frame(&mut self, ctx: &egui::Context) {
        let pal = self.theme_mgr.current_palette().clone();
        let colors = Colors::from_palette(&pal, self.theme_mgr.is_neon_hud());

        // 主題切換時才重新套用樣式
        if self.applied_theme != self.theme_mgr.current_theme_name() {
            style::apply_style(ctx, &colors);
            self.applied_theme = self.theme_mgr.current_theme_name().to_string();
        }
        // 延遲載入龍形 Logo 貼圖
        if self.logo.is_none() {
            if let Some(img) = style::decode_png(style::ICON_PNG) {
                self.logo = Some(ctx.load_texture("alexcode-logo", img, egui::TextureOptions::LINEAR));
            }
        }

        // 視窗標題顯示目前檔案（只在標題真的改變時才送出命令）
        if let Some(buf) = self.active_buffer() {
            let title = format!(
                "{}{} — AlexCode Neon Edition",
                if buf.modified { "● " } else { "" },
                Self::tab_title(buf)
            );
            if title != self.last_title {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
                self.last_title = title;
            }
        }

        // 結束程式（視窗 X、Alt+F4、選單「結束」）：有未儲存的變更時攔下並詢問
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_exit {
            self.commit_md_edit();
            if self.tabs.iter().any(|b| b.modified) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                let ids = self.tabs.iter().map(|b| b.id).collect();
                self.request_close(ids, true);
            }
        }
        if self.allow_exit && self.pending_close.is_none() && !ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(text) = self.pending_copy.take() {
            self.status_message = format!("已複製路徑：{}", text);
            ctx.copy_text(text);
        }

        // 詢問對話框開著時不處理快捷鍵（Esc 取消除外）
        if self.pending_close.is_some() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.pending_close = None;
            }
        } else {
            self.handle_shortcuts(ctx);
        }

        self.menu_bar(ctx, &colors);
        self.tool_bar(ctx, &colors);
        self.status_bar(ctx, &colors);
        self.terminal_dock(ctx, &colors);
        self.graph_dock(ctx, &colors);
        self.tab_bar(ctx, &colors);
        self.find_bar(ctx, &colors);
        self.editor_area(ctx, &colors);
        self.popup_windows(ctx, &colors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, RawInput, ViewportCommand, ViewportEvent, ViewportId};

    fn ctx() -> egui::Context {
        let ctx = egui::Context::default();
        style::setup_fonts(&ctx);
        ctx
    }

    fn frame_with(app: &mut AlexCodeApp, ctx: &egui::Context, edit: impl FnOnce(&mut RawInput)) -> egui::FullOutput {
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            focused: true,
            ..Default::default()
        };
        edit(&mut input);
        ctx.run(input, |ctx| app.ui_frame(ctx))
    }

    fn frame(app: &mut AlexCodeApp, ctx: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
        frame_with(app, ctx, |i| i.events = events)
    }

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    /// 開一個只有單一程式碼分頁的 App，並讓編輯區取得焦點
    fn app_with(ctx: &egui::Context, text: &str) -> AlexCodeApp {
        let mut app = AlexCodeApp::new();
        let mut b = EditorBuffer::from_text(text);
        b.file_path = Some("t.rs".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        app.pending_select = Some((0, 0));
        frame(&mut app, ctx, vec![]);
        frame(&mut app, ctx, vec![]);
        assert!(app.editor_focused, "editor should have keyboard focus");
        app
    }

    #[test]
    fn big5_file_survives_edit_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big5.txt");
        // 「測試」+ CRLF + "abc"
        let original = [0xB4, 0xFA, 0xB8, 0xD5, b'\r', b'\n', b'a', b'b', b'c'];
        std::fs::write(&path, original).unwrap();

        let mut app = AlexCodeApp::new();
        app.open_path(&path);
        let idx = app.active_tab_idx;
        assert_eq!(app.tabs[idx].encoding, FileEncoding::Big5);
        assert_eq!(app.tabs[idx].lines, vec!["測試", "abc"]);

        // 未修改直接存：位元組完全相同
        assert!(app.save_tab(idx, false));
        assert_eq!(std::fs::read(&path).unwrap(), original);

        // 修改後存：仍是 Big5，其餘位元組不變
        app.tabs[idx].replace_all_text("測試\nabcd");
        assert!(app.save_tab(idx, false));
        let mut expected = original.to_vec();
        expected.push(b'd');
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        assert!(!app.tabs[idx].modified);

        // 加入 Big5 無法表示的字元：拒絕寫入，檔案保持原樣
        app.tabs[idx].replace_all_text("測試😀");
        assert!(!app.save_tab(idx, false));
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        assert!(app.tabs[idx].modified);
    }

    #[test]
    fn closing_modified_tab_asks_first() {
        let mut app = AlexCodeApp::new();
        let n = app.tabs.len();
        app.tabs[0].replace_all_text("changed");
        app.close_tab(0);
        assert_eq!(app.tabs.len(), n, "modified tab must not be dropped silently");
        assert!(app.pending_close.is_some());

        // 未修改的分頁直接關閉
        app.pending_close = None;
        app.close_tab(1);
        assert_eq!(app.tabs.len(), n - 1);
        assert!(app.pending_close.is_none());
    }

    #[test]
    fn discard_choice_closes_and_keeps_active_tab() {
        let mut app = AlexCodeApp::new();
        app.new_tab(); // 3 個分頁，作用中為最後一個
        let keep = app.tabs[2].id;
        app.tabs[0].replace_all_text("changed");
        let ids = vec![app.tabs[0].id, app.tabs[1].id];
        app.request_close(ids.clone(), false);
        assert!(app.pending_close.is_some());
        let p = app.pending_close.take().unwrap();
        app.finish_close(&p.ids, p.exit); // 「不儲存」
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs[app.active_tab_idx].id, keep);
    }

    #[test]
    fn window_close_is_cancelled_while_there_are_unsaved_changes() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn main() {}");
        app.tabs[0].replace_all_text("fn main() { changed }");
        let close = |i: &mut RawInput| {
            i.viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
        };
        let out = frame_with(&mut app, &ctx, close);
        let cmds = &out.viewport_output[&ViewportId::ROOT].commands;
        assert!(cmds.contains(&ViewportCommand::CancelClose), "close must be cancelled: {cmds:?}");
        assert!(app.pending_close.as_ref().map_or(false, |p| p.exit));

        // 沒有未儲存的變更時不攔截
        app.pending_close = None;
        app.tabs[0].mark_saved();
        let out = frame_with(&mut app, &ctx, close);
        assert!(!out.viewport_output[&ViewportId::ROOT].commands.contains(&ViewportCommand::CancelClose));
    }

    #[test]
    fn ctrl_z_and_toolbar_undo_share_one_history() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "abc");
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!(app.tabs[0].edit_text(), "Xabc");
        assert!(app.tabs[0].modified);

        // Ctrl+Z 走文件的復原紀錄（不是 TextEdit 內建的那一套）
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "abc");
        assert!(!app.tabs[0].modified);

        // 工具列 / 選單的「重做」能重做 Ctrl+Z 復原的內容
        app.run_cmd(Cmd::Redo);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "Xabc");

        // Ctrl+Y 與工具列「復原」交錯使用也一致
        app.run_cmd(Cmd::Undo);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "abc");
        frame(&mut app, &ctx, vec![key(Key::Y, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "Xabc");
    }

    #[test]
    fn typing_after_undo_edits_the_restored_text() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "abc");
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![Event::Text("Y".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "Yabc");
    }

    #[test]
    fn filter_and_find_results_follow_edits() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "error one\nok\nerror two");
        app.filter_query = "error".into();
        app.filter_engine.compile("error");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.filter_cache.1, vec![0, 2]);
        app.tabs[0].replace_all_text("error one\nok\nfine");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.filter_cache.1, vec![0]);

        app.filter_query.clear();
        app.filter_engine.compile("");
        app.show_find_bar = true;
        app.find_bar_text = "o".into();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 3); // err[o]r、[o]ne、[o]k
        app.tabs[0].replace_all_text("zzz");
        frame(&mut app, &ctx, vec![]);
        assert!(app.find_cache.1.is_empty());
    }

    #[test]
    fn layout_cache_is_dropped_when_font_atlas_is_rebuilt() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn main() {}");
        frame(&mut app, &ctx, vec![]);
        let before = app.highlight_cache.as_ref().unwrap().1.clone();
        frame(&mut app, &ctx, vec![]);
        assert!(Arc::ptr_eq(&before, &app.highlight_cache.as_ref().unwrap().1), "idle frames reuse the layout");

        // 縮放改變 → egui 重建字型圖集 → 舊排版不可再用
        ctx.set_pixels_per_point(2.0);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(!Arc::ptr_eq(&before, &app.highlight_cache.as_ref().unwrap().1));
    }

    #[test]
    fn tab_dir_uses_file_folder_or_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "x").unwrap();
        let mut app = AlexCodeApp::new();
        let root = app.project_root.clone();
        assert_eq!(app.tab_dir(0), root, "unsaved tab falls back to the project folder");
        app.open_path(&path);
        assert_eq!(app.tab_dir(app.active_tab_idx), dir.path());
    }

    /// 量測閒置（沒有輸入）時每一幀的成本：`cargo test --release idle_frame -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn idle_frame_cost_report() {
        let ctx = ctx();
        let text: String = (0..20_000).map(|i| format!("    let value_{i} = compute(\"{i}\"); // error line {i}\n")).collect();
        let mut app = app_with(&ctx, &text);
        app.show_find_bar = true;
        app.find_bar_text = "error".into();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let t = std::time::Instant::now();
        for _ in 0..20 {
            frame(&mut app, &ctx, vec![]);
        }
        println!("IDLE_FRAME_MS editor+find = {:.2}", t.elapsed().as_secs_f64() * 1000.0 / 20.0);

        app.show_find_bar = false;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let t = std::time::Instant::now();
        for _ in 0..20 {
            frame(&mut app, &ctx, vec![]);
        }
        println!("IDLE_FRAME_MS editor only = {:.2}", t.elapsed().as_secs_f64() * 1000.0 / 20.0);
        app.filter_query = "error".into();
        app.filter_engine.compile("error");
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let t = std::time::Instant::now();
        for _ in 0..20 {
            frame(&mut app, &ctx, vec![]);
        }
        println!("IDLE_FRAME_MS filter view = {:.2}", t.elapsed().as_secs_f64() * 1000.0 / 20.0);
    }
}
