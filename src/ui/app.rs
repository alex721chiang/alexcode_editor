//! AlexCode 主應用程式介面與事件迴圈 (Egui / Eframe)
//!
//! 介面配置對應原版 Qt `MainWindow`：
//! - 選單列（龍形 Logo + Orbitron 字標）
//! - 主工具列（霓虹線條圖示：新增 / 開啟 / 儲存 | 復原 / 重做 | 搜尋）+ ⚡ FILTER 篩選工具列
//! - 平面分頁列（強調色底線指示）
//! - 程式碼編輯區（行號 gutter、目前行高亮、語法高亮、搜尋命中底色）+ Neon HUD 角標
//! - 底部 Dock（終端機 / Markdown 關聯圖，左側強調色識別條）
//! - 狀態列（漸層 + 光暈上邊框）

use super::highlight::{highlight, highlight_ranges, Lang};
use super::icons::{paint_icon, tool_button, IconKind};
use super::markdown_view::{
    render_markdown_toolbar, render_outline_panel, render_typora_document, MarkdownViewState, MdAction, MdViewMode,
};
use super::style::{self, hgradient, paint_hud_corners, vgradient, with_alpha, Colors, ORBITRON};
use crate::core::{
    available_shells, list_dir, open_terminal, reveal_in_file_manager, stamp_of, AppState, DirEntryInfo, EditorConfig,
    FileWatcher, ShellKind, Stamp,
};
use crate::editor::{
    decide as pair_decide, detect_and_decode, is_relevant as pair_relevant, should_delete_pair, Action as PairAction,
    sticky_headers, EditorBuffer, FileEncoding, Position, Selection,
};
use crate::core::config::{MAX_FONT_SIZE, MAX_TAB_SIZE, MIN_FONT_SIZE, MIN_TAB_SIZE};
use crate::index::{extract_symbols, Symbol, SymbolLang};
use crate::git::{diff_line_states, ADDED, DELETED_ABOVE, MODIFIED};
use crate::filter::{FilterEngine, FilterLogic};
use crate::markdown::{compute_graph_layout, MarkdownLinkIndex};
use crate::plugins::PluginManager;
use crate::search::{command_match_score, find_all, replace_all, replace_range, search_in_files, FileMatch, FindOptions, SearchStats};
use crate::search::{apply_replace_in_files, plan_replace_in_files, ReplaceHit, ReplaceResult};
use crate::snippets::SnippetsController;
use crate::terminal::VtParser;
use crate::theme::ThemeManager;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{
    self, text::CCursor, text::CCursorRange, Align, Align2, Color32, FontFamily, FontId, Key, KeyboardShortcut,
    Layout, Modifiers, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const WELCOME: &str = r##"// ========================================================
//  AlexCode — Neon Edition (Rust)
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
    OpenFolder,
    OpenRecent(String),
    ToggleExplorer,
    Replace,
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
    Settings,
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
    /// 上一幀結束時編輯區是否有焦點（本幀處理輸入事件前的狀態）
    editor_was_focused: bool,
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

    // ---- 跨次啟動的狀態 ----
    /// `state.json` 的位置；None（測試）時不讀寫
    state_path: Option<PathBuf>,
    /// 上次寫入磁碟的內容（相同就不重寫）
    saved_state: AppState,
    last_persist: std::time::Instant,
    recent_files: Vec<String>,
    /// 專案資料夾由使用者以「開啟資料夾」指定：之後開檔不再改變它
    root_pinned: bool,

    // ---- 檔案總管側邊欄 ----
    show_explorer: bool,
    tree_cache: HashMap<PathBuf, TreeDir>,
    tree_expanded: HashSet<PathBuf>,
    tree_chan: (Sender<(PathBuf, Result<Vec<DirEntryInfo>, String>)>, Receiver<(PathBuf, Result<Vec<DirEntryInfo>, String>)>),

    // ---- 尋找 / 取代 ----
    find_opts: FindOptions,
    show_replace: bool,
    replace_text: String,
    find_error: Option<String>,
    /// 下一次套用 `pending_select` 時不要把鍵盤焦點移到編輯區（從尋找列操作時，焦點要留在尋找列）
    select_keeps_focus: bool,
    /// 目前的命中（字元區間, 當時的內容版本）：編輯區沒有焦點時以外框標示
    find_current: Option<((usize, usize), u64)>,
    /// 下一幀把鍵盤焦點交還給編輯區（關閉尋找列時）
    focus_editor: bool,

    // ---- 外部變更偵測 ----
    watcher: Option<FileWatcher>,
    /// 各分頁（依 id）上次讀寫磁碟時的檔案版本
    disk_stamps: HashMap<u64, Option<Stamp>>,
    /// 有未儲存修改、同時又被外部修改的分頁 id：等待使用者決定
    disk_conflict: Option<u64>,

    // ---- 單一實例：其他實例交來的檔案 ----
    incoming: Option<Receiver<Vec<PathBuf>>>,

    // ---- Git 行號區標記 ----
    /// 各分頁（依 id）在 HEAD 的內容；None = 不在 Git 儲存庫內或未追蹤
    git_head: HashMap<u64, Option<Arc<Vec<String>>>>,
    git_chan: (Sender<(u64, Option<Vec<String>>)>, Receiver<(u64, Option<Vec<String>>)>),
    /// 目前分頁的行狀態快取：鍵為 (分頁 id, 內容版本)
    git_marks: ((u64, u64), HashMap<usize, u8>),

    // ---- Sticky Scroll ----
    /// 目前分頁的符號（函式 / 類別…區塊）快取：鍵為 (分頁 id, 內容版本, 語言)
    sticky_key: Option<(u64, u64, Lang)>,
    sticky_syms: Arc<Vec<Symbol>>,
    /// 上一幀固定在編輯區頂端的標頭：(行號, 畫面上的位置)
    sticky_rows: Vec<(usize, Rect)>,
    /// 上一幀沒有被固定標頭蓋住的第一行
    sticky_below: usize,

    // ---- 自動換行 ----
    /// 上一幀編輯區每一列（換行後畫面上的一列）所屬的行號；沒有開啟自動換行時為空
    row_lines: Vec<usize>,
    /// 上一幀編輯區是否以自動換行排版（用來察覺設定剛被切換）
    wrap_applied: bool,

    // ---- 全專案取代 ----
    /// 「全專案搜尋」視窗中「取代為」欄位的內容
    replace_in_files_text: String,
    /// 背景執行中的取代預覽 / 寫入
    replace_job: Option<ReplaceJob>,
    /// 已預覽、等待使用者按「執行取代」確認的內容
    replace_plan: Option<ReplacePlan>,
    /// 顯示在視窗中的結果或錯誤：(是否為錯誤, 訊息)
    replace_note: Option<(bool, String)>,

    // ---- 背景開檔 ----
    /// 內容還在背景執行緒讀取的分頁（依 id）：分頁已經出現，但還不能編輯或存檔
    loading: HashMap<u64, LoadJob>,

    // ---- 設定對話框 ----
    show_settings: bool,
    /// 測試用：上一幀具名控制項在畫面上的位置（讓無視窗測試能點到它們）
    #[cfg(test)]
    probe: HashMap<&'static str, Rect>,

    repaint: Option<egui::Context>,
}

/// 固定在編輯區頂端的標頭最多幾層（超過時保留最內層）
const STICKY_MAX_ROWS: usize = 5;

enum TreeDir {
    Loading,
    Loaded(Arc<Vec<DirEntryInfo>>),
    Failed(String),
}

enum TreeAction {
    Open(PathBuf),
    Toggle(PathBuf),
    Reveal(PathBuf),
    Shell(PathBuf, ShellKind),
    CopyPath(PathBuf),
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

/// 全專案取代的預覽結果：只有搜尋字串、取代字串、資料夾都沒變時才能執行
struct ReplacePlan {
    root: PathBuf,
    query: String,
    replacement: String,
    hits: Vec<ReplaceHit>,
    /// 所有檔案合計會取代幾處
    total: usize,
}

enum ReplaceMsg {
    Planned(Vec<ReplaceHit>, SearchStats),
    Applied(ReplaceResult),
}

struct ReplaceJob {
    rx: Receiver<ReplaceMsg>,
    cancel: Arc<AtomicBool>,
    /// 這個工作所依據的預覽條件（寫入完成後用來找出要重新載入的分頁）
    plan: ReplacePlan,
}

/// 背景讀取中的檔案：讀檔、偵測編碼、切行都在背景執行緒完成，結果是一個可以直接換上的緩衝區
struct LoadJob {
    rx: Receiver<Result<EditorBuffer, String>>,
    /// 載入完成後要跳到的行（從全專案搜尋的結果開檔時）
    goto_line: Option<usize>,
}

/// 達到這個大小的檔案改在背景讀取（網路磁碟上的檔案不論大小一律在背景讀取）
const BACKGROUND_OPEN_BYTES: u64 = 2 * 1024 * 1024;

/// 兩個路徑字串是否指向同一個檔案（Windows 上不分大小寫與斜線方向）
fn same_file(a: &str, b: &str) -> bool {
    Path::new(a) == Path::new(b) || (cfg!(windows) && a.replace('/', "\\").eq_ignore_ascii_case(&b.replace('/', "\\")))
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
            editor_was_focused: false,
            text_cache: ((0, 0), String::new()),
            highlight_cache: None,
            find_cache: ((0, 0, String::new()), Vec::new()),
            filter_cache: (None, Vec::new()),
            find_job: None,
            last_title: String::new(),
            pending_copy: None,
            state_path: None,
            saved_state: AppState::default(),
            last_persist: std::time::Instant::now(),
            recent_files: Vec::new(),
            root_pinned: false,
            show_explorer: false,
            tree_cache: HashMap::new(),
            tree_expanded: HashSet::new(),
            tree_chan: unbounded(),
            find_opts: FindOptions::default(),
            show_replace: false,
            replace_text: String::new(),
            find_error: None,
            select_keeps_focus: false,
            find_current: None,
            focus_editor: false,
            watcher: None,
            disk_stamps: HashMap::new(),
            disk_conflict: None,
            incoming: None,
            git_head: HashMap::new(),
            git_chan: unbounded(),
            git_marks: ((0, 0), HashMap::new()),
            sticky_key: None,
            sticky_syms: Arc::new(Vec::new()),
            sticky_rows: Vec::new(),
            sticky_below: 0,
            row_lines: Vec::new(),
            wrap_applied: false,
            replace_in_files_text: String::new(),
            replace_job: None,
            replace_plan: None,
            replace_note: None,
            loading: HashMap::new(),
            show_settings: false,
            #[cfg(test)]
            probe: HashMap::new(),
            repaint: None,
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
        app.repaint = Some(cc.egui_ctx.clone());
        // 背景檢查已開啟的檔案是否被其他程式修改
        let ctx = cc.egui_ctx.clone();
        app.watcher = Some(FileWatcher::new(std::time::Duration::from_millis(1000), move || ctx.request_repaint()));
        // 還原上次的主題、字型、資料夾與開啟的檔案
        app.state_path = crate::core::config_dir().map(|d| d.join("state.json"));
        if let Some(st) = app.state_path.as_deref().and_then(AppState::load) {
            app.apply_state(st);
        }
        // 命令列參數：直接開啟檔案
        for arg in std::env::args().skip(1) {
            let p = PathBuf::from(&arg);
            if p.is_file() {
                app.open_path(&std::path::absolute(&p).unwrap_or(p));
            }
        }
        app.saved_state = app.snapshot_state();
        app
    }

    /// 接上單一實例的通道：之後其他實例交來的檔案會在本視窗開啟
    pub fn set_incoming(&mut self, rx: Receiver<Vec<PathBuf>>) {
        self.incoming = Some(rx);
    }

    /// 已存檔（磁碟上有對應檔案）的分頁路徑
    fn file_tabs(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter_map(|b| b.file_path.clone())
            .filter(|p| Path::new(p).is_absolute())
            .collect()
    }

    fn snapshot_state(&self) -> AppState {
        AppState {
            theme: self.theme_mgr.current_theme_name().to_string(),
            font_size: self.config.font_size,
            open_files: self.file_tabs(),
            active_file: self
                .active_buffer()
                .and_then(|b| b.file_path.clone())
                .filter(|p| Path::new(p).is_absolute()),
            recent_files: self.recent_files.clone(),
            folder: self.root_pinned.then(|| self.project_root.to_string_lossy().to_string()),
            show_explorer: self.show_explorer,
            auto_pair: self.config.auto_pair,
            config: Some(EditorConfig {
                theme: self.theme_mgr.current_theme_name().to_string(),
                ..self.config.clone()
            }),
        }
    }

    /// 套用上次保存的狀態；已不存在的檔案 / 資料夾略過
    fn apply_state(&mut self, st: AppState) {
        if ThemeManager::theme_names().iter().any(|n| *n == st.theme) {
            self.theme_mgr.set_theme(&st.theme);
        }
        match st.config {
            // 主題由 ThemeManager 管理：config.theme 只是存檔時的副本
            Some(cfg) => self.config = EditorConfig { theme: self.config.theme.clone(), ..cfg.sanitized() },
            // 沒有設定對話框的舊版本存的檔：沿用當時的字型大小與自動配對
            None => {
                if (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&st.font_size) {
                    self.config.font_size = st.font_size;
                }
                self.config.auto_pair = st.auto_pair;
            }
        }
        self.recent_files = st.recent_files;
        self.show_explorer = st.show_explorer;
        if !self.config.restore_session {
            return; // 偏好設定與「最近開啟」照常還原，但不重新開啟上次的資料夾與檔案
        }
        if let Some(dir) = st.folder.map(PathBuf::from).filter(|d| d.is_dir()) {
            self.set_folder(dir);
            self.show_explorer = st.show_explorer;
        }
        let recent = self.recent_files.clone(); // 還原分頁不該改變「最近開啟」的順序
        for f in &st.open_files {
            let p = PathBuf::from(f);
            if p.is_file() {
                self.open_path(&p);
            }
        }
        self.recent_files = recent;
        if let Some(i) = st.active_file.and_then(|a| self.tabs.iter().position(|b| b.file_path.as_deref() == Some(a.as_str()))) {
            self.active_tab_idx = i;
        }
    }

    /// 狀態有變就寫回 `state.json`（測試中 `state_path` 為 None，不寫）
    fn persist_state(&mut self) {
        self.last_persist = std::time::Instant::now();
        let Some(path) = self.state_path.clone() else { return };
        let st = self.snapshot_state();
        if st != self.saved_state && st.save(&path).is_ok() {
            self.saved_state = st;
        }
    }

    fn push_recent(&mut self, path: &str) {
        let mut st = AppState { recent_files: std::mem::take(&mut self.recent_files), ..Default::default() };
        st.push_recent(path);
        self.recent_files = st.recent_files;
    }

    /// 指定專案資料夾（檔案總管的根目錄、全專案搜尋的範圍）
    fn set_folder(&mut self, dir: PathBuf) {
        self.project_root = dir.clone();
        self.term_cwd = dir.clone();
        self.root_pinned = true;
        self.show_explorer = true;
        self.tree_cache.clear();
        self.tree_expanded.clear();
        self.request_dir(dir);
    }

    /// 在背景執行緒列出資料夾內容（網路磁碟上列舉可能很慢，不能卡住介面）
    fn request_dir(&mut self, dir: PathBuf) {
        self.tree_cache.insert(dir.clone(), TreeDir::Loading);
        let tx = self.tree_chan.0.clone();
        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            let res = list_dir(&dir).map_err(|e| e.to_string());
            let _ = tx.send((dir, res));
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
    }

    /// 在背景取得分頁檔案於 Git HEAD 的內容（行號區的新增 / 修改 / 刪除標記用）
    fn request_git_head(&mut self, idx: usize) {
        let Some(buf) = self.tabs.get(idx) else { return };
        let Some(path) = buf.file_path.clone().map(PathBuf::from).filter(|p| p.is_absolute()) else { return };
        let (id, tx, repaint) = (buf.id, self.git_chan.0.clone(), self.repaint.clone());
        std::thread::spawn(move || {
            let _ = tx.send((id, crate::git::head_lines(&path)));
            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
    }

    /// 分頁內容剛從磁碟讀入或寫回磁碟後呼叫：記下磁碟版本，之後才能分辨「外部修改」與「自己存的檔」
    fn note_disk_state(&mut self, idx: usize) {
        let Some(buf) = self.tabs.get(idx) else { return };
        if let Some(path) = buf.file_path.as_deref().map(Path::new).filter(|p| p.is_absolute()) {
            self.disk_stamps.insert(buf.id, stamp_of(path));
        }
    }

    /// 處理背景偵測到的磁碟變更
    fn handle_disk_events(&mut self, events: Vec<(PathBuf, Option<Stamp>)>) {
        for (path, stamp) in events {
            let key = path.to_string_lossy().to_string();
            let Some(idx) = self.tabs.iter().position(|b| b.file_path.as_deref() == Some(key.as_str())) else { continue };
            let id = self.tabs[idx].id;
            if self.loading.contains_key(&id) {
                continue; // 內容還在背景載入：讀到的就會是最新的版本
            }
            if self.disk_stamps.get(&id) == Some(&stamp) {
                continue; // 就是我們自己剛存的版本
            }
            self.disk_stamps.insert(id, stamp);
            if stamp.is_none() {
                self.status_message = format!("「{}」已被刪除或移動（內容仍保留在分頁中）", Self::tab_title(&self.tabs[idx]));
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let (text, _) = self.tabs[idx].encoding.decode(&bytes);
            if EditorBuffer::from_text(&text).lines == self.tabs[idx].lines {
                continue; // 內容其實沒變（例如只是被 touch）
            }
            if self.tabs[idx].modified {
                self.disk_conflict = Some(id); // 交給使用者決定
            } else {
                self.reload_tab_from(idx, &text);
            }
        }
    }

    /// 以磁碟上的新內容更新分頁。游標原本在最後一行（看 log 的常見情況）時跟著移到新的結尾。
    fn reload_tab_from(&mut self, idx: usize, text: &str) {
        let buf = &mut self.tabs[idx];
        let at_end = buf.selections[0].head.line + 1 >= buf.lines.len();
        buf.reload_text(text);
        if at_end {
            let line = buf.lines.len() - 1;
            let col = buf.line_len(line);
            buf.selections = vec![Selection::point(Position { line, col })];
        }
        self.status_message = format!("「{}」已被其他程式修改，已重新載入", Self::tab_title(&self.tabs[idx]));
        if idx == self.active_tab_idx && self.editor_focused {
            self.sync_cursor_to_editor();
            self.scroll_to_cursor = at_end;
        }
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
        let alive: HashSet<u64> = self.tabs.iter().map(|b| b.id).collect();
        self.disk_stamps.retain(|id, _| alive.contains(id));
        self.git_head.retain(|id, _| alive.contains(id));
        self.loading.retain(|id, _| alive.contains(id));
        if self.disk_conflict.map_or(false, |id| !alive.contains(&id)) {
            self.disk_conflict = None;
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

    /// 這個檔案是否要在背景讀取。網路路徑只看路徑字串就決定：
    /// 連查詢檔案大小都可能在伺服器沒有回應時卡住視窗。
    fn opens_in_background(path: &Path) -> bool {
        crate::core::is_network_path(&path.to_string_lossy())
            || std::fs::metadata(path).is_ok_and(|m| m.len() >= BACKGROUND_OPEN_BYTES)
    }

    /// 讀檔並偵測編碼（UTF-8 / BOM / Big5 / GBK / Shift_JIS …）；編碼記在緩衝區上，存檔時以同一編碼寫回
    fn read_buffer(path: &Path) -> std::io::Result<EditorBuffer> {
        let bytes = std::fs::read(path)?;
        let (text, encoding) = detect_and_decode(&bytes);
        let mut buf = EditorBuffer::from_text(&text);
        buf.encoding = encoding;
        Ok(buf)
    }

    /// 把剛開啟的檔案加成分頁並切換過去（內容可能還在背景載入）
    fn add_file_tab(&mut self, mut buf: EditorBuffer, path: &Path, full: &str) {
        buf.file_path = Some(full.to_string());
        // 開啟第一個真正的檔案時，收掉未修改的歡迎頁 / 空白頁
        if self.file_tabs().is_empty() {
            self.tabs.retain(|b| b.modified);
        }
        self.tabs.push(buf);
        self.active_tab_idx = self.tabs.len() - 1;
        if Lang::from_path(full) == Lang::Markdown {
            self.md_view.mode = MdViewMode::Typora;
        }
        // 沒有以「開啟資料夾」指定專案資料夾時，才跟著目前檔案走
        if !self.root_pinned {
            if let Some(parent) = path.parent() {
                self.project_root = parent.to_path_buf();
            }
        }
        self.push_recent(full);
    }

    pub fn open_path(&mut self, path: &Path) {
        self.commit_md_edit();
        let full = path.to_string_lossy().to_string();
        if let Some(i) = self.tabs.iter().position(|b| b.file_path.as_deref() == Some(full.as_str())) {
            self.active_tab_idx = i;
            return;
        }
        if Self::opens_in_background(path) {
            // 大檔案 / 網路磁碟：分頁先出現並顯示「載入中」，讀檔交給背景執行緒，視窗不會凍結
            let buf = EditorBuffer::new();
            let id = buf.id;
            self.add_file_tab(buf, path, &full);
            let (tx, rx) = unbounded();
            let (path, repaint) = (path.to_path_buf(), self.repaint.clone());
            std::thread::spawn(move || {
                let _ = tx.send(Self::read_buffer(&path).map_err(|e| e.to_string()));
                if let Some(ctx) = repaint {
                    ctx.request_repaint();
                }
            });
            self.loading.insert(id, LoadJob { rx, goto_line: None });
            self.status_message = format!("載入中… {}", full);
            return;
        }
        match Self::read_buffer(path) {
            Ok(buf) => {
                self.add_file_tab(buf, path, &full);
                self.note_disk_state(self.active_tab_idx);
                self.request_git_head(self.active_tab_idx);
                self.status_message = format!("已開啟 {}", full);
            }
            Err(e) => self.status_message = format!("開啟失敗：{} ({})", full, e),
        }
    }

    /// 這個分頁的內容是否還在背景載入
    fn is_loading(&self, idx: usize) -> bool {
        self.tabs.get(idx).is_some_and(|b| self.loading.contains_key(&b.id))
    }

    /// 收取背景讀檔的結果
    fn poll_loading(&mut self, ctx: &egui::Context) {
        use crossbeam_channel::TryRecvError;
        let done: Vec<(u64, Result<EditorBuffer, String>)> = self
            .loading
            .iter()
            .filter_map(|(id, job)| match job.rx.try_recv() {
                Ok(res) => Some((*id, res)),
                Err(TryRecvError::Empty) => None,
                // 讀取執行緒沒有回報就結束了：當成失敗，不要永遠停在「載入中」
                Err(TryRecvError::Disconnected) => Some((*id, Err("讀取中斷".to_string()))),
            })
            .collect();
        for (id, res) in done {
            let Some(job) = self.loading.remove(&id) else { continue };
            // 載入中就被關掉的分頁：結果直接丟棄
            let Some(idx) = self.tabs.iter().position(|b| b.id == id) else { continue };
            let full = self.tabs[idx].file_path.clone().unwrap_or_default();
            match res {
                Ok(mut buf) => {
                    // 換上讀好的內容，分頁的身分（id、路徑）不變
                    buf.id = id;
                    buf.file_path = Some(full.clone());
                    self.tabs[idx] = buf;
                    // 各種快取以 (分頁 id, 內容版本) 為鍵，而新內容的版本與空白的佔位內容相同：
                    // 全部作廢，否則編輯區會拿到載入前的空白全文
                    self.text_cache = ((0, 0), String::new());
                    self.find_cache.0 = (0, 0, String::new());
                    self.filter_cache.0 = None;
                    self.git_marks.0 = (0, 0);
                    self.sticky_key = None;
                    self.note_disk_state(idx);
                    self.request_git_head(idx);
                    self.status_message = format!("已開啟 {}", full);
                    if let Some(line) = job.goto_line {
                        let line = line.min(self.tabs[idx].lines.len() - 1);
                        self.tabs[idx].selections = vec![Selection::point(Position { line, col: 0 })];
                    }
                    if idx == self.active_tab_idx {
                        // 把游標交給編輯區（它在載入期間沒有顯示）
                        let head = self.tabs[idx].selections[0].head;
                        let i = Self::char_index_of(&self.tabs[idx], head.line, head.col);
                        self.pending_select = Some((i, i));
                        self.scroll_to_cursor = job.goto_line.is_some();
                        // 使用者正在別的輸入框打字（尋找列、命令面板…）時不搶走鍵盤焦點
                        self.select_keeps_focus = ctx.memory(|m| m.focused().is_some());
                    }
                }
                Err(e) => {
                    // 與一般開檔失敗相同：不留下空白分頁
                    self.finish_close(&[id], false);
                    self.status_message = format!("開啟失敗：{} ({})", full, e);
                }
            }
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
        if self.loading.contains_key(&buf.id) {
            // 分頁裡還是空白的佔位內容：寫出去會蓋掉磁碟上真正的檔案
            self.status_message = format!("「{}」還在載入中，尚未儲存", Self::tab_title(buf));
            return false;
        }
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
                let full = path.to_string_lossy().to_string();
                let path_changed = b.file_path.as_deref() != Some(full.as_str());
                b.file_path = Some(full.clone());
                b.mark_saved();
                self.note_disk_state(idx);
                if path_changed {
                    self.request_git_head(idx);
                    self.push_recent(&full);
                }
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
        if self.loading.contains_key(&buf.id) {
            self.status_message = "檔案還在載入中，請等載入完成後再以其他編碼重新讀取".to_string();
            return;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                let (text, bad) = encoding.decode(&bytes);
                let mut nb = EditorBuffer::from_text(&text);
                nb.encoding = encoding;
                nb.file_path = Some(path);
                self.tabs[idx] = nb;
                self.note_disk_state(idx);
                self.request_git_head(idx);
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
            ("檔案：開啟資料夾 (Open Folder)".into(), Cmd::OpenFolder),
            ("檢視：檔案總管側邊欄 (Explorer)".into(), Cmd::ToggleExplorer),
            ("搜尋：取代 (Replace)".into(), Cmd::Replace),
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
            ("檔案：設定 (Settings)".into(), Cmd::Settings),
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
            Cmd::OpenFolder => {
                if let Some(dir) = rfd::FileDialog::new().set_directory(&self.project_root).pick_folder() {
                    self.set_folder(dir);
                }
            }
            Cmd::OpenRecent(path) => {
                let p = PathBuf::from(&path);
                if p.is_file() {
                    self.open_path(&p);
                } else {
                    self.recent_files.retain(|r| r != &path);
                    self.status_message = format!("檔案已不存在：{}", path);
                }
            }
            Cmd::ToggleExplorer => {
                self.show_explorer = !self.show_explorer;
                if self.show_explorer && !self.tree_cache.contains_key(&self.project_root) {
                    self.request_dir(self.project_root.clone());
                }
            }
            Cmd::Replace => {
                self.show_find_bar = true;
                self.show_replace = true;
                self.find_bar_focus = true;
            }
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
            Cmd::ZoomIn => self.config.font_size = (self.config.font_size + 1.0).min(MAX_FONT_SIZE),
            Cmd::ZoomOut => self.config.font_size = (self.config.font_size - 1.0).max(MIN_FONT_SIZE),
            Cmd::Settings => self.show_settings = true,
            Cmd::About => self.show_about = true,
            Cmd::Plugin(name) => {
                // 外掛指令產生的文字插入到游標位置
                if let Some(res) = self.plugin_mgr.run_command(&name) {
                    self.commit_md_edit();
                    if let Some(b) = self.active_buffer_mut() {
                        b.insert_text(&res);
                    }
                    self.sync_cursor_to_editor();
                    self.status_message = format!("已插入：{}", res);
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
        let shortcuts: [(Modifiers, Key, Cmd); 23] = [
            (Modifiers::COMMAND, Key::Comma, Cmd::Settings),
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
            (Modifiers::COMMAND, Key::H, Cmd::Replace),
            (Modifiers::COMMAND, Key::B, Cmd::ToggleExplorer),
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
            if self.show_find_bar {
                self.focus_editor = true; // 關閉尋找列後回到編輯區（游標停在目前的命中）
            }
            self.show_find_bar = false;
            self.show_goto = false;
            self.show_settings = false;
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
                        if mi(ui, "開啟資料夾 (Open Folder)…", "") { cmd = Some(Cmd::OpenFolder); }
                        ui.menu_button("最近開啟的檔案", |ui| {
                            ui.set_min_width(360.0);
                            if self.recent_files.is_empty() {
                                ui.label(RichText::new("（尚無紀錄）").color(c.fg_dim2));
                            }
                            for path in &self.recent_files {
                                let name = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                if ui.button(name).on_hover_text(path).clicked() {
                                    cmd = Some(Cmd::OpenRecent(path.clone()));
                                    ui.close_menu();
                                }
                            }
                        });
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
                        if mi(ui, "設定 (Settings)…", "Ctrl+,") { cmd = Some(Cmd::Settings); }
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
                        if mi(ui, "取代 (Replace)", "Ctrl+H") { cmd = Some(Cmd::Replace); }
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
                        let mut ex = self.show_explorer;
                        if ui.checkbox(&mut ex, "檔案總管側邊欄 (Explorer)  Ctrl+B").clicked() {
                            cmd = Some(Cmd::ToggleExplorer);
                            ui.close_menu();
                        }
                        if ui.checkbox(&mut self.config.auto_pair, "自動配對括號與引號").clicked() {
                            ui.close_menu();
                        }
                        if ui.checkbox(&mut self.config.sticky_scroll, "固定目前的函式標題 (Sticky Scroll)").clicked() {
                            ui.close_menu();
                        }
                        if ui.checkbox(&mut self.config.word_wrap, "自動換行 (Word Wrap)").clicked() {
                            ui.close_menu();
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
                                RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
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
                            let mut title = Self::tab_title(&self.tabs[i]);
                            if self.is_loading(i) {
                                title += "（載入中…）";
                            }
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

    /// 目前游標在全文中的 byte 位置
    fn caret_byte(&self, text: &str) -> usize {
        let Some(buf) = self.active_buffer() else { return 0 };
        let h = buf.selections[0].head;
        let ci = Self::char_index_of(buf, h.line, h.col);
        text.char_indices().nth(ci).map_or(text.len(), |(b, _)| b)
    }

    /// 選取 `text` 中的 byte 區間並捲動到該處
    fn select_bytes(&mut self, text: &str, a: usize, b: usize) {
        let ca = text[..a].chars().count();
        let cb = ca + text[a..b].chars().count();
        self.pending_select = Some((ca, cb));
        self.scroll_to_cursor = true;
        self.select_keeps_focus = true;
        self.find_current = self.active_buffer().map(|b| ((ca, cb), b.revision()));
        // 緩衝區的游標也同步，連續操作（下一個、取代）才會以這個命中為基準
        if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
            let line = text[..b].matches('\n').count();
            let col = text[..b].rsplit('\n').next().map_or(0, |l| l.chars().count());
            buf.selections = vec![Selection::point(Position { line, col })];
        }
    }

    /// 尋找下一個 / 上一個（以游標位置為基準，到底後從頭繞回）
    fn find_step(&mut self, forward: bool) {
        self.commit_md_edit();
        let Some(text) = self.active_buffer().map(|b| b.edit_text()) else { return };
        let matches = match find_all(&text, &self.find_bar_text, &self.find_opts) {
            Ok(m) => m,
            Err(e) => {
                self.find_error = Some(e);
                return;
            }
        };
        if matches.is_empty() {
            return;
        }
        let caret = self.caret_byte(&text);
        let i = if forward {
            matches.iter().position(|m| m.0 >= caret).unwrap_or(0)
        } else {
            matches.iter().rposition(|m| m.1 < caret).unwrap_or(matches.len() - 1)
        };
        self.find_cursor = i;
        self.select_bytes(&text, matches[i].0, matches[i].1);
    }

    /// 取代目前選取的命中並跳到下一個；游標不在命中上時先跳到下一個命中
    fn replace_current(&mut self) {
        self.commit_md_edit();
        let Some(text) = self.active_buffer().map(|b| b.edit_text()) else { return };
        let Ok(matches) = find_all(&text, &self.find_bar_text, &self.find_opts) else { return };
        let caret = self.caret_byte(&text);
        let Some(&hit) = matches.iter().find(|m| m.1 == caret) else {
            self.find_step(true);
            return;
        };
        match replace_range(&text, hit, &self.find_bar_text, &self.replace_text, &self.find_opts) {
            Ok((new_text, end)) => {
                if let Some(buf) = self.active_buffer_mut() {
                    buf.replace_all_text(&new_text);
                }
                let rest = find_all(&new_text, &self.find_bar_text, &self.find_opts).unwrap_or_default();
                match rest.iter().position(|m| m.0 >= end).or(if rest.is_empty() { None } else { Some(0) }) {
                    Some(i) => {
                        self.find_cursor = i;
                        self.select_bytes(&new_text, rest[i].0, rest[i].1);
                    }
                    None => self.select_bytes(&new_text, end, end),
                }
                self.status_message = "已取代 1 處".to_string();
            }
            Err(e) => self.status_message = e,
        }
    }

    /// 取代文件中的所有命中（一次復原即可還原）
    fn replace_all_matches(&mut self) {
        self.commit_md_edit();
        let Some(text) = self.active_buffer().map(|b| b.edit_text()) else { return };
        match replace_all(&text, &self.find_bar_text, &self.replace_text, &self.find_opts) {
            Ok((new_text, n)) => {
                if n > 0 {
                    if let Some(buf) = self.active_buffer_mut() {
                        buf.replace_all_text(&new_text);
                    }
                    self.sync_cursor_to_editor();
                }
                self.status_message = format!("已取代 {} 處", n);
            }
            Err(e) => self.status_message = e,
        }
    }

    fn find_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_find_bar {
            return;
        }
        // 全文與命中位置都只在內容、搜尋字串或選項改變時重算
        self.sync_text_cache();
        let fkey = (
            self.text_cache.0 .0,
            self.text_cache.0 .1,
            format!("{}\u{0}{:?}", self.find_bar_text, self.find_opts),
        );
        if self.find_cache.0 != fkey {
            let found = find_all(&self.text_cache.1, &self.find_bar_text, &self.find_opts);
            self.find_error = found.as_ref().err().cloned();
            self.find_cache = (fkey, found.unwrap_or_default());
        }
        let total = self.find_cache.1.len();
        // (往下找?), 取代, 全部取代
        let mut go: Option<bool> = None;
        let (mut do_replace, mut do_replace_all) = (false, false);
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
                    if enter {
                        go = Some(!shift);
                        resp.request_focus();
                        // 這個 Enter 屬於尋找列：不能再被後面的編輯區當成「輸入換行」
                        ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Enter, .. })));
                    }
                    if ui.button("▲").on_hover_text("上一個 (Shift+Enter)").clicked() {
                        go = Some(false);
                    }
                    if ui.button("▼").on_hover_text("下一個 (Enter)").clicked() {
                        go = Some(true);
                    }
                    let o = &mut self.find_opts;
                    if ui.selectable_label(o.case_sensitive, "Aa").on_hover_text("區分大小寫").clicked() {
                        o.case_sensitive = !o.case_sensitive;
                    }
                    if ui.selectable_label(o.whole_word, "全字").on_hover_text("全字拼寫須相符").clicked() {
                        o.whole_word = !o.whole_word;
                    }
                    if ui.selectable_label(o.regex, ".*").on_hover_text("使用正規表示式").clicked() {
                        o.regex = !o.regex;
                    }
                    if ui.selectable_label(self.show_replace, "取代").on_hover_text("顯示 / 隱藏取代列 (Ctrl+H)").clicked() {
                        self.show_replace = !self.show_replace;
                    }
                    let (label, ok) = if let Some(e) = &self.find_error {
                        (e.clone(), false)
                    } else if self.find_bar_text.is_empty() {
                        (String::new(), true)
                    } else if total == 0 {
                        ("無結果".to_string(), false)
                    } else {
                        (format!("{} / {}", self.find_cursor.min(total - 1) + 1, total), true)
                    };
                    ui.label(RichText::new(label).color(if ok { c.accent } else { c.accent2 }));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            self.show_find_bar = false;
                            self.focus_editor = true;
                        }
                    });
                });
                if self.show_replace {
                    ui.horizontal(|ui| {
                        ui.add_space(18.0 + ui.spacing().item_spacing.x);
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.replace_text)
                                .hint_text(if self.find_opts.regex { "取代為…（可用 $1 引用群組）" } else { "取代為…" })
                                .desired_width(280.0),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            do_replace = true;
                            resp.request_focus();
                            ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Enter, .. })));
                        }
                        if ui.button("取代").on_hover_text("取代目前的命中並跳到下一個 (Enter)").clicked() {
                            do_replace = true;
                        }
                        if ui.button("全部取代").clicked() {
                            do_replace_all = true;
                        }
                    });
                }
            });
        if let Some(forward) = go {
            self.find_step(forward);
        }
        if do_replace {
            self.replace_current();
        }
        if do_replace_all {
            self.replace_all_matches();
        }
    }

    /// 左側檔案總管：專案資料夾的樹狀檢視（資料夾內容在背景載入）
    fn explorer_panel(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_explorer {
            return;
        }
        let root = self.project_root.clone();
        if !self.tree_cache.contains_key(&root) {
            self.request_dir(root.clone());
        }
        let active = self.active_buffer().and_then(|b| b.file_path.clone());
        let mut actions: Vec<TreeAction> = Vec::new();
        let (mut close, mut open_folder, mut refresh) = (false, false, false);
        egui::SidePanel::left("explorer_panel")
            .resizable(true)
            .default_width(240.0)
            .width_range(160.0..=520.0)
            .frame(egui::Frame::none().fill(c.panel).stroke(Stroke::new(1.0_f32, c.border)))
            .show(ctx, |ui| {
                if Self::dock_title(ui, c, "EXPLORER  檔案總管") {
                    close = true;
                }
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| root.display().to_string());
                    ui.add(egui::Label::new(RichText::new(name).color(c.accent).strong()).truncate(true))
                        .on_hover_text(root.display().to_string());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(6.0);
                        if ui.small_button("重新整理").clicked() {
                            refresh = true;
                        }
                        if ui.small_button("開啟…").on_hover_text("選擇另一個資料夾").clicked() {
                            open_folder = true;
                        }
                    });
                });
                ui.add_space(2.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.tree_rows(ui, c, &root, 0, active.as_deref(), &mut actions);
                });
            });
        if close {
            self.show_explorer = false;
        }
        if open_folder {
            self.run_cmd(Cmd::OpenFolder);
        }
        if refresh {
            let dirs: Vec<PathBuf> = std::iter::once(root).chain(self.tree_expanded.iter().cloned()).collect();
            self.tree_cache.clear();
            for d in dirs {
                self.request_dir(d);
            }
        }
        for a in actions {
            match a {
                TreeAction::Open(p) => self.open_path(&p),
                TreeAction::Toggle(d) => {
                    if !self.tree_expanded.remove(&d) {
                        if !self.tree_cache.contains_key(&d) {
                            self.request_dir(d.clone());
                        }
                        self.tree_expanded.insert(d);
                    }
                }
                TreeAction::Reveal(p) => {
                    self.status_message = match reveal_in_file_manager(&p) {
                        Ok(_) => format!("已在檔案總管顯示：{}", p.display()),
                        Err(e) => format!("無法開啟檔案總管：{}", e),
                    };
                }
                TreeAction::Shell(d, kind) => {
                    self.status_message = match open_terminal(&d, kind) {
                        Ok(_) => format!("已在 {} 開啟終端機", d.display()),
                        Err(e) => format!("無法開啟終端機：{}", e),
                    };
                }
                TreeAction::CopyPath(p) => {
                    let text = p.to_string_lossy().to_string();
                    self.status_message = format!("已複製路徑：{}", text);
                    ctx.copy_text(text);
                }
            }
        }
    }

    fn tree_rows(&self, ui: &mut egui::Ui, c: &Colors, dir: &Path, depth: usize, active: Option<&str>, actions: &mut Vec<TreeAction>) {
        let indent = 8.0 + depth as f32 * 14.0;
        let note = |ui: &mut egui::Ui, text: String, col: Color32| {
            ui.horizontal(|ui| {
                ui.add_space(indent + 14.0);
                ui.label(RichText::new(text).size(12.0).color(col));
            });
        };
        let list = match self.tree_cache.get(dir) {
            Some(TreeDir::Loaded(list)) => list.clone(),
            Some(TreeDir::Failed(e)) => return note(ui, format!("無法讀取：{}", e), c.accent2),
            _ => return note(ui, "載入中…".to_string(), c.fg_dim2),
        };
        if list.is_empty() {
            return note(ui, "（空資料夾）".to_string(), c.fg_dim2);
        }
        let row_h = 22.0;
        for e in list.iter() {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), Sense::click());
            let expanded = e.is_dir && self.tree_expanded.contains(&e.path);
            // 只畫看得到的列（大資料夾也不會拖慢每一幀）
            if ui.is_rect_visible(rect) {
                let is_active = !e.is_dir && active.map_or(false, |a| Path::new(a) == e.path);
                let p = ui.painter();
                if is_active {
                    p.rect_filled(rect, 0.0, c.accent_faint);
                    p.rect_filled(Rect::from_min_size(rect.min, egui::vec2(2.0, row_h)), 0.0, c.accent);
                } else if resp.hovered() {
                    p.rect_filled(rect, 0.0, c.hover);
                }
                let x = rect.left() + indent;
                let cy = rect.center().y;
                if e.is_dir {
                    let pts = if expanded {
                        vec![Pos2::new(x, cy - 2.5), Pos2::new(x + 8.0, cy - 2.5), Pos2::new(x + 4.0, cy + 3.0)]
                    } else {
                        vec![Pos2::new(x + 1.5, cy - 4.0), Pos2::new(x + 7.0, cy), Pos2::new(x + 1.5, cy + 4.0)]
                    };
                    p.add(Shape::convex_polygon(pts, c.fg_dim2, Stroke::NONE));
                }
                let col = if is_active {
                    c.accent
                } else if e.is_dir {
                    c.fg
                } else {
                    c.fg_list
                };
                p.text(Pos2::new(x + 14.0, cy), Align2::LEFT_CENTER, &e.name, FontId::proportional(13.0), col);
            }
            if resp.clicked() {
                actions.push(if e.is_dir { TreeAction::Toggle(e.path.clone()) } else { TreeAction::Open(e.path.clone()) });
            }
            resp.context_menu(|ui| {
                ui.set_min_width(230.0);
                let folder = if e.is_dir { e.path.clone() } else { e.path.parent().map(|p| p.to_path_buf()).unwrap_or_default() };
                if ui.button("開啟檔案所在資料夾").clicked() {
                    actions.push(TreeAction::Reveal(e.path.clone()));
                    ui.close_menu();
                }
                for (kind, label) in available_shells() {
                    if ui.button(*label).clicked() {
                        actions.push(TreeAction::Shell(folder.clone(), *kind));
                        ui.close_menu();
                    }
                }
                ui.separator();
                if ui.button("複製完整路徑").clicked() {
                    actions.push(TreeAction::CopyPath(e.path.clone()));
                    ui.close_menu();
                }
            });
            if expanded {
                self.tree_rows(ui, c, &e.path, depth + 1, active, actions);
            }
        }
    }

    /// 分頁有未儲存的修改、檔案又被其他程式改了：讓使用者決定要哪一份
    fn disk_conflict_bar(&mut self, ctx: &egui::Context, c: &Colors) {
        let idx = self.active_tab_idx;
        let Some(buf) = self.tabs.get(idx).filter(|b| Some(b.id) == self.disk_conflict) else { return };
        let name = Self::tab_title(buf);
        let mut choice: Option<bool> = None; // true = 重新載入
        egui::TopBottomPanel::top("disk_conflict_bar")
            .frame(egui::Frame::none().fill(c.panel).inner_margin(egui::Margin::symmetric(10.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("「{}」已被其他程式修改，而你有尚未儲存的變更。", name)).color(c.warn));
                    if ui.button("重新載入（放棄我的修改）").clicked() {
                        choice = Some(true);
                    }
                    if ui.button("保留我的版本").clicked() {
                        choice = Some(false);
                    }
                });
            });
        match choice {
            Some(true) => {
                self.disk_conflict = None;
                let path = self.tabs[idx].file_path.clone().unwrap_or_default();
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let (text, _) = self.tabs[idx].encoding.decode(&bytes);
                        self.reload_tab_from(idx, &text);
                        self.note_disk_state(idx);
                    }
                    Err(e) => self.status_message = format!("重新載入失敗：{}", e),
                }
            }
            Some(false) => {
                self.disk_conflict = None;
                self.status_message = "已保留你的版本；儲存時會覆寫磁碟上的檔案".to_string();
            }
            None => {}
        }
    }

    /// 「按 Tab 鍵插入空白」：在 TextEdit 處理輸入事件之前，把 Tab 按鍵改寫成補到下一個定位點的空白。
    /// Shift+Tab（減少縮排）維持 TextEdit 內建行為。
    fn tab_to_spaces_events(ui: &egui::Ui, editor_id: egui::Id, text: &str, tab_size: usize) {
        let is_tab = |e: &egui::Event| {
            matches!(e, egui::Event::Key { key: Key::Tab, pressed: true, modifiers, .. } if modifiers.is_none())
        };
        if !ui.input(|i| i.events.iter().any(is_tab)) {
            return;
        }
        let Some(range) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).and_then(|s| s.cursor.char_range()) else {
            return;
        };
        let tab_size = tab_size.max(1);
        let start = range.primary.index.min(range.secondary.index);
        // 選取範圍起點所在的欄（既有的 Tab 字元算到它的下一個定位點）
        let mut col = 0usize;
        for ch in text.chars().take(start) {
            col = match ch {
                '\n' => 0,
                '\t' => col + tab_size - col % tab_size,
                _ => col + 1,
            };
        }
        ui.input_mut(|i| {
            for e in &mut i.events {
                if is_tab(e) {
                    let n = tab_size - col % tab_size;
                    *e = egui::Event::Text(" ".repeat(n));
                    col += n;
                }
            }
        });
    }

    /// 自動配對：在 TextEdit 處理輸入事件之前改寫事件。回傳 TextEdit 處理完後游標要再移動的字元數。
    fn auto_pair_events(ui: &egui::Ui, editor_id: egui::Id, text: &str) -> isize {
        let Some(range) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).and_then(|s| s.cursor.char_range()) else {
            return 0;
        };
        let (a, b) = (range.primary.index.min(range.secondary.index), range.primary.index.max(range.secondary.index));
        // 本幀恰好輸入一個字元（輸入法組字、貼上等情況不處理）
        let typed: Option<char> = ui.input(|i| {
            let mut texts = i.events.iter().filter_map(|e| match e {
                egui::Event::Text(t) => Some(t),
                _ => None,
            });
            match (texts.next(), texts.next()) {
                (Some(t), None) if t.chars().count() == 1 => t.chars().next(),
                _ => None,
            }
        });
        let backspace = typed.is_none() && ui.input(|i| i.modifiers.is_none() && i.key_pressed(Key::Backspace));
        if !backspace && !typed.map_or(false, pair_relevant) {
            return 0;
        }
        let before = if a > 0 { text.chars().nth(a - 1) } else { None };
        let after = text.chars().nth(b);
        if backspace {
            // 游標在一對空的括號 / 引號中間：連右邊那個一起刪
            if a == b && should_delete_pair(before, after) {
                ui.input_mut(|i| {
                    i.events.push(egui::Event::Key {
                        key: Key::Delete,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    })
                });
            }
            return 0;
        }
        let Some(ch) = typed else { return 0 };
        let d = pair_decide(ch, before, after, a != b);
        let set_text = |new: String| {
            ui.input_mut(|i| {
                for e in &mut i.events {
                    if let egui::Event::Text(t) = e {
                        *t = new.clone();
                    }
                }
            })
        };
        match d.action {
            PairAction::AutoClose => match d.close {
                Some(close) => {
                    set_text(format!("{ch}{close}"));
                    -1
                }
                None => 0,
            },
            PairAction::SkipOver => {
                ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Text(_))));
                1
            }
            PairAction::Surround => match (d.open, d.close) {
                (Some(open), Some(close)) => {
                    let sel: String = text.chars().skip(a).take(b - a).collect();
                    set_text(format!("{open}{sel}{close}"));
                    0
                }
                _ => 0,
            },
            PairAction::Insert => 0,
        }
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
                        ui.label(RichText::new(format!("{}%", (self.config.font_size / 14.0 * 100.0).round())).size(12.0).color(c.fg_dim2));
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
        // 內容還在背景載入的分頁：只顯示「載入中」，沒有可以輸入的編輯區
        if self.is_loading(self.active_tab_idx) {
            self.editor_was_focused = false;
            self.editor_focused = false;
            let path = self.tabs[self.active_tab_idx].file_path.clone().unwrap_or_default();
            egui::CentralPanel::default().frame(egui::Frame::none().fill(c.bg)).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.4).max(0.0));
                    ui.spinner();
                    ui.add_space(6.0);
                    ui.label(RichText::new("載入中…").color(c.fg_dim));
                    ui.label(RichText::new(path).size(11.5).color(c.fg_dim2));
                });
            });
            return;
        }
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

        self.disk_conflict_bar(ctx, c);
        self.editor_was_focused = self.editor_focused;
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
                    let fsize = self.config.font_size;
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

    /// 上一幀固定在編輯區頂端的標頭行號（由外層到內層）
    #[cfg(test)]
    fn sticky_lines(&self) -> Vec<usize> {
        self.sticky_rows.iter().map(|(line, _)| *line).collect()
    }

    /// 目前分頁中可固定在頂端的區塊；只在內容或語言改變時重新擷取
    fn sticky_symbols(&mut self, idx: usize, lang: Lang, bytes: usize) -> Arc<Vec<Symbol>> {
        let key = Some((self.tabs[idx].id, self.tabs[idx].revision(), lang));
        if self.sticky_key != key {
            let syntax = match lang {
                Lang::Rust => Some(SymbolLang::Rust),
                Lang::CLike | Lang::JavaScript => Some(SymbolLang::CFamily),
                Lang::Python => Some(SymbolLang::Python),
                _ => None,
            };
            let syms = match syntax {
                Some(s) if bytes <= HIGHLIGHT_MAX_BYTES => extract_symbols(&self.tabs[idx].lines, s),
                _ => Vec::new(),
            };
            self.sticky_key = key;
            self.sticky_syms = Arc::new(syms);
        }
        self.sticky_syms.clone()
    }

    fn code_editor(&mut self, ui: &mut egui::Ui, c: &Colors, pal: &crate::theme::Palette) {
        let idx = self.active_tab_idx;
        if idx >= self.tabs.len() {
            return;
        }
        let lang = self.tab_lang(idx);
        let font = FontId::monospace(self.config.font_size);
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
            if !std::mem::take(&mut self.select_keeps_focus) {
                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            }
        }
        if std::mem::take(&mut self.focus_editor) {
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }
        let find_current = if self.show_find_bar {
            self.find_current.filter(|(_, rev)| *rev == self.tabs[idx].revision()).map(|(r, _)| r)
        } else {
            None
        };

        let row_h = ui.fonts(|f| f.row_height(&font));
        let char_w = ui.fonts(|f| f.glyph_width(&font, '0'));
        let digits = line_count.to_string().len().max(3);
        let gutter_w = digits as f32 * char_w + 30.0;

        // 語法高亮與排版只在輸入（文字 / 語言 / 主題 / 字型 / 搜尋字串）改變時重算；
        // 其餘每一幀只花一次全文雜湊。過大的文件不做語法高亮。
        let pal_c = pal.clone();
        let font_c = font.clone();
        let theme = self.theme_mgr.current_theme_name().to_string();
        let fopts = self.find_opts;
        // 自動換行：以編輯區的可用寬度折行。行號區要在排版之前配置，高度沿用上一幀的排版結果。
        let wrap = self.config.word_wrap;
        let wrapped_h = self.highlight_cache.as_ref().map_or(0.0, |(_, g, _)| g.size().y);
        if std::mem::replace(&mut self.wrap_applied, wrap) != wrap {
            self.scroll_to_cursor = true; // 切換後每一行的位置都變了：讓游標留在畫面上
        }
        let mut hl_cache = self.highlight_cache.take();
        let mut layouter = |ui: &egui::Ui, s: &str, wrap_width: f32| {
            use std::hash::{Hash, Hasher};
            let max_width = if wrap { wrap_width.max(char_w * 20.0) } else { f32::INFINITY };
            let mut h = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut h);
            lang.display_name().hash(&mut h);
            theme.hash(&mut h);
            font_c.size.to_bits().hash(&mut h);
            find.hash(&mut h);
            fopts.hash(&mut h);
            max_width.to_bits().hash(&mut h);
            let key = h.finish();
            let atlas = ui.fonts(|f| f.texture_atlas());
            match &hl_cache {
                Some((k, galley, a)) if *k == key && Arc::ptr_eq(a, &atlas) => galley.clone(),
                _ => {
                    let l = if s.len() > HIGHLIGHT_MAX_BYTES { Lang::Plain } else { lang };
                    let hits = find_all(s, &find, &fopts).unwrap_or_default();
                    let mut job = highlight_ranges(s, l, &pal_c, font_c.clone(), &hits);
                    if wrap {
                        job.wrap.max_width = max_width;
                    }
                    let galley = ui.fonts(|f| f.layout_job(job));
                    hl_cache = Some((key, galley.clone(), atlas));
                    galley
                }
            }
        };

        // Git 行狀態：只在內容或 HEAD 內容改變時重算
        let buf_key = (self.tabs[idx].id, self.tabs[idx].revision());
        match self.git_head.get(&buf_key.0) {
            Some(Some(head)) => {
                if self.git_marks.0 != buf_key {
                    self.git_marks = (buf_key, diff_line_states(head, &self.tabs[idx].lines, 800));
                }
            }
            _ => self.git_marks = ((0, 0), HashMap::new()),
        }
        let git_marks = std::mem::take(&mut self.git_marks.1);
        let was_focused = self.editor_was_focused;
        let pair_enabled = self.config.auto_pair && !matches!(lang, Lang::Plain | Lang::Markdown);

        let mut new_cursor: Option<(usize, usize)> = None;
        let mut changed = false;
        let scroll_to_cursor = std::mem::take(&mut self.scroll_to_cursor);

        // Sticky Scroll：上一幀固定標頭佔用的高度，以及游標原本的位置（用來判斷游標是否移動）
        let sticky_syms = if self.config.sticky_scroll {
            self.sticky_symbols(idx, lang, text.len())
        } else {
            Arc::new(Vec::new())
        };
        let sticky_h = if sticky_syms.is_empty() { 0.0 } else { self.sticky_rows.len() as f32 * row_h };
        let prev_cursor = {
            let head = self.tabs[idx].selections[0].head;
            (head.line, head.col)
        };

        // 自動換行時每一列（畫面上的一列）所屬的行號；沒有換行時列 = 行，不需要這張表
        let mut row_lines = std::mem::take(&mut self.row_lines);
        row_lines.clear();
        // 自動換行時不需要水平捲動；兩種模式各自記住捲動位置
        let scroll_area = if wrap {
            egui::ScrollArea::vertical().id_source(("editor-scroll-wrap", idx))
        } else {
            egui::ScrollArea::both().id_source(("editor-scroll", idx))
        };
        let scroll = scroll_area
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let text_h = if wrap { wrapped_h } else { line_count as f32 * row_h };
                    let min_h = ui.available_height().max(text_h + 40.0);
                    let (gutter_rect, gutter_resp) =
                        ui.allocate_exact_size(egui::vec2(gutter_w, min_h), Sense::click());
                    let line_bg_idx = ui.painter().add(Shape::Noop);
                    let gutter_idx = ui.painter().add(Shape::Noop);

                    let cursor_shift =
                        if pair_enabled && was_focused { Self::auto_pair_events(ui, editor_id, &text) } else { 0 };
                    if self.config.insert_spaces && was_focused {
                        Self::tab_to_spaces_events(ui, editor_id, &text, self.config.tab_size);
                    }

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
                    if cursor_shift != 0 {
                        // 自動補上右括號後游標留在中間；跳過既有的右括號時游標右移
                        if let Some(mut st) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id) {
                            if let Some(r) = st.cursor.char_range() {
                                let i = (r.primary.index as isize + cursor_shift).max(0) as usize;
                                st.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(i))));
                                st.store(ui.ctx(), editor_id);
                            }
                        }
                    }
                    let galley = &output.galley;
                    let gpos = output.galley_pos;

                    if wrap {
                        let mut line = 0;
                        row_lines.extend(galley.rows.iter().map(|row| {
                            let l = line;
                            line += row.ends_with_newline as usize;
                            l
                        }));
                        // 行號區的高度是照上一幀的排版配置的：排版剛改變（切換分頁、視窗寬度改變）時再畫一幀
                        if (galley.size().y - wrapped_h).abs() > 0.5 {
                            ui.ctx().request_repaint();
                        }
                    }

                    // 目前游標所在的 (行, 欄)。自動換行時畫面上的列不等於行，改用段落座標。
                    let cur_row = output.cursor_range.map(|cr| {
                        if wrap {
                            (cr.primary.pcursor.paragraph, cr.primary.pcursor.offset)
                        } else {
                            (cr.primary.rcursor.row, cr.primary.rcursor.column)
                        }
                    });
                    if let Some((r, col)) = cur_row {
                        new_cursor = Some((r, col));
                    }
                    // 游標移到固定標頭底下（例如按方向鍵往上）時多捲一點，讓游標所在行露出來
                    if let Some(cr) = output.cursor_range.filter(|_| sticky_h > 0.0 && !scroll_to_cursor) {
                        let r = galley.pos_from_cursor(&cr.primary).translate(gpos.to_vec2());
                        if cur_row != Some(prev_cursor) && r.top() < ui.clip_rect().top() + sticky_h {
                            let target = Rect::from_min_max(Pos2::new(r.left(), r.top() - sticky_h), r.max);
                            ui.scroll_to_rect(target, None);
                        }
                    }
                    let active_line = cur_row
                        .map(|(r, _)| r)
                        .unwrap_or_else(|| self.tabs[idx].selections[0].head.line);

                    // 目前行底色（畫在文字下方）
                    let clip = ui.clip_rect();
                    // 自動換行時一行可能佔好幾列：整行都上底色
                    let (row_a, row_b) = if wrap {
                        let a = row_lines.partition_point(|&l| l < active_line);
                        (a, row_lines.partition_point(|&l| l <= active_line).saturating_sub(1).max(a))
                    } else {
                        (active_line, active_line)
                    };
                    if let (Some(row), Some(last)) = (galley.rows.get(row_a), galley.rows.get(row_b)) {
                        let y0 = gpos.y + row.rect.top();
                        let y1 = gpos.y + last.rect.top() + last.rect.height().max(row_h);
                        let r = Rect::from_min_max(
                            Pos2::new(gutter_rect.right(), y0),
                            Pos2::new(clip.right().max(output.response.rect.right()), y1),
                        );
                        ui.painter().set(
                            line_bg_idx,
                            Shape::rect_filled(r, 0.0, c.current_line),
                        );
                    }

                    // 行號 gutter
                    // 文字比預留的高度還高時（自動換行剛重新排版），行號區的底色跟著延伸到文字底端
                    let gutter_bg = Rect::from_min_max(
                        gutter_rect.min,
                        Pos2::new(gutter_rect.right(), gutter_rect.bottom().max(output.response.rect.bottom())),
                    );
                    let mut shapes = vec![
                        Shape::rect_filled(gutter_bg, 0.0, c.panel),
                        Shape::line_segment(
                            [gutter_bg.right_top(), gutter_bg.right_bottom()],
                            Stroke::new(1.0_f32, c.border),
                        ),
                    ];
                    let num_font = FontId::monospace((self.config.font_size - 1.0).max(8.0));
                    ui.fonts(|f| {
                        for (i, row) in galley.rows.iter().enumerate() {
                            let y = gpos.y + row.rect.top();
                            if y + row_h < clip.top() || y > clip.bottom() {
                                continue;
                            }
                            // 自動換行時 i 是畫面上的列、line 是它所屬的行：行號、書籤只畫在每一行的第一列
                            let line = if wrap { row_lines[i] } else { i };
                            let line_start = !wrap || i == 0 || row_lines[i - 1] != line;
                            let active = line == active_line;
                            let col = if active { c.accent } else { c.faint };
                            if line_start {
                                let g = f.layout_no_wrap(format!("{}", line + 1), num_font.clone(), col);
                                let pos = Pos2::new(
                                    gutter_rect.right() - 10.0 - g.size().x,
                                    y + (row.rect.height().max(row_h) - g.size().y) * 0.5,
                                );
                                shapes.push(Shape::galley(pos, g, col));
                            }
                            if active {
                                shapes.push(Shape::rect_filled(
                                    Rect::from_min_size(Pos2::new(gutter_rect.left(), y), egui::vec2(2.0, row_h)),
                                    0.0,
                                    c.accent,
                                ));
                            }
                            // Git：新增（強調色條）/ 修改（黃色條）/ 此行上方有刪除（洋紅三角）
                            if let Some(&st) = git_marks.get(&line) {
                                let x = gutter_rect.right() - 4.0;
                                if st & (ADDED | MODIFIED) != 0 {
                                    let col = if st & MODIFIED != 0 { c.warn } else { c.accent };
                                    shapes.push(Shape::rect_filled(
                                        Rect::from_min_size(Pos2::new(x, y), egui::vec2(3.0, row_h)),
                                        0.0,
                                        col,
                                    ));
                                }
                                if st & DELETED_ABOVE != 0 && line_start {
                                    shapes.push(Shape::convex_polygon(
                                        vec![Pos2::new(x - 3.0, y - 3.0), Pos2::new(x + 3.0, y), Pos2::new(x - 3.0, y + 3.0)],
                                        c.accent2,
                                        Stroke::NONE,
                                    ));
                                }
                            }
                            if line_start && bookmarks.contains(&line) {
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
                            let row = ((pp.y - gpos.y) / row_h).floor().max(0.0) as usize;
                            let line = if wrap { row_lines.get(row).copied().unwrap_or(line_count) } else { row };
                            if line < line_count {
                                self.tabs[idx].toggle_bookmark(line);
                            }
                        }
                    }

                    // 編輯區沒有焦點時（焦點在尋找列）egui 不畫選取：自行以外框標出目前的命中
                    if let Some((a, b)) = find_current.filter(|_| !output.response.has_focus()) {
                        let ra = galley.pos_from_cursor(&galley.from_ccursor(CCursor::new(a)));
                        let rb = galley.pos_from_cursor(&galley.from_ccursor(CCursor::new(b)));
                        if (ra.top() - rb.top()).abs() < 0.5 {
                            let r = Rect::from_min_max(ra.left_top(), rb.right_bottom()).translate(gpos.to_vec2());
                            ui.painter().rect_stroke(r.expand(1.0), 2.0, Stroke::new(1.5_f32, c.accent));
                        }
                    }

                    if scroll_to_cursor {
                        // 沒有焦點時 output.cursor_range 為 None：改用編輯狀態中記錄的游標
                        let primary = output.cursor_range.map(|cr| cr.primary).or_else(|| {
                            egui::text_edit::TextEditState::load(ui.ctx(), editor_id)
                                .and_then(|st| st.cursor.char_range())
                                .map(|r| galley.from_ccursor(r.primary))
                        });
                        if let Some(cur) = primary {
                            let r = galley.pos_from_cursor(&cur).translate(gpos.to_vec2());
                            ui.scroll_to_rect(r.expand2(egui::vec2(40.0, row_h * 2.0)), Some(Align::Center));
                        }
                    }
                });
            });

        // Sticky Scroll：把游標上方已捲出畫面的函式 / 類別標題固定在編輯區頂端
        self.sticky_rows.clear();
        self.sticky_below = 0;
        if !sticky_syms.is_empty() {
            let view = scroll.inner_rect;
            let offset = scroll.state.offset;
            // 第一個完整可見的行（文字上方有 4px 邊界）
            let first = ((offset.y - 4.0) / row_h - 0.01).ceil().max(0.0) as usize;
            // 畫面上第 row 列起第一個完整可見的行。自動換行時一行佔好幾列：
            // 落在某一行中間（它的第一列已捲出畫面）就算下一行。
            let line_at = |row: usize| match row_lines.get(row) {
                _ if !wrap => row,
                Some(&line) => line + (row > 0 && row_lines[row - 1] == line) as usize,
                None => row_lines.last().map_or(0, |l| l + 1),
            };
            // 固定的標頭本身會蓋住幾行：以「標頭底下的第一行」為準反覆計算到穩定
            let mut rows = 0;
            for _ in 0..=STICKY_MAX_ROWS {
                let n = sticky_headers(&sticky_syms, line_at(first + rows), STICKY_MAX_ROWS).len();
                if n == rows {
                    break;
                }
                rows = n;
            }
            let headers = sticky_headers(&sticky_syms, line_at(first + rows), STICKY_MAX_ROWS);
            self.sticky_below = line_at(first + headers.len());

            let painter = ui.painter_at(view);
            let area = Rect::from_min_size(view.min, egui::vec2(view.width(), headers.len() as f32 * row_h));
            painter.rect_filled(area, 0.0, c.panel);
            let num_font = FontId::monospace((self.config.font_size - 1.0).max(8.0));
            let hl_lang = if text.len() > HIGHLIGHT_MAX_BYTES { Lang::Plain } else { lang };
            let mut jump = None;
            for (k, sym) in headers.iter().enumerate() {
                let Some(line_text) = self.tabs[idx].lines.get(sym.line) else { continue };
                let row = Rect::from_min_size(
                    Pos2::new(view.left(), view.top() + k as f32 * row_h),
                    egui::vec2(view.width(), row_h),
                );
                let resp = ui
                    .interact(row, ui.id().with(("sticky-header", k)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.hovered() {
                    painter.rect_filled(row, 0.0, c.hover);
                }
                if resp.clicked() {
                    jump = Some(sym.line);
                }
                let num = painter.layout_no_wrap((sym.line + 1).to_string(), num_font.clone(), c.faint);
                let num_pos = Pos2::new(view.left() + gutter_w - 10.0 - num.size().x, row.top() + (row_h - num.size().y) * 0.5);
                painter.galley(num_pos, num, c.faint);
                let galley = ui.fonts(|f| f.layout_job(highlight_ranges(line_text, hl_lang, pal, font.clone(), &[])));
                let text_clip = Rect::from_min_max(Pos2::new(view.left() + gutter_w, row.top()), row.max);
                ui.painter_at(text_clip.intersect(view)).galley(
                    Pos2::new(view.left() + gutter_w + 10.0 - offset.x, row.top()),
                    galley,
                    c.fg,
                );
                self.sticky_rows.push((sym.line, row));
            }
            if !self.sticky_rows.is_empty() {
                painter.line_segment([area.left_bottom(), area.right_bottom()], Stroke::new(1.0_f32, c.border));
            }
            if let Some(line) = jump {
                let i = Self::char_index_of(&self.tabs[idx], line, 0);
                self.pending_select = Some((i, i));
                self.scroll_to_cursor = true;
            }
        }

        self.highlight_cache = hl_cache;
        self.git_marks.1 = git_marks;
        self.row_lines = row_lines;
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
        let font = FontId::monospace(self.config.font_size);
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
                    FontId::monospace(self.config.font_size - 1.0),
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
        self.poll_replace_job();
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
                    // 取代：先預覽會影響幾個檔案，使用者確認後才寫入
                    ui.horizontal(|ui| {
                        ui.add_space(18.0 + ui.spacing().item_spacing.x);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.replace_in_files_text)
                                .hint_text("取代為…（留空 = 刪除）")
                                .desired_width(ui.available_width() - 90.0),
                        );
                        let ready = !self.find_query.is_empty() && self.replace_job.is_none();
                        let r = ui.add_enabled(ready, egui::Button::new("預覽取代"));
                        self.mark("fif.preview", &r);
                        if r.clicked() {
                            self.start_replace_preview(ui.ctx());
                        }
                    });
                    if let Some(job) = &self.replace_job {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            let label = if job.plan.hits.is_empty() { "計算取代預覽中…" } else { "取代中…" };
                            ui.label(RichText::new(label).color(c.fg_dim));
                            if ui.small_button("取消").clicked() {
                                job.cancel.store(true, Ordering::Relaxed);
                            }
                        });
                    }
                    let mut apply: Option<egui::Response> = None;
                    let mut discard = false;
                    if let Some(plan) = &self.replace_plan {
                        ui.label(
                            RichText::new(format!(
                                "將把 {} 處「{}」取代為「{}」，分佈於 {} 個檔案（不分大小寫）。",
                                plan.total,
                                plan.query,
                                plan.replacement,
                                plan.hits.len()
                            ))
                            .color(c.warn),
                        );
                        ui.label(RichText::new("會直接寫入磁碟上的檔案，無法用復原 (Ctrl+Z) 還原。").size(11.5).color(c.fg_dim2));
                        ui.horizontal(|ui| {
                            apply = Some(ui.button("執行取代"));
                            discard = ui.button("放棄").clicked();
                        });
                    }
                    if let Some(r) = apply {
                        self.mark("fif.apply", &r);
                        if r.clicked() {
                            self.apply_replace_plan(ui.ctx());
                        }
                    }
                    if discard {
                        self.replace_plan = None;
                    }
                    if let Some((is_error, msg)) = &self.replace_note {
                        ui.label(RichText::new(msg).color(if *is_error { c.warn } else { c.fg_dim }));
                    }
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
                        // 有取代預覽時列出會被改寫的檔案與處數，否則列出搜尋結果
                        if let Some(plan) = &self.replace_plan {
                            for h in &plan.hits {
                                let rel = Path::new(&h.path)
                                    .strip_prefix(&plan.root)
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|_| h.path.clone());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(rel).color(c.accent));
                                    ui.label(RichText::new(format!("{} 處", h.count)).color(c.fg_dim));
                                });
                            }
                        }
                        let results: &[FileMatch] = if self.replace_plan.is_some() { &[] } else { &self.find_results };
                        for m in results {
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
                let active_id = self.active_buffer().map(|b| b.id);
                if let Some(job) = active_id.and_then(|id| self.loading.get_mut(&id)) {
                    job.goto_line = Some(line.saturating_sub(1)); // 內容還沒到：載入完成後再跳到這一行
                } else if let Some(buf) = self.active_buffer() {
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
                // 關閉視窗 = 放棄預覽；已經開始寫入的取代則讓它做完（結果顯示在狀態列）
                self.replace_plan = None;
                if let Some(job) = self.replace_job.as_ref().filter(|j| j.plan.hits.is_empty()) {
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
                            RichText::new(concat!("NEON EDITION · v", env!("CARGO_PKG_VERSION"), " RUST"))
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

        self.settings_dialog(ctx, c);
    }

    /// 記下具名控制項的位置（只在測試中有作用）
    #[allow(unused_variables)]
    fn mark(&mut self, name: &'static str, resp: &egui::Response) {
        #[cfg(test)]
        self.probe.insert(name, resp.rect);
    }

    /// 設定對話框：修改立即生效，並隨 `state.json` 保存
    fn settings_dialog(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_settings {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(RichText::new("SETTINGS").font(FontId::new(12.0, FontFamily::Name(ORBITRON.into()))).color(c.accent))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                let note = |ui: &mut egui::Ui, text: &str| {
                    ui.label(RichText::new(text).size(11.5).color(c.fg_dim2));
                };

                ui.horizontal(|ui| {
                    ui.label("字型大小");
                    let r = ui.add(egui::Slider::new(&mut self.config.font_size, MIN_FONT_SIZE..=MAX_FONT_SIZE).step_by(1.0).suffix(" pt"));
                    self.mark("settings.font_size", &r);
                });

                ui.separator();
                let r = ui.checkbox(&mut self.config.insert_spaces, "按 Tab 鍵插入空白（取代 Tab 字元）");
                self.mark("settings.insert_spaces", &r);
                ui.add_enabled_ui(self.config.insert_spaces, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Tab 寬度");
                        let r = ui.add(egui::Slider::new(&mut self.config.tab_size, MIN_TAB_SIZE..=MAX_TAB_SIZE).suffix(" 格"));
                        self.mark("settings.tab_size", &r);
                    });
                });
                note(ui, "Tab 寬度決定按 Tab 鍵時對齊到的定位點；檔案中既有的 Tab 字元固定以 4 格顯示。");

                ui.separator();
                let r = ui.checkbox(&mut self.config.word_wrap, "自動換行：過長的行折到下一列顯示");
                self.mark("settings.word_wrap", &r);
                note(ui, "只影響程式碼編輯區的顯示，不會在檔案中插入換行；行號仍以檔案中的行計算。");

                ui.separator();
                let r = ui.checkbox(&mut self.config.restore_session, "啟動時還原上次開啟的檔案與資料夾");
                self.mark("settings.restore_session", &r);
                let r = ui.checkbox(&mut self.config.auto_pair, "自動配對括號與引號");
                self.mark("settings.auto_pair", &r);
                let r = ui.checkbox(&mut self.config.sticky_scroll, "固定目前的函式標題 (Sticky Scroll)");
                self.mark("settings.sticky_scroll", &r);

                ui.separator();
                ui.horizontal(|ui| {
                    let r = ui.button("還原預設值");
                    self.mark("settings.reset", &r);
                    if r.clicked() {
                        self.config = EditorConfig::default();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("  關閉  ").clicked() {
                            close = true;
                        }
                    });
                });
                note(ui, "修改會立即生效並自動保存。");
            });
        if !open || close {
            self.show_settings = false;
        }
    }
}

/// 全專案取代：預覽（只計數）→ 使用者確認 → 寫入。兩個步驟都在背景執行緒進行。
impl AlexCodeApp {
    /// 預覽是否仍對應目前的搜尋字串、取代字串與資料夾
    fn plan_is_current(&self, plan: &ReplacePlan) -> bool {
        plan.query == self.find_query && plan.replacement == self.replace_in_files_text && plan.root == self.project_root
    }

    /// 第一步：計算每個檔案會被取代幾處，不寫入任何東西
    fn start_replace_preview(&mut self, ctx: &egui::Context) {
        if self.find_query.is_empty() || self.replace_job.is_some() {
            return;
        }
        self.replace_plan = None;
        self.replace_note = None;
        let plan = ReplacePlan {
            root: self.project_root.clone(),
            query: self.find_query.clone(),
            replacement: self.replace_in_files_text.clone(),
            hits: Vec::new(),
            total: 0,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded();
        let (root, query, flag, repaint) = (plan.root.clone(), plan.query.clone(), cancel.clone(), ctx.clone());
        std::thread::spawn(move || {
            let (hits, stats) = plan_replace_in_files(&root, &[], &query, &flag);
            let _ = tx.send(ReplaceMsg::Planned(hits, stats));
            repaint.request_repaint();
        });
        self.replace_job = Some(ReplaceJob { rx, cancel, plan });
    }

    /// 第二步：使用者看過預覽並確認後，把取代寫入磁碟
    fn apply_replace_plan(&mut self, ctx: &egui::Context) {
        if self.replace_job.is_some() {
            return;
        }
        self.commit_md_edit();
        let Some(plan) = self.replace_plan.take() else { return };
        // 有未儲存修改的分頁：磁碟上的內容不是使用者正在看的內容，改寫它只會造成衝突。請使用者先存檔。
        let dirty: Vec<String> = self
            .tabs
            .iter()
            .filter(|b| b.modified)
            .filter(|b| b.file_path.as_deref().is_some_and(|p| plan.hits.iter().any(|h| same_file(&h.path, p))))
            .map(Self::tab_title)
            .collect();
        if !dirty.is_empty() {
            let msg = format!("尚未取代：這些分頁有未儲存的變更，請先儲存再執行：{}", dirty.join("、"));
            self.status_message = msg.clone();
            self.replace_note = Some((true, msg));
            self.replace_plan = Some(plan);
            return;
        }
        self.replace_note = None;
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded();
        let (hits, query, replacement) = (plan.hits.clone(), plan.query.clone(), plan.replacement.clone());
        let (flag, repaint) = (cancel.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(ReplaceMsg::Applied(apply_replace_in_files(&hits, &query, &replacement, &flag)));
            repaint.request_repaint();
        });
        self.replace_job = Some(ReplaceJob { rx, cancel, plan });
    }

    /// 收取背景取代工作的結果
    fn poll_replace_job(&mut self) {
        // 預覽之後改了搜尋字串、取代字串或資料夾：預覽作廢，必須重新預覽才能執行
        if self.replace_plan.as_ref().is_some_and(|p| !self.plan_is_current(p)) {
            self.replace_plan = None;
        }
        let Some(msg) = self.replace_job.as_ref().and_then(|j| j.rx.try_recv().ok()) else { return };
        let Some(job) = self.replace_job.take() else { return };
        match msg {
            ReplaceMsg::Planned(hits, stats) => {
                if stats.canceled {
                    self.replace_note = Some((false, "已取消預覽".to_string()));
                } else if hits.is_empty() {
                    let msg = format!("沒有可取代的內容：找不到「{}」（掃描 {} 個檔案）", job.plan.query, stats.files_scanned);
                    self.replace_note = Some((false, msg));
                } else if self.plan_is_current(&job.plan) {
                    self.replace_plan = Some(ReplacePlan { hits, total: stats.matches, ..job.plan });
                }
            }
            ReplaceMsg::Applied(res) => {
                // 已開啟、且剛被改寫的檔案：重新載入分頁。
                // 取代進行中又被使用者修改的分頁不動它，之後由外部變更偵測詢問使用者。
                for hit in &job.plan.hits {
                    if res.skipped.iter().any(|(p, _)| p == &hit.path) {
                        continue;
                    }
                    let open = self.tabs.iter().position(|b| b.file_path.as_deref().is_some_and(|p| same_file(p, &hit.path)));
                    let Some(idx) = open.filter(|&i| !self.tabs[i].modified) else { continue };
                    let Ok(bytes) = std::fs::read(&hit.path) else { continue };
                    let (text, _) = self.tabs[idx].encoding.decode(&bytes);
                    if EditorBuffer::from_text(&text).lines != self.tabs[idx].lines {
                        self.reload_tab_from(idx, &text);
                    }
                    self.note_disk_state(idx);
                }
                let mut msg = format!("已取代 {} 處（{} 個檔案）", res.replacements, res.files_written);
                if res.canceled {
                    msg += "；已取消，其餘檔案沒有更動";
                }
                if !res.skipped.is_empty() {
                    let names: Vec<String> = res
                        .skipped
                        .iter()
                        .take(5)
                        .map(|(p, why)| {
                            let name = Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| p.clone());
                            format!("{name}（{why}）")
                        })
                        .collect();
                    let more = if res.skipped.len() > names.len() { "…" } else { "" };
                    msg += &format!("；略過 {} 個檔案：{}{}", res.skipped.len(), names.join("、"), more);
                }
                self.find_results.clear(); // 取代前的搜尋結果已經過時
                self.status_message = msg.clone();
                self.replace_note = Some((!res.skipped.is_empty(), msg));
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_state();
    }
}

impl AlexCodeApp {
    /// 收取背景工作的結果：外部檔案變更、其他實例交來的檔案、資料夾列舉、Git HEAD 內容；並定期保存狀態
    fn poll_services(&mut self, ctx: &egui::Context) {
        if let Some(w) = &self.watcher {
            w.set_paths(self.file_tabs().into_iter().map(PathBuf::from).collect());
            let events = w.poll();
            if !events.is_empty() {
                self.handle_disk_events(events);
            }
        }
        let batches: Vec<Vec<PathBuf>> = self.incoming.as_ref().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for files in batches {
            for f in files {
                self.open_path(&f);
            }
            // 把視窗帶到前景
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let dirs: Vec<_> = self.tree_chan.1.try_iter().collect();
        for (dir, res) in dirs {
            let entry = match res {
                Ok(list) => TreeDir::Loaded(Arc::new(list)),
                Err(e) => TreeDir::Failed(e),
            };
            self.tree_cache.insert(dir, entry);
        }
        self.poll_loading(ctx);
        let heads: Vec<_> = self.git_chan.1.try_iter().collect();
        for (id, lines) in heads {
            self.git_head.insert(id, lines.map(Arc::new));
            self.git_marks.0 = (0, 0);
        }
        if self.last_persist.elapsed() > std::time::Duration::from_secs(3) {
            self.persist_state();
        }
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

        self.poll_services(ctx);

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
        self.explorer_panel(ctx, &colors);
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

    fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> PathBuf {
        let p = dir.path().join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn session_state_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(&dir, "a.rs", "fn a() {}");
        let b = write(&dir, "b.txt", "bee");
        let state_file = dir.path().join("cfg").join("state.json");

        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&a);
        app.open_path(&b);
        app.active_tab_idx = 0;
        app.theme_mgr.set_theme("Matrix");
        app.config.font_size = 17.0;
        app.set_folder(dir.path().to_path_buf());
        app.config.auto_pair = false;
        app.persist_state();
        assert!(state_file.exists());

        // 「重新啟動」：新的 App 讀回同一份狀態
        let mut app2 = AlexCodeApp::new();
        app2.apply_state(AppState::load(&state_file).unwrap());
        assert_eq!(app2.file_tabs(), app.file_tabs());
        assert_eq!(app2.tabs.len(), 2, "welcome tabs are replaced by the restored files");
        assert_eq!(app2.tabs[app2.active_tab_idx].file_path, app.tabs[0].file_path);
        assert_eq!(app2.theme_mgr.current_theme_name(), "Matrix");
        assert_eq!(app2.config.font_size, 17.0);
        assert_eq!(app2.project_root, dir.path());
        assert!(app2.root_pinned && app2.show_explorer && !app2.config.auto_pair);
        assert_eq!(app2.recent_files, app.recent_files);

        // 已不存在的檔案略過，不會出錯
        std::fs::remove_file(&b).unwrap();
        let mut app3 = AlexCodeApp::new();
        app3.apply_state(AppState::load(&state_file).unwrap());
        assert_eq!(app3.file_tabs().len(), 1);
    }

    #[test]
    fn opening_files_keeps_pinned_folder_and_tracks_recent() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let a = write(&dir, "a.txt", "a");
        std::fs::write(sub.join("b.txt"), "b").unwrap();

        let mut app = AlexCodeApp::new();
        app.open_path(&a);
        assert_eq!(app.project_root, dir.path(), "without a pinned folder the root follows the file");
        app.set_folder(dir.path().to_path_buf());
        app.open_path(&sub.join("b.txt"));
        assert_eq!(app.project_root, dir.path(), "a pinned folder is not changed by opening files");
        assert_eq!(app.recent_files.len(), 2);
        assert!(app.recent_files[0].ends_with("b.txt"));
    }

    #[test]
    fn external_change_reloads_clean_tab_and_asks_for_dirty_tab() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(&dir, "log.txt", "one\ntwo");
        let mut app = AlexCodeApp::new();
        app.open_path(&path);
        let idx = app.active_tab_idx;
        let id = app.tabs[idx].id;

        // 自己存檔造成的變更不算外部修改
        app.tabs[idx].replace_all_text("one\ntwo\nmine");
        assert!(app.save_tab(idx, false));
        app.handle_disk_events(vec![(path.clone(), stamp_of(&path))]);
        assert_eq!(app.tabs[idx].edit_text(), "one\ntwo\nmine");
        assert!(app.disk_conflict.is_none());

        // 未修改的分頁：自動重新載入，游標在最後一行時跟到新的結尾
        let last = app.tabs[idx].lines.len() - 1;
        app.tabs[idx].selections = vec![Selection::point(Position { line: last, col: 0 })];
        std::fs::write(&path, "one\ntwo\nmine\nappended by another program").unwrap();
        app.handle_disk_events(vec![(path.clone(), stamp_of(&path))]);
        assert_eq!(app.tabs[idx].lines.len(), 4);
        assert!(!app.tabs[idx].modified);
        assert_eq!(app.tabs[idx].selections[0].head.line, 3);
        assert_eq!(app.tabs[idx].id, id);

        // 有未儲存修改的分頁：不覆蓋，交給使用者決定
        app.tabs[idx].replace_all_text("my unsaved edit");
        std::fs::write(&path, "changed again on disk, longer").unwrap();
        app.handle_disk_events(vec![(path.clone(), stamp_of(&path))]);
        assert_eq!(app.tabs[idx].edit_text(), "my unsaved edit");
        assert_eq!(app.disk_conflict, Some(id));

        // 內容相同（只是時間戳變了）不重新載入也不詢問
        app.disk_conflict = None;
        app.tabs[idx].mark_saved();
        std::fs::write(&path, "my unsaved edit").unwrap();
        let rev = app.tabs[idx].revision();
        app.handle_disk_events(vec![(path.clone(), stamp_of(&path))]);
        assert_eq!(app.tabs[idx].revision(), rev);
        assert!(app.disk_conflict.is_none());
    }

    #[test]
    fn find_next_previous_and_replace() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "foo bar\nfoo baz\nFOO end");
        app.find_bar_text = "foo".into();
        app.replace_text = "qux".into();

        app.find_step(true);
        assert_eq!(app.pending_select, Some((0, 3)), "first Enter selects the first match");
        frame(&mut app, &ctx, vec![]);
        app.find_step(true);
        assert_eq!(app.pending_select, Some((8, 11)));
        frame(&mut app, &ctx, vec![]);
        app.find_step(false);
        assert_eq!(app.pending_select, Some((0, 3)));
        frame(&mut app, &ctx, vec![]);
        app.find_step(false);
        assert_eq!(app.pending_select, Some((16, 19)), "wraps around to the last match");
        frame(&mut app, &ctx, vec![]);

        // 取代目前命中（最後一個），接著繞回第一個
        app.replace_current();
        assert_eq!(app.tabs[0].edit_text(), "foo bar\nfoo baz\nqux end");
        assert_eq!(app.pending_select, Some((0, 3)));
        frame(&mut app, &ctx, vec![]);

        // 區分大小寫 + 全部取代；一次復原就能還原
        app.tabs[0].replace_all_text("foo Foo foo");
        app.find_opts.case_sensitive = true;
        app.replace_all_matches();
        assert_eq!(app.tabs[0].edit_text(), "qux Foo qux");
        app.run_cmd(Cmd::Undo);
        assert_eq!(app.tabs[0].edit_text(), "foo Foo foo");

        // 正規表示式錯誤：顯示訊息，不更動文件
        app.find_opts.regex = true;
        app.find_bar_text = "(".into();
        app.show_find_bar = true;
        frame(&mut app, &ctx, vec![]);
        assert!(app.find_error.is_some());
        app.replace_all_matches();
        assert_eq!(app.tabs[0].edit_text(), "foo Foo foo");
    }

    #[test]
    fn enter_in_find_field_never_edits_the_document() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "let foo = 1;\nfoo();");
        app.run_cmd(Cmd::Replace);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![Event::Text("foo".into())]);
        assert_eq!(app.find_bar_text, "foo");
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
            frame(&mut app, &ctx, vec![]);
            assert_eq!(app.tabs[0].edit_text(), "let foo = 1;\nfoo();", "Enter in the find field must only search");
            assert!(!app.tabs[0].modified);
            assert!(!app.editor_focused, "focus stays in the find field");
        }
        assert_eq!(app.tabs[0].selections[0].head, Position { line: 0, col: 7 }, "3rd Enter wrapped to the first match");

        // 關閉尋找列（Esc）後焦點回到編輯區
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.show_find_bar);
        assert!(app.editor_focused);
    }

    #[test]
    fn auto_pair_closes_skips_and_deletes() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "");
        let type_ = |app: &mut AlexCodeApp, t: &str| {
            frame(app, &ctx, vec![Event::Text(t.into())]);
            frame(app, &ctx, vec![]);
        };
        type_(&mut app, "(");
        assert_eq!(app.tabs[0].edit_text(), "()");
        type_(&mut app, "x");
        assert_eq!(app.tabs[0].edit_text(), "(x)", "cursor stays between the pair");
        type_(&mut app, ")");
        assert_eq!(app.tabs[0].edit_text(), "(x)", "typing the closer skips over it");
        type_(&mut app, ";");
        assert_eq!(app.tabs[0].edit_text(), "(x);");

        // 空的一對：退格連右邊一起刪
        type_(&mut app, "[");
        assert_eq!(app.tabs[0].edit_text(), "(x);[]");
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "(x);");

        // 關閉設定後照常輸入
        app.config.auto_pair = false;
        type_(&mut app, "{");
        assert_eq!(app.tabs[0].edit_text(), "(x);{");
    }

    #[test]
    fn git_marks_follow_head_content() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "a\nb CHANGED\nc\nnew");
        let id = app.tabs[0].id;
        let head: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        app.git_head.insert(id, Some(Arc::new(head)));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.git_marks.1.get(&1), Some(&MODIFIED));
        assert_eq!(app.git_marks.1.get(&3), Some(&ADDED));
        assert_eq!(app.git_marks.1.get(&0), None);

        // 改回與 HEAD 相同 → 標記消失
        app.tabs[0].replace_all_text("a\nb\nc");
        frame(&mut app, &ctx, vec![]);
        assert!(app.git_marks.1.is_empty());

        // 外部修改後重新載入：標記跟著新內容
        app.reload_tab_from(0, "a\nb\nc\nappended");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.git_marks.1.get(&3), Some(&ADDED));

        // 不在 Git 內的檔案沒有標記
        app.git_head.insert(id, None);
        app.tabs[0].replace_all_text("zzz");
        frame(&mut app, &ctx, vec![]);
        assert!(app.git_marks.1.is_empty());
    }

    #[test]
    fn head_lines_reads_committed_version() {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git").current_dir(dir.path()).args(args).output().map_or(false, |o| o.status.success())
        };
        if !git(&["init", "-q"]) {
            return; // 測試環境沒有 git
        }
        let f = write(&dir, "f.txt", "v1\nline");
        assert!(crate::git::head_lines(&f).is_none(), "untracked file has no HEAD version");
        assert!(git(&["add", "."]));
        assert!(git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "init"]));
        std::fs::write(&f, "v2").unwrap();
        assert_eq!(crate::git::head_lines(&f).unwrap(), vec!["v1", "line"]);
        assert!(crate::git::head_lines(&dir.path().join("nope.txt")).is_none());
    }

    #[test]
    fn explorer_lists_folder_in_background_and_opens_files() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "main.rs", "fn main() {}");
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let mut app = app_with(&ctx, "x");
        app.set_folder(dir.path().to_path_buf());
        let t = std::time::Instant::now();
        while !matches!(app.tree_cache.get(dir.path()), Some(TreeDir::Loaded(_))) {
            assert!(t.elapsed() < std::time::Duration::from_secs(5), "folder listing never arrived");
            std::thread::sleep(std::time::Duration::from_millis(10));
            frame(&mut app, &ctx, vec![]);
        }
        let Some(TreeDir::Loaded(list)) = app.tree_cache.get(dir.path()) else { unreachable!() };
        let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["src", "main.rs"]);
        frame(&mut app, &ctx, vec![]); // 含檔案總管的一幀可正常繪製
    }

    #[test]
    fn plugin_command_inserts_its_result_at_the_cursor() {
        let mut app = AlexCodeApp::new();
        app.plugin_mgr.register_command("stamp", || "<<X>>".to_string());
        app.tabs[0] = EditorBuffer::from_text("ab");
        app.active_tab_idx = 0;
        app.tabs[0].selections = vec![Selection::point(Position { line: 0, col: 1 })];
        app.run_cmd(Cmd::Plugin("stamp".into()));
        assert_eq!(app.tabs[0].edit_text(), "a<<X>>b");
    }

    /// 0: `impl`、1: `fn first`（到 122）、124: `fn second`（到 245）、247: `}`
    fn sticky_source() -> String {
        let mut s = String::from("impl Thing {\n");
        for name in ["first", "second"] {
            s += &format!("    fn {name}(&self) {{\n");
            for i in 0..120 {
                s += &format!("        let v{i} = {i};\n");
            }
            s += "    }\n\n";
        }
        s + "}\n"
    }

    /// 把游標移到指定行並捲動到畫面中央
    fn goto_line(app: &mut AlexCodeApp, ctx: &egui::Context, line: usize) {
        let i = AlexCodeApp::char_index_of(&app.tabs[0], line, 0);
        app.pending_select = Some((i, i));
        app.scroll_to_cursor = true;
        settle(app, ctx);
    }

    /// egui 的捲動有動畫：多跑幾幀讓它停下來
    fn settle(app: &mut AlexCodeApp, ctx: &egui::Context) {
        for _ in 0..90 {
            frame(app, ctx, vec![]);
        }
    }

    fn cursor_line(app: &AlexCodeApp) -> usize {
        app.tabs[0].selections[0].head.line
    }

    #[test]
    fn sticky_scroll_pins_enclosing_headers() {
        let ctx = ctx();
        let mut app = app_with(&ctx, &sticky_source());
        assert_eq!(app.sticky_lines(), Vec::<usize>::new(), "nothing is pinned at the top of the file");

        goto_line(&mut app, &ctx, 60);
        assert_eq!(app.sticky_lines(), vec![0, 1]);
        goto_line(&mut app, &ctx, 200);
        assert_eq!(app.sticky_lines(), vec![0, 124]);
        goto_line(&mut app, &ctx, 0);
        assert_eq!(app.sticky_lines(), Vec::<usize>::new());

        // 編輯後符號重新計算：把 first 改名不影響行號，刪掉 impl 那一行則只剩函式
        goto_line(&mut app, &ctx, 60);
        let text = app.tabs[0].edit_text().replacen("impl Thing {\n", "\n", 1);
        app.tabs[0].replace_all_text(&text);
        settle(&mut app, &ctx);
        assert_eq!(app.sticky_lines(), vec![1]);

        // 可在設定中關閉
        app.config.sticky_scroll = false;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.sticky_lines(), Vec::<usize>::new());
    }

    #[test]
    fn sticky_header_click_jumps_to_its_line() {
        let ctx = ctx();
        let mut app = app_with(&ctx, &sticky_source());
        goto_line(&mut app, &ctx, 200);
        let (line, rect) = app.sticky_rows[1];
        assert_eq!(line, 124);
        let pos = rect.center();
        let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(&mut app, &ctx, vec![Event::PointerMoved(pos)]);
        frame(&mut app, &ctx, vec![button(true)]);
        frame(&mut app, &ctx, vec![button(false)]);
        settle(&mut app, &ctx);
        assert_eq!(cursor_line(&app), 124, "clicking a pinned header moves the cursor to that line");
        assert_eq!(app.sticky_lines(), vec![0, 1], "line 124 is centred, so the top of the view is back inside `first`");
        assert!(app.sticky_below <= 124, "the clicked header is visible in the text itself");
    }

    #[test]
    fn cursor_never_hides_under_sticky_headers() {
        let ctx = ctx();
        // 五層巢狀：固定的標頭有五行
        let mut src = String::from("mod a {\nmod b {\nmod c {\nimpl Thing {\nfn deep(&self) {\n");
        for i in 0..300 {
            src += &format!("    let v{i} = {i};\n");
        }
        src += "}\n}\n}\n}\n}\n";
        let mut app = app_with(&ctx, &src);
        goto_line(&mut app, &ctx, 200);
        assert_eq!(app.sticky_lines(), vec![0, 1, 2, 3, 4]);
        let start = cursor_line(&app);
        for step in 1..=60 {
            frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
            settle(&mut app, &ctx);
            assert_eq!(cursor_line(&app), start - step);
            assert!(
                cursor_line(&app) >= app.sticky_below,
                "cursor line {} is covered by the pinned headers (first uncovered line {})",
                cursor_line(&app),
                app.sticky_below
            );
        }
        assert!(app.sticky_below < 190, "the view must have scrolled up with the cursor");
    }

    #[test]
    fn settings_are_saved_in_state_json_and_restored() {
        let dir = tempfile::tempdir().unwrap();
        let state_file = dir.path().join("state.json");
        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.config.font_size = 18.0;
        app.config.tab_size = 2;
        app.config.insert_spaces = true;
        app.config.sticky_scroll = false;
        app.config.auto_pair = false;
        app.persist_state();

        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&state_file).unwrap()).unwrap();
        assert_eq!(json["config"]["tab_size"], 2);
        assert_eq!(json["config"]["insert_spaces"], true);
        assert_eq!(json["config"]["restore_session"], true);

        let mut app2 = AlexCodeApp::new();
        app2.apply_state(AppState::load(&state_file).unwrap());
        assert_eq!(app2.config.font_size, 18.0);
        assert_eq!(app2.config.tab_size, 2);
        assert!(app2.config.insert_spaces && !app2.config.sticky_scroll && !app2.config.auto_pair);
        assert!(app2.config.restore_session);
    }

    #[test]
    fn state_from_older_version_keeps_font_size_and_gets_default_settings() {
        // v5.5.0 的 state.json 沒有 config 區塊
        let dir = tempfile::tempdir().unwrap();
        let old = write(&dir, "state.json", r#"{"theme":"Matrix","font_size":17.0,"auto_pair":false,"show_explorer":true}"#);
        let mut app = AlexCodeApp::new();
        app.apply_state(AppState::load(&old).unwrap());
        assert_eq!(app.config.font_size, 17.0);
        assert!(!app.config.auto_pair);
        assert_eq!(app.config.tab_size, 4);
        assert!(app.config.restore_session && app.config.sticky_scroll && !app.config.insert_spaces);

        // 超出範圍的值（手動改壞設定檔）會被修正，不會讓介面壞掉
        let bad = write(&dir, "bad.json", r#"{"config":{"font_size":900.0,"tab_size":0}}"#);
        let mut app = AlexCodeApp::new();
        app.apply_state(AppState::load(&bad).unwrap());
        assert_eq!(app.config.font_size, 36.0);
        assert_eq!(app.config.tab_size, 1);
        assert!(app.config.restore_session, "fields missing from the config block use their defaults");
    }

    #[test]
    fn restore_session_off_starts_without_last_files_and_folder() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(&dir, "a.rs", "fn a() {}");
        let state_file = dir.path().join("cfg").join("state.json");
        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&a);
        app.set_folder(dir.path().to_path_buf());
        app.config.restore_session = false;
        app.config.font_size = 20.0;
        app.persist_state();

        let mut app2 = AlexCodeApp::new();
        let welcome_tabs = app2.tabs.len();
        app2.apply_state(AppState::load(&state_file).unwrap());
        assert!(app2.file_tabs().is_empty(), "last session's files are not reopened");
        assert_eq!(app2.tabs.len(), welcome_tabs);
        assert!(!app2.root_pinned, "last session's folder is not reopened");
        assert_eq!(app2.config.font_size, 20.0, "preferences are still restored");
        assert!(!app2.config.restore_session);
        assert_eq!(app2.recent_files, app.recent_files, "the recent files list is kept");
    }

    #[test]
    fn tab_key_inserts_spaces_to_the_next_tab_stop_when_enabled() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "ab");
        app.pending_select = Some((2, 2));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);

        // 預設：插入 Tab 字元
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].edit_text(), "ab\t");
        app.run_cmd(Cmd::Undo);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "ab");

        app.config.insert_spaces = true;
        app.config.tab_size = 4;
        app.pending_select = Some((2, 2));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].edit_text(), "ab  ", "from column 2 the next stop is column 4");
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].edit_text(), "ab      ", "from a tab stop a full tab width is inserted");
        assert!(app.editor_focused, "Tab must not move keyboard focus away from the editor");

        app.config.tab_size = 3;
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].edit_text(), "ab       ", "column 8 -> 9 with a tab width of 3");
    }

    fn click(app: &mut AlexCodeApp, ctx: &egui::Context, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(app, ctx, vec![Event::PointerMoved(pos)]);
        frame(app, ctx, vec![button(true)]);
        frame(app, ctx, vec![button(false)]);
        frame(app, ctx, vec![]);
    }

    #[test]
    fn settings_dialog_edits_the_config() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn main() {}");
        assert!(!app.show_settings);
        frame(&mut app, &ctx, vec![key(Key::Comma, Modifiers::COMMAND)]);
        assert!(app.show_settings, "Ctrl+, opens the settings dialog");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);

        assert!(app.config.restore_session && !app.config.insert_spaces);
        let pos = app.probe["settings.restore_session"].center();
        click(&mut app, &ctx, pos);
        assert!(!app.config.restore_session, "clicking the checkbox turns session restore off");
        let pos = app.probe["settings.insert_spaces"].center();
        click(&mut app, &ctx, pos);
        assert!(app.config.insert_spaces);

        app.config.font_size = 22.0;
        app.config.tab_size = 8;
        let pos = app.probe["settings.reset"].center();
        click(&mut app, &ctx, pos);
        assert_eq!(app.config, EditorConfig::default(), "the reset button restores every default");

        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        assert!(!app.show_settings, "Esc closes the dialog");

        // 命令面板 / 選單共用的命令
        app.run_cmd(Cmd::Settings);
        assert!(app.show_settings);
    }

    /// 0: 短行、1: 很長的一行（自動換行後佔好幾列）、2: 短行、3: 結尾的空行
    fn wrap_source() -> String {
        format!("fn a() {{}}\n// {}\nfn b() {{}}\n", "word ".repeat(200))
    }

    fn cursor(app: &AlexCodeApp) -> (usize, usize) {
        let head = app.tabs[0].selections[0].head;
        (head.line, head.col)
    }

    #[test]
    fn word_wrap_wraps_long_lines_and_keeps_logical_line_numbers() {
        let ctx = ctx();
        let mut app = app_with(&ctx, &wrap_source());
        assert!(!app.config.word_wrap, "word wrap is off by default");
        assert!(app.row_lines.is_empty(), "without word wrap every row is one line");

        app.config.word_wrap = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let rows = app.row_lines.clone();
        assert!(rows.len() >= 4 + 3, "the 1000-character line must take several rows, got {rows:?}");
        assert_eq!((rows[0], rows[1]), (0, 1));
        assert_eq!(rows[rows.len() - 2..], [2, 3], "the lines after the wrapped one keep their numbers");
        assert!(rows.windows(2).all(|w| w[1] == w[0] || w[1] == w[0] + 1));

        // 游標位置以「行」回報，不是畫面上的列
        let i = AlexCodeApp::char_index_of(&app.tabs[0], 2, 3);
        app.pending_select = Some((i, i));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (2, 3));
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!(app.tabs[0].lines[2], "fn Xb() {}", "typing lands on the logical line");
        assert_eq!(cursor(&app), (2, 4));

        // 在換行後的長行裡往下一列：仍在同一行，欄位往後移
        let i = AlexCodeApp::char_index_of(&app.tabs[0], 1, 0);
        app.pending_select = Some((i, i));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        let (line, col) = cursor(&app);
        assert_eq!(line, 1, "arrow down inside a wrapped line stays on that line");
        assert!(col > 20, "the cursor moved one row down, i.e. far to the right within the line (col {col})");

        // 文字內容不因換行顯示而改變
        assert_eq!(app.tabs[0].lines.len(), 4);
        assert_eq!(app.tabs[0].lines[1].chars().count(), 3 + 200 * 5);

        app.config.word_wrap = false;
        frame(&mut app, &ctx, vec![]);
        assert!(app.row_lines.is_empty());
    }

    #[test]
    fn sticky_scroll_follows_wrapped_lines() {
        let ctx = ctx();
        // 與 sticky_source 相同的結構，但每個敘述都長到換行後佔好幾列
        let long = format!("; // {}\n", "long comment ".repeat(30));
        let src = sticky_source().replace(";\n", &long);
        let mut app = app_with(&ctx, &src);
        app.config.word_wrap = true;
        settle(&mut app, &ctx);
        assert!(app.row_lines.len() > app.tabs[0].lines.len() * 2, "the statements wrap");
        assert_eq!(app.sticky_lines(), Vec::<usize>::new());

        goto_line(&mut app, &ctx, 60);
        assert_eq!(cursor_line(&app), 60);
        assert_eq!(app.sticky_lines(), vec![0, 1]);
        assert!(cursor_line(&app) >= app.sticky_below);
        goto_line(&mut app, &ctx, 200);
        assert_eq!(cursor_line(&app), 200);
        assert_eq!(app.sticky_lines(), vec![0, 124]);
        assert!(app.sticky_below > 124 && app.sticky_below <= 200, "first uncovered line {}", app.sticky_below);
    }

    #[test]
    fn word_wrap_is_a_saved_setting() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let state_file = dir.path().join("state.json");
        let mut app = app_with(&ctx, "fn main() {}");
        app.state_path = Some(state_file.clone());
        app.run_cmd(Cmd::Settings);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let pos = app.probe["settings.word_wrap"].center();
        click(&mut app, &ctx, pos);
        assert!(app.config.word_wrap, "clicking the checkbox turns word wrap on");
        app.persist_state();
        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&state_file).unwrap()).unwrap();
        assert_eq!(json["config"]["word_wrap"], true);

        let mut app2 = AlexCodeApp::new();
        app2.apply_state(AppState::load(&state_file).unwrap());
        assert!(app2.config.word_wrap);

        // v5.5.0 的 state.json（沒有 config 區塊）：維持原本不換行的行為
        let old = write(&dir, "old.json", r#"{"theme":"Matrix","font_size":17.0}"#);
        let mut app3 = AlexCodeApp::new();
        app3.apply_state(AppState::load(&old).unwrap());
        assert!(!app3.config.word_wrap);
    }

    /// 一幀一幀跑到背景工作完成為止
    fn pump_until(app: &mut AlexCodeApp, ctx: &egui::Context, what: &str, done: impl Fn(&AlexCodeApp) -> bool) {
        let t = std::time::Instant::now();
        while !done(app) {
            assert!(t.elapsed() < std::time::Duration::from_secs(10), "{what} never finished");
            std::thread::sleep(std::time::Duration::from_millis(5));
            frame(app, ctx, vec![]);
        }
    }

    /// a.rs（3 處，CRLF）、sub/b.txt（1 處）、c.rs（沒有命中）；回傳 (資料夾, a, b, c)
    fn replace_project() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let a = write(&dir, "a.rs", "foo foo\r\nbar Foo\r\n");
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let b = write(&dir, "sub/b.txt", "x foo");
        let c = write(&dir, "c.rs", "nothing here");
        (dir, a, b, c)
    }

    fn read(p: &Path) -> String {
        String::from_utf8(std::fs::read(p).unwrap()).unwrap()
    }

    #[test]
    fn replace_in_files_previews_then_writes_and_reloads_open_tabs() {
        let ctx = ctx();
        let (dir, a, b, c) = replace_project();
        let mut app = app_with(&ctx, "x");
        app.set_folder(dir.path().to_path_buf());
        app.open_path(&a);
        let a_key = a.to_string_lossy().to_string();
        app.run_cmd(Cmd::FindInFiles);
        app.find_query = "foo".into();
        app.replace_in_files_text = "qux".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.probe.contains_key("fif.apply"), "nothing can be written before a preview");

        // 預覽：只計數，不寫入
        let pos = app.probe["fif.preview"].center();
        click(&mut app, &ctx, pos);
        pump_until(&mut app, &ctx, "replace preview", |app| app.replace_plan.is_some());
        let plan = app.replace_plan.as_ref().unwrap();
        assert_eq!((plan.total, plan.hits.len()), (4, 2), "4 matches in 2 files");
        assert_eq!(read(&a), "foo foo\r\nbar Foo\r\n", "the preview writes nothing");
        assert_eq!(read(&b), "x foo");

        // 確認後才寫入；已開啟的分頁跟著更新
        frame(&mut app, &ctx, vec![]);
        let pos = app.probe["fif.apply"].center();
        click(&mut app, &ctx, pos);
        pump_until(&mut app, &ctx, "replace", |app| app.replace_job.is_none() && app.replace_plan.is_none());
        assert_eq!(read(&a), "qux qux\r\nbar qux\r\n", "line endings are kept");
        assert_eq!(read(&b), "x qux");
        assert_eq!(read(&c), "nothing here");
        let tab = app.tabs.iter().find(|t| t.file_path.as_deref() == Some(a_key.as_str())).unwrap();
        assert_eq!(tab.edit_text(), "qux qux\nbar qux\n", "the open tab shows the new content");
        assert!(!tab.modified);
        assert!(app.status_message.contains("已取代 4 處（2 個檔案）"), "{}", app.status_message);
        assert!(app.find_results.is_empty());

        // 自己寫入的版本不會被當成外部修改
        app.handle_disk_events(vec![(a.clone(), stamp_of(&a))]);
        assert!(app.disk_conflict.is_none());
        frame(&mut app, &ctx, vec![]);
    }

    #[test]
    fn replace_in_files_refuses_unsaved_tabs_and_stale_previews() {
        let ctx = ctx();
        let (dir, a, b, _c) = replace_project();
        let mut app = app_with(&ctx, "x");
        app.set_folder(dir.path().to_path_buf());
        app.open_path(&a);
        let idx = app.active_tab_idx;
        app.tabs[idx].replace_all_text("foo, edited but not saved");
        assert!(app.tabs[idx].modified);
        app.run_cmd(Cmd::FindInFiles);
        app.find_query = "foo".into();
        app.replace_in_files_text = "qux".into();

        app.start_replace_preview(&ctx);
        pump_until(&mut app, &ctx, "replace preview", |app| app.replace_plan.is_some());
        app.apply_replace_plan(&ctx);
        assert!(app.replace_job.is_none(), "nothing is written while an affected tab has unsaved changes");
        let (is_error, msg) = app.replace_note.clone().unwrap();
        assert!(is_error && msg.contains("a.rs"), "{msg}");
        assert!(app.replace_plan.is_some(), "the preview is kept so the user can save and retry");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(read(&a), "foo foo\r\nbar Foo\r\n");
        assert_eq!(read(&b), "x foo");
        assert_eq!(app.tabs[idx].edit_text(), "foo, edited but not saved");

        // 預覽之後改了取代字串：預覽作廢，不能直接執行
        app.replace_in_files_text = "other".into();
        frame(&mut app, &ctx, vec![]);
        assert!(app.replace_plan.is_none());
        app.apply_replace_plan(&ctx);
        assert!(app.replace_job.is_none());
        assert_eq!(read(&b), "x foo");

        // 找不到任何命中：沒有預覽可以執行
        app.find_query = "no such text".into();
        app.start_replace_preview(&ctx);
        pump_until(&mut app, &ctx, "empty preview", |app| app.replace_job.is_none());
        assert!(app.replace_plan.is_none());
        assert!(app.replace_note.as_ref().is_some_and(|(e, m)| !e && m.contains("no such text")));
    }

    #[test]
    fn large_files_open_in_the_background() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let small = write(&dir, "small.rs", "fn main() {}");
        // 約 2.6 MB、CRLF 換行
        let text = "0123456789 abcdefghij 0123456789 abcdefghi\r\n".repeat(60_000);
        let big = write(&dir, "big.log", &text);
        assert!(!AlexCodeApp::opens_in_background(&small));
        assert!(AlexCodeApp::opens_in_background(&big));
        #[cfg(windows)]
        assert!(AlexCodeApp::opens_in_background(Path::new(r"\\server\share\notes.txt")), "network paths never block the window");

        // 小檔案維持原本的行為：open_path 回來時內容已經在分頁裡
        let mut app = app_with(&ctx, "x");
        app.open_path(&small);
        assert!(app.loading.is_empty());
        assert_eq!(app.tabs[app.active_tab_idx].edit_text(), "fn main() {}");

        // 大檔案：分頁立刻出現，內容稍後才到
        app.open_path(&big);
        let idx = app.active_tab_idx;
        let id = app.tabs[idx].id;
        assert!(app.loading.contains_key(&id), "the tab appears at once and is marked as loading");
        assert_eq!(app.tabs[idx].file_path.as_deref(), Some(big.to_string_lossy().as_ref()));
        assert_eq!(app.tabs[idx].lines.len(), 1, "open_path did not read the file on the UI thread");
        assert!(app.recent_files.contains(&big.to_string_lossy().to_string()));

        // 載入中的分頁不能存檔（否則會把空白內容寫回去蓋掉檔案）
        assert!(!app.save_tab(idx, false));
        assert_eq!(std::fs::metadata(&big).unwrap().len(), text.len() as u64);
        // 再開一次同一個檔案不會多出分頁
        let tabs = app.tabs.len();
        app.open_path(&big);
        assert_eq!(app.tabs.len(), tabs);

        pump_until(&mut app, &ctx, "background load", |app| app.loading.is_empty());
        assert_eq!(app.tabs[idx].id, id, "the tab keeps its identity");
        assert_eq!(app.tabs[idx].lines.len(), 60_001);
        assert_eq!(app.tabs[idx].line_ending, "\r\n");
        assert!(!app.tabs[idx].modified);
        assert!(app.status_message.starts_with("已開啟"), "{}", app.status_message);

        // 之後與一般分頁相同：自己讀到的版本不算外部修改，存檔寫回原本的位元組
        let rev = app.tabs[idx].revision();
        app.handle_disk_events(vec![(big.clone(), stamp_of(&big))]);
        assert_eq!(app.tabs[idx].revision(), rev);
        assert!(app.save_tab(idx, false));
        assert!(std::fs::read(&big).unwrap() == text.as_bytes(), "saving writes the same bytes back");
    }

    /// 加入一個內容「還在載入」的分頁；回傳 (分頁 id, 用來交付載入結果的通道)
    fn loading_tab(app: &mut AlexCodeApp, dir: &tempfile::TempDir, name: &str) -> (u64, Sender<Result<EditorBuffer, String>>) {
        let mut b = EditorBuffer::new();
        b.file_path = Some(dir.path().join(name).to_string_lossy().to_string());
        let id = b.id;
        app.tabs.push(b);
        app.active_tab_idx = app.tabs.len() - 1;
        let (tx, rx) = unbounded();
        app.loading.insert(id, LoadJob { rx, goto_line: None });
        (id, tx)
    }

    #[test]
    fn loading_tab_shows_a_placeholder_until_its_content_arrives() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with(&ctx, "x");
        let (id, tx) = loading_tab(&mut app, &dir, "slow.rs");
        app.loading.get_mut(&id).unwrap().goto_line = Some(2);

        // 載入中：畫面照常繪製，但沒有可以輸入的編輯區
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.editor_focused);
        frame(&mut app, &ctx, vec![Event::Text("typed".into())]);
        assert_eq!(app.tabs[1].edit_text(), "", "nothing can be typed into a tab that is still loading");
        assert!(!app.tabs[1].modified);
        // 外部變更偵測不會在載入中同步讀檔
        let p = PathBuf::from(app.tabs[1].file_path.clone().unwrap());
        std::fs::write(&p, "on disk").unwrap();
        app.handle_disk_events(vec![(p.clone(), stamp_of(&p))]);
        assert_eq!(app.tabs[1].edit_text(), "");
        // 載入期間用過的快取（尋找、Log 篩選）不能留到內容到了之後
        app.show_find_bar = true;
        app.find_bar_text = "b".into();
        app.filter_query = "c".into();
        app.filter_engine.compile("c");
        frame(&mut app, &ctx, vec![]);
        app.filter_query.clear();
        app.filter_engine.compile("");
        app.show_find_bar = false;
        frame(&mut app, &ctx, vec![]);

        tx.send(Ok(EditorBuffer::from_text("a\nb\nc\nd"))).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert!(app.loading.is_empty());
        assert_eq!(app.tabs[1].id, id);
        assert_eq!(app.tabs[1].edit_text(), "a\nb\nc\nd");
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.editor_focused, "the editor is back once the content is there");
        assert_eq!(app.tabs[1].selections[0].head.line, 2, "the line requested while loading is applied afterwards");

        // 編輯區拿到的是載入後的全文：打字不會把內容換成載入前的空白
        frame(&mut app, &ctx, vec![Event::Text("Z".into())]);
        assert_eq!(app.tabs[1].edit_text(), "a\nb\nZc\nd");
        app.show_find_bar = true;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 1, "find sees the loaded text");
        assert!(app.save_tab(1, false), "a loaded tab can be saved again");
        assert_eq!(read(&p), "a\nb\nZc\nd");
    }

    #[test]
    fn failed_background_load_closes_the_tab_and_reports_it() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with(&ctx, "x");
        let (id, tx) = loading_tab(&mut app, &dir, "gone.rs");
        tx.send(Err("存取被拒".to_string())).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert!(app.loading.is_empty());
        assert!(app.tabs.iter().all(|b| b.id != id), "a file that could not be read does not stay open as an empty tab");
        assert!(app.status_message.contains("開啟失敗") && app.status_message.contains("存取被拒"), "{}", app.status_message);

        // 讀取執行緒沒有回報就結束：同樣收掉，不會永遠停在「載入中」
        let (id, tx) = loading_tab(&mut app, &dir, "dead.rs");
        drop(tx);
        frame(&mut app, &ctx, vec![]);
        assert!(app.loading.is_empty() && app.tabs.iter().all(|b| b.id != id));

        // 載入中就被關掉的分頁：結果到了也不會出錯
        let (id, tx) = loading_tab(&mut app, &dir, "closed.rs");
        let idx = app.tabs.iter().position(|b| b.id == id).unwrap();
        app.close_tab(idx);
        assert!(app.loading.is_empty());
        let _ = tx.send(Ok(EditorBuffer::from_text("late")));
        frame(&mut app, &ctx, vec![]);
        assert!(app.tabs.iter().all(|b| b.id != id));
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
