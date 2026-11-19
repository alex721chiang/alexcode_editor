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
use crate::core::{disk_signature, HotExit, UnsavedTab};
use crate::editor::{
    decide as pair_decide, detect_and_decode, is_relevant as pair_relevant, should_delete_pair, Action as PairAction,
    sticky_headers, EditorBuffer, FileEncoding, Position, Selection,
};
use crate::core::config::{MAX_FONT_SIZE, MAX_TAB_SIZE, MIN_FONT_SIZE, MIN_TAB_SIZE};
use crate::index::{extract_symbols, Symbol, SymbolLang};
use crate::git::{blame_contents, blame_status_text, diff_line_states, LineInfo, ADDED, DELETED_ABOVE, MODIFIED};
use crate::git::{align_trimmed, change_starts, Row as DiffRow, RowType as DiffRowType};
use crate::filter::{FilterEngine, FilterLogic};
use crate::lsp::{
    char_col_to_utf16, completion_params, extract_completion_items, extract_definition, text_document_position_params, utf16_to_char_col, Diagnostic, Launcher,
    LspManager, LspStatus, OpenDoc, RequestId, ServerKind,
};
use crate::markdown::{compute_graph_layout, MarkdownLinkIndex};
use crate::plugins::PluginManager;
use crate::search::{command_match_score, find_all, replace_all, replace_range, search_in_files, FileMatch, FindOptions, Finder, SearchStats};
use crate::search::{apply_replace_in_files, plan_replace_in_files, ReplaceHit, ReplaceResult};
use crate::snippets::SnippetsController;
// （`core::available_shells` 是「在此開啟終端機」選單用的外部終端機清單，這裡的是內建終端機能用的 shell）
use crate::terminal::{available_shells as pty_shells, PtySession, ShellChoice, TerminalCell, VtParser};
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
    GotoDefinition,
    FindInFiles,
    CommandPalette,
    DiffHead,
    SplitEditor,
    SplitOrientation,
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
    /// 終端機下方輸入列（含上方間距）上一幀的高度：輸出區留給它這麼多，面板才不會每一幀變大或變小
    term_input_h: f32,
    /// 終端機輸出區有焦點時輸入法組字中的文字（還沒送進 shell，畫在游標處）
    term_preedit: String,
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
    /// 在虛擬終端（ConPTY / PTY）裡執行的 shell；終端機第一次打開時啟動
    term_pty: Option<PtySession>,
    /// 虛擬終端啟動失敗：改回逐指令執行（`run_terminal_command`）
    term_pty_failed: bool,
    /// shell 結束過一次之後就不再自動啟動，等使用者在輸入列按 Enter
    term_pty_exited: bool,
    /// 可以選的 shell（`shells()` 第一次用到時才找）
    term_shells: Vec<ShellChoice>,
    /// 語言伺服器（rust-analyzer / clangd / pylsp）：開著那種檔案時才啟動，開著的文件每一幀同步給它
    lsp: LspManager,
    /// 送給語言伺服器的文件的大小快取：分頁 id → (內容版本, 位元組數)。大檔案（大檔案檢視）不送
    lsp_sizes: HashMap<u64, (u64, usize)>,
    /// 跳到定義：等著送出的位置（檔案、行、UTF-16 欄；下一次 `lsp_tick` 同步完文件後才送），
    /// 與已送出、在等回覆的請求（送出的時間：太久沒回就放棄）
    def_want: Option<(String, usize, usize)>,
    def_wait: Option<(RequestId, std::time::Instant)>,
    /// 滑鼠停留說明（hover）：指標停在哪個字元上、從什麼時候開始；停夠久之後等著送出的請求、
    /// 已送出在等回覆的請求，與收到的說明（空字串表示那裡沒有說明）
    hover_at: Option<(HoverKey, std::time::Instant)>,
    hover_want: Option<(String, usize, usize, HoverKey)>,
    hover_wait: Option<(RequestId, HoverKey)>,
    hover_text: Option<(HoverKey, String)>,
    /// 自動完成：目前的選單、等著送出 / 已送出在等回覆的請求、要接受的項目（`items` 的索引）、
    /// 按了 Ctrl+Space、上一幀選單畫在哪裡（點選單時編輯區會失去焦點，指標在選單上就不關閉）
    comp: Option<CompletionMenu>,
    comp_want: Option<CompletionReq>,
    comp_wait: Option<(RequestId, CompletionReq)>,
    comp_accept: Option<usize>,
    comp_invoke: bool,
    comp_rect: Option<Rect>,
    comp_caret: Option<Rect>,
    /// 遠端 AI 補全（設定裡啟用時，Ctrl+Space 另外在背景問一次）：送出請求的背景執行緒，與在等的那一個
    ai: crate::ai::RemoteAi,
    ai_wait: Option<(u64, AiReq)>,
    /// 矩形選取、是否正在拖曳，與上一幀算出的每一行選取範圍（行, 起欄, 迄欄）
    box_sel: Option<BoxSel>,
    box_dragging: bool,
    box_cols: Vec<(usize, usize, usize)>,
    /// Shift+Alt+方向鍵：要擴充（或從游標開始）矩形選取的方向，排版後在編輯區套用
    box_keys: Vec<Key>,
    /// 分割檢視：另一欄顯示的分頁與哪一欄有焦點（有焦點的那一欄顯示目前分頁）
    split: Option<SplitView>,
    /// 每個分頁上一次在一般編輯區的捲動位置（分割檢視換欄時讓另一欄接著用）
    editor_offsets: HashMap<u64, Vec2>,
    /// 下一次畫這個分頁的一般編輯區時要用的捲動位置
    pending_offset: Option<(u64, Vec2)>,
    /// 在分割檢視的另一欄按下滑鼠換過去之後、放開之前：選取的起點（字元位置），由編輯區跟著指標延伸選取
    split_drag: Option<usize>,
    /// 上一幀分割檢視中間分隔線的 x（沒有分割時 None）
    split_mid: Option<f32>,
    /// 在獨立視窗裡顯示的分頁（分頁 id）
    detached: Vec<u64>,
    /// 各獨立視窗的 Git 標記快取：((revision, HEAD 指標), 標記)
    detached_git: HashMap<u64, DetachedGit>,
    /// 各獨立視窗上一幀量到的位置與大小（存檔用）
    detached_rects: HashMap<u64, Rect>,
    /// 從上次的狀態還原的獨立視窗：開視窗時放在這裡（之後不再改，免得和使用者移動視窗互相干擾）
    detached_place: HashMap<u64, Rect>,

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
    /// 上一次排的探測文字（`check_font_atlas`：拿回來的不是同一份表示字型圖集重建過）
    atlas_probe: Option<Arc<egui::Galley>>,
    /// 尋找列命中位置快取
    find_cache: ((u64, u64, String), Vec<(usize, usize)>),
    /// 大檔案裡尋找列的命中數等內容停止變動才重算：(最後一次看到的 (分頁 id, 內容版本), 發現變動的時間)
    find_seen: ((u64, u64), std::time::Instant),
    /// Log 篩選命中的行號快取
    filter_cache: (Option<FilterKey>, Vec<usize>),
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
    tree_chan: (Sender<TreeMsg>, Receiver<TreeMsg>),

    // ---- 尋找 / 取代 ----
    find_opts: FindOptions,
    /// Markdown 預覽用的尋找條件快取：(查詢與選項組成的鍵, 編譯好的條件)
    md_find: (String, Finder),
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

    // ---- 外部交來要開啟的檔案：單一實例（其他實例）、macOS 的 Finder 開檔（Apple 事件） ----
    incoming: Vec<Receiver<Vec<PathBuf>>>,

    // ---- Git 行號區標記 ----
    /// 各分頁（依 id）在 HEAD 的內容；None = 不在 Git 儲存庫內或未追蹤
    git_head: HashMap<u64, Option<Arc<Vec<String>>>>,
    git_chan: (Sender<GitHeadMsg>, Receiver<GitHeadMsg>),
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

    // ---- Git blame ----
    /// 各分頁（依 id）最近一次 blame 的結果：(當時的內容版本, 每一行的資訊)；
    /// 資訊為 None = 不在 Git 儲存庫內 / 未追蹤 / 檔案太大而略過
    blame: HashMap<u64, (u64, Option<BlameLines>)>,
    blame_chan: (Sender<BlameResult>, Receiver<BlameResult>),
    /// 背景執行中的 blame：(分頁 id, 內容版本)。一次只跑一個 git
    blame_running: Option<(u64, u64)>,
    /// 目前分頁的內容最後一次變動：(分頁 id, 內容版本, 發現變動的時間)
    blame_seen: (u64, u64, std::time::Instant),

    // ---- Git 差異檢視 ----
    /// 目前分頁與 HEAD 的對齊結果
    diff_cache: Option<DiffCache>,
    /// 目前選取的是第幾段變更
    diff_focus: usize,
    /// 下一幀把選取的那段變更捲到畫面上
    diff_scroll_focus: bool,

    // ---- 大檔案檢視 ----
    /// 文件達到幾個位元組就改用大檔案檢視（測試會調低）
    large_view_bytes: usize,
    /// 目前分頁的大小統計快取：鍵為 (分頁 id, 內容版本)，值為 (位元組數, 最寬一行的欄數)。
    /// 欄數只在文件達到門檻時才計算。
    large_stats: ((u64, u64), (usize, usize)),
    /// 大檔案檢視開著自動換行時，很長的行自己切好的列（依分頁、內容版本與折行寬度快取）
    large_rows: LongRows,
    /// 大檔案檢視中輸入法正在組字的文字（還沒寫進文件）
    large_preedit: String,
    /// 大檔案檢視組字中輸入法報的游標位置（組字文字裡的第幾個字元；沒有報時是組字文字的長度）
    large_ime_at: usize,
    /// 一般編輯區輸入法組字時，輸入法報的游標 / 選字位置（相對於組字文字的字元範圍）；沒在組字時是 None
    editor_ime_active: Option<std::ops::Range<usize>>,
    /// 大檔案檢視中內容最後一次變動：((分頁 id, 內容版本), 發現變動的時間)；Git 標記等內容停下來才重算
    large_marks_seen: ((u64, u64), std::time::Instant),
    /// 拖曳選取到可見範圍外時，下一幀大檔案檢視要捲到的位置：(分頁 id, 捲動位移)
    large_scroll_to: Option<(u64, Vec2)>,
    /// 大檔案檢視上一幀的折行寬度（沒有開自動換行時為 None）：上下鍵要用它算出游標在哪一列
    large_wrap_w: Option<f32>,
    /// Markdown 檔達到多少位元組就以原始碼顯示（測試中可調低）
    md_preview_bytes: usize,
    /// 使用者選擇仍要用閱讀模式的大 Markdown 分頁（依 id）
    md_big_preview: HashSet<u64>,

    // ---- 熱結束：尚未存檔的內容跨次啟動保留 ----
    /// 上次寫進備份檔的是哪些分頁的哪個版本：(分頁 id, 內容版本, 編碼, 路徑)
    hot_exit_key: HotExitKey,
    /// 正在背景執行緒寫的備份（內容很大時才用）：寫完交回 (寫的是哪些分頁的哪個版本, 內容大小, 寫入結果)
    hot_exit_job: Option<std::thread::JoinHandle<(HotExitKey, usize, std::io::Result<()>)>>,
    /// 上次寫入備份檔的時間與內容大小（位元組）：內容很大時降低寫入頻率
    hot_exit_written: (std::time::Instant, usize),
    /// 等背景載入完成後要套回分頁的未存檔內容（依分頁 id）
    hot_restore: HashMap<u64, UnsavedTab>,

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

/// 排版快取記下的字型狀態：(pixels_per_point, 字型圖集寬度)，變了舊排版就不能用。
/// egui 0.33 起拿不到圖集本身（之前用 `Arc` 是不是同一個來判斷）；圖集快滿時的重建另由 `check_font_atlas` 處理
type FontAtlas = (f32, usize);

/// Log 篩選快取的鍵：(分頁 id, 內容版本, 篩選字串, 兩個篩選選項)
type FilterKey = (u64, u64, String, bool, bool);
/// 檔案總管背景讀取資料夾的結果：(資料夾, 內容或錯誤訊息)
type TreeMsg = (PathBuf, Result<Vec<DirEntryInfo>, String>);
/// 背景讀取 HEAD 版本的結果：(分頁 id, HEAD 的內容；None = 不在 Git 儲存庫裡)
type GitHeadMsg = (u64, Option<Vec<String>>);

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

/// 差異檢視的對齊結果，以及它是針對哪一份內容算的
struct DiffCache {
    tab_id: u64,
    revision: u64,
    /// 當時的 HEAD 內容（握著它，位址才不會被之後的版本重複使用）
    head: Arc<Vec<String>>,
    /// None = 差異太大，沒有對齊
    rows: Option<Arc<Vec<DiffRow>>>,
}

/// 差異檢視的比對表格上限（HEAD 與目前內容中間不同部分的行數相乘）：約 2000 × 2000 行，佔 16 MB
const DIFF_MAX_CELLS: usize = 4_000_000;

/// 一份文件每一行（0-based）的 blame 資訊
type BlameLines = Arc<HashMap<usize, LineInfo>>;

/// 背景 blame 的結果：(分頁 id, 當時的內容版本, 每一行的資訊)
type BlameResult = (u64, u64, Option<HashMap<usize, LineInfo>>);

/// 內容變動後要停頓這麼久才重新執行 blame（打字途中不反覆啟動 git）
const BLAME_DELAY: std::time::Duration = std::time::Duration::from_millis(600);

/// 狀態列上 blame 文字的長度上限（字元）；完整內容在滑鼠停留時顯示
const BLAME_MAX_CHARS: usize = 70;

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
    /// 在獨立的視窗裡顯示這個分頁（唯讀）
    Detach(usize),
}

/// 超過此大小的文件不做語法高亮（仍可編輯與搜尋）
const HIGHLIGHT_MAX_BYTES: usize = 2 * 1024 * 1024;

/// 達到這個大小的文件改用大檔案檢視（只排版畫面上看得到的行）。
/// 一般編輯區把整份文字交給 egui 排成一個 Galley，每個位元組的文字約佔 200 位元組記憶體
/// （`tests/memory_report.rs` 的量測：3 MB 的檔案約 600 MB、4 MB 約 790 MB、29 MB 約 5.7 GB）。
/// 門檻就是不再做語法高亮的大小：全文超過 `HIGHLIGHT_MAX_BYTES` 的文件在一般編輯區裡也沒有語法高亮與
/// Sticky Scroll，換成大檔案檢視不會少東西；更小的文件要留在一般編輯區才有語法高亮。
/// （`+ 2`：這個門檻比的是每行長度 + 1 的總和，也就是全文長度 + 1，而且用 `>=` 比。）
const LARGE_VIEW_BYTES: usize = HIGHLIGHT_MAX_BYTES + 2;

/// Markdown 檔達到這個大小就不用閱讀 / 雙欄模式，改以原始碼顯示：那兩種模式每一幀都解析並排版整份文件
/// （`large_file_cost_report` 的量測：1 MB 的檔案每一幀 181 ms、4 MB 676 ms；這個門檻大約是每一幀 45 ms）。
const MD_PREVIEW_MAX_BYTES: usize = 256 * 1024;

/// 閱讀模式下每 MB 的 Markdown 大約多用幾 MB 記憶體（`tests/memory_report.rs` 的量測：135–155 倍）
const MD_PREVIEW_MEM_FACTOR: f64 = 150.0;

/// 大檔案檢視中超過這麼多位元組的行算「很長」：不整行排版，只排畫面上看得到的那一段
///（不換行時是水平方向的一段，自動換行時是自己切好的那幾列），避免單獨一行就排出巨大的版面
const LARGE_VIEW_MAX_COLS: usize = 20_000;

/// 大檔案檢視中，內容停止變動這麼久之後才重算 Git 行號區標記
/// （比對要掃過整份文件與 HEAD 內容，每按一個鍵就算一次會拖慢輸入）
const LARGE_GIT_DELAY: std::time::Duration = std::time::Duration::from_millis(300);

/// 大檔案檢視中，內容停止變動這麼久之後才重算尋找列的命中數（要串接全文再搜尋一次）
const LARGE_FIND_DELAY: std::time::Duration = std::time::Duration::from_millis(300);

/// 一行在等寬字型下大約佔幾欄（全形字算 2 欄、Tab 算 4 欄）：決定大檔案檢視不換行時可以往右捲多遠
fn display_cols(line: &str) -> usize {
    use unicode_width::UnicodeWidthChar;
    if line.is_ascii() && !line.contains('\t') {
        return line.len();
    }
    line.chars().map(|ch| if ch == '\t' { 4 } else { ch.width().unwrap_or(1).max(1) }).sum()
}

/// 第 `col` 個字元在 `line` 裡的位元組位置（超過行尾時是行尾）
fn char_byte(line: &str, col: usize) -> usize {
    if line.is_ascii() {
        return col.min(line.len());
    }
    line.char_indices().nth(col).map_or(line.len(), |(i, _)| i)
}

/// 前 `col` 個字元排出來的寬度（像素）。`width` 是每個字元的寬度；全是 ASCII 又沒有 Tab 時直接以 `cw` 計算。
/// 逐字累加用 f64：很長的行（幾十萬像素寬）以 f32 累加會差到幾十像素
fn prefix_x(line: &str, col: usize, cw: f32, mut width: impl FnMut(char) -> f32) -> f32 {
    if line.is_ascii() && !line.contains('\t') {
        return col.min(line.len()) as f32 * cw;
    }
    line.chars().take(col).map(|c| f64::from(width(c))).sum::<f64>() as f32
}

/// 不換行時的一行很長的文字，只排版水平範圍 `x0..x1`（相對於行首的像素）涵蓋到的字元：
/// 回傳 (起始字元索引, 結束字元索引, 起始字元左緣的 x)。`width` / `cw` 同 `prefix_x`
fn line_window(line: &str, x0: f32, x1: f32, cw: f32, mut width: impl FnMut(char) -> f32) -> (usize, usize, f32) {
    if line.is_ascii() && !line.contains('\t') {
        let n = line.len();
        let a = ((x0 / cw).floor().max(0.0) as usize).min(n);
        let b = ((x1 / cw).ceil().max(0.0) as usize).clamp(a, n);
        return (a, b, a as f32 * cw);
    }
    // 逐字累加用 f64（同 `prefix_x`）
    let mut start = None;
    let (x0, x1) = (f64::from(x0), f64::from(x1));
    let (mut x, mut i) = (0.0_f64, 0);
    for ch in line.chars() {
        if x >= x1 && start.is_some() {
            break;
        }
        let w = f64::from(width(ch));
        if start.is_none() && x + w > x0 {
            start = Some((i, x));
        }
        x += w;
        i += 1;
    }
    let (a, ax) = start.unwrap_or((i, x));
    (a, i.max(a), ax as f32)
}

/// 自動換行時把一行很長的文字切成寬度不超過 `max_w` 的列（不看字的邊界，排滿就換列）：
/// 每一列從第幾個字元、第幾個位元組開始。`width` / `cw` 同 `prefix_x`；一列至少放一個字元
fn wrap_rows(line: &str, max_w: f32, cw: f32, mut width: impl FnMut(char) -> f32) -> Vec<(usize, usize)> {
    if line.is_ascii() && !line.contains('\t') {
        let k = ((max_w / cw).floor() as usize).max(1);
        return (0..line.len().max(1)).step_by(k).map(|i| (i, i)).collect();
    }
    let mut starts = vec![(0, 0)];
    let mut x = 0.0_f32;
    for (i, (b, ch)) in line.char_indices().enumerate() {
        let w = width(ch);
        if x > 0.0 && x + w > max_w {
            starts.push((i, b));
            x = 0.0;
        }
        x += w;
    }
    starts
}

/// 大檔案檢視開著自動換行時，很長的行（超過 `LARGE_VIEW_MAX_COLS` 個位元組）自己切好的列。
/// 這些行的每一列各算一個捲動單位（其他行不論折成幾列都只算一個），所以捲動可以停在長行的中間，
/// 也只要排版畫面上的那幾列；總高度不必排版就算得出來
#[derive(Default)]
struct LongRows {
    /// (分頁 id, 內容版本, 折行寬度的位元)
    key: (u64, u64, u32),
    /// 依行號排好：(行, 這一行之前的長行一共多出幾列, 每一列從第幾個字元 / 位元組開始)
    lines: Vec<(usize, usize, RowStarts)>,
}

/// 一行切成的各列：每一列從第幾個字元、第幾個位元組開始（`wrap_rows`）
type RowStarts = Vec<(usize, usize)>;

impl LongRows {
    fn build(key: (u64, u64, u32), lines: &[String], max_w: f32, cw: f32, mut width: impl FnMut(char) -> f32) -> Self {
        let mut out = Vec::new();
        let mut extra = 0;
        for (i, l) in lines.iter().enumerate().filter(|(_, l)| l.len() > LARGE_VIEW_MAX_COLS) {
            let rows = wrap_rows(l, max_w, cw, &mut width);
            let n = rows.len();
            out.push((i, extra, rows));
            extra += n - 1;
        }
        LongRows { key, lines: out }
    }

    /// 所有長行一共比「每行一個單位」多出幾列
    fn extra(&self) -> usize {
        self.lines.last().map_or(0, |(_, before, rows)| before + rows.len() - 1)
    }

    /// 這一行是長行時，它的各列
    fn rows(&self, line: usize) -> Option<&[(usize, usize)]> {
        self.lines.binary_search_by_key(&line, |e| e.0).ok().map(|i| self.lines[i].2.as_slice())
    }

    /// 這一行的第一列是第幾個捲動單位
    fn unit_of(&self, line: usize) -> usize {
        let i = self.lines.partition_point(|e| e.0 < line);
        line + if i == 0 { 0 } else { self.lines[i - 1].1 + self.lines[i - 1].2.len() - 1 }
    }

    /// 第 `u` 個捲動單位是哪一行的第幾列
    fn at_unit(&self, u: usize) -> (usize, usize) {
        let i = self.lines.partition_point(|e| e.0 + e.1 <= u);
        if i == 0 {
            return (u, 0);
        }
        let (line, before, rows) = &self.lines[i - 1];
        let start = line + before;
        if u < start + rows.len() {
            (*line, u - start)
        } else {
            (u - before - (rows.len() - 1), 0)
        }
    }
}

/// 欄位 `col` 在長行的哪一列（落在兩列交界時算下一列的開頭，與 egui 的折行游標相同）
fn row_of(rows: &[(usize, usize)], col: usize) -> usize {
    rows.partition_point(|r| r.0 <= col).saturating_sub(1)
}

/// 把輸入法組字中的文字 `preedit` 插進 `line` 的第 `col` 個字元前面（`col` 超過行尾時接在最後）：
/// 大檔案檢視用它排出組字期間的那一行，文件本身不變
fn with_preedit(line: &str, col: usize, preedit: &str) -> String {
    let at = line.char_indices().nth(col).map_or(line.len(), |(i, _)| i);
    format!("{}{}{}", &line[..at], preedit, &line[at..])
}

/// 排版 `g`（大檔案檢視的一行，可能折成好幾列）裡離 `pos`（相對於排版左上角）最近的欄位。
/// 折行處的那個欄位算下一列的開頭，所以落在一列結尾右邊時退一格，游標才會留在那一列
fn wrapped_col_at(g: &egui::Galley, pos: Vec2) -> usize {
    let cur = g.cursor_from_pos(pos);
    let (row, column) = rc_of(g, cur);
    let at_wrapped_end = row + 1 < g.rows.len() && column > 0 && column >= g.rows[row].char_count_excluding_newline().0;
    cur.index.0 - usize::from(at_wrapped_end)
}

/// egui 0.32 起游標只剩字元索引（`CCursor`）：換算成排版上的（第幾列, 列裡的第幾個字元）。
/// 折行處依 `prefer_next_row` 決定算上一列的結尾還是下一列的開頭（與 0.31 的 `rcursor` 相同）
fn rc_of(g: &egui::Galley, c: CCursor) -> (usize, usize) {
    let l = g.layout_from_cursor(c);
    (l.row, l.column.0)
}

/// 字元索引 `index` 在排版 `g` 裡是第幾段（以 `\n` 分開的段落，也就是檔案的行）、段落裡的第幾個字元
/// （與 0.31 的 `pcursor` 相同；超過結尾時算最後一段）
fn para_of(g: &egui::Galley, index: usize) -> (usize, usize) {
    let (mut para, mut start, mut at) = (0, 0, 0);
    for row in &g.rows {
        if index <= at + row.char_count_excluding_newline().0 {
            return (para, index - start);
        }
        at += row.char_count_including_newline().0;
        if row.ends_with_newline {
            para += 1;
            start = at;
        }
    }
    (para, index.saturating_sub(start).min(at - start))
}

/// 一個字元排版時往右移的距離。egui 0.35 起字型裡沒有的字（例如沒有中文字型的 Linux / macOS CI 上的「中」）
/// `glyph_width` 回傳 0，但排版時會畫成替代字元、有寬度：這時改量排出那一個字的寬度
fn advance_of(f: &mut egui::epaint::text::FontsView<'_>, font: &FontId, ch: char) -> f32 {
    let w = f.glyph_width(font, ch);
    if w > 0.0 || ch == '\n' {
        return w;
    }
    f.layout_no_wrap(ch.to_string(), font.clone(), Color32::WHITE).size().x
}

/// 列裡第 `col` 個字元左緣的 x（相對於排版左上角；0.32 起字形的位置相對於所在的列）
fn row_x(row: &egui::epaint::text::PlacedRow, col: usize) -> f32 {
    row.pos.x + row.x_offset((col).into())
}

/// 欄位範圍 [a, b) 在排版 `g` 的每一列上佔的區段：(列的上緣, 列的下緣, 左, 右)，座標相對於排版左上角。
/// `eol` 為真時範圍延伸到行尾：最後一列即使沒有選到字元也算進來（呼叫端用來標出換行）
fn row_spans(g: &egui::Galley, a: usize, b: usize, eol: bool) -> Vec<(f32, f32, f32, f32)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, row) in g.rows.iter().enumerate() {
        let end = start + row.char_count_excluding_newline().0;
        let (lo, hi) = (a.max(start), b.min(end));
        if lo < hi || (eol && i + 1 == g.rows.len() && lo <= hi) {
            out.push((row.rect().top(), row.rect().bottom(), row_x(row, lo - start), row_x(row, hi - start)));
        }
        start = end;
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// 雙擊選字的範圍（欄位，含頭不含尾）：點到的字元，連同左右相鄰、同一類（字 / 空白 / 其他符號）的字元。
/// 點在一個字的右側（行尾，或右邊不是字）時選左邊那個字。只看點到的位置之後 `LARGE_VIEW_MAX_COLS` 個字元以內。
fn word_bounds(line: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().take(col.saturating_add(LARGE_VIEW_MAX_COLS)).collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let class = |c: char| if is_word_char(c) { 0 } else if c.is_whitespace() { 1 } else { 2 };
    let mut i = col.min(chars.len() - 1);
    if col > 0 && col <= chars.len() && is_word_char(chars[col - 1]) && !chars.get(col).is_some_and(|&c| is_word_char(c)) {
        i = col - 1;
    }
    let k = class(chars[i]);
    let a = (0..i).rev().take_while(|&j| class(chars[j]) == k).last().unwrap_or(i);
    let b = (i + 1..chars.len()).take_while(|&j| class(chars[j]) == k).last().map_or(i + 1, |j| j + 1);
    (a, b)
}

/// 拖曳選取時指標超出可見範圍 `over` 像素：每秒捲動幾像素（帶正負號）。
/// 超出一點點時每秒約 8 個 `unit`（行高或字寬），越遠越快，最多每秒 6000 像素。
fn edge_scroll_speed(over: f32, unit: f32) -> f32 {
    if over == 0.0 {
        return 0.0;
    }
    over.signum() * (unit * 8.0 + over.abs() * 10.0).min(6000.0)
}

/// 啟動語言伺服器的方式：執行 PATH 裡的伺服器程式。
/// 單元測試裡一律當作「未安裝」，免得每個開了 `.rs` 分頁的測試都去啟動真的 rust-analyzer
fn default_lsp_launcher() -> Launcher {
    #[cfg(not(test))]
    return Box::new(crate::lsp::spawn_server);
    #[cfg(test)]
    return Box::new(|_, _| Err(std::io::Error::from(std::io::ErrorKind::NotFound)));
}

/// 語言伺服器沒辦法用時，狀態列提示怎麼安裝
fn lsp_install_hint(kind: ServerKind) -> &'static str {
    match kind {
        ServerKind::RustAnalyzer => "安裝：rustup component add rust-analyzer（rustup 有代理程式但沒裝元件時，伺服器一啟動就會結束）",
        ServerKind::Clangd => "安裝 clangd（LLVM），並確認它在 PATH 裡",
        ServerKind::Pylsp => "安裝：pip install python-lsp-server，並確認 pylsp 在 PATH 裡",
    }
}

/// 語言伺服器診斷的顏色（不隨主題改變，與 VS Code 預設的錯誤 / 警告 / 資訊色相近）
const DIAG_ERROR: Color32 = Color32::from_rgb(0xF1, 0x4C, 0x4C);
const DIAG_WARNING: Color32 = Color32::from_rgb(0xCC, 0xA7, 0x00);
const DIAG_INFO: Color32 = Color32::from_rgb(0x37, 0x94, 0xFF);
const DIAG_HINT: Color32 = Color32::from_rgb(0x8C, 0x8C, 0x8C);

/// 嚴重程度（1 錯誤、2 警告、3 資訊、4 提示）的顏色與名稱
fn diag_color(severity: i32) -> Color32 {
    match severity {
        1 => DIAG_ERROR,
        2 => DIAG_WARNING,
        3 => DIAG_INFO,
        _ => DIAG_HINT,
    }
}

fn diag_label(severity: i32) -> &'static str {
    match severity {
        1 => "錯誤",
        2 => "警告",
        3 => "資訊",
        _ => "提示",
    }
}

/// 一個診斷在程式碼編輯區的排版 `g`（整份文字）上，每一列佔的區段：(上緣, 下緣, 左, 右)，座標相對於排版左上角。
/// `row_lines` 是自動換行時每一列所屬的行（不換行時是空的，列就是行）。位置以 UTF-16 單位計，換成字元；
/// 超過行尾的截到行尾。範圍是空的時標出那一個字元（在行尾時標前一個字元，空行標 `min_w` 寬）
fn diag_spans(g: &egui::Galley, lines: &[String], row_lines: &[usize], d: &Diagnostic, min_w: f32) -> Vec<(f32, f32, f32, f32)> {
    let mut out = Vec::new();
    if lines.is_empty() {
        return out;
    }
    let first = (d.start_line.max(0) as usize).min(lines.len() - 1);
    let last = (d.end_line.max(0) as usize).clamp(first, lines.len() - 1).min(first + 500);
    for (l, text) in lines.iter().enumerate().take(last + 1).skip(first) {
        let len = text.chars().count();
        let mut a = if l == first { utf16_to_char_col(text, d.start_char.max(0) as usize) } else { 0 };
        let mut b = if l as i32 == d.end_line { utf16_to_char_col(text, d.end_char.max(0) as usize) } else { len };
        b = b.max(a);
        if a == b {
            if l != first {
                continue; // 跨行範圍結尾停在行首：這一行沒有被涵蓋到
            }
            if b < len {
                b += 1;
            } else {
                a = a.saturating_sub(1);
            }
        }
        let (r0, r1) = if row_lines.is_empty() {
            (l, (l + 1).min(g.rows.len()))
        } else {
            (row_lines.partition_point(|&x| x < l), row_lines.partition_point(|&x| x <= l))
        };
        let mut start = 0;
        for row in g.rows.get(r0..r1.max(r0)).unwrap_or(&[]) {
            let end = start + row.char_count_excluding_newline().0;
            let (lo, hi) = (a.max(start), b.min(end));
            if lo < hi {
                out.push((row.rect().top(), row.rect().bottom(), row_x(row, lo - start), row_x(row, hi - start)));
            } else if a == b && a >= start && a <= end {
                let x = row_x(row, a - start);
                out.push((row.rect().top(), row.rect().bottom(), x, x + min_w));
                break;
            }
            start = end;
        }
    }
    out
}

/// 滑鼠停留說明的位置：(分頁 id, 內容版本, 行, 字元欄)。內容一變就是另一個位置，舊的說明不再顯示
type HoverKey = (u64, u64, usize, usize);

/// 指標在同一個字元上停這麼久才問語言伺服器（移動中不送請求）
const HOVER_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

/// 補全選單最多留幾項、一次顯示幾列
const COMPLETION_MAX: usize = 200;
const COMPLETION_ROWS: usize = 10;
/// 沒有語言伺服器時（本機補全）打到幾個字才自動列出
const LOCAL_COMPLETION_MIN: usize = 3;
/// 本機補全收集字的範圍：游標前後各幾行
const LOCAL_COMPLETION_LINES: usize = 2000;

/// 名稱（識別字）裡的字元：補全以游標前連續的這些字元當作「打到一半的字」
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `needle` 的字元依序出現在 `hay` 裡（不必相連）
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|n| it.any(|h| h == n))
}

/// 語言伺服器的補全選單：第 `line` 行、從第 `start` 個字元開始的那個字（接受時取代游標前的這一段）
struct CompletionMenu {
    tab: u64,
    line: usize,
    start: usize,
    items: Vec<crate::lsp::CompletionItem>,
    /// 伺服器只給了一部分（isIncomplete）：再打字要重問
    incomplete: bool,
    /// 符合目前打過的字的項目（`items` 的索引），依顯示順序
    shown: Vec<usize>,
    /// 上一次篩選時打過的字（變了就選回第一項）
    prefix: String,
    selected: usize,
}

/// 送出遠端 AI 補全時游標所在的位置：回覆到的時候分頁、內容版本、游標都沒變才用（否則丟掉）。
/// `start` 是游標前那個字的開頭，`prefix` 是那個字已經打的部分（接受時以 `prefix + 回覆` 取代它）
struct AiReq {
    tab: u64,
    rev: u64,
    line: usize,
    col: usize,
    start: usize,
    prefix: String,
    line_before: String,
}

/// 遠端 AI 的建議在補全選單裡的樣子：顯示接受後那一行會變成的字（只取第一行，多行時加「 …」）
fn ai_completion_item(prefix: &str, text: &str) -> crate::lsp::CompletionItem {
    let mut lines = text.lines();
    let first: String = lines.next().unwrap_or("").chars().take(80).collect();
    let more = lines.count();
    let label = if more > 0 || first.chars().count() < text.lines().next().unwrap_or("").chars().count() {
        format!("{prefix}{first} …")
    } else {
        format!("{prefix}{first}")
    };
    crate::lsp::CompletionItem {
        label,
        insert: format!("{prefix}{text}"),
        filter: prefix.to_string(),
        sort: String::new(),
        detail: if more > 0 { format!("AI（{} 行）", more + 1) } else { "AI".into() },
    }
}

/// 矩形選取（Alt+拖曳）：起點與終點所在的行，以及畫面上的 x（相對於文字排版的左緣）。
/// 每一行選取的欄是那一行在這兩個 x 之間的字元，所以 Tab、全形字也對得齊；內容一變就取消
struct BoxSel {
    tab: u64,
    rev: u64,
    a_line: usize,
    b_line: usize,
    a_x: f32,
    b_x: f32,
    /// 剛在區塊裡打字 / 刪除：兩個 x 要改成這一欄（選取裡最右邊那一行的新欄）在畫面上的位置，排版後才算得出來
    col: Option<(usize, usize)>,
}

/// 分割檢視（左右兩欄）：有焦點的那一欄是完整的編輯區、顯示目前分頁；另一欄唯讀顯示 `other` 分頁
struct SplitView {
    other: u64,
    /// 有焦點的是右欄
    right_active: bool,
    /// 上一幀另一欄畫出的每一行：（行號, 文字左上角, 排版），點下去時換算成游標位置
    rows: Vec<(usize, Pos2, Arc<egui::Galley>)>,
    /// 另一欄的 Git 行狀態：鍵為（分頁 id, 內容版本, HEAD 內容的位址），變了才重算
    git: ((u64, u64, usize), HashMap<usize, u8>),
    /// 另一欄上一幀的捲動位置，與換欄後下一幀要用的捲動位置
    offset: Vec2,
    pending: Option<Vec2>,
    /// 另一欄是 Markdown 時閱讀模式自己的狀態（圖片快取等；不與有焦點的那一欄共用）
    md: MarkdownViewState,
    /// 左欄佔編輯區寬度的比例（拖曳分隔線改變）；上下分割時是上欄佔的高度比例
    ratio: f32,
    /// 上下分割（right_active 時有焦點的是下面那一欄）
    vertical: bool,
}

/// 一般編輯區的文字在捲動區裡比另一欄多一段上邊界（TextEdit 的 margin）：換欄時捲動位置要差這麼多，同一行才會在同一個高度
const SPLIT_TEXT_MARGIN_Y: f32 = 4.0;

/// 一個獨立視窗的 Git 標記快取：((內容版本, HEAD 指標), 行號 → 標記)
type DetachedGit = ((u64, usize), HashMap<usize, u8>);

/// 矩形選取裡的一次編輯（每一行同時套用）
enum BoxEdit {
    Text(String),
    /// 貼上：每一行各自的文字（行數與區塊相同時逐行，單行時每一行都貼同一段）
    Lines(Vec<String>),
    Backspace,
    Delete,
    /// 剪下：刪掉選到的部分（複製在前面另外做）
    Cut,
}

/// 一個補全請求：要問的位置（UTF-16）與回覆來了之後選單的位置
struct CompletionReq {
    path: String,
    tab: u64,
    line: usize,
    start: usize,
    utf16: usize,
    trigger: Option<char>,
}

/// 語言伺服器的說明（通常是 Markdown）轉成要顯示的文字：拿掉程式碼區塊的 ``` 與分隔線
fn hover_display_text(result: &serde_json::Value) -> String {
    let text = crate::lsp::extract_hover_text(result, 4000);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("```") && t != "---" && t != "***"
        })
        .collect();
    lines.join("\n").trim().to_string()
}

/// 診斷的波浪底線：從 `x0` 到 `x1`、底端在 `y`
fn squiggle(x0: f32, x1: f32, y: f32, color: Color32) -> Shape {
    let mut pts = Vec::new();
    let (mut x, mut up) = (x0, true);
    while x < x1 {
        pts.push(Pos2::new(x, if up { y - 2.0 } else { y }));
        up = !up;
        x += 2.0;
    }
    pts.push(Pos2::new(x1, if up { y - 2.0 } else { y }));
    Shape::line(pts, Stroke::new(1.0_f32, color))
}

/// 終端機的 16 色（ANSI 0–15，數值與 Windows Terminal 的 Campbell 配色相同）。
/// 沒有指定顏色的字用主題的文字色、沒有指定底色的格子不畫底色
const TERM_COLORS: [Color32; 16] = [
    Color32::from_rgb(12, 12, 12),
    Color32::from_rgb(197, 15, 31),
    Color32::from_rgb(19, 161, 14),
    Color32::from_rgb(193, 156, 0),
    Color32::from_rgb(0, 55, 218),
    Color32::from_rgb(136, 23, 152),
    Color32::from_rgb(58, 150, 221),
    Color32::from_rgb(204, 204, 204),
    Color32::from_rgb(118, 118, 118),
    Color32::from_rgb(231, 72, 86),
    Color32::from_rgb(22, 198, 12),
    Color32::from_rgb(249, 241, 165),
    Color32::from_rgb(59, 120, 255),
    Color32::from_rgb(180, 0, 158),
    Color32::from_rgb(97, 214, 214),
    Color32::from_rgb(242, 242, 242),
];

/// 終端機的一列排成一段文字：相鄰、顏色相同的格子合成一段，依 `VtParser` 記下的前景 / 背景色與粗體上色
///（粗體的基本色 0–7 以亮色 8–15 顯示，與多數終端機相同）。行尾沒有底色的空白不排；空的一列排一個空白，保留列高
fn terminal_row_job(cells: &[TerminalCell], font: &FontId, default_fg: Color32) -> egui::text::LayoutJob {
    let end = cells.iter().rposition(|c| c.ch != ' ' || c.bg >= 0).map_or(0, |i| i + 1);
    let mut job = egui::text::LayoutJob::default();
    let mut i = 0;
    while i < end {
        let (fg, bg, bold) = (cells[i].fg, cells[i].bg, cells[i].bold);
        let mut text = String::new();
        while i < end && (cells[i].fg, cells[i].bg, cells[i].bold) == (fg, bg, bold) {
            text.push(cells[i].ch);
            i += 1;
        }
        let fg = if bold && (0..8).contains(&fg) { fg + 8 } else { fg };
        let color = if fg >= 0 { TERM_COLORS[fg as usize % 16] } else { default_fg };
        let background = if bg >= 0 { TERM_COLORS[bg as usize % 16] } else { Color32::TRANSPARENT };
        job.append(&text, 0.0, egui::TextFormat { font_id: font.clone(), color, background, ..Default::default() });
    }
    if job.text.is_empty() {
        job.append(" ", 0.0, egui::TextFormat { font_id: font.clone(), color: default_fg, ..Default::default() });
    }
    job
}

/// 終端機有鍵盤焦點時，一個按鍵要送給 shell 的位元組（一般終端機送的 VT 序列；
/// Ctrl + 字母是控制字元，例如 Ctrl+C 是 0x03）。一般的字由 `Event::Text` 送，這裡不管；不送的鍵回傳 None
fn terminal_key_bytes(key: Key, m: Modifiers) -> Option<Vec<u8>> {
    let seq: &[u8] = match key {
        Key::Enter => b"\r",
        Key::Tab if m.shift => b"\x1b[Z",
        Key::Tab => b"\t",
        Key::Backspace => b"\x7f",
        Key::Escape => b"\x1b",
        Key::ArrowUp => b"\x1b[A",
        Key::ArrowDown => b"\x1b[B",
        Key::ArrowRight => b"\x1b[C",
        Key::ArrowLeft => b"\x1b[D",
        Key::Home => b"\x1b[H",
        Key::End => b"\x1b[F",
        Key::Insert => b"\x1b[2~",
        Key::Delete => b"\x1b[3~",
        Key::PageUp => b"\x1b[5~",
        Key::PageDown => b"\x1b[6~",
        _ if m.ctrl && !m.alt => {
            let mut name = key.name().chars();
            return match (name.next(), name.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => Some(vec![c.to_ascii_uppercase() as u8 - b'@']),
                _ => None,
            };
        }
        _ => return None,
    };
    Some(seq.to_vec())
}

/// 終端機有鍵盤焦點時，這一幀的輸入事件要送給 shell 的位元組：輸入的字照送、貼上的文字換行改成 `\r`、
/// 特殊鍵見 `terminal_key_bytes`。egui 把 Ctrl+C / X / V 變成「複製 / 剪下 / 貼上」事件（同一幀也有那個按鍵的事件）：
/// 在終端機裡 Ctrl+C / X 是控制字元（0x03 中斷 / 0x18），Ctrl+V 是貼上剪貼簿的文字，按鍵本身不再重送
#[cfg(test)]
fn terminal_input_bytes(events: &[egui::Event]) -> Vec<u8> {
    terminal_input_bytes_ime(events, &mut String::new())
}

/// 同 `terminal_input_bytes`，再加上輸入法：`composing` 是組字中的文字（跨幀保留，畫在游標處）。
/// 組字期間的按鍵與輸入屬於輸入法、不送進 shell；選定的字（Commit）以 UTF-8 送出
fn terminal_input_bytes_ime(events: &[egui::Event], composing: &mut String) -> Vec<u8> {
    use egui::Event;
    let clip = events.iter().any(|e| matches!(e, Event::Copy | Event::Cut | Event::Paste(_)));
    let mut out = Vec::new();
    for ev in events {
        match ev {
            Event::Ime(egui::ImeEvent::Preedit { text, .. }) => *composing = text.clone(),
            Event::Ime(egui::ImeEvent::Commit(t)) => {
                composing.clear();
                out.extend_from_slice(t.as_bytes());
            }
            _ if !composing.is_empty() => {}
            Event::Text(t) => out.extend_from_slice(t.as_bytes()),
            Event::Paste(t) => out.extend_from_slice(t.replace("\r\n", "\r").replace('\n', "\r").as_bytes()),
            Event::Copy => out.push(0x03),
            Event::Cut => out.push(0x18),
            Event::Key { key, pressed: true, modifiers, .. } => {
                if clip && modifiers.ctrl && matches!(key, Key::C | Key::X | Key::V) {
                    continue;
                }
                if let Some(b) = terminal_key_bytes(*key, *modifiers) {
                    out.extend(b);
                }
            }
            _ => {}
        }
    }
    out
}

/// 往左一個字元（`word` 為 true 時一個字）的位置；在行首時是上一行的行尾
fn step_left(buf: &EditorBuffer, p: Position, word: bool) -> Position {
    if p.col == 0 {
        return match p.line {
            0 => p,
            line => Position { line: line - 1, col: buf.line_len(line - 1) },
        };
    }
    if !word {
        return Position { col: p.col - 1, ..p };
    }
    let chars: Vec<char> = buf.lines[p.line].chars().take(p.col).collect();
    let mut col = chars.len();
    while col > 0 && !is_word_char(chars[col - 1]) {
        col -= 1;
    }
    while col > 0 && is_word_char(chars[col - 1]) {
        col -= 1;
    }
    Position { col, ..p }
}

/// 往右一個字元（`word` 為 true 時一個字）的位置；在行尾時是下一行的行首
fn step_right(buf: &EditorBuffer, p: Position, word: bool) -> Position {
    let len = buf.line_len(p.line);
    if p.col >= len {
        return if p.line + 1 < buf.lines.len() { Position { line: p.line + 1, col: 0 } } else { p };
    }
    if !word {
        return Position { col: p.col + 1, ..p };
    }
    let mut rest = buf.lines[p.line].chars().skip(p.col).peekable();
    let mut col = p.col;
    while rest.next_if(|c| !is_word_char(*c)).is_some() {
        col += 1;
    }
    while rest.next_if(|c| is_word_char(*c)).is_some() {
        col += 1;
    }
    Position { col, ..p }
}

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
            term_input_h: 28.0,
            term_preedit: String::new(),
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
            term_cwd: cwd.clone(),
            term_pty: None,
            term_pty_failed: false,
            term_pty_exited: false,
            term_shells: Vec::new(),
            lsp: LspManager::new(cwd.clone(), default_lsp_launcher()),
            lsp_sizes: HashMap::new(),
            def_want: None,
            def_wait: None,
            hover_at: None,
            hover_want: None,
            hover_wait: None,
            hover_text: None,
            comp: None,
            comp_want: None,
            comp_wait: None,
            comp_accept: None,
            comp_invoke: false,
            comp_rect: None,
            comp_caret: None,
            ai: crate::ai::RemoteAi::default(),
            ai_wait: None,
            box_sel: None,
            box_dragging: false,
            box_cols: Vec::new(),
            box_keys: Vec::new(),
            split: None,
            editor_offsets: HashMap::new(),
            pending_offset: None,
            split_drag: None,
            split_mid: None,
            detached: Vec::new(),
            detached_git: HashMap::new(),
            detached_rects: HashMap::new(),
            detached_place: HashMap::new(),
            pending_close: None,
            allow_exit: false,
            editor_focused: false,
            editor_was_focused: false,
            text_cache: ((0, 0), String::new()),
            highlight_cache: None,
            atlas_probe: None,
            find_cache: ((0, 0, String::new()), Vec::new()),
            find_seen: ((0, 0), std::time::Instant::now()),
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
            md_find: (String::new(), Finder::default()),
            show_replace: false,
            replace_text: String::new(),
            find_error: None,
            select_keeps_focus: false,
            find_current: None,
            focus_editor: false,
            watcher: None,
            disk_stamps: HashMap::new(),
            disk_conflict: None,
            incoming: Vec::new(),
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
            blame: HashMap::new(),
            blame_chan: unbounded(),
            blame_running: None,
            blame_seen: (0, 0, std::time::Instant::now()),
            diff_cache: None,
            diff_focus: 0,
            diff_scroll_focus: false,
            large_view_bytes: LARGE_VIEW_BYTES,
            large_stats: ((0, 0), (0, 0)),
            large_rows: LongRows::default(),
            large_preedit: String::new(),
            large_ime_at: 0,
            editor_ime_active: None,
            large_marks_seen: ((0, 0), std::time::Instant::now()),
            large_scroll_to: None,
            large_wrap_w: None,
            md_preview_bytes: MD_PREVIEW_MAX_BYTES,
            md_big_preview: HashSet::new(),
            hot_exit_key: Vec::new(),
            hot_exit_job: None,
            hot_exit_written: (std::time::Instant::now(), 0),
            hot_restore: HashMap::new(),
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
        // 還原上次的主題、字型、資料夾、開啟的檔案，以及尚未存檔的內容
        app.state_path = crate::core::config_dir().map(|d| d.join("state.json"));
        app.restore_saved_session();
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

    /// 接上一條「要開啟的檔案」通道（單一實例、macOS 的 Finder 開檔）：之後送來的檔案會在本視窗開啟。可以接多條
    pub fn add_incoming(&mut self, rx: Receiver<Vec<PathBuf>>) {
        self.incoming.push(rx);
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
            // 另一欄的分頁是存過的檔案時才記（新檔沒有路徑可以還原）
            split: self.split.as_ref().and_then(|s| {
                let other_file = self.tabs.iter().find(|b| b.id == s.other)?.file_path.clone()?;
                Path::new(&other_file).is_absolute().then_some(crate::core::app_state::SplitState {
                    other_file,
                    right_active: s.right_active,
                    ratio: s.ratio,
                    vertical: s.vertical,
                })
            }),
            // 獨立視窗：同樣只記存過的檔案
            detached: self
                .detached
                .iter()
                .filter_map(|id| {
                    let file = self.tabs.iter().find(|b| b.id == *id)?.file_path.clone()?;
                    let rect = self.detached_rects.get(id).map(|r| [r.min.x, r.min.y, r.width(), r.height()]);
                    Path::new(&file).is_absolute().then_some(crate::core::app_state::DetachedState { file, rect })
                })
                .collect(),
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
        // 分割檢視：另一欄的檔案有重新開起來才還原
        if let Some(sp) = st.split {
            if let Some(b) = self.tabs.iter().find(|b| b.file_path.as_deref() == Some(sp.other_file.as_str())) {
                self.split = Some(SplitView {
                    other: b.id,
                    right_active: sp.right_active,
                    rows: Vec::new(),
                    git: ((0, 0, 0), HashMap::new()),
                    offset: Vec2::ZERO,
                    pending: None,
                    md: MarkdownViewState::default(),
                    ratio: if sp.ratio.is_finite() { sp.ratio.clamp(0.1, 0.9) } else { 0.5 },
                    vertical: sp.vertical,
                });
            }
        }
        // 獨立視窗：檔案有重新開起來才還原；量過的位置大小合理才用（太小或不是數字就讓視窗用預設的）
        for d in st.detached {
            let Some(id) = self.tabs.iter().find(|b| b.file_path.as_deref() == Some(d.file.as_str())).map(|b| b.id) else { continue };
            if self.detached.contains(&id) {
                continue;
            }
            self.detached.push(id);
            if let Some([x, y, w, h]) = d.rect.filter(|r| r.iter().all(|v| v.is_finite()) && r[2] >= 100.0 && r[3] >= 60.0) {
                let r = Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, h));
                self.detached_place.insert(id, r);
                self.detached_rects.insert(id, r);
            }
        }
    }

    /// 狀態有變就寫回 `state.json`（測試中 `state_path` 為 None，不寫）
    fn persist_state(&mut self) {
        self.last_persist = std::time::Instant::now();
        self.save_hot_exit(false);
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
        self.blame.retain(|id, _| alive.contains(id));
        if self.disk_conflict.is_some_and(|id| !alive.contains(&id)) {
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
            TabAction::Detach(i) => {
                if let Some(id) = self.tabs.get(i).map(|b| b.id) {
                    if !self.detached.contains(&id) {
                        self.detached.push(id);
                    }
                }
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
                    self.large_stats.0 = (0, 0);
                    self.note_disk_state(idx);
                    self.request_git_head(idx);
                    self.status_message = format!("已開啟 {}", full);
                    // 上次結束時這個檔案有未存檔的內容：檔案讀好了，現在套回去
                    if let Some(tab) = self.hot_restore.remove(&id) {
                        self.status_message = if self.apply_unsaved(idx, &tab) {
                            format!("已還原未存檔的內容；注意：{} 在磁碟上已被修改，存檔會蓋掉磁碟上的版本", full)
                        } else {
                            format!("已開啟 {}（已還原未存檔的內容）", full)
                        };
                    }
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
                Err(e) => match self.hot_restore.remove(&id) {
                    // 檔案讀不到了，但上次未存檔的內容還在：留下分頁並把內容還原進去
                    Some(tab) => {
                        self.apply_unsaved(idx, &tab);
                        self.status_message = format!("讀取失敗：{} ({})；已還原上次未存檔的內容", full, e);
                    }
                    None => {
                        // 與一般開檔失敗相同：不留下空白分頁
                        self.finish_close(&[id], false);
                        self.status_message = format!("開啟失敗：{} ({})", full, e);
                    }
                },
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
                .is_some_and(|fp| crate::markdown::normalize_key(fp) == key)
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
            ("搜尋：跳到定義 (Go to Definition)".into(), Cmd::GotoDefinition),
            ("搜尋：全專案搜尋 (Find in Files)".into(), Cmd::FindInFiles),
            ("檢視：切換 Typora Markdown 模式 (Typora / 雙欄 / 原始碼)".into(), Cmd::CycleMarkdownMode),
            ("檢視：切換 Markdown 大綱側邊欄 (Outline)".into(), Cmd::ToggleMarkdownOutline),
            ("檢視：終端機 (Terminal)".into(), Cmd::ToggleTerminal),
            ("檢視：Markdown 關聯圖 (Graph)".into(), Cmd::ToggleGraph),
            ("檢視：與 Git HEAD 比較 (Diff)".into(), Cmd::DiffHead),
            ("檢視：分割編輯器 (Split Editor)".into(), Cmd::SplitEditor),
            ("檢視：切換左右 / 上下分割 (Toggle Split Orientation)".into(), Cmd::SplitOrientation),
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

    /// 在指定的分頁（獨立視窗的分頁，依 id）上執行指令：暫時把它當成目前分頁，執行完換回主視窗原本的分頁。
    /// 指令自己換到別的分頁（新增 / 開啟檔案）就照它的；`tab` 是 None 時同 `run_cmd`
    pub fn run_cmd_on(&mut self, cmd: Cmd, tab: Option<u64>) {
        let Some(tab) = tab else { return self.run_cmd(cmd) };
        let Some(idx) = self.tabs.iter().position(|b| b.id == tab) else { return };
        let prev = self.tabs.get(self.active_tab_idx).map(|b| b.id);
        self.active_tab_idx = idx;
        self.run_cmd(cmd);
        let cur = self.tabs.get(self.active_tab_idx).map(|b| b.id);
        let closed = !self.tabs.iter().any(|b| b.id == tab);
        if cur == Some(tab) || closed {
            if let Some(i) = prev.and_then(|p| self.tabs.iter().position(|b| b.id == p)) {
                self.active_tab_idx = i;
            }
        }
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
                if self.active_buffer_mut().is_some_and(|b| b.undo()) {
                    self.sync_cursor_to_editor();
                    self.scroll_to_cursor = true;
                }
            }
            Cmd::Redo => {
                self.commit_md_edit();
                if self.active_buffer_mut().is_some_and(|b| b.redo()) {
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
            Cmd::GotoDefinition => self.goto_definition(),
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
            Cmd::DiffHead => {
                self.commit_md_edit();
                self.show_diff_dialog = true;
                self.diff_focus = 0;
                self.diff_scroll_focus = true;
                // HEAD 可能在開檔之後變了（在終端機提交）：重新讀取
                self.request_git_head(self.active_tab_idx);
            }
            Cmd::SplitEditor => {
                // 分割時新的右欄有焦點、先顯示同一份文件（左欄唯讀顯示它）；再按一次回到單欄
                self.split = match self.split.take() {
                    Some(_) => None,
                    None => self
                        .tabs
                        .get(self.active_tab_idx)
                        .map(|b| SplitView {
                            other: b.id,
                            right_active: true,
                            rows: Vec::new(),
                            git: ((0, 0, 0), HashMap::new()),
                            offset: Vec2::ZERO,
                            pending: None,
                            md: MarkdownViewState::default(),
                            ratio: 0.5,
                            vertical: false,
                        }),
                };
                self.focus_editor = true;
            }
            Cmd::SplitOrientation => {
                // 還沒分割時先分割再切成上下
                if self.split.is_none() {
                    self.run_cmd(Cmd::SplitEditor);
                }
                if let Some(s) = self.split.as_mut() {
                    s.vertical = !s.vertical;
                }
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

    /// 每一幀：把開著的、語言伺服器支援的文件（有檔名、載入完成、不是大檔案）交給 `LspManager` 同步；
    /// 伺服器還在啟動時繼續重畫，才看得到它就緒
    fn lsp_tick(&mut self, ctx: &egui::Context) {
        self.lsp.set_root(self.project_root.clone());
        if !self.lsp.has_waker() {
            // 診斷是伺服器算好之後才送來的：收到訊息就重畫，不必一直輪詢
            let ctx = ctx.clone();
            self.lsp.set_waker(Arc::new(move || ctx.request_repaint()));
        }
        let mut docs = Vec::new();
        let mut idx = Vec::new();
        for (i, b) in self.tabs.iter().enumerate() {
            let Some(path) = b.file_path.as_deref() else { continue };
            if ServerKind::for_path(path).is_none() || self.loading.contains_key(&b.id) {
                continue;
            }
            let rev = b.revision();
            let size = match self.lsp_sizes.get(&b.id) {
                Some(&(r, size)) if r == rev => size,
                _ => {
                    let size = b.lines.iter().map(|l| l.len() + 1).sum::<usize>();
                    self.lsp_sizes.insert(b.id, (rev, size));
                    size
                }
            };
            if size >= self.large_view_bytes {
                continue;
            }
            docs.push(OpenDoc { path: path.to_string(), revision: rev });
            idx.push(i);
        }
        let tabs = &self.tabs;
        self.lsp_sizes.retain(|id, _| tabs.iter().any(|b| b.id == *id));
        self.lsp.sync(&docs, |k| tabs[idx[k]].edit_text());
        if self.lsp.is_starting() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // 跳到定義：文件同步完才送（伺服器看到的是目前的內容），回覆到了就跳過去
        if let Some((path, line, ch)) = self.def_want.take() {
            let params = text_document_position_params(&path, line as i32, ch as i32);
            match self.lsp.request(&path, "textDocument/definition", params) {
                Ok(r) => self.def_wait = Some((r, std::time::Instant::now())),
                Err(e) => self.status_message = format!("跳到定義：{e}"),
            }
        }
        if let Some((r, sent)) = self.def_wait {
            match self.lsp.take_reply(r) {
                Some(reply) => {
                    self.def_wait = None;
                    match reply.map(|v| extract_definition(&v)) {
                        Ok(Some((path, line, ch))) => self.jump_to_location(&path, line.max(0) as usize, ch.max(0) as usize),
                        Ok(None) => self.status_message = "跳到定義：找不到定義".into(),
                        Err(e) => self.status_message = format!("跳到定義：{e}"),
                    }
                }
                None if sent.elapsed() > std::time::Duration::from_secs(10) => {
                    self.lsp.forget(r);
                    self.def_wait = None;
                    self.status_message = "跳到定義：語言伺服器 10 秒內沒有回應".into();
                }
                // 回覆到的時候 waker 會叫醒介面；這裡只是為了逾時檢查
                None => ctx.request_repaint_after(std::time::Duration::from_secs(1)),
            }
        }

        // 滑鼠停留說明：問不了（伺服器沒就緒等）就當作那裡沒有說明，不在狀態列說明
        if let Some((path, line, ch, key)) = self.hover_want.take() {
            let params = text_document_position_params(&path, line as i32, ch as i32);
            match self.lsp.request(&path, "textDocument/hover", params) {
                Ok(r) => self.hover_wait = Some((r, key)),
                Err(_) => self.hover_text = Some((key, String::new())),
            }
        }
        if let Some((r, key)) = self.hover_wait {
            if let Some(reply) = self.lsp.take_reply(r) {
                self.hover_wait = None;
                self.hover_text = Some((key, reply.map(|v| hover_display_text(&v)).unwrap_or_default()));
            }
        }

        // 自動完成：新的請求取代還在等的那一個；問不了、伺服器回錯誤或沒有項目時就是沒有選單（不在狀態列說明）
        if let Some(req) = self.comp_want.take() {
            if let Some((r, _)) = self.comp_wait.take() {
                self.lsp.forget(r);
            }
            let params = completion_params(&req.path, req.line as i32, req.utf16 as i32, req.trigger);
            if let Ok(r) = self.lsp.request(&req.path, "textDocument/completion", params) {
                self.comp_wait = Some((r, req));
            }
        }
        if let Some(reply) = self.comp_wait.as_ref().and_then(|(r, _)| self.lsp.take_reply(*r)) {
            let (_, req) = self.comp_wait.take().unwrap();
            let (items, incomplete) = reply.map(|v| extract_completion_items(&v, COMPLETION_MAX)).unwrap_or_default();
            self.comp = (!items.is_empty()).then(|| CompletionMenu {
                tab: req.tab,
                line: req.line,
                start: req.start,
                items,
                incomplete,
                shown: Vec::new(),
                prefix: String::new(),
                selected: 0,
            });
        }
    }

    /// 程式碼編輯區每一幀最後呼叫（編輯與游標都已套用）：游標離開那個字就關閉選單；
    /// 依打的字決定要不要問（Ctrl+Space、打名稱的字元而還沒有選單、伺服器的觸發字元），再依打過的字篩選
    fn update_completion(&mut self, ctx: &egui::Context, idx: usize, changed: bool, typed: Option<char>) {
        let invoke = std::mem::take(&mut self.comp_invoke);
        // 沒有語言伺服器的檔案（含還沒存檔的新檔）：用文件裡出現過的字補全（`crate::ai` 的本機補全）。
        // 程式碼檔案打到 LOCAL_COMPLETION_MIN 個字自動列出；純文字與 Markdown 只在 Ctrl+Space 時列出
        let auto_local = !matches!(self.tab_lang(idx), Lang::Plain | Lang::Markdown);
        let buf = &self.tabs[idx];
        let path = buf.file_path.clone().filter(|p| ServerKind::for_path(p).is_some());
        let sel = buf.selections[0];
        let head = sel.head;
        let line: Vec<char> = buf.lines.get(head.line).map(|l| l.chars().collect()).unwrap_or_default();
        let col = head.col.min(line.len());
        let mut word_start = col;
        while word_start > 0 && is_ident_char(line[word_start - 1]) {
            word_start -= 1;
        }
        let over_menu = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| self.comp_rect.is_some_and(|r| r.contains(p)));
        if let Some(m) = &self.comp {
            let ok = m.tab == buf.id
                && m.line == head.line
                && sel.anchor == sel.head
                && col >= m.start
                && line[m.start..col].iter().all(|&c| is_ident_char(c))
                && (self.editor_focused || over_menu || self.comp_accept.is_some());
            if !ok {
                self.comp = None;
            }
        }
        let mut want: Option<(usize, Option<char>)> = None;
        if invoke {
            want = Some((word_start, None));
        } else if let Some(ch) = typed.filter(|_| changed) {
            if is_ident_char(ch) {
                match &self.comp {
                    None if line[word_start].is_ascii_digit() => {}
                    None if path.is_some() => want = Some((word_start, None)),
                    None if auto_local && col - word_start >= LOCAL_COMPLETION_MIN => want = Some((word_start, None)),
                    Some(m) if m.incomplete => want = Some((m.start, None)),
                    _ => {}
                }
            } else if path.as_ref().is_some_and(|p| self.lsp.completion_triggers(p).contains(ch)) {
                self.comp = None;
                want = Some((col, Some(ch)));
            }
        }
        if let Some((start, trigger)) = want {
            if let Some(path) = path {
                let utf16 = buf.lines.get(head.line).map_or(0, |l| char_col_to_utf16(l, col));
                self.comp_want = Some(CompletionReq { path, tab: buf.id, line: head.line, start, utf16, trigger });
                ctx.request_repaint();
            } else {
                // 本機補全：只看游標前後 LOCAL_COMPLETION_LINES 行（大檔案不必每次串接整份文件）
                let prefix: String = line[start..col].iter().collect();
                let from = head.line.saturating_sub(LOCAL_COMPLETION_LINES);
                let to = (head.line + LOCAL_COMPLETION_LINES + 1).min(buf.lines.len());
                let (mut text, mut cursor) = (String::new(), 0);
                for (i, l) in buf.lines[from..to].iter().enumerate() {
                    if from + i == head.line {
                        cursor = text.len() + l.char_indices().nth(col).map_or(l.len(), |(b, _)| b);
                    }
                    text.push_str(l);
                    text.push('\n');
                }
                let items: Vec<crate::lsp::CompletionItem> = crate::ai::suggest_local_completions(&text, cursor, &prefix, COMPLETION_MAX)
                    .into_iter()
                    .map(|w| crate::lsp::CompletionItem {
                        label: w.clone(),
                        insert: w.clone(),
                        filter: w.clone(),
                        sort: w,
                        detail: String::new(),
                    })
                    .collect();
                let (tab, line_no) = (buf.id, head.line);
                self.comp = (!items.is_empty()).then(|| CompletionMenu {
                    tab,
                    line: line_no,
                    start,
                    items,
                    incomplete: false,
                    shown: Vec::new(),
                    prefix: String::new(),
                    selected: 0,
                });
            }
        }
        // 遠端 AI（設定裡啟用、有端點時）：Ctrl+Space 另外在背景問一次，本機 / 語言伺服器的選單照常馬上出現
        if invoke && self.config.ai.enabled && !self.config.ai.endpoint.trim().is_empty() {
            let buf = &self.tabs[idx];
            let (before, after) = crate::ai::remote::context_around(&buf.lines, head.line, col);
            let language = buf
                .file_path
                .as_deref()
                .and_then(|p| Path::new(p).extension())
                .and_then(|e| e.to_str())
                .unwrap_or("text")
                .to_string();
            let body = crate::ai::remote::build_request(&self.config.ai, &language, &before, &after);
            let req = AiReq {
                tab: buf.id,
                rev: buf.revision(),
                line: head.line,
                col,
                start: word_start,
                prefix: line[word_start..col].iter().collect(),
                line_before: line[..col].iter().collect(),
            };
            let wake = ctx.clone();
            let id = self.ai.send(&self.config.ai, body, move || wake.request_repaint());
            self.ai_wait = Some((id, req)); // 之前那一次還沒回來：以這一次為準
            self.status_message = "AI 補全：等待回覆…".into();
        }
        // 回覆到了：分頁、內容與游標都沒變才加進選單（排在第一項）；錯誤在狀態列說明，本機的選單不受影響
        while let Some((id, result)) = self.ai.poll() {
            if self.ai_wait.as_ref().map(|w| w.0) != Some(id) {
                continue; // 已經被取代或取消
            }
            let Some((_, req)) = self.ai_wait.take() else { continue };
            let buf = &self.tabs[idx];
            let s = buf.selections[0];
            let fresh = buf.id == req.tab && buf.revision() == req.rev && s.anchor == s.head && s.head.line == req.line && s.head.col == req.col;
            match result {
                Err(e) => self.status_message = format!("AI 補全：{e}"),
                Ok(_) if !fresh => self.status_message = "AI 補全：回覆到的時候游標已經移動，略過這個建議".into(),
                Ok(reply) => match crate::ai::remote::clean_completion(&reply, &req.line_before) {
                    None => self.status_message = "AI 補全：沒有建議".into(),
                    Some(text) => {
                        let item = ai_completion_item(&req.prefix, &text);
                        match &mut self.comp {
                            Some(m) if m.tab == req.tab && m.line == req.line && m.start == req.start => {
                                m.items.insert(0, item);
                                m.selected = 0;
                            }
                            _ => {
                                self.comp = Some(CompletionMenu {
                                    tab: req.tab,
                                    line: req.line,
                                    start: req.start,
                                    items: vec![item],
                                    incomplete: false,
                                    shown: Vec::new(),
                                    prefix: String::new(),
                                    selected: 0,
                                })
                            }
                        }
                        self.status_message = "AI 補全：建議列在選單第一項".into();
                    }
                },
            }
        }
        // 篩選（不分大小寫）：開頭相同的在前，其次是第一個字相同、其餘字元依序出現的（例如 gtv → get_value）；
        // 各自維持伺服器的順序
        if let Some(m) = &mut self.comp {
            let prefix: String = line[m.start..col].iter().collect::<String>().to_lowercase();
            let (mut first, mut rest) = (Vec::new(), Vec::new());
            for (i, it) in m.items.iter().enumerate() {
                let f = it.filter.to_lowercase();
                if f.starts_with(&prefix) {
                    first.push(i);
                } else if f.chars().next() == prefix.chars().next() && is_subsequence(&prefix, &f) {
                    rest.push(i);
                }
            }
            first.extend(rest);
            m.shown = first;
            if m.prefix != prefix {
                m.prefix = prefix;
                m.selected = 0;
            }
            m.selected = m.selected.min(m.shown.len().saturating_sub(1));
            if m.shown.is_empty() && !m.incomplete {
                self.comp = None;
            }
        }
        if self.comp.is_none() {
            self.comp_rect = None;
        }
    }

    /// 在游標底下畫出補全選單；點一項就接受它（下一幀在編輯區套用）
    fn completion_popup(&mut self, ctx: &egui::Context, c: &Colors, caret: Rect, font: &FontId) {
        let Some(m) = self.comp.as_ref().filter(|m| !m.shown.is_empty()) else { return };
        let first = m.selected.saturating_sub(COMPLETION_ROWS - 1);
        let mut clicked = None;
        let resp = egui::Area::new(egui::Id::unique("lsp-completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(caret.left_bottom() + egui::vec2(0.0, 2.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(220.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (k, &i) in m.shown.iter().enumerate().skip(first).take(COMPLETION_ROWS) {
                        let it = &m.items[i];
                        let r = ui
                            .horizontal(|ui| {
                                let label = egui::Button::selectable(k == m.selected, RichText::new(&it.label).font(font.clone()));
                                let r = ui.add(label);
                                if !it.detail.is_empty() {
                                    let mut d: String = it.detail.chars().take(60).collect();
                                    if d.len() < it.detail.len() {
                                        d.push('…');
                                    }
                                    ui.label(RichText::new(d).color(c.fg_dim2).small());
                                }
                                r
                            })
                            .inner;
                        if r.clicked() {
                            clicked = Some(i);
                        }
                    }
                    if m.shown.len() > COMPLETION_ROWS {
                        ui.label(RichText::new(format!("{} / {}", m.selected + 1, m.shown.len())).color(c.fg_dim2).small());
                    }
                });
            });
        self.comp_rect = Some(resp.response.rect);
        if let Some(i) = clicked {
            self.comp_accept = Some(i);
            self.focus_editor = true;
            ctx.request_repaint();
        }
    }

    /// 程式碼編輯區每一幀呼叫：指標停在 `at`（行, 字元欄；不在文字上是 None）。
    /// 換了位置就丟掉舊的說明；在同一個位置停滿 `HOVER_DELAY` 才排一個請求（由 `lsp_tick` 送出）
    fn track_hover(&mut self, ctx: &egui::Context, idx: usize, at: Option<(usize, usize)>) {
        let buf = &self.tabs[idx];
        let key = at.map(|(l, c)| (buf.id, buf.revision(), l, c));
        if key != self.hover_at.map(|h| h.0) {
            if let Some((r, _)) = self.hover_wait.take() {
                self.lsp.forget(r);
            }
            self.hover_want = None;
            self.hover_text = None;
            self.hover_at = key.map(|k| (k, std::time::Instant::now()));
            if key.is_some() {
                ctx.request_repaint_after(HOVER_DELAY);
            }
            return;
        }
        let Some((key, since)) = self.hover_at else { return };
        if self.hover_want.is_some() || self.hover_wait.is_some() || self.hover_text.is_some() {
            return;
        }
        let Some(path) = buf.file_path.clone().filter(|p| ServerKind::for_path(p).is_some()) else { return };
        let wait = HOVER_DELAY.saturating_sub(since.elapsed());
        if !wait.is_zero() {
            ctx.request_repaint_after(wait);
            return;
        }
        let ch = buf.lines.get(key.2).map_or(0, |l| char_col_to_utf16(l, key.3));
        self.hover_want = Some((path, key.2, ch, key));
        ctx.request_repaint();
    }

    /// F12 / Ctrl+點擊：以目前分頁的游標位置問語言伺服器定義在哪裡（送出與處理回覆在 `lsp_tick`）
    fn goto_definition(&mut self) {
        let Some(buf) = self.tabs.get(self.active_tab_idx) else { return };
        let Some(path) = buf.file_path.clone() else {
            self.status_message = "跳到定義：這個分頁還沒有存成檔案".into();
            return;
        };
        if ServerKind::for_path(&path).is_none() {
            self.status_message = "跳到定義：這種檔案沒有語言伺服器".into();
            return;
        }
        let head = buf.selections[0].head;
        let ch = buf.lines.get(head.line).map_or(0, |l| char_col_to_utf16(l, head.col));
        if let Some((r, _)) = self.def_wait.take() {
            self.lsp.forget(r); // 之前那一次還沒回來：以這一次為準
        }
        self.def_want = Some((path, head.line, ch));
        self.status_message = "跳到定義：尋找中…".into();
        if let Some(ctx) = &self.repaint {
            ctx.request_repaint();
        }
    }

    /// 開啟（或切換到）`path`，游標移到第 `line` 行、UTF-16 第 `utf16` 個單位
    fn jump_to_location(&mut self, path: &str, line: usize, utf16: usize) {
        let find = |tabs: &[EditorBuffer]| tabs.iter().position(|b| b.file_path.as_deref().is_some_and(|p| same_file(p, path)));
        let idx = match find(&self.tabs) {
            Some(i) => i,
            None => {
                self.open_path(Path::new(path));
                match find(&self.tabs) {
                    Some(i) => i,
                    None => return, // 開不了：狀態列已經說明原因
                }
            }
        };
        self.commit_md_edit();
        self.active_tab_idx = idx;
        let id = self.tabs[idx].id;
        if let Some(job) = self.loading.get_mut(&id) {
            job.goto_line = Some(line); // 還在背景載入：載入完成後跳到那一行
            return;
        }
        let buf = &self.tabs[idx];
        let line = line.min(buf.lines.len().saturating_sub(1));
        let col = buf.lines.get(line).map_or(0, |l| utf16_to_char_col(l, utf16));
        let ci = Self::char_index_of(buf, line, col);
        self.pending_select = Some((ci, ci));
        self.scroll_to_cursor = true;
        self.focus_editor = true;
        self.status_message = format!("定義：{}:{}", path, line + 1);
    }

    /// 在虛擬終端裡啟動 shell（目前的終端機資料夾、與 `terminal` 同樣的大小）。
    /// 失敗時說明原因並改回逐指令執行
    fn start_shell(&mut self, ctx: &egui::Context) {
        let ctx = ctx.clone();
        let (rows, cols) = (self.terminal.rows() as u16, self.terminal.cols() as u16);
        let sh = self.chosen_shell();
        match PtySession::spawn(&sh.program, &sh.args, &self.term_cwd, rows, cols, move || ctx.request_repaint()) {
            Ok(s) => self.term_pty = Some(s),
            Err(e) => {
                self.term_pty_failed = true;
                self.terminal.feed(format!("\x1b[31m{e}\x1b[0m\r\n（改為逐指令執行：每個指令跑完才顯示輸出）\r\n").as_bytes());
            }
        }
    }

    /// 終端機的大小改成 `rows` × `cols`：畫面模型跟著改，shell 在跑時也通知它（它會依新的大小重排輸出）
    fn resize_terminal(&mut self, rows: usize, cols: usize) {
        self.terminal.resize(rows, cols);
        if let Some(s) = &self.term_pty {
            let _ = s.resize(rows as u16, cols as u16);
        }
    }

    /// 可以選的 shell（第一次用到時才去找）
    fn shells(&mut self) -> &[ShellChoice] {
        if self.term_shells.is_empty() {
            self.term_shells = pty_shells();
        }
        &self.term_shells
    }

    /// 目前選的 shell：設定裡的名稱找不到時（或沒選過）用第一個，也就是系統預設的
    fn chosen_shell(&mut self) -> ShellChoice {
        let name = self.config.terminal_shell.clone();
        let shells = self.shells();
        shells.iter().find(|s| s.name == name).unwrap_or(&shells[0]).clone()
    }

    /// 換成另一個 shell：結束目前的（若還在跑），在畫面上註明後啟動新的，並記在設定裡
    fn switch_shell(&mut self, ctx: &egui::Context, name: &str) {
        self.config.terminal_shell = name.to_string();
        self.term_pty = None; // `PtySession` 被丟掉時會結束它的 shell
        self.term_pty_exited = false;
        self.terminal.feed(format!("\r\n\x1b[33m[切換到 {name}]\x1b[0m\r\n").as_bytes());
        if !self.term_pty_failed {
            self.start_shell(ctx);
        }
    }

    /// 把 shell 到目前為止的輸出餵給 `terminal`，並回覆它的查詢（例如 ConPTY 啟動時問的游標位置，
    /// 沒有回覆它就不會繼續輸出）。shell 結束時收掉它，並在畫面上說明
    fn pump_shell(&mut self) {
        let Some(s) = &mut self.term_pty else { return };
        let data = s.read_available();
        if !data.is_empty() {
            self.terminal.feed(&data);
            let reply = self.terminal.take_responses();
            if !reply.is_empty() {
                let _ = s.write(&reply);
            }
        }
        if let Some(code) = s.exit_code() {
            let rest = s.read_available();
            self.terminal.feed(&rest);
            self.term_pty = None;
            self.term_pty_exited = true;
            self.terminal.feed(format!("\r\n\x1b[33m[shell 已結束（代碼 {code}）；按 Enter 重新啟動]\x1b[0m\r\n").as_bytes());
        }
    }

    /// 終端機輸入列按下 Enter：有 shell 時把這一行送進去（shell 自己會回顯並執行），
    /// shell 結束了就重新啟動再送；虛擬終端無法使用時照舊逐指令執行
    fn submit_terminal_line(&mut self, ctx: &egui::Context, line: String) {
        if self.term_pty_failed {
            self.run_terminal_command(line);
            return;
        }
        if self.term_pty.is_none() {
            self.start_shell(ctx);
        }
        match &mut self.term_pty {
            Some(s) => {
                if let Err(e) = s.write(format!("{line}\r").as_bytes()) {
                    self.terminal.feed(format!("\x1b[31m無法送出指令：{e}\x1b[0m\r\n").as_bytes());
                }
            }
            None => self.run_terminal_command(line), // 剛剛啟動失敗
        }
    }

    fn run_terminal_command(&mut self, cmd: String) {
        let prompt = format!("\x1b[36m{}\x1b[0m> {}\r\n", self.term_cwd.display(), cmd);
        self.terminal.feed(prompt.as_bytes());
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        if trimmed == "cls" || trimmed == "clear" {
            self.terminal = VtParser::new(self.terminal.rows(), self.terminal.cols());
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
        let shortcuts: [(Modifiers, Key, Cmd); 25] = [
            (Modifiers::COMMAND, Key::Backslash, Cmd::SplitEditor),
            (Modifiers::NONE, Key::F12, Cmd::GotoDefinition),
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
        // Shift+Alt+方向鍵：擴充矩形選取（沒有時從游標開始）。要在 Alt+上 / 下（移動行）之前攔下，
        // 因為 egui 的快捷鍵比對不管多按的 Shift
        if self.editor_focused {
            let mut keys = Vec::new();
            ctx.input_mut(|i| {
                i.events.retain(|e| match e {
                    egui::Event::Key { key, pressed, modifiers, .. }
                        if modifiers.alt
                            && modifiers.shift
                            && !modifiers.command
                            && matches!(key, Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight) =>
                    {
                        if *pressed {
                            keys.push(*key);
                        }
                        false
                    }
                    _ => true,
                })
            });
            self.box_keys.extend(keys);
        }
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
        // 自動完成：Ctrl+Space 叫出選單；選單開著時上下鍵選擇、Enter / Tab 接受、Esc 關閉（這些鍵不交給編輯區）
        if self.editor_focused {
            let space = |m| KeyboardShortcut::new(m, Key::Space);
            if ctx.input_mut(|i| i.consume_shortcut(&space(Modifiers::CTRL)) || i.consume_shortcut(&space(Modifiers::COMMAND))) {
                self.comp_invoke = true;
            }
            if let Some(m) = self.comp.as_mut().filter(|m| !m.shown.is_empty()) {
                let n = m.shown.len();
                ctx.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        m.selected = (m.selected + 1) % n;
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        m.selected = (m.selected + n - 1) % n;
                    }
                });
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter) || i.consume_key(Modifiers::NONE, Key::Tab)) {
                    self.comp_accept = Some(m.shown[m.selected]);
                } else if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                    self.comp = None;
                    self.ai_wait = None; // 還在等的 AI 建議也不要了
                    self.focus_editor = true; // egui 收到 Esc 時會拿掉鍵盤焦點：留在編輯區
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.commit_md_edit();
            if self.box_sel.take().is_some() {
                self.box_cols.clear();
                self.focus_editor = true; // egui 收到 Esc 時會拿掉鍵盤焦點：留在編輯區
            }
            if self.show_find_bar {
                self.focus_editor = true; // 關閉尋找列後回到編輯區（游標停在目前的命中）
            }
            self.show_find_bar = false;
            self.show_goto = false;
            self.show_settings = false;
            self.show_diff_dialog = false;
            self.show_command_palette = false;
            self.pending_close = None;
        }
        // 拖放檔案開啟
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        for p in dropped {
            self.open_path(&p);
        }
    }

    /// 帶漸層底與光暈邊線的面板 Frame
    fn bar_frame(c: &Colors) -> egui::Frame {
        egui::Frame::NONE.fill(c.panel).inner_margin(egui::Margin::symmetric(6, 3))
    }

    fn paint_bar_bg(ui: &egui::Ui, top: Color32, bottom: Color32, line: Option<(bool, Color32)>) {
        let rect = ui.max_rect().expand2(egui::vec2(6.0, 3.0));
        ui.painter().add(vgradient(rect, top, bottom));
        if let Some((at_bottom, col)) = line {
            let y = if at_bottom { rect.bottom() - 0.5 } else { rect.top() + 0.5 };
            ui.painter().hline(rect.x_range(), y, Stroke::new(1.0_f32, col));
        }
    }

    fn menu_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        self.menu_bar_for(root, c, None);
    }

    /// 選單列。`tab` 是獨立視窗的分頁 id：那個視窗的選單指令作用在它的分頁上（見 `run_cmd_on`）
    fn menu_bar_for(&mut self, root: &mut egui::Ui, c: &Colors, tab: Option<u64>) {
        let panel = match tab {
            Some(id) => egui::Panel::top(egui::Id::unique(("menu_bar_detached", id))),
            None => egui::Panel::top("menu_bar"),
        };
        panel
            .frame(Self::bar_frame(c))
            .show_separator_line(false)
            .show(root, |ui| {
                let line = if c.hud { c.accent } else { c.glow };
                Self::paint_bar_bg(ui, c.panel, c.bg, Some((true, line)));
                egui::MenuBar::new().ui(ui, |ui| {
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
                            ui.close_kind(egui::UiKind::Menu);
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
                                    ui.close_kind(egui::UiKind::Menu);
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
                        if mi(ui, "跳到定義 (Go to Definition)", "F12") { cmd = Some(Cmd::GotoDefinition); }
                        ui.separator();
                        if mi(ui, "命令面板 (Command Palette)", "Ctrl+Shift+P") { cmd = Some(Cmd::CommandPalette); }
                        if mi(ui, "全專案搜尋 (Find in Files)", "Ctrl+Shift+F") { cmd = Some(Cmd::FindInFiles); }
                    });

                    ui.menu_button("檢視 (View)", |ui| {
                        ui.set_min_width(260.0);
                        if mi(ui, "切換 Typora Markdown 模式", "Ctrl+Shift+M") {
                            cmd = Some(Cmd::CycleMarkdownMode);
                        }
                        if mi(ui, "與 Git HEAD 比較 (Diff)", "") {
                            cmd = Some(Cmd::DiffHead);
                        }
                        let split_label = if self.split.is_some() { "取消分割編輯器" } else { "分割編輯器 (Split Editor)" };
                        if mi(ui, split_label, "Ctrl+\\") {
                            cmd = Some(Cmd::SplitEditor);
                        }
                        let vertical = self.split.as_ref().is_some_and(|s| s.vertical);
                        if mi(ui, if vertical { "改成左右分割" } else { "上下分割" }, "") {
                            cmd = Some(Cmd::SplitOrientation);
                        }
                        let mut ex = self.show_explorer;
                        if ui.checkbox(&mut ex, "檔案總管側邊欄 (Explorer)  Ctrl+B").clicked() {
                            cmd = Some(Cmd::ToggleExplorer);
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui.checkbox(&mut self.config.auto_pair, "自動配對括號與引號").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui.checkbox(&mut self.config.sticky_scroll, "固定目前的函式標題 (Sticky Scroll)").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui.checkbox(&mut self.config.word_wrap, "自動換行 (Word Wrap)").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui.checkbox(&mut self.md_view.show_outline, "Markdown 大綱側邊欄 (Outline)").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.show_terminal, "互動終端機 (Terminal)").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        let mut g = self.show_graph;
                        if ui.checkbox(&mut g, "Markdown 筆記關聯圖 (Graph)").clicked() {
                            cmd = Some(Cmd::ToggleGraph);
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui.checkbox(&mut self.filter_active, "Log 行篩選工具列 (Filter)").clicked() {
                            ui.close_kind(egui::UiKind::Menu);
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
                                    ui.close_kind(egui::UiKind::Menu);
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
                        self.run_cmd_on(cmd, tab);
                    }
                });
            });
    }

    fn tool_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        egui::Panel::top("tool_bar")
            .frame(Self::bar_frame(c).inner_margin(egui::Margin::symmetric(6, 4)))
            .show_separator_line(false)
            .show(root, |ui| {
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
        egui::ComboBox::from_id_salt("filter_logic")
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

    fn tab_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        let ctx = &root.ctx().clone();
        egui::Panel::top("tab_bar")
            .frame(egui::Frame::NONE.fill(c.panel).inner_margin(egui::Margin { left: 4, right: 4, top: 2, bottom: 0 }))
            .show_separator_line(false)
            .show(root, |ui| {
                let full = ui.max_rect().expand2(egui::vec2(4.0, 1.0));
                ui.painter().add(vgradient(full, c.panel, c.bg));
                let mut close_idx = None;
                let mut tab_action: Option<TabAction> = None;
                let tab_count = self.tabs.len();
                // 不接拖曳捲動：拖曳要留給分頁（拖到分頁列外面開成獨立視窗）；捲動用滾輪
                egui::ScrollArea::horizontal().auto_shrink([false, true]).scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() }).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        // 分割檢視另一欄顯示的分頁：細一點的強調色底線（有焦點的分頁照舊是粗線）
                        let other_id = self.split.as_ref().map(|s| s.other);
                        for i in 0..self.tabs.len() {
                            // 拖出去開成獨立視窗的分頁不在主視窗的分頁列（關掉那個視窗後回來）
                            if self.detached.contains(&self.tabs[i].id) {
                                continue;
                            }
                            let is_active = i == self.active_tab_idx;
                            let is_other = !is_active && Some(self.tabs[i].id) == other_id;
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
                            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
                            let hovered = resp.hovered();
                            // 拖曳分頁：指標旁邊跟著分頁名稱；在分頁列外面放開就把分頁開成獨立視窗（主視窗也留著）
                            if resp.dragged() {
                                if let Some(pp) = ui.input(|inp| inp.pointer.hover_pos()) {
                                    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("alexcode-tab-drag"));
                                    let gp = ui.ctx().layer_painter(layer);
                                    let at = pp + egui::vec2(14.0, 10.0);
                                    let g = gp.layout_no_wrap(title.clone(), FontId::proportional(13.0), c.fg);
                                    gp.rect(Rect::from_min_size(at, g.size()).expand2(egui::vec2(8.0, 4.0)), 3.0, c.panel, Stroke::new(1.0_f32, c.accent), egui::StrokeKind::Middle);
                                    gp.galley(at, g, c.fg);
                                }
                            }
                            if resp.drag_stopped() {
                                let out = ui.input(|inp| inp.pointer.interact_pos()).is_some_and(|pp| !full.expand(24.0).contains(pp));
                                if out {
                                    tab_action = Some(TabAction::Detach(i));
                                }
                            }
                            let p = ui.painter();
                            if is_active {
                                let grad = Rect::from_min_max(
                                    Pos2::new(rect.left(), rect.top() + rect.height() * 0.5),
                                    rect.right_bottom(),
                                );
                                p.add(vgradient(grad, Color32::TRANSPARENT, c.accent_faint));
                                p.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0_f32, c.accent));
                            } else if is_other {
                                p.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(1.0_f32, c.accent));
                            } else if hovered {
                                p.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0_f32, c.glow));
                            }
                            let tcol = if is_active {
                                c.accent
                            } else if hovered || is_other {
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
                                .is_some_and(|pp| cr.contains(pp));
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
                            let on_disk = self.tabs[i].file_path.as_deref().is_some_and(|p| Path::new(p).is_absolute());
                            let tip = self.tabs[i].file_path.clone().unwrap_or_default();
                            let resp = resp.on_hover_text(if is_other { format!("{tip}\n（顯示在分割檢視的另一欄）") } else { tip });
                            // 右鍵選單：檔案總管 / 終端機 / 複製路徑 / 關閉
                            resp.context_menu(|ui| {
                                ui.set_min_width(250.0);
                                let mut pick = |ui: &mut egui::Ui, enabled: bool, label: &str, a: TabAction| {
                                    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                        tab_action = Some(a);
                                        ui.close_kind(egui::UiKind::Menu);
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
                                ui.separator();
                                pick(ui, true, "在新視窗開啟", TabAction::Detach(i));
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

    fn find_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        let ctx = &root.ctx().clone();
        if !self.show_find_bar {
            return;
        }
        // 全文與命中位置都只在內容、搜尋字串或選項改變時重算
        let idx = self.active_tab_idx;
        let key = self.tabs.get(idx).map_or(self.text_cache.0, |b| (b.id, b.revision()));
        let fkey = (key.0, key.1, format!("{}\u{0}{:?}", self.find_bar_text, self.find_opts));
        if self.find_cache.0 != fkey {
            // 大檔案（大檔案檢視）裡每打一個字就串接全文再搜尋一次太慢（29 MB 的檔案約 30 ms）：
            // 只有內容變了的時候，等停止輸入 `LARGE_FIND_DELAY` 之後才重算，這段時間命中數沿用舊的。
            // 尋找下一個 / 取代不看這份快取，不受影響
            let large = self.uses_large_view(idx);
            let only_content = self.find_cache.0 .0 == fkey.0 && self.find_cache.0 .2 == fkey.2;
            let mut wait = None;
            if large && only_content {
                if self.find_seen.0 != key {
                    self.find_seen = (key, std::time::Instant::now());
                }
                wait = LARGE_FIND_DELAY.checked_sub(self.find_seen.1.elapsed()).filter(|d| !d.is_zero());
            }
            match wait {
                Some(d) => ctx.request_repaint_after(d),
                None => {
                    self.sync_text_cache();
                    let found = find_all(&self.text_cache.1, &self.find_bar_text, &self.find_opts);
                    self.find_error = found.as_ref().err().cloned();
                    self.find_cache = (fkey, found.unwrap_or_default());
                    if large {
                        // 大檔案檢視不用全文快取：算完就放掉，不留一份與檔案一樣大的複本
                        self.text_cache = ((0, 0), String::new());
                    }
                }
            }
        }
        let total = self.find_cache.1.len();
        // (往下找?), 取代, 全部取代
        let mut go: Option<bool> = None;
        let (mut do_replace, mut do_replace_all) = (false, false);
        egui::Panel::top("find_bar")
            .frame(egui::Frame::NONE.fill(c.panel).inner_margin(egui::Margin::symmetric(10, 5)))
            .show_separator_line(false)
            .show(root, |ui| {
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
    fn explorer_panel(&mut self, root_ui: &mut egui::Ui, c: &Colors) {
        let ctx = &root_ui.ctx().clone();
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
        egui::Panel::left("explorer_panel")
            .resizable(true)
            .default_size(240.0)
            .size_range(160.0..=520.0)
            .frame(egui::Frame::NONE.fill(c.panel).stroke(Stroke::new(1.0_f32, c.border)))
            .show(root_ui, |ui| {
                if Self::dock_title(ui, c, "EXPLORER  檔案總管") {
                    close = true;
                }
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| root.display().to_string());
                    ui.add(egui::Label::new(RichText::new(name).color(c.accent).strong()).truncate())
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
                let is_active = !e.is_dir && active.is_some_and(|a| Path::new(a) == e.path);
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
                    ui.close_kind(egui::UiKind::Menu);
                }
                for (kind, label) in available_shells() {
                    if ui.button(*label).clicked() {
                        actions.push(TreeAction::Shell(folder.clone(), *kind));
                        ui.close_kind(egui::UiKind::Menu);
                    }
                }
                ui.separator();
                if ui.button("複製完整路徑").clicked() {
                    actions.push(TreeAction::CopyPath(e.path.clone()));
                    ui.close_kind(egui::UiKind::Menu);
                }
            });
            if expanded {
                self.tree_rows(ui, c, &e.path, depth + 1, active, actions);
            }
        }
    }

    /// 分頁有未儲存的修改、檔案又被其他程式改了：讓使用者決定要哪一份
    fn disk_conflict_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        let idx = self.active_tab_idx;
        let Some(buf) = self.tabs.get(idx).filter(|b| Some(b.id) == self.disk_conflict) else { return };
        let name = Self::tab_title(buf);
        let mut choice: Option<bool> = None; // true = 重新載入
        egui::Panel::top("disk_conflict_bar")
            .frame(egui::Frame::NONE.fill(c.panel).inner_margin(egui::Margin::symmetric(10, 5)))
            .show(root, |ui| {
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
        let start = range.primary.index.0.min(range.secondary.index.0);
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
        let (a, b) = (range.primary.index.0.min(range.secondary.index.0), range.primary.index.0.max(range.secondary.index.0));
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
        if !backspace && !typed.is_some_and(pair_relevant) {
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

    fn status_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        egui::Panel::bottom("status_bar")
            .frame(egui::Frame::NONE.fill(c.bg).inner_margin(egui::Margin::symmetric(4, 3)))
            .show_separator_line(false)
            .show(root, |ui| {
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
                        let r = ui.add(egui::Label::new(RichText::new(t).size(12.0).color(col)).extend());
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
                                ui.close_kind(egui::UiKind::Menu);
                            }
                        }
                        ui.separator();
                        ui.menu_button("以其他編碼重新讀取", |ui| {
                            for e in FileEncoding::ALL {
                                if ui.button(e.label()).clicked() {
                                    enc_choice = Some((e, true));
                                    ui.close_kind(egui::UiKind::Menu);
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
                    // 語言伺服器：這個分頁的檔案類型有對應的伺服器（而且已經用到它）時顯示它的狀態；
                    // 用不了時滑鼠停留顯示怎麼安裝
                    let kind = self.active_buffer().and_then(|b| b.file_path.as_deref()).and_then(ServerKind::for_path);
                    if let Some((kind, status)) = kind.and_then(|k| self.lsp.status(k).map(|s| (k, s.clone()))) {
                        let color = match status {
                            LspStatus::Ready => c.accent,
                            LspStatus::Starting => c.fg_dim2,
                            _ => c.warn,
                        };
                        let text = format!("{}：{}", kind.program(), status.label());
                        let r = ui.add(egui::Label::new(RichText::new(text).size(12.0).color(color)).extend());
                        let hint = if status == LspStatus::Ready || status == LspStatus::Starting {
                            "語言伺服器（開著的文件會同步給它）"
                        } else {
                            lsp_install_hint(kind)
                        };
                        let r = r.on_hover_text(hint);
                        self.mark("lsp-status", &r);
                        ui.add_space(10.0);
                        let (sr, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), Sense::hover());
                        ui.painter().rect_filled(sr, 0.0, c.border);
                        ui.add_space(10.0);
                    }
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
                        // Git blame：游標所在行最後一次修改的提交（太長時截斷，完整內容在滑鼠停留時顯示）
                        if let Some(full) = self.blame_text() {
                            ui.add_space(14.0);
                            let short = if full.chars().count() > BLAME_MAX_CHARS {
                                full.chars().take(BLAME_MAX_CHARS).collect::<String>() + "…"
                            } else {
                                full.clone()
                            };
                            ui.add(egui::Label::new(RichText::new(short).size(12.0).color(c.fg_dim2)).truncate())
                                .on_hover_text(full);
                        }
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
        let resp = ui.interact(cr, ui.scope_id().with(title), Sense::click());
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

    fn terminal_dock(&mut self, root: &mut egui::Ui, c: &Colors) {
        let ctx = &root.ctx().clone();
        // 互動式 shell：終端機第一次打開時在虛擬終端裡啟動；輸出由背景執行緒讀進來後要求重畫，
        // 這裡每一幀把讀到的部分餵給 `terminal`（終端機收起來時也照讀，免得輸出堆著）
        if self.show_terminal && self.term_pty.is_none() && !self.term_pty_failed && !self.term_pty_exited {
            self.start_shell(ctx);
        }
        self.pump_shell();
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
        egui::Panel::bottom("terminal_dock")
            .resizable(true)
            .default_size(200.0)
            .frame(egui::Frame::NONE.fill(c.bg).stroke(Stroke::new(1.0_f32, c.border)))
            .show(root, |ui| {
                self.mark_rect("terminal-panel", ui.max_rect());
                if Self::dock_title(ui, c, "TERMINAL  整合終端機") {
                    self.show_terminal = false;
                }
                egui::Frame::NONE.inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                    // 內容一定佔滿整個高度：可以改大小的面板下一幀會縮成內容用掉的高度，而最後一列（輸入列）
                    // 後面的間距不算在內，不佔滿的話每一幀縮 2 像素（shell 有輸出就一直重畫，終端機越縮越小、大小一直變）
                    ui.set_min_height(ui.available_height());
                    let input_h = self.term_input_h;
                    // 選擇 shell：換了就結束目前的、啟動選的那一個（記在設定裡）
                    let mut switch_to = None;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Shell").size(12.0).color(c.fg_dim));
                        let current = self.chosen_shell().name;
                        let names: Vec<String> = self.shells().iter().map(|s| s.name.clone()).collect();
                        let combo = egui::ComboBox::from_id_salt("terminal-shell").selected_text(current.as_str()).show_ui(ui, |ui| {
                            for n in &names {
                                if ui.selectable_label(*n == current, n.as_str()).clicked() && *n != current {
                                    switch_to = Some(n.clone());
                                }
                            }
                        });
                        self.mark("terminal-shell", &combo.response);
                        if self.term_pty.is_some() {
                            ui.label(
                                RichText::new("點一下輸出區後，鍵盤（含方向鍵、Tab、Ctrl+C）直接送進 shell")
                                    .size(11.0)
                                    .color(c.faint),
                            );
                        }
                    });
                    if let Some(name) = switch_to {
                        self.switch_shell(ctx, &name);
                    }
                    // 輸出區有鍵盤焦點時，按鍵與輸入的字直接送進 shell（shell 自己回顯）；
                    // Tab / 方向鍵 / Esc 也留給 shell，不拿來移動 egui 的焦點
                    let term_id = egui::Id::unique("alexcode-terminal-output");
                    let focused = ui.memory(|m| m.has_focus(term_id));
                    if focused {
                        ui.memory_mut(|m| {
                            m.set_focus_lock_filter(
                                term_id,
                                egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
                            )
                        });
                        let bytes = terminal_input_bytes_ime(&ui.input(|i| i.events.clone()), &mut self.term_preedit);
                        if let (false, Some(s)) = (bytes.is_empty(), &mut self.term_pty) {
                            let _ = s.write(&bytes);
                        }
                    } else {
                        self.term_preedit.clear();
                    }
                    // 終端機的大小跟著輸出區：可見的寬高能放幾欄 × 幾列，變了就改 `terminal` 並通知 shell
                    let font = FontId::monospace(13.0);
                    // 每個字元往右移的距離就是字寬（egui 0.33 起以次像素累加，不再對齊到像素）
                    let adv = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
                    let row_h = ui.fonts_mut(|f| f.row_height(&font));
                    let pitch = row_h + ui.spacing().item_spacing.y;
                    let avail = egui::vec2(ui.available_width() - ui.spacing().scroll.bar_width - 8.0, ui.available_height() - input_h);
                    let size = (
                        ((avail.y / pitch).floor().max(0.0) as usize).clamp(2, 200),
                        ((avail.x / adv).floor().max(0.0) as usize).clamp(20, 500),
                    );
                    if size != (self.terminal.rows(), self.terminal.cols()) {
                        self.resize_terminal(size.0, size.1);
                    }
                    let alive = self.term_pty.is_some();
                    let mut cursor_rect = None;
                    let out = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(ui.available_height() - input_h)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            // 捲出畫面的歷史稍暗一點；顏色照 `VtParser` 記下的
                            for i in 0..self.terminal.scrollback_count() {
                                ui.label(terminal_row_job(self.terminal.scrollback_cells(i), &font, c.fg_dim));
                            }
                            // 畫面上的列：排到最後一列有字的、或游標所在的那一列為止
                            let t = &self.terminal;
                            let last = (0..t.rows()).rev().find(|&r| !t.line_text(r).trim().is_empty()).map_or(0, |r| r + 1);
                            let (cy, cx) = (t.cursor_row(), t.cursor_col());
                            for r in 0..last.max(cy + 1).min(t.rows()) {
                                let galley = ui.fonts_mut(|f| f.layout_job(terminal_row_job(t.row_cells(r), &font, c.fg)));
                                let resp = ui.label(galley.clone());
                                // 游標：shell 在跑時畫在游標所在的格子（有焦點時是實心方塊，沒有時是外框）
                                if r == cy && alive {
                                    // 排出來的字數（空的一列排的是一個佔位的空白，不算）；游標在字的後面時每格加一個字寬
                                    let blank = t.row_cells(r).iter().all(|c| c.ch == ' ' && c.bg < 0);
                                    let n = if blank { 0 } else { galley.job.text.chars().count() };
                                    let at = |i: usize| galley.pos_from_cursor(CCursor::new(i)).left();
                                    let x = at(cx.min(n)) + cx.saturating_sub(n) as f32 * adv;
                                    cursor_rect = Some(Rect::from_min_size(
                                        Pos2::new(resp.rect.left() + x, resp.rect.top()),
                                        egui::vec2(adv, row_h),
                                    ));
                                }
                            }
                            if let Some(r) = cursor_rect {
                                if focused {
                                    ui.painter().rect_filled(r, 0.0, c.accent.gamma_multiply(0.7));
                                } else {
                                    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0_f32, c.accent), egui::StrokeKind::Middle);
                                }
                            }
                        });
                    if let Some(r) = cursor_rect {
                        self.mark_rect("terminal-cursor", r);
                        if focused {
                            // 輸入法：組字中的文字畫在游標處（蓋住後面的字，送進 shell 之前不改畫面內容），候選字視窗放在它後面。
                            // 告訴作業系統游標在哪裡也是讓它在這裡啟用輸入法的條件（沒有的話 egui 會關掉輸入法）
                            let mut caret = r;
                            if !self.term_preedit.is_empty() {
                                let g = ui.fonts_mut(|f| f.layout_no_wrap(self.term_preedit.clone(), font.clone(), c.fg));
                                let pre = Rect::from_min_size(r.min, egui::vec2(g.size().x, r.height()));
                                let painter = ui.painter_at(out.inner_rect);
                                painter.rect_filled(pre, 0.0, c.panel);
                                painter.galley(pre.min, g, c.fg);
                                painter.line_segment([pre.left_bottom(), pre.right_bottom()], Stroke::new(1.0_f32, c.accent));
                                caret = Rect::from_min_size(Pos2::new(pre.right(), r.top()), r.size());
                                self.mark_rect("terminal-preedit", pre);
                            }
                            let rect = out.inner_rect;
                            ui.ctx().output_mut(|o| {
                                o.ime = Some(egui::output::IMEOutput {
                                    purpose: egui::IMEPurpose::Normal,
                                    rect,
                                    cursor_rect: caret,
                                    should_interrupt_composition: false,
                                })
                            });
                        }
                    }
                    // 點一下輸出區取得鍵盤焦點（只接點擊：拖曳仍然是選取輸出的文字）；有焦點時加一圈外框
                    let resp = ui.interact(out.inner_rect, term_id, Sense::click());
                    if resp.clicked() {
                        resp.request_focus();
                    }
                    if focused {
                        ui.painter().rect_stroke(out.inner_rect.expand(2.0), 2.0, Stroke::new(1.0_f32, c.accent), egui::StrokeKind::Middle);
                    }
                    self.mark_rect("terminal-output", out.inner_rect);
                    let input_row = ui.horizontal(|ui| {
                        ui.label(RichText::new("❯").color(c.accent).strong());
                        // 逐指令執行（虛擬終端無法使用時）要等指令跑完；互動式 shell 隨時可以再送
                        let busy = self.term_rx.is_some();
                        let hint = if busy {
                            "執行中…"
                        } else if self.term_pty.is_some() {
                            "輸入指令後按 Enter 送進 shell，輸出即時顯示（例如 dir、git status、cargo build）"
                        } else {
                            "輸入指令後按 Enter（例如 dir、git status、cargo --version）"
                        };
                        let resp = ui.add_enabled(
                            !busy,
                            egui::TextEdit::singleline(&mut self.term_input)
                                .font(FontId::monospace(13.0))
                                .hint_text(hint)
                                .desired_width(f32::INFINITY),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            let cmd = std::mem::take(&mut self.term_input);
                            self.submit_terminal_line(ctx, cmd);
                            resp.request_focus();
                        }
                    });
                    self.term_input_h = input_row.response.rect.height() + ui.spacing().item_spacing.y;
                    // 之前固定留 28 px；egui 0.37 下輸入列量到 26 px、加上間距 4 px 共 30 px，每一幀多出 2 px、面板一直變大：改成量出來的高度
                });
            });
    }

    fn graph_dock(&mut self, root: &mut egui::Ui, c: &Colors) {
        if !self.show_graph {
            return;
        }
        egui::Panel::bottom("graph_dock")
            .resizable(true)
            .default_size(240.0)
            .frame(egui::Frame::NONE.fill(c.bg).stroke(Stroke::new(1.0_f32, c.border)))
            .show(root, |ui| {
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
                    let hot = hover.is_some_and(|h| h.distance(*p) < r + 4.0);
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

    fn editor_area(&mut self, root: &mut egui::Ui, c: &Colors) {
        let ctx = &root.ctx().clone();
        // 內容還在背景載入的分頁：只顯示「載入中」，沒有可以輸入的編輯區
        if self.is_loading(self.active_tab_idx) {
            self.editor_was_focused = false;
            self.editor_focused = false;
            let path = self.tabs[self.active_tab_idx].file_path.clone().unwrap_or_default();
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(c.bg)).show(root, |ui| {
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
        // 很大的 Markdown 檔當成一般文字檔顯示（原始碼）：閱讀 / 雙欄模式每一幀都解析並排版整份文件
        let md_lang = self.tab_lang(self.active_tab_idx) == Lang::Markdown;
        let md_big = md_lang && self.md_too_big(self.active_tab_idx);
        let is_md = md_lang && !md_big;
        let filtering = self.filter_active && !self.filter_query.trim().is_empty();
        if !filtering && md_big {
            self.md_big_bar(root, c);
        }

        if !filtering && is_md {
            // Markdown 專屬頂部工具列（Typora 閱讀 / 雙欄 / 原始碼 切換 + 快速格式插入）
            egui::Panel::top("md_typora_toolbar")
                .frame(egui::Frame::NONE.fill(c.panel))
                .show_separator_line(false)
                .show(root, |ui| {
                    if let Some(buf) = self.tabs.get_mut(self.active_tab_idx) {
                        render_markdown_toolbar(ui, &mut self.md_view, buf, c);
                    }
                });

            // 左側 Typora 章節大綱 (Outline) 面板
            if self.md_view.show_outline {
                egui::Panel::left("md_outline_panel")
                    .resizable(true)
                    .default_size(220.0)
                    .min_size(150.0)
                    .max_size(380.0)
                    .frame(egui::Frame::NONE.fill(c.panel).stroke(Stroke::new(1.0_f32, c.border)))
                    .show(root, |ui| {
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

        self.disk_conflict_bar(root, c);
        self.editor_was_focused = self.editor_focused;
        self.editor_focused = false; // 由 code_editor 在本幀重新設定
        let mut md_action = MdAction::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(c.bg))
            .show(root, |ui| {
                let full_rect = ui.max_rect();
                // 分割檢視：另一欄畫成唯讀，下面照常把目前分頁畫在有焦點的那一欄
                let mut pane = self.split_panes(ui, c, &pal);
                let ui: &mut egui::Ui = match pane.as_mut() {
                    Some(p) => p,
                    None => ui,
                };
                if filtering {
                    self.filtered_view(ui, c, &pal);
                } else if is_md {
                    // 尋找列開著時，預覽用與編輯區相同的選項標示命中；查詢或選項沒變就沿用編譯好的條件
                    let find = if self.show_find_bar {
                        let key = format!("{}\u{0}{:?}", self.find_bar_text, self.find_opts);
                        if self.md_find.0 != key {
                            self.md_find = (key, Finder::new(&self.find_bar_text, &self.find_opts));
                        }
                        self.md_find.1.clone()
                    } else {
                        Finder::default()
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

    /// 分割檢視：把編輯區分成左右兩欄，在另一欄唯讀畫出另一個分頁，回傳有焦點那一欄的 Ui（沒有分割時 None）。
    /// 點另一欄就換成編輯它：目前分頁改成那一個，原本的分頁改到另一欄唯讀顯示（兩欄的位置不變）。
    /// 另一欄的分頁被關掉時結束分割
    fn split_panes(&mut self, ui: &mut egui::Ui, c: &Colors, pal: &crate::theme::Palette) -> Option<egui::Ui> {
        let other = self.split.as_ref().and_then(|s| self.tabs.iter().position(|b| b.id == s.other));
        let Some(mut other) = other else {
            self.split = None;
            self.split_mid = None;
            return None;
        };
        let full = ui.max_rect();
        // 中間的分隔線可以拖曳改兩欄的寬度（上下分割時是高度；存成比例，視窗改大小時照比例；兩欄都至少留 120 像素）。
        // 以下的 left / right 在上下分割時是上 / 下
        let vertical = self.split.as_ref().is_some_and(|s| s.vertical);
        let (edge, span) = if vertical { (full.top(), full.height()) } else { (full.left(), full.width()) };
        let along = move |p: &Pos2| if vertical { p.y } else { p.x };
        let band = |at: f32, half: f32| {
            if vertical {
                Rect::from_min_max(Pos2::new(full.left(), at - half), Pos2::new(full.right(), at + half))
            } else {
                Rect::from_min_max(Pos2::new(at - half, full.top()), Pos2::new(at + half, full.bottom()))
            }
        };
        let ratio = self.split.as_ref().map_or(0.5, |s| s.ratio);
        let old_mid = (edge + span * ratio).round();
        let grip_resp = ui.interact(band(old_mid, 4.0), ui.scope_id().with("split-divider"), Sense::drag());
        if grip_resp.hovered() || grip_resp.dragged() {
            ui.ctx().set_cursor_icon(if vertical { egui::CursorIcon::ResizeVertical } else { egui::CursorIcon::ResizeHorizontal });
        }
        if let (true, Some(p), Some(s)) = (grip_resp.dragged(), ui.input(|i| i.pointer.interact_pos()), self.split.as_mut()) {
            let lo = (120.0 / span).min(0.5);
            s.ratio = ((along(&p) - edge) / span).clamp(lo, 1.0 - lo);
        }
        let ratio = self.split.as_ref().map_or(0.5, |s| s.ratio);
        let mid = (edge + span * ratio).round();
        self.split_mid = Some(mid);
        let on_grip = |p: &Pos2| (along(p) - old_mid).abs() <= 4.0 || grip_resp.dragged();
        let (left, right) = if vertical {
            (Rect::from_min_max(full.min, Pos2::new(full.max.x, mid - 1.0)), Rect::from_min_max(Pos2::new(full.min.x, mid + 1.0), full.max))
        } else {
            (Rect::from_min_max(full.min, Pos2::new(mid - 1.0, full.max.y)), Rect::from_min_max(Pos2::new(mid + 1.0, full.min.y), full.max))
        };
        let right_active = self.split.as_ref().is_none_or(|s| s.right_active);
        let other_rect = if right_active { left } else { right };
        // 先看這一幀有沒有在另一欄按下滑鼠（在畫之前，這樣換過去的那一幀就畫在對的位置）。
        // 按下時就換：接著拖曳由編輯區延伸選取，放開時沒移動就只是把游標放在那裡
        // 不看那個位置最上面是哪個元件（閱讀模式的 Markdown 有自己的元件），只看是不是按在這一欄、沒有被彈出視窗蓋住
        let press = ui.input(|i| i.pointer.press_origin().filter(|_| i.pointer.primary_pressed()));
        let layer = ui.layer_id();
        // 右邊與下面留給捲軸（拖捲軸不換欄）
        let bar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_outer_margin + ui.spacing().scroll.bar_inner_margin + 2.0;
        let hit = Rect::from_min_max(other_rect.min, other_rect.max - egui::vec2(bar, bar));
        if let Some(press) = press.filter(|p| hit.contains(*p) && !on_grip(p) && ui.ctx().layer_id_at(*p) == Some(layer)) {
            // 按下的位置換成（行, 欄）：用上一幀畫出的那幾行；按在最後一行下面就放在那一行的行尾
            let at = Some(press).and_then(|p| {
                let rows = &self.split.as_ref()?.rows;
                match rows.iter().find(|(_, o, g)| p.y < o.y + g.size().y.max(1.0)) {
                    Some((line, o, g)) => Some((*line, g.cursor_from_pos(p - *o).index.0)),
                    None => rows
                        .last()
                        .map(|(line, _, g)| (*line, g.cursor_from_pos(egui::vec2(f32::INFINITY, 0.0)).index.0)),
                }
            });
            // 換欄：兩邊的捲動位置互換，所以畫面上的內容不動（編輯區的文字多一段上邊界）
            let other_id = self.tabs[other].id;
            if let (Some(s), Some(cur)) = (self.split.as_mut(), self.tabs.get(self.active_tab_idx).map(|b| b.id)) {
                let was = self.editor_offsets.get(&cur).copied().unwrap_or_default();
                self.pending_offset = Some((other_id, s.offset + egui::vec2(0.0, SPLIT_TEXT_MARGIN_Y)));
                s.pending = Some(egui::vec2(was.x, (was.y - SPLIT_TEXT_MARGIN_Y).max(0.0)));
                s.other = cur;
                s.right_active = !s.right_active;
            }
            self.active_tab_idx = other;
            if let Some((line, col)) = at {
                let buf = &mut self.tabs[other];
                let line = line.min(buf.lines.len().saturating_sub(1));
                let col = col.min(buf.line_len(line));
                buf.selections = vec![Selection::point(Position { line, col })];
                let i = buf.lines.iter().take(line).map(|s| s.chars().count() + 1).sum::<usize>() + col;
                self.pending_select = Some((i, i));
                self.split_drag = Some(i);
            }
            other = self.tabs.iter().position(|b| Some(b.id) == self.split.as_ref().map(|s| s.other)).unwrap_or(other);
            self.focus_editor = true;
        }
        let right_active = self.split.as_ref().is_none_or(|s| s.right_active);
        let (active_rect, other_rect) = if right_active { (right, left) } else { (left, right) };
        ui.painter().rect_filled(band(mid, 1.0), 0.0, c.border);
        let clip = ui.clip_rect();
        let mut pane = ui.new_child(egui::UiBuilder::new().id_salt("split-other").max_rect(other_rect).layout(Layout::top_down(Align::Min)));
        pane.set_clip_rect(other_rect.intersect(clip));
        // 另一欄的 Git 標記：內容或 HEAD 變了才重算
        if let (Some(s), Some(b)) = (self.split.as_mut(), self.tabs.get(other)) {
            let head = self.git_head.get(&b.id).and_then(|h| h.as_ref());
            let key = (b.id, b.revision(), head.map_or(0, |h| Arc::as_ptr(h) as usize));
            if s.git.0 != key {
                s.git = (key, head.map(|h| diff_line_states(h, &b.lines, 800)).unwrap_or_default());
            }
        }
        let pending = self.split.as_mut().and_then(|s| s.pending.take());
        // Markdown 分頁在閱讀 / 雙欄模式下，另一欄也照閱讀模式顯示（不能在上面直接編輯；連結不跟著開）
        let md_other = self.md_view.mode != MdViewMode::Source
            && self.tab_lang(other) == Lang::Markdown
            && !self.md_too_big(other);
        if md_other {
            pane.painter().rect_filled(pane.max_rect(), 0.0, c.panel);
            let find = Finder::default();
            let (root, fsize) = (self.project_root.clone(), self.config.font_size);
            if let (Some(s), Some(buf)) = (self.split.as_mut(), self.tabs.get_mut(other)) {
                let id = buf.id;
                pane.push_id(("split-markdown", id), |ui| {
                    let _ = render_typora_document(ui, &mut s.md, buf, fsize, c, pal, &find, &root, false);
                });
                s.rows.clear();
            }
        } else {
            let empty = HashMap::new();
            let git = self.split.as_ref().map_or(&empty, |s| &s.git.1);
            let (rows, offset) = self.split_preview(&mut pane, other, c, pal, pending, git);
            if let Some(s) = self.split.as_mut() {
                s.rows = rows;
                s.offset = offset;
            }
        }
        let mut active = ui.new_child(egui::UiBuilder::new().id_salt("split-active").max_rect(active_rect).layout(Layout::top_down(Align::Min)));
        active.set_clip_rect(active_rect.intersect(clip));
        Some(active)
    }

    /// 分割檢視裡沒有焦點的那一欄：唯讀顯示分頁 `idx`（行號、語法高亮，可捲動；每一行最多畫 4000 個字元）。
    /// 回傳畫出的每一行（行號, 文字左上角, 排版）
    fn split_preview(
        &self,
        ui: &mut egui::Ui,
        idx: usize,
        c: &Colors,
        pal: &crate::theme::Palette,
        offset: Option<Vec2>,
        git: &HashMap<usize, u8>,
    ) -> (Vec<(usize, Pos2, Arc<egui::Galley>)>, Vec2) {
        let mut drawn = Vec::new();
        let Some(buf) = self.tabs.get(idx) else { return (drawn, Vec2::ZERO) };
        let lang = self.tab_lang(idx);
        let font = FontId::monospace(self.config.font_size);
        // 行高取編輯區排版實際的行距（排版會把每一列對齊到像素，和 row_height 差一點點，幾百行後就差很多）
        let row_h = ui.fonts_mut(|f| {
            let g = f.layout_no_wrap("0\n0".into(), font.clone(), Color32::WHITE);
            g.rows.get(1).map_or_else(|| f.row_height(&font), |r| r.rect().top())
        });
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        let digits = buf.lines.len().to_string().len().max(3);
        let gutter_w = digits as f32 * char_w + 30.0;
        ui.painter().rect_filled(ui.max_rect(), 0.0, c.panel);
        // 尋找列開著時與編輯區用同樣的條件標示命中
        let finder = (self.show_find_bar && !self.find_bar_text.is_empty()).then(|| Finder::new(&self.find_bar_text, &self.find_opts));
        let head = buf.selections.first().map(|s| s.head);
        let sel_range = buf.selections.first().filter(|s| s.anchor != s.head).map(|s| (s.start(), s.end()));
        // 語言伺服器的診斷：行號以那一行最嚴重的診斷上色，範圍畫波浪線（與編輯區相同）
        let diags = buf.file_path.as_deref().map(|p| self.lsp.diagnostics(p)).unwrap_or_default();
        let mut diag_sev: HashMap<usize, i32> = HashMap::new();
        for d in diags.iter() {
            let s = diag_sev.entry(d.start_line.max(0) as usize).or_insert(d.severity);
            *s = (*s).min(d.severity);
        }
        // 不接拖曳捲動：點擊與拖曳要交給上面那個切換欄位的區域（可以捲動時捲動區會蓋掉它）
        let mut area = egui::ScrollArea::both()
            .id_salt(("split-preview", buf.id))
            .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
            .auto_shrink([false, false]);
        if let Some(o) = offset {
            area = area.scroll_offset(o);
        }
        // show_rows 以「行高 + 外層的列距」換算捲動位置：列距要先設成 0，第 n 行才會在 n × 行高的位置（與編輯區相同）
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let out = area.show_rows(ui, row_h, buf.lines.len(), |ui, range| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for ln in range {
                    let line = &buf.lines[ln];
                    let shown = line.char_indices().nth(4000).map_or(line.as_str(), |(i, _)| &line[..i]);
                    let hits = finder.as_ref().map(|f| f.find_all(shown)).unwrap_or_default();
                    let g = ui.fonts_mut(|f| f.layout_job(highlight_ranges(shown, lang, pal, font.clone(), &hits)));
                    let w = (gutter_w + 10.0 + g.size().x + 20.0).max(ui.available_width());
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, row_h), Sense::hover());
                    ui.painter().text(
                        Pos2::new(rect.left() + gutter_w - 10.0, rect.center().y),
                        Align2::RIGHT_CENTER,
                        format!("{}", ln + 1),
                        FontId::monospace(self.config.font_size - 1.0),
                        diag_sev.get(&ln).map_or(c.fg_dim2, |&s| diag_color(s)),
                    );
                    // 書籤與 Git 標記（與編輯區的行號區相同）
                    let y = rect.top();
                    if buf.bookmarks.contains(&ln) {
                        ui.painter().circle_filled(Pos2::new(rect.left() + 9.0, rect.center().y), 3.5, c.accent2);
                    }
                    if let Some(&st) = git.get(&ln) {
                        let x = rect.left() + gutter_w - 4.0;
                        if st & (ADDED | MODIFIED) != 0 {
                            let col = if st & MODIFIED != 0 { c.warn } else { c.accent };
                            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(x, y), egui::vec2(3.0, row_h)), 0.0, col);
                        }
                        if st & DELETED_ABOVE != 0 {
                            ui.painter().add(Shape::convex_polygon(
                                vec![Pos2::new(x - 3.0, y - 3.0), Pos2::new(x + 3.0, y), Pos2::new(x - 3.0, y + 3.0)],
                                c.accent2,
                                Stroke::NONE,
                            ));
                        }
                    }
                    let at = Pos2::new(rect.left() + gutter_w + 10.0, rect.top());
                    // 選取範圍的底色（跨到下一行的部分在行尾多畫半個字寬，表示換行也選到了）
                    if let Some((s, e)) = sel_range.filter(|(s, e)| s.line <= ln && ln <= e.line) {
                        let n = shown.chars().count();
                        let from = if ln == s.line { s.col.min(n) } else { 0 };
                        let to = if ln == e.line { e.col.min(n) } else { n };
                        let x = |ci: usize| g.pos_from_cursor(CCursor::new(ci)).left();
                        let extra = if ln < e.line { char_w * 0.5 } else { 0.0 };
                        let (x0, x1) = (x(from), x(to) + extra);
                        if x1 > x0 {
                            let r = Rect::from_min_max(Pos2::new(at.x + x0, rect.top()), Pos2::new(at.x + x1, rect.top() + row_h));
                            ui.painter().rect_filled(r, 0.0, c.sel_bg);
                        }
                    }
                    // 那個分頁的游標位置：一條細線（沒有焦點，不閃爍）
                    if let Some(h) = head.filter(|h| h.line == ln) {
                        let ci = h.col.min(shown.chars().count());
                        let r = g.pos_from_cursor(CCursor::new(ci));
                        ui.painter().rect_filled(Rect::from_min_size(at + egui::vec2(r.left(), 0.0), egui::vec2(1.5, row_h)), 0.0, c.fg_dim);
                    }
                    // 波浪線：把跨到這一行的診斷換成只有這一行的範圍（開頭在前面的行就從行首、結尾在後面的行就到行尾）
                    let one = [shown.to_string()];
                    for d in diags.iter().filter(|d| d.start_line.max(0) as usize <= ln && d.end_line.max(0) as usize >= ln) {
                        let mut d1 = d.clone();
                        if (d.start_line.max(0) as usize) < ln {
                            (d1.start_line, d1.start_char) = (0, 0);
                        } else {
                            d1.start_line = 0;
                        }
                        d1.end_line = if (d.end_line.max(0) as usize) > ln { 1 } else { 0 };
                        for (_, bottom, l, r) in diag_spans(&g, &one, &[], &d1, char_w) {
                            ui.painter().add(squiggle(at.x + l, at.x + r, at.y + bottom - 1.0, diag_color(d.severity)));
                        }
                    }
                    drawn.push((ln, at, g.clone()));
                    ui.painter().galley(at, g, c.fg);
                }
            });
        (drawn, out.state.clamped_offset())
    }

    /// 在獨立視窗裡顯示、可編輯的分頁（分頁右鍵「在新視窗開啟」，或把分頁拖到分頁列外面）。每個分頁一個作業系統視窗（egui 的 viewport），
    /// 不支援多視窗的環境（例如測試）改畫成主視窗裡的浮動視窗。分頁關掉或視窗被關掉時就不再顯示
    fn detached_windows(&mut self, ctx: &egui::Context, c: &Colors, pal: &crate::theme::Palette) {
        let alive: Vec<u64> = self.tabs.iter().map(|b| b.id).collect();
        self.detached.retain(|id| alive.contains(id));
        self.detached_git.retain(|id, _| alive.contains(id));
        // 主視窗不顯示拖出去的分頁：目前分頁被拖出去了（或開檔時切到它），把它的視窗帶到前面，
        // 主視窗改顯示最近的另一個分頁；全部都拖出去了就開一個新的空白分頁
        if let Some(cur) = self.tabs.get(self.active_tab_idx).map(|b| b.id).filter(|id| self.detached.contains(id)) {
            ctx.send_viewport_cmd_to(egui::ViewportId::from_hash_of(("alexcode-detached", cur)), egui::ViewportCommand::Focus);
            let at = self.active_tab_idx;
            let next = (0..self.tabs.len()).filter(|&i| !self.detached.contains(&self.tabs[i].id)).min_by_key(|&i| i.abs_diff(at));
            match next {
                Some(i) => self.active_tab_idx = i,
                None => self.new_tab(),
            }
        }
        let mut closed = Vec::new();
        for id in self.detached.clone() {
            let Some(idx) = self.tabs.iter().position(|b| b.id == id) else { continue };
            // 獨立視窗的 Git 標記：內容或 HEAD 變了才重算
            {
                let b = &self.tabs[idx];
                let head = self.git_head.get(&id).and_then(|h| h.as_ref());
                let key = (b.revision(), head.map_or(0, |h| Arc::as_ptr(h) as usize));
                let stale = self.detached_git.get(&id).is_none_or(|(k, _)| *k != key);
                if stale {
                    let marks = head.map(|h| diff_line_states(h, &b.lines, 800)).unwrap_or_default();
                    self.detached_git.insert(id, (key, marks));
                }
            }
            // 先拿出來，畫的時候才能同時改分頁（畫完放回去）
            let cache = self.detached_git.remove(&id);
            let empty = HashMap::new();
            let git = cache.as_ref().map_or(&empty, |(_, m)| m);
            let title = format!("{} — AlexCode", Self::tab_title(&self.tabs[idx]));
            let vid = egui::ViewportId::from_hash_of(("alexcode-detached", id));
            // 上次結束時的位置大小（還原的視窗）；這裡傳的內容每一幀都要一樣，不然 egui 會一直把視窗移回去
            let place = self.detached_place.get(&id).copied();
            let mut builder = egui::ViewportBuilder::default().with_title(title.clone()).with_inner_size([800.0, 600.0]);
            if let Some(r) = place {
                builder = builder.with_position(r.min).with_inner_size(r.size());
            }
            let this = &mut *self;
            // 不能開作業系統視窗時（例如無視窗測試）自己畫成主視窗裡的浮動視窗：egui 0.34 起會自動包一個視窗，
            // 但那個視窗沒有關閉鈕、也不能指定位置，所以這種情況不交給 show_viewport_immediate
            let (close, rect) = if ctx.embed_viewports() {
                {
                    let mut open = true;
                    let mut win = egui::Window::new(title.clone())
                        .id(egui::Id::unique(("alexcode-detached-window", id)))
                        .open(&mut open)
                        // egui 0.28 起視窗打開時會淡入：獨立視窗一開就要完整顯示（與 0.27 相同）
                        .fade_in(false)
                        .default_size([600.0, 400.0]);
                    if let Some(r) = place {
                        win = win.default_pos(r.min);
                    }
                    let shown = win.show(ctx, |ui| {
                        // 和主視窗一樣的選單；指令作用在這個視窗的分頁（指令可能關掉它，關掉了就不畫內容）
                        this.menu_bar_for(ui, c, Some(id));
                        if this.tabs.get(idx).is_some_and(|b| b.id == id) {
                            // 內容放在自己的區塊裡：內容接點擊的範圍是它所在區塊的大小，不包在裡面會蓋住上面的選單
                            ui.scope(|ui| this.detached_body(ui, idx, c, pal, git));
                        }
                    });
                    // 內嵌的浮動視窗：記整個視窗的左上角（大小固定用預設的）
                    (!open, shown.map(|s| Rect::from_min_size(s.response.rect.min, egui::vec2(600.0, 400.0))))
                }
            } else {
                ctx.show_viewport_immediate(vid, builder, |ui, _class| {
                    this.menu_bar_for(ui, c, Some(id));
                    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(c.panel)).show(ui, |ui| {
                        if this.tabs.get(idx).is_some_and(|b| b.id == id) {
                            this.detached_body(ui, idx, c, pal, git);
                        }
                    });
                    // 作業系統視窗：外框左上角 + 內容大小（與 with_position / with_inner_size 對應）
                    let (outer, inner) = ui.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));
                    let rect = outer.zip(inner).map(|(o, i)| Rect::from_min_size(o.min, i.size()));
                    (ui.input(|i| i.viewport().close_requested()), rect)
                })
            };
            if let Some(r) = rect {
                self.detached_rects.insert(id, r);
            }
            if let Some(cache) = cache {
                self.detached_git.insert(id, cache);
            }
            if close {
                closed.push(id);
            }
        }
        self.detached.retain(|id| !closed.contains(id));
        // 關掉獨立視窗：分頁回到主視窗的分頁列，並切到它
        if let Some(i) = closed.last().and_then(|id| self.tabs.iter().position(|b| b.id == *id)) {
            self.active_tab_idx = i;
        }
        let open = self.detached.clone();
        self.detached_rects.retain(|id, _| open.contains(id));
        self.detached_place.retain(|id, _| open.contains(id));
    }

    /// 獨立視窗的內容：畫法與分割檢視的另一欄相同，點一下取得鍵盤焦點並放游標，
    /// 之後打字、Enter、Backspace / Delete、方向鍵、Home / End、Ctrl+Z / Ctrl+Y 直接改那個分頁的緩衝區（與主視窗共用）
    fn detached_body(&mut self, ui: &mut egui::Ui, idx: usize, c: &Colors, pal: &crate::theme::Palette, git: &HashMap<usize, u8>) {
        let Some(tab_id) = self.tabs.get(idx).map(|b| b.id) else { return };
        let (rows, _) = self.split_preview(ui, idx, c, pal, None, git);
        let area = ui.min_rect();
        // 右邊與下面留給捲軸
        let hit = Rect::from_min_max(area.min, (area.max - egui::vec2(14.0, 14.0)).max(area.min));
        let id = egui::Id::unique(("alexcode-detached-edit", tab_id));
        let resp = ui.interact(hit, id, Sense::click_and_drag());
        let before = (self.tabs[idx].revision(), self.tabs[idx].selections.first().copied());
        // 指標位置換成 (行, 欄)：在最後一行下面就算最後一行
        let pos_at = |p: Pos2| {
            rows.iter().rev().find(|(_, at, _)| at.y <= p.y).or(rows.first()).map(|(ln, at, g)| Position {
                line: *ln,
                col: g.cursor_from_pos(p - *at).index.0,
            })
        };
        let pressed = resp.hovered() && ui.input(|i| i.pointer.primary_pressed());
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if pressed || resp.clicked() {
            resp.request_focus();
            if let Some(pos) = pointer.and_then(pos_at) {
                let buf = &mut self.tabs[idx];
                // Shift+點擊延伸選取，否則從這裡重新開始
                let anchor = match buf.selections.first() {
                    Some(s) if ui.input(|i| i.modifiers.shift) => s.anchor,
                    _ => pos,
                };
                buf.selections = vec![Selection { anchor, head: pos }];
            }
        } else if resp.dragged() {
            if let (Some(pos), Some(s)) = (pointer.and_then(pos_at), self.tabs[idx].selections.first_mut()) {
                s.head = pos;
            }
        }
        if ui.memory(|m| m.has_focus(id)) {
            self.detached_keys(ui, idx, id);
        }
        // 同一個分頁也是主視窗目前的分頁：主視窗編輯區的游標跟上（不搶焦點）
        let after = (self.tabs[idx].revision(), self.tabs[idx].selections.first().copied());
        if after != before && idx == self.active_tab_idx {
            if let Some(s) = after.1 {
                let buf = &self.tabs[idx];
                let (a, b) = (Self::char_index_of(buf, s.anchor.line, s.anchor.col), Self::char_index_of(buf, s.head.line, s.head.col));
                self.pending_select = Some((a, b));
                self.select_keeps_focus = true;
            }
        }
    }

    /// 獨立視窗有鍵盤焦點時的按鍵：編輯、移動游標（按著 Shift 時延伸選取）、剪貼簿、復原
    fn detached_keys(&mut self, ui: &mut egui::Ui, idx: usize, id: egui::Id) {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(id, egui::EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: true, escape: false })
        });
        let events = ui.input(|i| i.events.clone());
        let buf = &mut self.tabs[idx];
        for e in events {
            let Some(sel) = buf.selections.first().copied() else { break };
            let (a, h) = (sel.anchor, sel.head);
            let n = buf.lines.len();
            let len = |b: &EditorBuffer, l: usize| b.line_len(l);
            let at = |line, col| Position { line, col };
            let shift = matches!(e, egui::Event::Key { modifiers, .. } if modifiers.shift);
            // 移動游標；按著 Shift 時起點不動、延伸選取
            let go = |b: &mut EditorBuffer, p: Position| b.selections = vec![Selection { anchor: if shift { a } else { p }, head: p }];
            match e {
                egui::Event::Text(t) if !t.is_empty() => buf.replace_range(a, h, &t, true),
                egui::Event::Copy if a != h => ui.ctx().copy_text(Self::selection_text(buf, sel)),
                egui::Event::Cut if a != h => {
                    ui.ctx().copy_text(Self::selection_text(buf, sel));
                    buf.replace_range(a, h, "", false);
                }
                egui::Event::Paste(t) if !t.is_empty() => buf.replace_range(a, h, &t, false),
                egui::Event::Key { key, pressed: true, modifiers, .. } => match key {
                    Key::Z if modifiers.command && modifiers.shift => {
                        buf.redo();
                    }
                    Key::Z if modifiers.command => {
                        buf.undo();
                    }
                    Key::Y if modifiers.command => {
                        buf.redo();
                    }
                    Key::Enter => buf.replace_range(a, h, "\n", true),
                    Key::Backspace if a != h => buf.replace_range(a, h, "", true),
                    Key::Backspace if h.col > 0 => buf.replace_range(at(h.line, h.col - 1), h, "", true),
                    Key::Backspace if h.line > 0 => buf.replace_range(at(h.line - 1, len(buf, h.line - 1)), h, "", true),
                    Key::Delete if a != h => buf.replace_range(a, h, "", true),
                    Key::Delete if h.col < len(buf, h.line) => buf.replace_range(h, at(h.line, h.col + 1), "", true),
                    Key::Delete if h.line + 1 < n => buf.replace_range(h, at(h.line + 1, 0), "", true),
                    Key::ArrowLeft if h.col > 0 => go(buf, at(h.line, h.col - 1)),
                    Key::ArrowLeft if h.line > 0 => go(buf, at(h.line - 1, len(buf, h.line - 1))),
                    Key::ArrowRight if h.col < len(buf, h.line) => go(buf, at(h.line, h.col + 1)),
                    Key::ArrowRight if h.line + 1 < n => go(buf, at(h.line + 1, 0)),
                    Key::ArrowUp if h.line > 0 => go(buf, at(h.line - 1, h.col.min(len(buf, h.line - 1)))),
                    Key::ArrowDown if h.line + 1 < n => go(buf, at(h.line + 1, h.col.min(len(buf, h.line + 1)))),
                    Key::Home => go(buf, at(h.line, 0)),
                    Key::End => go(buf, at(h.line, len(buf, h.line))),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    /// 上一幀分割檢視中間分隔線的 x
    #[cfg(test)]
    fn split_divider_x(&self) -> Option<f32> {
        self.split_mid.filter(|_| self.split.is_some())
    }

    /// 上一幀算出的矩形選取：每一行的（行, 起欄, 迄欄）
    #[cfg(test)]
    fn box_ranges(&self) -> Vec<(usize, usize, usize)> {
        self.box_cols.clone()
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
        // Shift+Alt+方向鍵只用在一般編輯區（大檔案檢視沒有矩形選取）
        let box_keys = std::mem::take(&mut self.box_keys);
        if idx >= self.tabs.len() {
            return;
        }
        if self.uses_large_view(idx) {
            self.large_view(ui, c, pal);
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
        let editor_id = egui::Id::unique(("alexcode-editor", idx, path_key));

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
        // 接受補全選單的一項：選取游標前已經打的那個字，再以輸入事件換成那一項（TextEdit 照一般輸入處理，一次復原）
        let mut accepted = false;
        if let Some(i) = self.comp_accept.take() {
            let head = self.tabs[idx].selections[0].head;
            let menu = self.comp.take().filter(|m| m.tab == self.tabs[idx].id && m.line == head.line && head.col >= m.start);
            let state = egui::text_edit::TextEditState::load(ui.ctx(), editor_id);
            if let (Some(m), Some(mut st)) = (menu, state) {
                let back = head.col - m.start;
                if let (Some(item), Some(r)) = (m.items.get(i), st.cursor.char_range().filter(|r| r.primary.index.0 >= back)) {
                    let ci = r.primary.index.0;
                    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(ci - back), CCursor::new(ci))));
                    st.store(ui.ctx(), editor_id);
                    let text = item.insert.clone();
                    ui.input_mut(|inp| inp.events.push(egui::Event::Text(text)));
                    accepted = true;
                }
            }
        }
        // 這一幀剛好打了一個字元（決定要不要問補全）：要在自動配對改寫事件之前看
        let typed: Option<char> = if accepted {
            None
        } else {
            ui.input(|i| {
                let mut texts = i.events.iter().filter_map(|e| match e {
                    egui::Event::Text(t) => Some(t),
                    _ => None,
                });
                match (texts.next(), texts.next()) {
                    (Some(t), None) if t.chars().count() == 1 => t.chars().next(),
                    _ => None,
                }
            })
        };
        let mut caret: Option<Rect> = None;
        // 矩形選取：Ctrl+C 複製區塊（每一行選到的部分以換行連接）、Ctrl+X 剪下；打字、Backspace、Delete、
        // 貼上（行數相同或只有一行時）每一行同時修改。其他按鍵、行數不合的貼上、沒按 Alt 的點擊都取消它
        // （之後照一般輸入處理）。內容變了、換了分頁、開了自動換行也取消
        if let Some(b) = &self.box_sel {
            let mut cancel = b.tab != self.tabs[idx].id
                || b.rev != self.tabs[idx].revision()
                || self.config.word_wrap
                || self.box_cols.is_empty();
            let rows = self.box_cols.len();
            let mut copy = false;
            // 打字、Backspace、Delete：每一行同時修改（照事件順序）
            let mut edits: Vec<BoxEdit> = Vec::new();
            ui.input_mut(|i| {
                i.events.retain(|e| match e {
                    egui::Event::Copy => {
                        copy = true;
                        false
                    }
                    egui::Event::Text(t) if !cancel && !t.is_empty() && !t.contains(['\n', '\r']) => {
                        edits.push(BoxEdit::Text(t.clone()));
                        false
                    }
                    egui::Event::Key { key: Key::Backspace | Key::Delete, pressed: true, modifiers, .. }
                        if !cancel && !modifiers.command && !modifiers.alt =>
                    {
                        if let egui::Event::Key { key, .. } = e {
                            edits.push(if *key == Key::Backspace { BoxEdit::Backspace } else { BoxEdit::Delete });
                        }
                        false
                    }
                    // 剪下：先複製區塊再刪掉選到的部分
                    egui::Event::Cut if !cancel => {
                        copy = true;
                        edits.push(BoxEdit::Cut);
                        false
                    }
                    // 貼上：剪貼簿的行數與區塊相同時逐行貼，只有一行時每一行都貼它；其他情況取消矩形選取、照一般貼上
                    egui::Event::Paste(s) if !cancel => {
                        let s = s.replace("\r\n", "\n").replace('\r', "\n");
                        let mut v: Vec<String> = s.split('\n').map(str::to_string).collect();
                        if v.len() > 1 && v.last().is_some_and(|l| l.is_empty()) {
                            v.pop(); // 從別處複製的多行文字常以換行結尾
                        }
                        if v.len() == rows {
                            edits.push(BoxEdit::Lines(v));
                            false
                        } else if v.len() == 1 {
                            edits.push(BoxEdit::Lines(vec![v[0].clone(); rows]));
                            false
                        } else {
                            cancel = true;
                            true
                        }
                    }
                    egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut => {
                        cancel = true;
                        true
                    }
                    // 輸入法開始組字（egui 0.35 起不再送 Ime(Enabled)，第一個不是空的組字文字就是開始）
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) if !text.is_empty() => {
                        cancel = true;
                        true
                    }
                    egui::Event::Key { pressed: true, modifiers, .. } if !modifiers.command => {
                        cancel = true;
                        true
                    }
                    egui::Event::PointerButton { pressed: true, modifiers, .. } if !modifiers.alt => {
                        cancel = true;
                        true
                    }
                    _ => true,
                })
            });
            if copy && !self.box_cols.is_empty() {
                let lines = &self.tabs[idx].lines;
                let block: Vec<String> = self
                    .box_cols
                    .iter()
                    .map(|&(l, a, b)| lines.get(l).map_or(String::new(), |s| s.chars().skip(a).take(b - a).collect()))
                    .collect();
                ui.ctx().copy_text(block.join("\n"));
            }
            if !edits.is_empty() && !self.box_cols.is_empty() {
                let mut cols = self.box_cols.clone();
                for e in &edits {
                    // 區塊有寬度時 Backspace / Delete 刪掉選到的部分；零寬時刪游標前 / 後一個字
                    let wide = cols.iter().any(|&(_, a, b)| b > a);
                    let none = vec![String::new(); cols.len()];
                    let (ranges, texts): (Vec<(usize, usize, usize)>, Vec<String>) = match e {
                        BoxEdit::Text(t) => (cols.clone(), vec![t.clone(); cols.len()]),
                        BoxEdit::Lines(v) => (cols.clone(), v.clone()),
                        BoxEdit::Cut => (cols.clone(), none),
                        _ if wide => (cols.clone(), none),
                        BoxEdit::Backspace => (cols.iter().map(|&(l, a, _)| (l, a.saturating_sub(1), a)).collect(), none),
                        BoxEdit::Delete => (cols.iter().map(|&(l, a, _)| (l, a, a + 1)).collect(), none),
                    };
                    let lens: Vec<usize> = ranges.iter().map(|r| self.tabs[idx].line_len(r.0)).collect();
                    // 打字與刪字照一般輸入合併復原；貼上、剪下各自一步
                    let typing = !matches!(e, BoxEdit::Lines(_) | BoxEdit::Cut);
                    self.tabs[idx].replace_columns_each(&ranges, &texts, typing);
                    cols = ranges
                        .iter()
                        .zip(lens)
                        .zip(&texts)
                        .map(|((&(l, a, _), len), t)| {
                            let c = a.min(len) + t.chars().count();
                            (l, c, c)
                        })
                        .collect();
                }
                let rev = self.tabs[idx].revision();
                // 之後區塊收成零寬：x 取最右邊那一行的新欄（短的行之後照樣收在行尾）
                let right = cols.iter().copied().max_by_key(|&(l, a, _)| (a, std::cmp::Reverse(l)));
                let mut caret_col = None;
                if let Some(b) = self.box_sel.as_mut() {
                    b.rev = rev;
                    b.col = right.map(|(l, a, _)| (l, a));
                    caret_col = cols.iter().find(|c| c.0 == b.b_line).map(|c| (c.0, c.1));
                }
                text = self.tabs[idx].edit_text();
                // 游標留在拖曳終點那一行的新欄
                if let Some((line, col)) = caret_col {
                    let at: usize = self.tabs[idx].lines.iter().take(line).map(|s| s.chars().count() + 1).sum::<usize>() + col;
                    let mut st = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                    st.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at))));
                    st.store(ui.ctx(), editor_id);
                }
            }
            if cancel {
                self.box_sel = None;
                self.box_cols.clear();
                self.box_dragging = false;
            }
        }
        let find_current = if self.show_find_bar {
            self.find_current.filter(|(_, rev)| *rev == self.tabs[idx].revision()).map(|(r, _)| r)
        } else {
            None
        };

        let row_h = ui.fonts_mut(|f| f.row_height(&font));
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
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
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            use std::hash::{Hash, Hasher};
            let s = buf.as_str();
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
            let ppp = ui.ctx().pixels_per_point();
            let atlas: FontAtlas = ui.fonts_mut(|f| (ppp, f.font_image_size()[0]));
            match &hl_cache {
                Some((k, galley, a)) if *k == key && *a == atlas => galley.clone(),
                _ => {
                    let l = if s.len() > HIGHLIGHT_MAX_BYTES { Lang::Plain } else { lang };
                    let hits = find_all(s, &find, &fopts).unwrap_or_default();
                    let mut job = highlight_ranges(s, l, &pal_c, font_c.clone(), &hits);
                    if wrap {
                        job.wrap.max_width = max_width;
                    }
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
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
        // 語言伺服器的診斷：行號區以最嚴重的那一個上色
        let diags = self.tabs[idx].file_path.as_deref().map(|p| self.lsp.diagnostics(p)).unwrap_or_default();
        let mut diag_sev: HashMap<usize, i32> = HashMap::new();
        for d in diags.iter() {
            let line = d.start_line.max(0) as usize;
            let s = diag_sev.entry(line).or_insert(d.severity);
            *s = (*s).min(d.severity);
        }

        let mut new_cursor: Option<(usize, usize)> = None;
        let mut changed = false;
        // Ctrl+點擊：游標移到點的位置之後跳到定義
        let mut ctrl_click = false;
        // 指標停在哪個字元上（行, 字元欄）
        let mut hover_char: Option<(usize, usize)> = None;
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
        let mut scroll_area = if wrap {
            egui::ScrollArea::vertical().id_salt(("editor-scroll-wrap", idx))
        } else {
            egui::ScrollArea::both().id_salt(("editor-scroll", idx))
        };
        // 分割檢視換欄：接著用另一欄的捲動位置（自動換行時排版不同，不接）
        let tab_id = self.tabs[idx].id;
        if self.pending_offset.is_some_and(|(id, _)| id == tab_id) {
            if let Some((_, off)) = self.pending_offset.take().filter(|_| !wrap) {
                scroll_area = scroll_area.scroll_offset(off);
            }
        }
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
                    let box_idx = ui.painter().add(Shape::Noop);

                    let cursor_shift =
                        if pair_enabled && was_focused && !accepted { Self::auto_pair_events(ui, editor_id, &text) } else { 0 };
                    if self.config.insert_spaces && was_focused {
                        Self::tab_to_spaces_events(ui, editor_id, &text, self.config.tab_size);
                    }

                    // 輸入法組字中游標 / 選字的位置（TextEdit 收到同樣的事件；這裡記下來，下面用來放候選字視窗）
                    if ui.memory(|m| m.has_focus(editor_id)) {
                        ui.input(|i| {
                            for e in &i.events {
                                match e {
                                    egui::Event::Ime(egui::ImeEvent::Preedit { text: t, active_range_chars }) => {
                                        let n = t.chars().count();
                                        self.editor_ime_active = (n > 0).then(|| active_range_chars.clone().unwrap_or(n..n));
                                    }
                                    egui::Event::Ime(egui::ImeEvent::Commit(_)) => self.editor_ime_active = None,
                                    _ => {}
                                }
                            }
                        });
                    } else {
                        self.editor_ime_active = None;
                    }
                    let output = egui::TextEdit::multiline(&mut text)
                        .id(editor_id)
                        .font(font.clone())
                        .code_editor()
                        // 沒有外框、文字四周留白（0.34 起自訂外框時留白改由外框的 inner_margin 決定）
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(10, 4)))
                        .desired_width(f32::INFINITY)
                        .desired_rows((min_h / row_h) as usize)
                        .lock_focus(true)
                        .layouter(&mut layouter)
                        .show(ui);

                    changed = output.response.changed();
                    ctrl_click = output.response.clicked() && ui.input(|i| i.modifiers.command);
                    self.editor_focused = output.response.has_focus();
                    if cursor_shift != 0 {
                        // 自動補上右括號後游標留在中間；跳過既有的右括號時游標右移
                        if let Some(mut st) = egui::text_edit::TextEditState::load(ui.ctx(), editor_id) {
                            if let Some(r) = st.cursor.char_range() {
                                let i = (r.primary.index.0 as isize + cursor_shift).max(0) as usize;
                                st.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(i))));
                                st.store(ui.ctx(), editor_id);
                            }
                        }
                    }
                    let galley = &output.galley;
                    let gpos = output.galley_pos;

                    // 組字中：TextEdit 把候選字視窗放在組字文字的最後；改放在輸入法報的游標 / 選到的字那裡
                    if let (Some(act), Some(cr)) = (self.editor_ime_active.clone(), output.cursor_range.filter(|r| !r.is_empty())) {
                        if output.response.has_focus() {
                            let [start, end] = cr.sorted_cursors();
                            let at = (start.index.0 + act.start).min(end.index.0);
                            // 與 TextEdit 算游標框的方式相同（egui 的 cursor_rect：至少一列高、四周各放大 1.5）
                            let mut r = galley.pos_from_cursor(CCursor::new(at));
                            r.max.y = r.max.y.max(r.min.y + row_h);
                            let r = r.expand(1.5).translate(gpos.to_vec2());
                            let to_global = ui.ctx().layer_transform_to_global(ui.layer_id()).unwrap_or_default();
                            ui.ctx().output_mut(|o| {
                                if let Some(ime) = o.ime.as_mut() {
                                    ime.cursor_rect = to_global * r;
                                }
                            });
                        }
                    }

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
                            para_of(galley, cr.primary.index.0)
                        } else {
                            rc_of(galley, cr.primary)
                        }
                    });
                    if let Some((r, col)) = cur_row {
                        new_cursor = Some((r, col));
                    }
                    caret = output.cursor_range.map(|cr| galley.pos_from_cursor(cr.primary).translate(gpos.to_vec2()));

                    // 矩形選取：按住 Alt 時在文字上面多放一個接拖曳的區域（蓋過 TextEdit 自己的拖曳選取）。
                    // 自動換行時畫面上的列不等於行，不提供
                    if !wrap && (ui.input(|i| i.modifiers.alt) || self.box_dragging) {
                        let resp = ui.interact(output.response.rect, editor_id.with("box-select"), Sense::drag());
                        let line_at = |p: Pos2| rc_of(galley, galley.cursor_from_pos(p - gpos)).0;
                        if resp.drag_started() {
                            if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
                                let line = line_at(p);
                                let (tab, rev) = (self.tabs[idx].id, self.tabs[idx].revision());
                                let x = p.x - gpos.x;
                                self.box_sel = Some(BoxSel { tab, rev, a_line: line, b_line: line, a_x: x, b_x: x, col: None });
                                self.box_dragging = true;
                                ui.memory_mut(|m| m.request_focus(editor_id));
                            }
                        }
                        if self.box_dragging {
                            if let (Some(b), Some(p)) = (self.box_sel.as_mut(), ui.input(|i| i.pointer.interact_pos())) {
                                b.b_line = line_at(p);
                                b.b_x = p.x - gpos.x;
                                // 游標跟著拖曳的終點（TextEdit 自己的選取收成一個點）
                                let cur = galley.cursor_from_pos(p - gpos);
                                let mut st = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                                st.cursor.set_char_range(Some(CCursorRange::one(cur)));
                                st.store(ui.ctx(), editor_id);
                                new_cursor = Some(rc_of(galley, cur));
                            }
                            if !ui.input(|i| i.pointer.primary_down()) {
                                self.box_dragging = false;
                            }
                        }
                    }
                    // 在分割檢視的另一欄按下滑鼠換過來的：那一下按鍵不是按在 TextEdit 上，拖曳選取由這裡接手
                    if let Some(anchor) = self.split_drag {
                        if ui.input(|i| i.pointer.primary_down()) {
                            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                                let cur = galley.cursor_from_pos(p - gpos);
                                let mut st = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                                st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(anchor), cur)));
                                st.store(ui.ctx(), editor_id);
                                new_cursor = Some(rc_of(galley, cur));
                            }
                        } else {
                            // 放開的那一下 egui 算是點在別的元件上，TextEdit 會交出焦點：下一幀要回來
                            self.split_drag = None;
                            self.focus_editor = true;
                        }
                    }
                    // 每一行選取的欄：那一行落在兩個 x 之間的字元（短的行收在行尾）；底色畫在文字下面
                    self.box_cols.clear();
                    if let Some(b) = self.box_sel.as_mut().filter(|_| !wrap) {
                        if let Some((l, col)) = b.col.take() {
                            if let Some(row) = galley.rows.get(l) {
                                let x = row_x(row, col);
                                (b.a_x, b.b_x) = (x, x);
                            }
                        }
                    }
                    // Shift+Alt+方向鍵：沒有矩形選取時從游標開始；上下移動終點所在的行，左右移動終點一個字
                    // （超過那一行的行尾時以一個字寬往右延伸，所以短的行也能往右拉）。游標跟著終點
                    if !wrap && !box_keys.is_empty() {
                        if self.box_sel.is_none() {
                            if let Some(cr) = output.cursor_range {
                                let x = galley.pos_from_cursor(cr.primary).left();
                                let line = rc_of(galley, cr.primary).0;
                                let (tab, rev) = (self.tabs[idx].id, self.tabs[idx].revision());
                                self.box_sel = Some(BoxSel { tab, rev, a_line: line, b_line: line, a_x: x, b_x: x, col: None });
                            }
                        }
                        if let Some(b) = self.box_sel.as_mut() {
                            let last = galley.rows.len().min(line_count).saturating_sub(1);
                            for k in &box_keys {
                                match k {
                                    Key::ArrowUp => b.b_line = b.b_line.saturating_sub(1),
                                    Key::ArrowDown => b.b_line = (b.b_line + 1).min(last),
                                    _ => {
                                        let Some(row) = galley.rows.get(b.b_line) else { continue };
                                        let col = rc_of(galley, galley.cursor_from_pos(egui::vec2(b.b_x, row.rect().center().y))).1;
                                        let len = row.char_count_excluding_newline().0;
                                        let end = row_x(row, len);
                                        b.b_x = if *k == Key::ArrowRight {
                                            if col < len {
                                                row_x(row, col + 1)
                                            } else {
                                                b.b_x.max(end) + char_w
                                            }
                                        } else if b.b_x > end + 0.5 {
                                            (b.b_x - char_w).max(end)
                                        } else {
                                            row_x(row, col.saturating_sub(1))
                                        };
                                    }
                                }
                            }
                            if let Some(row) = galley.rows.get(b.b_line) {
                                let cur = galley.cursor_from_pos(egui::vec2(b.b_x, row.rect().center().y));
                                let mut st = egui::text_edit::TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                                st.cursor.set_char_range(Some(CCursorRange::one(cur)));
                                st.store(ui.ctx(), editor_id);
                                new_cursor = Some(rc_of(galley, cur));
                            }
                        }
                    }
                    if let Some(b) = self.box_sel.as_ref().filter(|_| !wrap) {
                        let (l0, l1) = (b.a_line.min(b.b_line), b.a_line.max(b.b_line));
                        let (x0, x1) = (b.a_x.min(b.b_x), b.a_x.max(b.b_x));
                        let fill = ui.visuals().selection.bg_fill;
                        let mut shapes = Vec::new();
                        for (l, row) in galley.rows.iter().enumerate().take(l1 + 1).skip(l0) {
                            let y = row.rect().center().y;
                            let ca = rc_of(galley, galley.cursor_from_pos(egui::vec2(x0, y))).1;
                            let cb = rc_of(galley, galley.cursor_from_pos(egui::vec2(x1, y))).1;
                            self.box_cols.push((l, ca, cb));
                            let (xa, xb) = (row_x(row, ca), row_x(row, cb));
                            let r = Rect::from_min_max(
                                gpos + egui::vec2(xa, row.rect().top()),
                                gpos + egui::vec2(xb.max(xa + 2.0), row.rect().top() + row_h),
                            );
                            shapes.push(Shape::rect_filled(r, 0.0, fill));
                        }
                        ui.painter().set(box_idx, Shape::Vec(shapes));
                    }
                    // 游標移到固定標頭底下（例如按方向鍵往上）時多捲一點，讓游標所在行露出來
                    if let Some(cr) = output.cursor_range.filter(|_| sticky_h > 0.0 && !scroll_to_cursor) {
                        let r = galley.pos_from_cursor(cr.primary).translate(gpos.to_vec2());
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
                        let y0 = gpos.y + row.rect().top();
                        let y1 = gpos.y + last.rect().top() + last.rect().height().max(row_h);
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
                    ui.fonts_mut(|f| {
                        for (i, row) in galley.rows.iter().enumerate() {
                            let y = gpos.y + row.rect().top();
                            if y + row_h < clip.top() || y > clip.bottom() {
                                continue;
                            }
                            // 自動換行時 i 是畫面上的列、line 是它所屬的行：行號、書籤只畫在每一行的第一列
                            let line = if wrap { row_lines[i] } else { i };
                            let line_start = !wrap || i == 0 || row_lines[i - 1] != line;
                            let active = line == active_line;
                            let col = match diag_sev.get(&line) {
                                Some(&s) => diag_color(s),
                                None if active => c.accent,
                                None => c.faint,
                            };
                            if line_start {
                                let g = f.layout_no_wrap(format!("{}", line + 1), num_font.clone(), col);
                                let pos = Pos2::new(
                                    gutter_rect.right() - 10.0 - g.size().x,
                                    y + (row.rect().height().max(row_h) - g.size().y) * 0.5,
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

                    // 指標停在哪個字元上（滑鼠停留說明用）：要在文字範圍內，不算行尾右邊的空白
                    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|_| ui.rect_contains_pointer(clip));
                    hover_char = hover.filter(|p| p.x >= gutter_rect.right()).and_then(|p| {
                        let rel = p - gpos;
                        let cur = galley.cursor_from_pos(rel);
                        let row = galley.rows.get(rc_of(galley, cur).0)?;
                        let inside = rel.y >= row.rect().top() && rel.y < row.rect().bottom() && rel.x <= row.rect().right();
                        inside.then_some(para_of(galley, cur.index.0))
                    });
                    let hover_note = self
                        .hover_text
                        .as_ref()
                        .filter(|(k, t)| !t.is_empty() && hover_char == Some((k.2, k.3)) && k.0 == self.tabs[idx].id)
                        .map(|(_, t)| t.clone());

                    // 診斷：畫面上的範圍畫波浪底線；滑鼠停在底線上或行號上時列出訊息（與滑鼠停留說明一起顯示）
                    if !diags.is_empty() || hover_note.is_some() {
                        let mut tips: Vec<&Diagnostic> = Vec::new();
                        for d in diags.iter() {
                            for (top, bottom, l, r) in diag_spans(galley, &self.tabs[idx].lines, &row_lines, d, char_w) {
                                let (y0, y1) = (gpos.y + top, gpos.y + bottom);
                                if y0 > clip.bottom() || y1 < clip.top() {
                                    continue;
                                }
                                let (x0, x1) = (gpos.x + l, gpos.x + r);
                                ui.painter().add(squiggle(x0, x1, y1 - 1.0, diag_color(d.severity)));
                                let over = hover.is_some_and(|p| p.x >= x0 - 1.0 && p.x <= x1 + 1.0 && p.y >= y0 && p.y <= y1);
                                if over && !tips.contains(&d) {
                                    tips.push(d);
                                }
                            }
                        }
                        if let Some(p) = hover.filter(|p| p.x >= gutter_rect.left() && p.x < gutter_rect.right()) {
                            let y = p.y - gpos.y;
                            if let Some(i) = galley.rows.iter().position(|r| y >= r.rect().top() && y < r.rect().bottom()) {
                                let line = if wrap { row_lines[i] } else { i };
                                tips = diags.iter().filter(|d| d.start_line.max(0) as usize == line).collect();
                            }
                        }
                        if !tips.is_empty() || hover_note.is_some() {
                            egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), egui::Id::unique("lsp-diagnostic-tip"), egui::PopupAnchor::Pointer).gap(12.0).show(|ui| {
                                ui.set_max_width(560.0);
                                for d in &tips {
                                    ui.horizontal_top(|ui| {
                                        ui.colored_label(diag_color(d.severity), diag_label(d.severity));
                                        let src = if d.source.is_empty() { String::new() } else { format!("（{}）", d.source) };
                                        ui.label(format!("{}{}", d.message, src));
                                    });
                                }
                                if let Some(note) = &hover_note {
                                    if !tips.is_empty() {
                                        ui.separator();
                                    }
                                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                        ui.label(RichText::new(note).monospace());
                                    });
                                }
                            });
                        }
                    }

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
                        let ra = galley.pos_from_cursor(CCursor::new(a));
                        let rb = galley.pos_from_cursor(CCursor::new(b));
                        if (ra.top() - rb.top()).abs() < 0.5 {
                            let r = Rect::from_min_max(ra.left_top(), rb.right_bottom()).translate(gpos.to_vec2());
                            ui.painter().rect_stroke(r.expand(1.0), 2.0, Stroke::new(1.5_f32, c.accent), egui::StrokeKind::Middle);
                        }
                    }

                    if scroll_to_cursor {
                        // 沒有焦點時 output.cursor_range 為 None：改用編輯狀態中記錄的游標
                        let primary = output.cursor_range.map(|cr| cr.primary).or_else(|| {
                            egui::text_edit::TextEditState::load(ui.ctx(), editor_id)
                                .and_then(|st| st.cursor.char_range())
                                .map(|r| r.primary)
                        });
                        if let Some(cur) = primary {
                            let r = galley.pos_from_cursor(cur).translate(gpos.to_vec2());
                            ui.scroll_to_rect(r.expand2(egui::vec2(40.0, row_h * 2.0)), Some(Align::Center));
                        }
                    }
                });
            });

        self.editor_offsets.insert(tab_id, scroll.state.clamped_offset());
        let alive: Vec<u64> = self.tabs.iter().map(|b| b.id).collect();
        if self.editor_offsets.len() > alive.len() {
            self.editor_offsets.retain(|id, _| alive.contains(id));
        }

        // Sticky Scroll：把游標上方已捲出畫面的函式 / 類別標題固定在編輯區頂端
        self.sticky_rows.clear();
        self.sticky_below = 0;
        if !sticky_syms.is_empty() {
            let view = scroll.inner_rect;
            let offset = scroll.state.clamped_offset();
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
                    .interact(row, ui.scope_id().with(("sticky-header", k)), Sense::click())
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
                let galley = ui.fonts_mut(|f| f.layout_job(highlight_ranges(line_text, hl_lang, pal, font.clone(), &[])));
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
        if ctrl_click {
            self.goto_definition();
        }
        self.track_hover(ui.ctx(), idx, hover_char);
        self.update_completion(ui.ctx(), idx, changed, typed);
        // 點選單的那一幀編輯區可能已經失去焦點（沒有游標位置）：沿用上一幀的位置
        let caret = caret.or(self.comp_caret.filter(|_| self.comp.is_some()));
        self.comp_caret = caret;
        match caret {
            Some(r) if ui.clip_rect().intersects(r) => self.completion_popup(ui.ctx(), c, r, &font),
            // 游標捲出畫面（或沒有焦點、沒有游標）時不畫選單
            _ => {}
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
        let row_h = ui.fonts_mut(|f| f.row_height(&font));
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
                let g = ui.fonts_mut(|f| f.layout_job(job));
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
                    cmds.sort_by_key(|c| std::cmp::Reverse(c.0));
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
                                egui::Button::selectable(
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
        self.diff_dialog(ctx, c);
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
                let r = ui.checkbox(&mut self.config.restore_session, "啟動時還原上次開啟的檔案、資料夾與尚未存檔的內容");
                self.mark("settings.restore_session", &r);
                let r = ui.checkbox(&mut self.config.auto_pair, "自動配對括號與引號");
                self.mark("settings.auto_pair", &r);
                let r = ui.checkbox(&mut self.config.sticky_scroll, "固定目前的函式標題 (Sticky Scroll)");
                self.mark("settings.sticky_scroll", &r);

                ui.separator();
                let r = ui.checkbox(&mut self.config.ai.enabled, "遠端 AI 補全（OpenAI 相容端點，按 Ctrl+Space 時詢問）");
                self.mark("settings.ai_enabled", &r);
                ui.add_enabled_ui(self.config.ai.enabled, |ui| {
                    egui::Grid::new("settings-ai").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("端點");
                        let r = ui.add(egui::TextEdit::singleline(&mut self.config.ai.endpoint).desired_width(280.0));
                        self.mark("settings.ai_endpoint", &r);
                        ui.end_row();
                        ui.label("模型");
                        let r = ui.add(egui::TextEdit::singleline(&mut self.config.ai.model).desired_width(280.0));
                        self.mark("settings.ai_model", &r);
                        ui.end_row();
                        ui.label("API 金鑰");
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut self.config.ai.api_key)
                                .password(true)
                                .hint_text("本機伺服器通常不需要")
                                .desired_width(280.0),
                        );
                        self.mark("settings.ai_key", &r);
                        ui.end_row();
                    });
                });
                note(
                    ui,
                    "例如 LM Studio：http://127.0.0.1:1234/v1/chat/completions。游標前後的程式碼會送到這個端點；\
                     金鑰以純文字存在設定資料夾的 state.json。沒有啟用或連不上時只用本機補全。",
                );

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

/// Git 差異檢視：目前分頁的內容與 HEAD 版本並排比較
impl AlexCodeApp {
    /// 目前分頁與 HEAD 的對齊結果；只在內容或 HEAD 內容改變時重算。
    /// 外層 None = 沒有 HEAD 版本可比（不在 Git 內、未追蹤、還在讀取）；內層 None = 差異太大。
    fn diff_rows(&mut self) -> Option<Option<Arc<Vec<DiffRow>>>> {
        let buf = self.tabs.get(self.active_tab_idx)?;
        let head = self.git_head.get(&buf.id)?.clone()?;
        let fresh = self
            .diff_cache
            .as_ref()
            .is_some_and(|c| c.tab_id == buf.id && c.revision == buf.revision() && Arc::ptr_eq(&c.head, &head));
        if !fresh {
            let rows = align_trimmed(&head, &buf.lines, DIFF_MAX_CELLS).map(Arc::new);
            self.diff_cache = Some(DiffCache { tab_id: buf.id, revision: buf.revision(), head, rows });
        }
        self.diff_cache.as_ref().map(|c| c.rows.clone())
    }

    /// 記下一塊區域的位置（只在測試中有作用）
    #[allow(unused_variables)]
    fn mark_rect(&mut self, name: &'static str, rect: Rect) {
        #[cfg(test)]
        self.probe.insert(name, rect);
    }

    fn diff_dialog(&mut self, ctx: &egui::Context, c: &Colors) {
        if !self.show_diff_dialog {
            return;
        }
        let idx = self.active_tab_idx;
        let Some(buf) = self.tabs.get(idx) else { return };
        let (id, title) = (buf.id, Self::tab_title(buf));
        let saved_file = buf.file_path.as_deref().is_some_and(|p| Path::new(p).is_absolute());
        // None = 還沒有結果；Some(None) = 不在 Git 內 / 未追蹤
        let head = self.git_head.get(&id).cloned();
        let rows = self.diff_rows().flatten();
        let starts = rows.as_deref().map(|r| change_starts(r)).unwrap_or_default();
        if self.diff_focus >= starts.len() {
            self.diff_focus = 0;
        }
        let focus_row = starts.get(self.diff_focus).copied();
        let scroll_to = if focus_row.is_some() && std::mem::take(&mut self.diff_scroll_focus) { focus_row } else { None };
        let message = match (&head, &rows) {
            (None, _) if !saved_file => Some("這個分頁還沒有存成檔案，沒有 HEAD 版本可以比較。"),
            (None, _) => Some("讀取 HEAD 版本中…"),
            (Some(None), _) => Some("這個檔案不在 Git 儲存庫內，或尚未加入追蹤。"),
            (Some(Some(_)), None) => Some("與 HEAD 的差異太大，無法並排顯示。"),
            (Some(Some(_)), Some(_)) if starts.is_empty() => Some("與 HEAD 相同，沒有變更。"),
            _ => None,
        };
        let font = FontId::monospace(self.config.font_size);
        let num_font = FontId::monospace((self.config.font_size - 1.0).max(8.0));
        let lines = &self.tabs[idx].lines;

        let mut open = true;
        let mut step: Option<bool> = None; // Some(true) = 下一段、Some(false) = 上一段
        let mut jump: Option<usize> = None;
        let mut marks: Vec<(&'static str, Rect)> = Vec::new();
        egui::Window::new(RichText::new("DIFF · HEAD").font(FontId::new(12.0, FontFamily::Name(ORBITRON.into()))).color(c.accent))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([920.0, 560.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&title).color(c.accent));
                    if !starts.is_empty() {
                        ui.label(RichText::new(format!("{} 段變更", starts.len())).color(c.fg_dim));
                        if ui.button("上一個").clicked() {
                            step = Some(false);
                        }
                        let r = ui.button("下一個");
                        marks.push(("diff.next", r.rect));
                        if r.clicked() {
                            step = Some(true);
                        }
                        ui.label(RichText::new("點右側的一列可跳到該行").size(11.5).color(c.fg_dim2));
                    }
                });
                ui.separator();
                if let Some(text) = message {
                    ui.label(RichText::new(text).color(c.fg_dim));
                }
                let (Some(rows), Some(Some(head))) = (&rows, &head) else { return };
                if starts.is_empty() {
                    return;
                }
                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("HEAD（最後一次提交）").size(11.5).color(c.fg_dim2));
                    cols[1].label(RichText::new("目前內容（含尚未存檔的修改）").size(11.5).color(c.fg_dim2));
                });

                let row_h = ui.fonts_mut(|f| f.row_height(&font));
                let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
                let num_w = head.len().max(lines.len()).to_string().len().max(3) as f32 * char_w + 16.0;
                ui.spacing_mut().item_spacing.y = 0.0; // 每一列緊貼：捲動位置才能用「列號 × 列高」計算
                let mut area = egui::ScrollArea::vertical().id_salt("diff-scroll").auto_shrink([false, false]);
                if let Some(row) = scroll_to {
                    // 選取的那段變更上方留三列；內容比視窗短時不捲動
                    let max_offset = (rows.len() as f32 * row_h - ui.available_height()).max(0.0);
                    area = area.vertical_scroll_offset(((row as f32 - 3.0).max(0.0) * row_h).min(max_offset));
                }
                area.show_rows(ui, row_h, rows.len(), |ui, range| {
                    for i in range {
                        let row = &rows[i];
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), Sense::click());
                        let mid = rect.center().x;
                        let left = Rect::from_min_max(rect.min, Pos2::new(mid, rect.bottom()));
                        let right = Rect::from_min_max(Pos2::new(mid, rect.top()), rect.max);
                        // 左：刪除（洋紅）/ 修改（黃）；右：新增（強調色）/ 修改（黃）；沒有對應行的一側留空
                        let (left_bg, right_bg) = match row.row_type {
                            DiffRowType::Same => (None, None),
                            DiffRowType::Removed => (Some(with_alpha(c.accent2, 50)), Some(c.panel)),
                            DiffRowType::Added => (Some(c.panel), Some(with_alpha(c.accent, 50))),
                            DiffRowType::Modified => (Some(with_alpha(c.warn, 50)), Some(with_alpha(c.warn, 50))),
                        };
                        let p = ui.painter();
                        if let Some(bg) = left_bg {
                            p.rect_filled(left, 0.0, bg);
                        }
                        if let Some(bg) = right_bg {
                            p.rect_filled(right, 0.0, bg);
                        }
                        let mut clipped = false;
                        for (side, line_idx, source) in [(left, row.left, head.as_slice()), (right, row.right, lines.as_slice())] {
                            let Some(text) = usize::try_from(line_idx).ok().and_then(|l| source.get(l)) else { continue };
                            p.text(
                                Pos2::new(side.left() + num_w - 8.0, side.top()),
                                Align2::RIGHT_TOP,
                                line_idx + 1,
                                num_font.clone(),
                                c.faint,
                            );
                            let area = Rect::from_min_max(Pos2::new(side.left() + num_w, side.top()), side.max);
                            clipped |= text.chars().count() as f32 * char_w > area.width();
                            p.with_clip_rect(area.intersect(ui.clip_rect())).text(
                                area.min + egui::vec2(2.0, 0.0),
                                Align2::LEFT_TOP,
                                text,
                                font.clone(),
                                c.fg,
                            );
                        }
                        p.vline(mid, rect.y_range(), Stroke::new(1.0_f32, c.border));
                        if Some(i) == focus_row {
                            p.rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.0_f32, c.accent), egui::StrokeKind::Middle);
                            marks.push(("diff.focus_row", rect));
                        }
                        if resp.clicked() {
                            jump = usize::try_from(row.right).ok();
                        }
                        // 太長而被截掉的行：滑鼠停留時顯示兩邊的全文
                        if clipped {
                            resp.on_hover_ui(|ui| {
                                for (label, line_idx, source) in [("HEAD", row.left, head.as_slice()), ("目前", row.right, lines.as_slice())] {
                                    if let Some(text) = usize::try_from(line_idx).ok().and_then(|l| source.get(l)) {
                                        ui.label(RichText::new(format!("{label}：{text}")).monospace());
                                    }
                                }
                            });
                        }
                    }
                });
            });

        for (name, rect) in marks {
            self.mark_rect(name, rect);
        }
        if let Some(forward) = step.filter(|_| !starts.is_empty()) {
            let n = starts.len();
            self.diff_focus = if forward { (self.diff_focus + 1) % n } else { (self.diff_focus + n - 1) % n };
            self.diff_scroll_focus = true;
        }
        if let Some(line) = jump {
            let i = Self::char_index_of(&self.tabs[idx], line, 0);
            self.pending_select = Some((i, i));
            self.scroll_to_cursor = true;
        }
        if !open {
            self.show_diff_dialog = false;
        }
    }
}

/// Git blame：狀態列顯示游標所在行最後一次修改的提交
impl AlexCodeApp {
    /// 游標所在行的 blame 文字；沒有結果、或結果是針對舊內容算的（行號可能已經對不上）時回傳 None
    fn blame_text(&self) -> Option<String> {
        let buf = self.active_buffer()?;
        let (rev, lines) = self.blame.get(&buf.id)?;
        if *rev != buf.revision() {
            return None;
        }
        let info = lines.as_ref()?.get(&buf.selections.first()?.head.line)?;
        // `status_text` 開頭的「⎇」在內建字型中沒有字形（會畫成方框）：狀態列不顯示它
        let text = blame_status_text(info);
        Some(text.trim_start_matches('⎇').trim_start().to_string()).filter(|s| !s.is_empty())
    }

    /// 收取背景 blame 的結果；目前分頁的內容變了（且已停止輸入一會兒）就重新執行
    fn update_blame(&mut self, ctx: &egui::Context) {
        let results: Vec<BlameResult> = self.blame_chan.1.try_iter().collect();
        for (id, rev, lines) in results {
            if self.blame_running == Some((id, rev)) {
                self.blame_running = None;
            }
            if self.tabs.iter().any(|b| b.id == id) {
                self.blame.insert(id, (rev, lines.map(Arc::new)));
            }
        }

        let idx = self.active_tab_idx;
        if self.is_loading(idx) {
            return;
        }
        let Some(buf) = self.tabs.get(idx) else { return };
        let Some(path) = buf.file_path.as_deref().map(PathBuf::from).filter(|p| p.is_absolute()) else { return };
        let key = (buf.id, buf.revision());
        if (self.blame_seen.0, self.blame_seen.1) != key {
            self.blame_seen = (key.0, key.1, std::time::Instant::now());
        }
        if self.blame.get(&key.0).is_some_and(|b| b.0 == key.1) || self.blame_running.is_some() {
            return; // 已經是最新的，或上一個 git 還沒跑完
        }
        // 剛開啟的檔案立刻執行；編輯中的檔案等輸入停頓後再執行
        if self.blame.contains_key(&key.0) {
            let idle = self.blame_seen.2.elapsed();
            if idle < BLAME_DELAY {
                ctx.request_repaint_after(BLAME_DELAY - idle);
                return;
            }
        }
        // blame 的對象是分頁目前的內容（可能尚未存檔），以檔案的編碼送給 git。
        // 太大的檔案、無法以該編碼表示的內容不做 blame。
        let bytes: usize = buf.lines.iter().map(|l| l.len() + 1).sum();
        let contents = if bytes > HIGHLIGHT_MAX_BYTES { None } else { buf.encoding.encode(&buf.text()).ok() };
        let Some(contents) = contents else {
            self.blame.insert(key.0, (key.1, None));
            return;
        };
        let (tx, repaint) = (self.blame_chan.0.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send((key.0, key.1, blame_contents(&path, &contents)));
            repaint.request_repaint();
        });
        self.blame_running = Some(key);
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
        egui::Area::new(egui::Id::unique("unsaved-dim"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let r = ctx.content_rect();
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
        egui::Area::new(egui::Id::unique("unsaved-dialog"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.global_style()).show(ui, |ui| {
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
                    let saved = self.tabs.iter().position(|b| b.id == *id).is_some_and(|i| self.save_tab(i, false));
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

impl AlexCodeApp {
    /// 這個分頁是否以大檔案檢視顯示（順便更新它的大小統計）
    fn uses_large_view(&mut self, idx: usize) -> bool {
        let Some(buf) = self.tabs.get(idx) else { return false };
        let key = (buf.id, buf.revision());
        if self.large_stats.0 != key {
            let bytes = buf.lines.iter().map(|l| l.len() + 1).sum::<usize>();
            let large = bytes >= self.large_view_bytes;
            let cols = if large { buf.lines.iter().map(|l| display_cols(l)).max().unwrap_or(0) } else { 0 };
            self.large_stats = (key, (bytes, cols));
        }
        self.large_stats.1 .0 >= self.large_view_bytes
    }

    /// 這個 Markdown 分頁是否大到不該用閱讀 / 雙欄模式（使用者按了「仍要使用閱讀模式」的分頁除外）
    fn md_too_big(&mut self, idx: usize) -> bool {
        self.uses_large_view(idx); // 更新 `large_stats`
        let Some(buf) = self.tabs.get(idx) else { return false };
        self.large_stats.0 .0 == buf.id && self.large_stats.1 .0 >= self.md_preview_bytes && !self.md_big_preview.contains(&buf.id)
    }

    /// 很大的 Markdown 檔以原始碼顯示時的說明列：為什麼，以及怎麼換回閱讀模式
    fn md_big_bar(&mut self, root: &mut egui::Ui, c: &Colors) {
        let ctx = &root.ctx().clone();
        let Some(id) = self.tabs.get(self.active_tab_idx).map(|b| b.id) else { return };
        let size = self.large_stats.1 .0 as f64 / (1024.0 * 1024.0);
        let mut preview = false;
        egui::Panel::top("md_big_bar")
            .frame(egui::Frame::NONE.fill(c.panel).inner_margin(egui::Margin::symmetric(10, 4)))
            .show_separator_line(false)
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "這個 Markdown 檔很大（{size:.1} MB）：以原始碼顯示。閱讀 / 雙欄模式每一幀都要排版整份文件，會拖慢畫面"
                        ))
                        .size(12.0)
                        .color(c.fg_dim),
                    );
                    let need = (size * MD_PREVIEW_MEM_FACTOR).max(1.0);
                    let resp = ui.button(RichText::new(format!("仍要使用閱讀模式（約需 {need:.0} MB 記憶體）")).size(12.0));
                    preview = resp.clicked();
                    self.mark("md-big-preview", &resp);
                });
            });
        if preview {
            self.md_big_preview.insert(id);
            self.md_view.mode = MdViewMode::Typora;
            ctx.request_repaint();
        }
    }

    /// TextEdit 字元索引 → (行, 列)（`char_index_of` 的反向）
    fn position_of(buf: &EditorBuffer, mut index: usize) -> Position {
        for (line, l) in buf.lines.iter().enumerate() {
            let n = l.chars().count();
            if index <= n {
                return Position { line, col: index };
            }
            index -= n + 1;
        }
        let line = buf.lines.len().saturating_sub(1);
        Position { line, col: buf.line_len(line) }
    }

    /// 選取範圍內的文字（行與行之間以 `\n` 連接）
    fn selection_text(buf: &EditorBuffer, sel: Selection) -> String {
        let (a, b) = (sel.start(), sel.end());
        let piece = |line: usize, from: usize, to: Option<usize>| -> String {
            let l = buf.lines.get(line).map_or("", |s| s.as_str());
            match to {
                Some(to) => l.chars().skip(from).take(to.saturating_sub(from)).collect(),
                None => l.chars().skip(from).collect(),
            }
        };
        if a.line == b.line {
            return piece(a.line, a.col, Some(b.col));
        }
        let mut out = piece(a.line, a.col, None);
        for l in &buf.lines[a.line + 1..b.line] {
            out.push('\n');
            out.push_str(l);
        }
        out.push('\n');
        out.push_str(&piece(b.line, 0, Some(b.col)));
        out
    }

    /// 大檔案檢視：只排版畫面上看得到的行，記憶體用量與檔案大小同一個量級。
    /// 可以捲動、移動游標、選取、複製、尋找、跳至行、設書籤，也可以直接輸入、刪除、貼上（含輸入法），
    /// 有自動配對與 Git 行號區標記；設定開啟時自動換行（只折畫面上的行）。
    fn large_view(&mut self, ui: &mut egui::Ui, c: &Colors, pal: &crate::theme::Palette) {
        use egui::Event;
        let idx = self.active_tab_idx;
        let tab_id = self.tabs[idx].id;
        let mut max_cols = self.large_stats.1 .1;
        let view_id = egui::Id::unique(("alexcode-large-view", tab_id));
        let font = FontId::monospace(self.config.font_size);
        let num_font = FontId::monospace((self.config.font_size - 1.0).max(8.0));
        let row_h = ui.fonts_mut(|f| f.row_height(&font));
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        // 排版時每個字元往右移的距離：egui 0.33 起以次像素累加字寬（0.32 以前每個字都對齊到像素，
        // 例如 7.70 → 8.0），所以就是字寬本身；不換行時的總寬度與只排一段的長行的位置都要用它算
        let adv = char_w;
        // 一般編輯區為其他分頁留下的排版結果已經用不到了
        self.highlight_cache = None;
        self.sticky_rows.clear();
        self.sticky_below = 0;

        let wrap = self.config.word_wrap;

        // 套用待定的游標選取（尋找 / 跳至行 / 行操作）：字元索引換算成 (行, 列)
        if let Some((a, b)) = self.pending_select.take() {
            let buf = &mut self.tabs[idx];
            let (anchor, head) = (Self::position_of(buf, a), Self::position_of(buf, b));
            buf.selections = vec![Selection { anchor, head }];
            if !std::mem::take(&mut self.select_keeps_focus) {
                ui.memory_mut(|m| m.request_focus(view_id));
            }
        }
        if std::mem::take(&mut self.focus_editor) {
            ui.memory_mut(|m| m.request_focus(view_id));
        }
        let scroll_to_cursor = std::mem::take(&mut self.scroll_to_cursor);
        let find = if self.show_find_bar { self.find_bar_text.clone() } else { String::new() };
        let fopts = self.find_opts;
        // 文件在一般編輯區裡長到超過門檻（貼上一大段）：鍵盤焦點還在那個已經不顯示的編輯區上，接過來
        let path_key = self.tabs[idx].file_path.clone().unwrap_or_default();
        let editor_id = egui::Id::unique(("alexcode-editor", idx, path_key));
        if ui.memory(|m| m.focused()) == Some(editor_id) {
            ui.memory_mut(|m| m.request_focus(view_id));
        }
        let has_focus = ui.memory(|m| m.has_focus(view_id));
        self.editor_focused = has_focus;

        // ---- 鍵盤：移動游標、選取、複製、輸入 ----
        // 輸入直接改緩衝區裡受影響的那幾行（`replace_range`），不經過整份文字
        let mut sel = self.tabs[idx].selections[0];
        let mut moved = false;
        if has_focus {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    view_id,
                    egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false },
                )
            });
            let page = ((ui.available_height() / row_h) as usize).max(2) - 1;
            let events = ui.input(|i| i.events.clone());
            let (insert_spaces, tab_size) = (self.config.insert_spaces, self.config.tab_size.max(1));
            // 自動配對的條件與一般編輯區相同：設定開啟，且不是純文字 / Markdown
            let pair_enabled = self.config.auto_pair && !matches!(self.tab_lang(idx), Lang::Plain | Lang::Markdown);
            let mut preedit = std::mem::take(&mut self.large_preedit);
            let mut ime_at = self.large_ime_at;
            let wrap_w = if self.config.word_wrap { self.large_wrap_w } else { None };
            let buf = &mut self.tabs[idx];
            let rev = buf.revision();
            // 自動換行時上下鍵以畫面上的「列」為單位：在折成好幾列的行裡先在行內移動，到頭了才換到上 / 下一行
            // 很長的行由 `LongRows` 切好列、一列一列排版，其他行整行交給 egui 折行
            let row_step = |buf: &EditorBuffer, long: &LongRows, p: Position, down: bool, wrap_w: f32| {
                let lay = |line: usize| {
                    let mut job = highlight_ranges(&buf.lines[line], Lang::Plain, pal, font.clone(), &[]);
                    job.wrap.max_width = wrap_w;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                // 長行的第 r 列（不再折行）
                let piece = |line: usize, rows: &[(usize, usize)], r: usize| {
                    let l = &buf.lines[line];
                    let text = &l[rows[r].1..rows.get(r + 1).map_or(l.len(), |n| n.1)];
                    ui.fonts_mut(|f| f.layout_job(highlight_ranges(text, Lang::Plain, pal, font.clone(), &[])))
                };
                // 目前位置的 x、在這一行的第幾列、這一行共幾列
                let (x, row, n) = match long.rows(p.line) {
                    Some(rows) => {
                        let r = row_of(rows, p.col);
                        let g = piece(p.line, rows, r);
                        let at = CCursor { index: (p.col - rows[r].0).into(), prefer_next_row: true };
                        (g.pos_from_cursor(at).left(), r, rows.len())
                    }
                    None => {
                        let g = lay(p.line);
                        let cur = CCursor { index: (p.col).into(), prefer_next_row: true };
                        (g.pos_from_cursor(cur).left(), rc_of(&g, cur).0, g.rows.len())
                    }
                };
                // 目標：同一行的下 / 上一列，到頭了才換到下一行的第一列 / 上一行的最後一列
                let (line, target) = if down && row + 1 < n {
                    (p.line, row + 1)
                } else if down && p.line + 1 < buf.lines.len() {
                    (p.line + 1, 0)
                } else if !down && row > 0 {
                    (p.line, row - 1)
                } else if !down && p.line > 0 {
                    (p.line - 1, usize::MAX)
                } else {
                    return p; // 已經在第一行的第一列 / 最後一行的最後一列
                };
                match long.rows(line) {
                    Some(rows) => {
                        let r = target.min(rows.len() - 1);
                        let g = piece(line, rows, r);
                        let mut c = g.cursor_from_pos(egui::vec2(x, row_h * 0.5)).index.0;
                        // 落在一列的結尾（不是整行的結尾）：那個位置算下一列的開頭，退一格才會留在這一列
                        if r + 1 < rows.len() && c >= rows[r + 1].0 - rows[r].0 {
                            c = c.saturating_sub(1);
                        }
                        Position { line, col: rows[r].0 + c }
                    }
                    None => {
                        let g = lay(line);
                        let y = g.rows.get(target.min(g.rows.len().saturating_sub(1))).map_or(0.0, |r| r.rect().center().y);
                        Position { line, col: wrapped_col_at(&g, egui::vec2(x, y)) }
                    }
                }
            };
            // 只有單行的逐字輸入時，大小統計可以就地更新，不必重新掃過整份文件
            let mut small_edits = true;
            for ev in &events {
                let last_line = buf.lines.len().saturating_sub(1); // 前一個事件可能增減了行數
                // 這個事件要做的修改：(範圍的兩端, 換上的文字, 是否算逐字輸入)
                let mut edit: Option<(Position, Position, String, bool)> = None;
                // 自動配對補上右括號 / 引號之後，游標要退回那一對的中間
                let mut pair_back = false;
                // 選取範圍（或游標）前後緊鄰的字元：自動配對用
                let around = |buf: &EditorBuffer, sel: Selection| {
                    let (a, b) = (sel.start(), sel.end());
                    let ch = |p: Position| buf.lines.get(p.line).and_then(|l| l.chars().nth(p.col));
                    (if a.col > 0 { ch(Position { line: a.line, col: a.col - 1 }) } else { None }, ch(b))
                };
                match ev {
                    Event::Copy if !sel.is_empty() => ui.ctx().copy_text(Self::selection_text(buf, sel)),
                    Event::Cut if !sel.is_empty() => {
                        ui.ctx().copy_text(Self::selection_text(buf, sel));
                        edit = Some((sel.start(), sel.end(), String::new(), false));
                    }
                    Event::Paste(t) if !t.is_empty() => edit = Some((sel.start(), sel.end(), t.clone(), false)),
                    Event::Text(t) if !t.is_empty() && t != "\n" && t != "\r" => {
                        let mut text = t.clone();
                        let mut skip = false;
                        let mut chars = t.chars();
                        // 自動配對只看單獨輸入的一個括號 / 引號（與一般編輯區相同的規則：`pair_decide`）
                        if let (true, Some(ch), None) = (pair_enabled, chars.next(), chars.next()) {
                            if pair_relevant(ch) {
                                let (before, after) = around(buf, sel);
                                let d = pair_decide(ch, before, after, !sel.is_empty());
                                match (d.action, d.open, d.close) {
                                    (PairAction::AutoClose, _, Some(close)) => {
                                        text = format!("{ch}{close}");
                                        pair_back = true;
                                    }
                                    // 右邊已經是同一個符號：不輸入，游標直接跨過去
                                    (PairAction::SkipOver, ..) => skip = true,
                                    (PairAction::Surround, Some(open), Some(close)) => {
                                        text = format!("{open}{}{close}", Self::selection_text(buf, sel));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if skip {
                            sel = Selection::point(Position { line: sel.head.line, col: sel.head.col + 1 });
                            moved = true;
                        } else {
                            edit = Some((sel.start(), sel.end(), text, true));
                        }
                    }
                    // 輸入法：組字中的文字只顯示在游標處（把後面的文字往後推），選定之後才寫進文件。
                    // egui 0.35 起不再送 Ime(Enabled / Disabled)：空的組字文字表示取消組字
                    Event::Ime(egui::ImeEvent::Preedit { text: t, active_range_chars }) => {
                        let n = t.chars().count();
                        ime_at = active_range_chars.as_ref().map_or(n, |r| r.start.min(n));
                        // 開始組字時先刪掉選取的文字（與一般編輯區相同），組字中的文字就排在那個位置
                        if !t.is_empty() && !sel.is_empty() {
                            edit = Some((sel.start(), sel.end(), String::new(), true));
                        }
                        preedit = t.clone();
                    }
                    Event::Ime(egui::ImeEvent::Commit(t)) => {
                        preedit.clear();
                        if !t.is_empty() {
                            edit = Some((sel.start(), sel.end(), t.clone(), true));
                        }
                    }
                    // 組字期間的按鍵屬於輸入法
                    Event::Key { key, pressed: true, modifiers, .. } if !modifiers.alt && preedit.is_empty() => {
                        let mut head = sel.head;
                        let mut all = false;
                        let word = modifiers.command;
                        let handled = match (key, wrap_w) {
                            (Key::ArrowUp | Key::ArrowDown, Some(w)) => {
                                // 長行切好的列：前一個事件可能剛改了內容，版本不同就重切
                                let rows_key = (tab_id, buf.revision(), w.to_bits());
                                if self.large_rows.key != rows_key {
                                    self.large_rows = ui.fonts_mut(|f| {
                                        LongRows::build(rows_key, &buf.lines, w, adv, |ch| advance_of(f, &font, ch))
                                    });
                                }
                                head = row_step(buf, &self.large_rows, head, *key == Key::ArrowDown, w);
                                true
                            }
                            _ => false,
                        };
                        let handled = handled || match key {
                            Key::ArrowUp if wrap_w.is_none() => {
                                head.line = head.line.saturating_sub(1);
                                true
                            }
                            Key::ArrowDown if wrap_w.is_none() => {
                                head.line = (head.line + 1).min(last_line);
                                true
                            }
                            Key::PageUp => {
                                head.line = head.line.saturating_sub(page);
                                true
                            }
                            Key::PageDown => {
                                head.line = (head.line + page).min(last_line);
                                true
                            }
                            Key::ArrowLeft => {
                                head = step_left(buf, head, word);
                                true
                            }
                            Key::ArrowRight => {
                                head = step_right(buf, head, word);
                                true
                            }
                            Key::Home => {
                                if modifiers.command {
                                    head.line = 0;
                                }
                                head.col = 0;
                                true
                            }
                            Key::End => {
                                if modifiers.command {
                                    head.line = last_line;
                                }
                                head.col = buf.line_len(head.line);
                                true
                            }
                            Key::A if modifiers.command => {
                                sel.anchor = Position { line: 0, col: 0 };
                                head = Position { line: last_line, col: buf.line_len(last_line) };
                                all = true;
                                true
                            }
                            Key::Enter => {
                                edit = Some((sel.start(), sel.end(), "\n".to_string(), true));
                                false
                            }
                            Key::Tab if !modifiers.shift => {
                                let text = if insert_spaces {
                                    " ".repeat(tab_size - sel.start().col % tab_size)
                                } else {
                                    "\t".to_string()
                                };
                                edit = Some((sel.start(), sel.end(), text, true));
                                false
                            }
                            // 有選取時刪掉選取的文字；否則刪掉游標前（Backspace）或後（Delete）的一個字元，按住 Ctrl 則是一個字
                            Key::Backspace | Key::Delete => {
                                let (before, after) = around(buf, sel);
                                let (a, b) = if !sel.is_empty() {
                                    (sel.start(), sel.end())
                                } else if *key == Key::Backspace && pair_enabled && modifiers.is_none() && should_delete_pair(before, after) {
                                    // 游標在一對空的括號 / 引號中間：連右邊那個一起刪
                                    let h = sel.head;
                                    (Position { line: h.line, col: h.col - 1 }, Position { line: h.line, col: h.col + 1 })
                                } else if *key == Key::Backspace {
                                    (step_left(buf, sel.head, word), sel.head)
                                } else {
                                    (sel.head, step_right(buf, sel.head, word))
                                };
                                if a != b {
                                    edit = Some((a, b, String::new(), true));
                                }
                                false
                            }
                            _ => false,
                        };
                        if handled {
                            head.col = head.col.min(buf.line_len(head.line));
                            sel.head = head;
                            if !(modifiers.shift || all) {
                                sel.anchor = head;
                            }
                            moved = !all;
                        }
                    }
                    _ => {}
                }
                if let Some((a, b, text, typing)) = edit {
                    small_edits &= typing && a.line == b.line && !text.contains('\n');
                    buf.replace_range(a, b, &text, typing);
                    sel = buf.selections[0];
                    if pair_back {
                        sel = Selection::point(Position { line: sel.head.line, col: sel.head.col.saturating_sub(1) });
                        buf.selections = vec![sel];
                    }
                    moved = true;
                }
            }
            let edited = buf.revision() != rev;
            if edited && small_edits {
                let bytes = buf.lines.iter().map(|l| l.len() + 1).sum::<usize>();
                max_cols = max_cols.max(display_cols(&buf.lines[sel.head.line]));
                self.large_stats = ((tab_id, buf.revision()), (bytes, max_cols));
            }
            self.large_preedit = preedit;
            self.large_ime_at = ime_at;
            if edited && !self.uses_large_view(idx) {
                // 刪到小於門檻了：下一幀起換回一般編輯區，游標與鍵盤焦點一併交給它
                self.sync_cursor_to_editor();
            }
            max_cols = self.large_stats.1 .1;
        }
        // 輸入可能增減了行數
        let line_count = self.tabs[idx].lines.len();
        let last_line = line_count.saturating_sub(1);
        let gutter_w = line_count.to_string().len().max(3) as f32 * char_w + 30.0;

        // Git 行狀態：與一般編輯區共用 `git_marks`，但等內容停止變動 `LARGE_GIT_DELAY` 之後才重算
        //（剛切到這個分頁時立刻算）；等待期間沿用上一次的結果
        let buf_key = (tab_id, self.tabs[idx].revision());
        match self.git_head.get(&tab_id) {
            Some(Some(head)) => {
                if self.git_marks.0 != buf_key {
                    if self.large_marks_seen.0 != buf_key {
                        self.large_marks_seen = (buf_key, std::time::Instant::now());
                    }
                    let waited = self.large_marks_seen.1.elapsed();
                    if self.git_marks.0 .0 != tab_id || waited >= LARGE_GIT_DELAY {
                        self.git_marks = (buf_key, diff_line_states(head, &self.tabs[idx].lines, 800));
                    } else {
                        ui.ctx().request_repaint_after(LARGE_GIT_DELAY - waited);
                    }
                }
            }
            _ => self.git_marks = ((0, 0), HashMap::new()),
        }

        let mut toggle_bookmark = None;
        // 組字中的文字排在哪裡（鍵盤事件處理完之後的游標位置）
        let compose_at = (has_focus && !self.large_preedit.is_empty()).then_some(sel.head);
        let mut area = egui::ScrollArea::both().id_salt(("large-scroll", tab_id)).auto_shrink([false, false]);
        // 上一幀算好的捲動位置（拖曳選取到邊緣以外、自動換行時讓游標留在畫面上）：照它捲過去
        if let Some((id, offset)) = self.large_scroll_to.take() {
            if id == tab_id {
                area = area.scroll_offset(offset);
            }
        }
        let scroll = area.show_viewport(
            ui,
            |ui, viewport| {
                let buf = &self.tabs[idx];
                // 自動換行：以可見寬度折行。捲動位置以「單位」計：一般的行不論折成幾列都算一個行高，
                // 很長的行（`LongRows` 自己切好列）每一列各算一個——所以不必排版整份文件就知道總高度，
                // 也能捲到長行的中間；畫面從「捲動位置 ÷ 行高」那個單位開始往下排到填滿為止
                let wrap_w = wrap.then(|| (viewport.width() - gutter_w - 10.0 - char_w * 2.0).max(char_w * 20.0));
                if let Some(w) = wrap_w {
                    let key = (tab_id, buf.revision(), w.to_bits());
                    if self.large_rows.key != key {
                        self.large_rows =
                            ui.fonts_mut(|f| LongRows::build(key, &buf.lines, w, adv, |ch| advance_of(f, &font, ch)));
                    }
                }
                // （沒有開自動換行時用不到，裡面可能是舊的）
                let long = &self.large_rows;
                let units = line_count + if wrap { long.extra() } else { 0 };
                let text_w = (max_cols + 8) as f32 * adv;
                let size = if wrap {
                    // 多留將近一個畫面的高度：最後一行（可能折成好幾列）也能捲到最上面
                    egui::vec2(viewport.width(), units as f32 * row_h + (viewport.height() - row_h).max(44.0))
                } else {
                    egui::vec2(
                        (gutter_w + 10.0 + text_w).max(viewport.width()),
                        (line_count as f32 * row_h + 44.0).max(viewport.height()),
                    )
                };
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let resp = ui.interact(rect, view_id, Sense::click_and_drag());
                let text_x = rect.left() + gutter_w + 10.0;
                let y_of = |line: usize| rect.top() + 4.0 + line as f32 * row_h;
                // 折行處的欄位同時是上一列的結尾與下一列的開頭：游標一律算在下一列的開頭
                let cc = |col: usize| CCursor { index: (col).into(), prefer_next_row: true };
                let layout = |ui: &egui::Ui, text: &str, hits: &[(usize, usize)]| {
                    let mut job = highlight_ranges(text, Lang::Plain, pal, font.clone(), hits);
                    if let Some(w) = wrap_w {
                        job.wrap.max_width = w;
                    }
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                // 不換行時很長的行只排版畫面上看得到的那一段（左右各多留一點）：要涵蓋的水平範圍，相對於行首
                let margin = char_w * 64.0;
                let win = (viewport.left() - gutter_w - 10.0 - margin, viewport.right() - gutter_w - 10.0 + margin);
                // 一行的排版結果，以及它從第幾個字元開始、左緣離行首多遠（不是只排一段時是 0）。
                // `hits` 為真時標出尋找命中。
                // 組字中：把輸入法還沒選定的文字排進游標所在的位置，後面的文字跟著往後移（文件本身不變）
                let line_galley = |ui: &egui::Ui, line: usize, hits: bool| -> (Arc<egui::Galley>, usize, f32) {
                    let full = buf.lines.get(line).map_or("", |s| s.as_str());
                    let (shown, start, end, x0) = if !wrap && full.len() > LARGE_VIEW_MAX_COLS {
                        let (a, b, x0) = ui.fonts_mut(|f| line_window(full, win.0, win.1, adv, |ch| advance_of(f, &font, ch)));
                        (&full[char_byte(full, a)..char_byte(full, b)], a, b, x0)
                    } else {
                        (full, 0, usize::MAX, 0.0)
                    };
                    let g = match compose_at {
                        Some(p) if p.line == line && (start..=end).contains(&p.col) => {
                            layout(ui, &with_preedit(shown, p.col - start, &self.large_preedit), &[])
                        }
                        _ if hits && !find.is_empty() => layout(ui, shown, &find_all(shown, &find, &fopts).unwrap_or_default()),
                        _ => layout(ui, shown, &[]),
                    };
                    (g, start, x0)
                };
                // 自動換行時長行的第 r 列（自己切好的，不再交給 egui 折行）：排版結果與這一列從第幾個字元開始
                let row_galley = |ui: &egui::Ui, line: usize, rows: &[(usize, usize)], r: usize, hits: bool| {
                    let full = buf.lines[line].as_str();
                    let (start, b0) = rows[r];
                    let (end, b1) = rows.get(r + 1).copied().unwrap_or((usize::MAX, full.len()));
                    let text = &full[b0..b1];
                    let job = match compose_at {
                        Some(p) if p.line == line && p.col >= start && p.col < end => {
                            highlight_ranges(&with_preedit(text, p.col - start, &self.large_preedit), Lang::Plain, pal, font.clone(), &[])
                        }
                        _ if hits && !find.is_empty() => {
                            highlight_ranges(text, Lang::Plain, pal, font.clone(), &find_all(text, &find, &fopts).unwrap_or_default())
                        }
                        _ => highlight_ranges(text, Lang::Plain, pal, font.clone(), &[]),
                    };
                    (ui.fonts_mut(|f| f.layout_job(job)), start)
                };
                let x_of = |galley: &egui::Galley, col: usize| galley.pos_from_cursor(cc(col)).left();

                // ---- 只排版看得到的行：(行, 這一行上緣在畫面上的 y, 排版結果, 排版從第幾個字元開始, 它的左緣離行首多遠) ----
                let view_top = rect.top() + viewport.top();
                let view_bottom = view_top + viewport.height();
                let mut vis: Vec<(usize, f32, Arc<egui::Galley>, usize, f32)> = Vec::new();
                if wrap {
                    let top_f = (viewport.top() / row_h).max(0.0);
                    let (top_line, top_row) = long.at_unit((top_f.floor() as usize).min(units - 1));
                    // 捲動位置的小數部分：最上面那個單位已經捲出畫面的比例（一般的行依它折行後的高度）
                    let frac = if top_f.floor() as usize >= units { 0.0 } else { top_f.fract() };
                    let mut y = view_top + 4.0;
                    'fill: for line in top_line..line_count {
                        if let Some(rows) = long.rows(line) {
                            // 長行：一列一列排，排到畫面填滿為止
                            let from = if line == top_line { top_row } else { 0 };
                            for r in from..rows.len() {
                                let (g, start) = row_galley(ui, line, rows, r, true);
                                if line == top_line && r == from {
                                    y -= frac * row_h;
                                }
                                vis.push((line, y, g, start, 0.0));
                                y += row_h;
                                if y >= view_bottom {
                                    break 'fill;
                                }
                            }
                            continue;
                        }
                        let (g, start, x0) = line_galley(ui, line, true);
                        let h = g.size().y.max(row_h);
                        if line == top_line {
                            y -= frac * h;
                        }
                        vis.push((line, y, g, start, x0));
                        y += h;
                        if y >= view_bottom {
                            break;
                        }
                    }
                } else {
                    let first = (((viewport.top() - 4.0) / row_h).floor().max(0.0) as usize).min(last_line);
                    let last = (((viewport.bottom() - 4.0) / row_h).ceil().max(0.0) as usize).min(line_count);
                    for line in first..last {
                        let (g, start, x0) = line_galley(ui, line, true);
                        vis.push((line, y_of(line), g, start, x0));
                    }
                }
                // 指標位置對應的 (行, 列)：在畫面上的行以外時，算最上面 / 最下面那一行
                let pos_at = |p: Pos2| {
                    let i = vis.iter().rposition(|(_, y, ..)| *y <= p.y).unwrap_or(0);
                    match vis.get(i) {
                        Some((line, y, g, start, x0)) => {
                            let mut col = start + wrapped_col_at(g, egui::vec2(p.x - text_x - x0, p.y - y));
                            // 點在長行某一列（不是最後一列）的結尾右邊：那個位置算下一列的開頭，退一格留在這一列
                            let end = start + g.job.text.chars().count();
                            if col == end && end < buf.line_len(*line) && col > *start {
                                col -= 1;
                            }
                            Position { line: *line, col }
                        }
                        None => Position { line: 0, col: 0 },
                    }
                };

                // ---- 滑鼠：點一下放游標（Shift 延伸選取）、雙擊選字、三擊選行、拖曳選取、點行號區切換書籤 ----
                let clip = ui.clip_rect();
                let in_gutter = |p: Pos2| p.x < rect.left() + gutter_w;
                let mut dragging = false;
                if resp.clicked() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        if in_gutter(p) {
                            toggle_bookmark = Some(pos_at(p).line);
                        } else {
                            sel.head = pos_at(p);
                            if !ui.input(|i| i.modifiers.shift) {
                                sel.anchor = sel.head;
                            }
                        }
                    }
                    ui.memory_mut(|m| m.request_focus(view_id));
                }
                // 雙擊 / 三擊時上面的單擊已經把游標放到點的位置，這裡再把選取擴大成那個字 / 那一行
                //（與一般編輯區相同：三擊選整行但不含換行）
                if resp.double_clicked() || resp.triple_clicked() {
                    if let Some(p) = resp.interact_pointer_pos().filter(|p| !in_gutter(*p)) {
                        let at = pos_at(p);
                        let (a, b) = if resp.triple_clicked() {
                            (0, buf.line_len(at.line))
                        } else {
                            word_bounds(&buf.lines[at.line], at.col)
                        };
                        sel = Selection { anchor: Position { line: at.line, col: a }, head: Position { line: at.line, col: b } };
                    }
                }
                if resp.drag_started() {
                    if let Some(origin) = ui.input(|i| i.pointer.press_origin()).filter(|p| !in_gutter(*p)) {
                        sel.anchor = pos_at(origin);
                        sel.head = sel.anchor;
                    }
                    ui.memory_mut(|m| m.request_focus(view_id));
                }
                // 拖曳中指標超出可見範圍：依超出的距離持續捲動（越遠越快，與幀率無關），
                // 選取的端點停在可見範圍的邊上，所以一直看得到
                let mut edge_scroll = Vec2::ZERO;
                if resp.dragged() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        let (left, right) = (clip.left().max(text_x), clip.right() - 1.0);
                        let (top, bottom) = (clip.top(), clip.bottom() - 1.0);
                        let over = |v: f32, lo: f32, hi: f32| if v < lo { v - lo } else if v > hi { v - hi } else { 0.0 };
                        let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.1);
                        edge_scroll = Vec2::new(
                            edge_scroll_speed(over(p.x, left, right), char_w),
                            edge_scroll_speed(over(p.y, top, bottom), row_h),
                        ) * dt;
                        sel.head = pos_at(Pos2::new(p.x.clamp(left, right.max(left)), p.y.clamp(top, bottom.max(top))));
                        dragging = true;
                    }
                }
                let max_offset = (size - viewport.size()).max(Vec2::ZERO);
                let mut next_offset = None;
                if edge_scroll != Vec2::ZERO {
                    let to = (viewport.min.to_vec2() + edge_scroll).clamp(Vec2::ZERO, max_offset);
                    if to != viewport.min.to_vec2() {
                        next_offset = Some(to);
                    }
                }

                // ---- 畫出這些行 ----
                let painter = ui.painter();
                let gutter = Rect::from_min_max(Pos2::new(rect.left(), clip.top()), Pos2::new(rect.left() + gutter_w, clip.bottom()));
                painter.rect_filled(gutter, 0.0, c.panel);
                painter.line_segment([gutter.right_top(), gutter.right_bottom()], Stroke::new(1.0_f32, c.border));
                let (s0, s1) = (sel.start(), sel.end());
                let preedit_cols = self.large_preedit.chars().count();
                // 組字中的文字從游標的位置開始，游標畫在輸入法報的位置（沒有報時在組字文字的後面）
                let caret_col = sel.head.col + if compose_at == Some(sel.head) { self.large_ime_at.min(preedit_cols) } else { 0 };
                let mut caret_rect = None;
                for (line, y, galley, start, x0) in &vis {
                    let (line, y, start) = (*line, *y, *start);
                    // 這一段排版的左緣（只排一段的長行從行首往右 `x0`）；欄位要先減掉 `start` 才是排版裡的位置
                    let gx = text_x + x0;
                    let g_end = start + galley.job.text.chars().count();
                    let h = galley.size().y.max(row_h);
                    // 這一段排到行尾了嗎（只排一段的長行、或自動換行時長行不是最後一列的那些列不是）
                    let line_end_here = g_end >= buf.line_len(line);
                    let active = line == sel.head.line;
                    if active {
                        let r = Rect::from_min_max(Pos2::new(gutter.right(), y), Pos2::new(clip.right().max(rect.right()), y + h));
                        painter.rect_filled(r, 0.0, c.current_line);
                    }
                    if !sel.is_empty() && (s0.line..=s1.line).contains(&line) {
                        let a = if line == s0.line { s0.col } else { 0 };
                        // 選到下一行時，把這一行的換行也標出來（標在行尾那一段）
                        let to_eol = line != s1.line;
                        let b = if to_eol { usize::MAX } else { s1.col.saturating_sub(start) };
                        let mut spans = row_spans(galley, a.saturating_sub(start), b, to_eol && line_end_here);
                        if to_eol && line_end_here {
                            if let Some(last) = spans.last_mut() {
                                last.3 += char_w * 0.5;
                            }
                        }
                        for (top, bottom, xa, xb) in spans {
                            let r = Rect::from_min_max(Pos2::new(gx + xa, y + top), Pos2::new(gx + xb, y + bottom));
                            painter.rect_filled(r, 0.0, c.sel_bg);
                        }
                    }
                    painter.galley(Pos2::new(gx, y), galley.clone(), c.fg);
                    // 游標畫在含有它的那一段：落在兩段交界時畫在後面那一段的開頭（與 egui 的折行游標相同）；
                    // 在只排了一段的長行裡、但不在這一段裡時，它在畫面外，不畫
                    if active && has_focus && caret_col >= start && (caret_col < g_end || (caret_col == g_end && line_end_here)) {
                        let r = galley.pos_from_cursor(cc(caret_col - start));
                        let caret = Rect::from_min_size(Pos2::new(gx + r.left(), y + r.top()), egui::vec2(1.5, row_h));
                        painter.rect_filled(caret, 0.0, c.accent);
                        caret_rect = Some(caret);
                        // 告訴作業系統游標在哪裡：輸入法的候選字視窗會跟著它，沒有這個輸入法也不會啟用
                        ui.ctx().output_mut(|o| o.ime = Some(egui::output::IMEOutput { purpose: egui::IMEPurpose::Normal, rect: clip, cursor_rect: caret, should_interrupt_composition: false }));
                    }
                    // 組字中的文字加底線，和已經在文件裡的文字區分開來
                    if let Some(p) = compose_at.filter(|p| p.line == line && p.col >= start) {
                        for (_, bottom, xa, xb) in row_spans(galley, p.col - start, p.col - start + preedit_cols, false) {
                            let b = y + bottom - 1.0;
                            painter.line_segment([Pos2::new(gx + xa, b), Pos2::new(gx + xb, b)], Stroke::new(1.0_f32, c.accent));
                        }
                    }
                    // 行號區：行號（折成好幾列時標在第一列；長行一列一段，只有第一段有）、目前行標記、書籤
                    let first_piece = start == 0 || !wrap;
                    if first_piece {
                        painter.text(
                            Pos2::new(gutter.right() - 10.0, y + row_h * 0.5),
                            Align2::RIGHT_CENTER,
                            (line + 1).to_string(),
                            num_font.clone(),
                            if active { c.accent } else { c.faint },
                        );
                    }
                    if active {
                        painter.rect_filled(Rect::from_min_size(Pos2::new(gutter.left(), y), egui::vec2(2.0, h)), 0.0, c.accent);
                    }
                    // Git：新增（強調色條）/ 修改（黃色條）/ 此行上方有刪除（洋紅三角），與一般編輯區相同
                    if let Some(&st) = self.git_marks.1.get(&line) {
                        let x = gutter.right() - 4.0;
                        if st & (ADDED | MODIFIED) != 0 {
                            let col = if st & MODIFIED != 0 { c.warn } else { c.accent };
                            painter.rect_filled(Rect::from_min_size(Pos2::new(x, y), egui::vec2(3.0, h)), 0.0, col);
                        }
                        if first_piece && st & DELETED_ABOVE != 0 {
                            painter.add(Shape::convex_polygon(
                                vec![Pos2::new(x - 3.0, y - 3.0), Pos2::new(x + 3.0, y), Pos2::new(x - 3.0, y + 3.0)],
                                c.accent2,
                                Stroke::NONE,
                            ));
                        }
                    }
                    if first_piece && buf.bookmarks.contains(&line) {
                        painter.circle_filled(Pos2::new(gutter.left() + 9.0, y + row_h * 0.5), 3.5, c.accent2);
                    }
                }

                // 游標移動後讓它留在畫面上；尋找 / 跳至行則把那一行帶到畫面中央。
                // 拖曳選取時不用這個：端點本來就在可見範圍內，到邊緣以外時由上面的 `edge_scroll` 捲動
                if dragging {
                    ui.ctx().request_repaint();
                }
                if (scroll_to_cursor || moved) && wrap {
                    // 自動換行：每一行的高度要排版了才知道，所以自己算捲動位置，下一幀套用
                    let head = sel.head;
                    // 捲動位置以「單位」計（見上面）：長行的一列、或一般的一整行（不論折成幾列）
                    let unit_at = |line: usize, col: usize| long.unit_of(line) + long.rows(line).map_or(0, |rows| row_of(rows, col));
                    // 一般的行要排版才知道高度（自動換行時這些行都從行首整行排版，`line_galley` 回傳的起點是 0）
                    let height = |line: usize| line_galley(ui, line, false).0.size().y.max(row_h);
                    let unit_h = |u: usize| {
                        let (line, _) = long.at_unit(u);
                        if long.rows(line).is_some() {
                            row_h
                        } else {
                            height(line)
                        }
                    };
                    // 游標所在的單位，以及游標那一列的上緣在這個單位裡往下多少
                    let caret_unit = unit_at(head.line, head.col);
                    let in_line = if long.rows(head.line).is_some() {
                        0.0
                    } else {
                        let g = line_galley(ui, head.line, false).0;
                        g.pos_from_cursor(cc(caret_col)).top()
                    };
                    // 游標那一列目前在畫面上的 y（那一列不在畫面上時為 None）
                    let on_screen =
                        vis.iter().find(|v| v.0 == head.line && unit_at(v.0, v.3) == caret_unit).map(|v| v.1 + in_line);
                    let above = on_screen.map_or(vis.first().is_some_and(|v| caret_unit < unit_at(v.0, v.3)), |y| y < view_top);
                    // 游標那一列的上緣要落在可見範圍上緣下方多少像素：置中 / 貼上緣 / 貼下緣；已經看得到就不動
                    let dist = if scroll_to_cursor {
                        Some(((viewport.height() - row_h) * 0.5).max(4.0))
                    } else if on_screen.is_some_and(|y| y >= view_top && y + row_h <= view_bottom) {
                        None
                    } else if above {
                        Some(4.0)
                    } else {
                        Some((viewport.height() - row_h - 4.0).max(4.0))
                    };
                    if let Some(dist) = dist {
                        // 從游標那一列往上數，看最上面該是哪個單位、它捲出畫面多少比例
                        let mut remaining = dist - 4.0 - in_line;
                        let mut u = caret_unit;
                        let top = loop {
                            if remaining <= 0.0 {
                                break u as f32 + (-remaining / unit_h(u)).min(0.999);
                            }
                            if u == 0 {
                                break 0.0;
                            }
                            u -= 1;
                            remaining -= unit_h(u);
                        };
                        let to = Vec2::new(0.0, (top * row_h).clamp(0.0, max_offset.y));
                        if (to.y - viewport.top()).abs() > 0.01 {
                            next_offset = Some(to);
                            ui.ctx().request_repaint();
                        }
                    }
                } else if scroll_to_cursor || moved {
                    // 只排了一段的長行：游標在那一段裡就照排版算，否則從行首累加前面字元的寬度
                    let (g, start, x0) = line_galley(ui, sel.head.line, false);
                    let x = if (start..=start + g.job.text.chars().count()).contains(&caret_col) {
                        text_x + x0 + x_of(&g, caret_col - start)
                    } else {
                        let full = &buf.lines[sel.head.line];
                        text_x + ui.fonts_mut(|f| prefix_x(full, sel.head.col, adv, |ch| advance_of(f, &font, ch)))
                    };
                    let r = Rect::from_min_size(Pos2::new(x, y_of(sel.head.line)), egui::vec2(char_w, row_h));
                    if scroll_to_cursor {
                        ui.scroll_to_rect(r.expand2(egui::vec2(40.0, row_h * 2.0)), Some(Align::Center));
                    } else {
                        // 行號區佔掉左邊一段：往左多留一點，游標才不會被它蓋住
                        let r = Rect::from_min_max(Pos2::new(r.left() - gutter_w - 10.0 - char_w * 4.0, r.top()), Pos2::new(r.right() + char_w * 4.0, r.bottom()));
                        ui.scroll_to_rect(r, None);
                    }
                }
                (Rect::from_min_max(Pos2::new(text_x, rect.top() + 4.0), rect.max), clip, next_offset, wrap_w, caret_rect)
            },
        );
        self.large_scroll_to = scroll.inner.2.map(|o| (tab_id, o));
        self.large_wrap_w = scroll.inner.3;
        self.mark_rect("large-text", scroll.inner.0);
        self.mark_rect("large-clip", scroll.inner.1);
        if let Some(caret) = scroll.inner.4 {
            self.mark_rect("large-caret", caret);
        }
        let buf = &mut self.tabs[idx];
        buf.selections = vec![sel];
        buf.clamp_cursors();
        if let Some(line) = toggle_bookmark {
            buf.toggle_bookmark(line);
        }
    }
}

/// 未存檔的內容合計超過這個大小時，備份檔最多每隔 `HOT_EXIT_SLOW_INTERVAL` 重寫一次（結束時一定會寫）
const HOT_EXIT_SLOW_BYTES: usize = 4 * 1024 * 1024;
const HOT_EXIT_SLOW_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// 備份檔裡是哪些分頁的哪個版本：(分頁 id, 內容版本, 編碼, 路徑)
type HotExitKey = Vec<(u64, u64, &'static str, Option<String>)>;

impl AlexCodeApp {
    /// 熱結束備份檔的位置（與 `state.json` 同一個資料夾）。
    /// 不保存狀態（測試）或關閉「啟動時還原」時為 None：既然下次不會還原，結束時就照舊詢問。
    fn hot_exit_path(&self) -> Option<PathBuf> {
        let state = self.state_path.as_ref()?;
        self.config.restore_session.then(|| state.with_file_name("hot_exit.json"))
    }

    /// 把尚未存檔的分頁內容寫進備份檔。回傳 true 表示目前所有未存檔的內容都在備份裡
    /// （結束程式時可以不必詢問）。`force` 為 false 時內容沒變就不重寫，內容很大時降低寫入頻率。
    fn save_hot_exit(&mut self, force: bool) -> bool {
        // 上一次交給背景執行緒寫的備份：寫完了就收回結果。還沒寫完時，定期的呼叫下次再來看；
        // 結束程式時的呼叫（`force`）則等它寫完，免得兩邊同時寫同一個檔案
        let mut just_written = false;
        if let Some(job) = self.hot_exit_job.take() {
            if !force && !job.is_finished() {
                self.hot_exit_job = Some(job);
                return false;
            }
            just_written = self.finish_hot_exit_write(job.join());
        }
        let Some(path) = self.hot_exit_path() else {
            // 這次執行中才關掉「啟動時還原」：先前寫的備份不會再被還原，一併移除
            if !self.hot_exit_key.is_empty() {
                if let Some(state) = &self.state_path {
                    let _ = std::fs::remove_file(state.with_file_name("hot_exit.json"));
                }
                self.hot_exit_key.clear();
            }
            return false;
        };
        // 還在等背景載入的還原內容也要留在備份裡，否則載入完成前結束程式就會遺失
        let mut waiting: Vec<u64> = self.hot_restore.keys().copied().collect();
        waiting.sort_unstable();
        let mut key: Vec<_> = self
            .tabs
            .iter()
            .filter(|b| b.modified)
            .map(|b| (b.id, b.revision(), b.encoding.label(), b.file_path.clone()))
            .collect();
        key.extend(waiting.iter().map(|id| (*id, u64::MAX, "", None)));
        if force && just_written && key == self.hot_exit_key {
            return true; // 背景剛寫完的就是現在的內容
        }
        if !force {
            if key == self.hot_exit_key {
                return true;
            }
            let (at, size) = self.hot_exit_written;
            if size > HOT_EXIT_SLOW_BYTES && at.elapsed() < HOT_EXIT_SLOW_INTERVAL {
                return false;
            }
        }
        let active_id = self.tabs.get(self.active_tab_idx).map(|b| b.id);
        let mut tabs: Vec<UnsavedTab> = self
            .tabs
            .iter()
            .filter(|b| b.modified)
            .map(|b| {
                let head = b.selections[0].head;
                UnsavedTab {
                    path: b.file_path.clone(),
                    text: b.edit_text(),
                    crlf: b.line_ending == "\r\n",
                    encoding: b.encoding.label().to_string(),
                    cursor_line: head.line,
                    cursor_col: head.col,
                    active: Some(b.id) == active_id,
                    // 這份內容是以磁碟上的哪個版本為基礎（上次讀寫時記下的，不另外查詢磁碟）
                    disk: disk_signature(self.disk_stamps.get(&b.id).copied().flatten()),
                }
            })
            .collect();
        tabs.extend(waiting.iter().filter_map(|id| self.hot_restore.get(id).cloned()));
        let size = tabs.iter().map(|t| t.text.len()).sum();
        let backup = HotExit { tabs };
        if !force && size > HOT_EXIT_SLOW_BYTES {
            // 內容很大：轉成 JSON 與寫檔放到背景執行緒，介面不必等（量測：29 MB 的內容一次約 150 ms）。
            // 寫完之前回報「還沒備份好」；結果由下一次呼叫收回來
            let repaint = self.repaint.clone();
            self.hot_exit_written = (std::time::Instant::now(), size);
            self.hot_exit_job = Some(std::thread::spawn(move || {
                let result = backup.save(&path);
                if let Some(ctx) = repaint {
                    ctx.request_repaint();
                }
                (key, size, result)
            }));
            return false;
        }
        let result = backup.save(&path);
        self.finish_hot_exit_write(Ok((key, size, result)))
    }

    /// 記下一次備份寫入的結果（同步寫的，或背景執行緒寫完交回來的）；回傳是否寫成功
    fn finish_hot_exit_write(&mut self, done: std::thread::Result<(HotExitKey, usize, std::io::Result<()>)>) -> bool {
        match done {
            Ok((key, size, Ok(()))) => {
                self.hot_exit_key = key;
                self.hot_exit_written = (std::time::Instant::now(), size);
                true
            }
            Ok((_, _, Err(e))) => {
                self.status_message = format!("未存檔內容的備份寫入失敗：{}", e);
                false
            }
            Err(_) => {
                self.status_message = "未存檔內容的備份寫入失敗：背景執行緒異常結束".to_string();
                false
            }
        }
    }

    /// 啟動時還原上次的狀態：主題、設定、資料夾、開啟的檔案，以及尚未存檔的內容
    fn restore_saved_session(&mut self) {
        if let Some(st) = self.state_path.as_deref().and_then(AppState::load) {
            self.apply_state(st);
        }
        if let Some(saved) = self.hot_exit_path().as_deref().and_then(HotExit::load) {
            self.restore_hot_exit(saved);
        }
    }

    /// 還原上次結束時尚未存檔的內容（在 `apply_state` 重新開啟檔案之後呼叫）
    fn restore_hot_exit(&mut self, saved: HotExit) {
        let recent = self.recent_files.clone(); // 還原不該改變「最近開啟」的順序
        let (mut active, mut count, mut changed) = (None, 0, Vec::new());
        for tab in saved.tabs {
            let idx = match tab.path.as_deref().filter(|p| Path::new(p).is_absolute()) {
                Some(p) => {
                    let find = |app: &Self| app.tabs.iter().position(|b| b.file_path.as_deref().is_some_and(|f| same_file(f, p)));
                    if find(self).is_none() {
                        self.open_path(Path::new(p)); // 檔案已經不在磁碟上時不會多出分頁
                    }
                    find(self)
                }
                // 未命名的分頁每次都另開一個；歡迎頁這類有名字但不在磁碟上的分頁套回原本那一個
                None => tab.path.as_ref().and_then(|p| self.tabs.iter().position(|b| !b.modified && b.file_path.as_ref() == Some(p))),
            };
            let idx = idx.unwrap_or_else(|| {
                let mut b = EditorBuffer::new();
                b.file_path = tab.path.clone();
                self.tabs.push(b);
                self.tabs.len() - 1
            });
            let id = self.tabs[idx].id;
            if tab.active {
                active = Some(id);
            }
            count += 1;
            if self.loading.contains_key(&id) {
                self.hot_restore.insert(id, tab); // 內容還在背景載入：載入完成後由 poll_loading 套回去
            } else if self.apply_unsaved(idx, &tab) {
                changed.push(Self::tab_title(&self.tabs[idx]));
            }
        }
        self.recent_files = recent;
        if count == 0 {
            return;
        }
        if let Some(i) = active.and_then(|id| self.tabs.iter().position(|b| b.id == id)) {
            self.active_tab_idx = i;
            self.sync_cursor_to_editor();
            self.scroll_to_cursor = true;
        }
        self.status_message = format!("已還原上次未存檔的內容（{} 個分頁）", count);
        if !changed.is_empty() {
            self.status_message +=
                &format!("；注意：{} 在磁碟上已被修改或刪除，存檔會蓋掉磁碟上的版本", changed.join("、"));
        }
    }

    /// 把保存的未存檔內容套到分頁上（成為一筆可以復原的修改）。
    /// 回傳磁碟上的檔案是否已經不是保存當時的那個版本。
    fn apply_unsaved(&mut self, idx: usize, tab: &UnsavedTab) -> bool {
        let buf = &mut self.tabs[idx];
        buf.replace_all_text(&tab.text);
        if let Some(e) = FileEncoding::ALL.into_iter().find(|e| e.label() == tab.encoding) {
            buf.set_encoding(e);
        }
        buf.line_ending = if tab.crlf { "\r\n" } else { "\n" };
        buf.selections = vec![Selection::point(Position { line: tab.cursor_line, col: tab.cursor_col })];
        buf.clamp_cursors();
        let now = disk_signature(self.disk_stamps.get(&buf.id).copied().flatten());
        tab.disk.is_some() && now != tab.disk
    }
}

impl eframe::App for AlexCodeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui_frame(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_hot_exit(true);
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
        let batches: Vec<Vec<PathBuf>> = self.incoming.iter().flat_map(|rx| rx.try_iter()).collect();
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
        self.update_blame(ctx);
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
    pub fn ui_frame(&mut self, ui: &mut egui::Ui) {
        self.check_font_atlas(&ui.ctx().clone());
        self.ui_frame_inner(ui);
        self.check_font_atlas(&ui.ctx().clone());
    }

    /// egui 快填滿字型圖集時會在下一幀開始時重建它，之前排好的文字（編輯區的排版快取）字形座標就失效了。
    /// egui 0.33 起拿不到圖集本身，改用 egui 自己的排版快取判斷：重建圖集時它會整個清空，
    /// 而上一幀用過的排版平常會留到這一幀。所以每次都排同一小段探測文字，拿回來的不是上一次那一份，
    /// 就表示圖集重建過（或兩次之間 egui 清過一次以上的快取，多排一次而已）：丟掉排版快取。
    /// 在每一幀的開頭與結尾各看一次（中間的獨立視窗也可能觸發重建）
    fn check_font_atlas(&mut self, ctx: &egui::Context) {
        let probe = ctx.fonts_mut(|f| f.layout_no_wrap("AlexCode".into(), FontId::monospace(9.0), Color32::WHITE));
        if self.atlas_probe.as_ref().is_some_and(|p| !Arc::ptr_eq(p, &probe)) {
            self.highlight_cache = None;
        }
        self.atlas_probe = Some(probe);
    }

    fn ui_frame_inner(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
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
        self.lsp_tick(ctx);

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
            // 熱結束：未存檔的內容已經寫進備份、下次啟動會還原時，直接結束不必詢問。
            // 備份寫不進去或功能關閉（不還原工作階段）時照舊詢問。
            if self.tabs.iter().any(|b| b.modified) && !self.save_hot_exit(true) {
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

        self.menu_bar(ui, &colors);
        self.detached_windows(ctx, &colors, &pal);
        self.tool_bar(ui, &colors);
        self.status_bar(ui, &colors);
        self.terminal_dock(ui, &colors);
        self.graph_dock(ui, &colors);
        self.explorer_panel(ui, &colors);
        self.tab_bar(ui, &colors);
        self.find_bar(ui, &colors);
        self.editor_area(ui, &colors);
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
        // egui 0.36 起修飾鍵是事件、會一直保持到下一次改變：沒有指定時每一幀都放開（與之前每一幀預設沒有按修飾鍵相同）
        if !input.events.iter().any(|e| matches!(e, Event::ModifiersChanged(_))) {
            input.events.insert(0, Event::ModifiersChanged(Modifiers::NONE));
        }
        let mut out = ctx.run_ui(input, |ui| app.ui_frame(ui));
        // egui 0.36 起沒有交給繪圖端就丟掉的貼圖變更會在除錯建置裡 panic；測試沒有繪圖端，直接清掉
        out.textures_delta.clear();
        out
    }

    /// 這一幀複製到剪貼簿的文字（egui 0.31 起放在 `platform_output.commands` 的 CopyText；沒有複製時是空字串）
    fn copied_text(out: &egui::FullOutput) -> String {
        out.platform_output
            .commands
            .iter()
            .rev()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(t) => Some(t.clone()),
                _ => None,
            })
            .unwrap_or_default()
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
        assert!(app.pending_close.as_ref().is_some_and(|p| p.exit));

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
    fn layout_cache_is_dropped_when_the_font_atlas_fills_up() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn main() {}");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let before = app.highlight_cache.as_ref().unwrap().1.clone();
        // 用很大的字把字型圖集填到八成以上：下一幀開始時 egui 會重建圖集，舊排版的字形座標就失效了
        let text: String = ('!'..='~').collect();
        let mut size = 120.0;
        while ctx.fonts_mut(|f| f.font_atlas_fill_ratio()) <= 0.8 && size < 400.0 {
            ctx.fonts_mut(|f| f.layout_no_wrap(text.clone(), FontId::proportional(size), Color32::WHITE));
            size += 1.0;
        }
        assert!(ctx.fonts_mut(|f| f.font_atlas_fill_ratio()) > 0.8, "the atlas is almost full");
        frame(&mut app, &ctx, vec![]);
        assert!(ctx.fonts_mut(|f| f.font_atlas_fill_ratio()) < 0.5, "egui rebuilt the atlas");
        assert!(!Arc::ptr_eq(&before, &app.highlight_cache.as_ref().unwrap().1), "laid out again on the new atlas");
        let after = app.highlight_cache.as_ref().unwrap().1.clone();
        frame(&mut app, &ctx, vec![]);
        assert!(Arc::ptr_eq(&after, &app.highlight_cache.as_ref().unwrap().1), "idle frames reuse the new layout");
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
    fn editor_ime_caret_and_candidate_window_follow_the_composition_cursor() {
        // 中文輸入法選字時（用方向鍵移回組字文字中間），游標與候選字視窗要跟著輸入法報的位置，不是一直停在組字文字的最後
        let ctx = ctx();
        let mut app = app_with(&ctx, "ab");
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        assert!(
            !ctx.global_style().visuals.ime_composition.legacy_visuals,
            "egui's legacy IME visuals (the Windows default) always draw the caret at the end of the composition"
        );
        let ime_x = |o: &egui::FullOutput| o.platform_output.ime.as_ref().expect("the IME is told where the cursor is").cursor_rect.left();
        let start = ime_x(&frame(&mut app, &ctx, vec![]));
        let font = FontId::monospace(app.config.font_size);
        let width = |s: &str| ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE).size().x);
        let text = "中文可以";
        let preedit = |r: Option<std::ops::Range<usize>>| Event::Ime(egui::ImeEvent::Preedit { text: text.into(), active_range_chars: r });
        // 輸入法的游標在第 1 個字後面
        let x = ime_x(&frame(&mut app, &ctx, vec![preedit(Some(1..1))]));
        assert!((x - (start + width("中"))).abs() < 1.5, "cursor after the 1st character: {x} vs {}", start + width("中"));
        // 選到第 3 個字（輸入法反白的子句）：候選字視窗對著那個字
        let x = ime_x(&frame(&mut app, &ctx, vec![preedit(Some(2..3))]));
        assert!((x - (start + width("中文"))).abs() < 1.5, "the selected clause starts after 2 characters: {x}");
        // 之後沒有新的事件，位置不變
        let x2 = ime_x(&frame(&mut app, &ctx, vec![]));
        assert!((x2 - x).abs() < 0.5, "{x2} vs {x}");
        // 輸入法沒有給位置：在組字文字的最後
        let x = ime_x(&frame(&mut app, &ctx, vec![preedit(None)]));
        assert!((x - (start + width(text))).abs() < 1.5, "no cursor info: at the end, {x}");
        // 選定後照常插入，游標在插入的字後面
        let x = ime_x(&frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Commit(text.into()))]));
        assert_eq!(app.tabs[0].lines[0], "ab中文可以");
        assert!((x - (start + width(text))).abs() < 1.5, "after the commit: {x}");
    }

    #[test]
    fn files_from_every_incoming_channel_open_in_this_window() {
        // 單一實例（其他實例交來的檔案）與 macOS 的 Finder 開檔（Apple 事件）各有一條通道，兩條都要接得上
        let dir = tempfile::tempdir().unwrap();
        let a = write(&dir, "a.txt", "a");
        let b = write(&dir, "b.txt", "b");
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();
        app.add_incoming(rx1);
        app.add_incoming(rx2);
        tx1.send(vec![a.clone()]).unwrap();
        tx2.send(vec![b.clone()]).unwrap();
        frame(&mut app, &ctx, vec![]);
        let open: Vec<String> = app.tabs.iter().filter_map(|t| t.file_path.clone()).collect();
        for p in [&a, &b] {
            let p = p.to_string_lossy().to_string();
            assert!(open.contains(&p), "{p} should be open, tabs: {open:?}");
        }
        // 之後再送來的檔案也會開啟
        let c = write(&dir, "c.txt", "c");
        tx2.send(vec![c.clone()]).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[app.active_tab_idx].file_path.as_deref(), Some(c.to_string_lossy().as_ref()));
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
            std::process::Command::new("git").current_dir(dir.path()).args(args).output().is_ok_and(|o| o.status.success())
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

    fn set_cursor_line(app: &mut AlexCodeApp, line: usize) {
        let idx = app.active_tab_idx;
        app.tabs[idx].selections = vec![Selection::point(Position { line, col: 0 })];
    }

    #[test]
    fn status_bar_shows_git_blame_for_the_cursor_line() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git").current_dir(dir.path()).args(args).output().is_ok_and(|o| o.status.success())
        };
        if !git(&["init", "-q"]) {
            return; // 測試環境沒有 git
        }
        let f = write(&dir, "f.rs", "fn one() {}\nfn two() {}\n");
        assert!(git(&["add", "."]));
        assert!(git(&["-c", "user.name=Alice", "-c", "user.email=a@example.com", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "first commit"]));

        let mut app = app_with(&ctx, "x");
        assert_eq!(app.blame_text(), None, "a tab without a file has no blame");
        app.open_path(&f);
        let id = app.tabs[app.active_tab_idx].id;
        pump_until(&mut app, &ctx, "git blame", |app| app.blame.contains_key(&id));
        let text = app.blame_text().expect("blame for line 1");
        assert!(text.contains("Alice") && text.contains("first commit"), "{text}");
        set_cursor_line(&mut app, 1);
        assert!(app.blame_text().unwrap().contains("Alice"));
        frame(&mut app, &ctx, vec![]); // 狀態列含 blame 的一幀可正常繪製

        // 編輯後（尚未存檔）：舊的結果對不上新的行號，先不顯示；停止輸入一會兒後重新計算
        let idx = app.active_tab_idx;
        app.tabs[idx].replace_all_text("// new first line\nfn one() {}\nfn two() {}\n");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.blame_text(), None, "stale blame is not shown against shifted lines");
        assert!(app.blame_running.is_none(), "blame waits until typing pauses");
        app.blame_seen.2 -= std::time::Duration::from_secs(2);
        let rev = app.tabs[idx].revision();
        pump_until(&mut app, &ctx, "git blame after edit", |app| app.blame.get(&id).is_some_and(|b| b.0 == rev));
        set_cursor_line(&mut app, 0);
        assert!(app.blame_text().unwrap().contains("未提交"), "the unsaved new line is uncommitted");
        set_cursor_line(&mut app, 2);
        assert!(app.blame_text().unwrap().contains("first commit"), "committed lines keep their blame after moving down");
        assert_eq!(read(&f), "fn one() {}\nfn two() {}\n", "blame reads the buffer, it never writes the file");

        // 未追蹤的檔案：沒有 blame，也不會每一幀重跑 git
        let g = write(&dir, "untracked.rs", "fn x() {}");
        app.open_path(&g);
        let gid = app.tabs[app.active_tab_idx].id;
        pump_until(&mut app, &ctx, "blame of untracked file", |app| app.blame.contains_key(&gid));
        assert_eq!(app.blame_text(), None);
        frame(&mut app, &ctx, vec![]);
        assert!(app.blame_running.is_none());

        // 關閉分頁後不留下結果
        let gidx = app.active_tab_idx;
        app.close_tab(gidx);
        assert!(!app.blame.contains_key(&gid));

        // 狀態列顯示的文字（短 SHA、作者、日期、「·」、摘要）每個字元在內建字型中都有字形，不會畫成方框。
        // 中文訊息用的是系統的中文字型（CI 的 Linux / macOS 機器上沒有），不在這裡檢查。
        assert!(!text.contains('⎇'), "{text}");
        let has = ctx.fonts_mut(|fonts| text.chars().all(|c| fonts.glyph_width(&FontId::proportional(12.0), c) > 0.0));
        assert!(has, "every character of `{text}` must be drawable");
    }

    #[test]
    fn diff_view_compares_the_buffer_with_head() {
        use crate::git::RowType::{Added, Modified, Same};
        let ctx = ctx();
        let mut app = app_with(&ctx, "a\nb CHANGED\nc\nd\nnew");
        let id = app.tabs[0].id;
        let lines = |list: &[&str]| Arc::new(list.iter().map(|s| s.to_string()).collect::<Vec<String>>());

        // 沒有 HEAD 版本可比：視窗照常開啟，只是沒有比較結果
        app.run_cmd(Cmd::DiffHead);
        assert!(app.show_diff_dialog);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.diff_rows().is_none());
        assert!(!app.probe.contains_key("diff.next"));

        app.git_head.insert(id, Some(lines(&["a", "b", "c", "d"])));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let rows = app.diff_rows().unwrap().expect("small diff");
        assert_eq!(rows.iter().map(|r| r.row_type).collect::<Vec<_>>(), [Same, Modified, Same, Same, Added]);
        assert_eq!(app.diff_focus, 0, "the first change is selected when the view opens");

        // 點右側（目前內容）的一列：游標跳到那一行
        let right_side = |app: &AlexCodeApp| app.probe["diff.focus_row"].right_center() - egui::vec2(40.0, 0.0);
        let pos = right_side(&app);
        click(&mut app, &ctx, pos);
        assert_eq!(cursor_line(&app), 1);

        // 「下一個」移到下一段變更，到底後繞回第一段
        let next = app.probe["diff.next"].center();
        click(&mut app, &ctx, next);
        assert_eq!(app.diff_focus, 1);
        let pos = right_side(&app);
        click(&mut app, &ctx, pos);
        assert_eq!(cursor_line(&app), 4, "the second change is the added last line");
        click(&mut app, &ctx, next);
        assert_eq!(app.diff_focus, 0);

        // 比較的是分頁目前的內容：編輯後（不必存檔）結果跟著變
        app.tabs[0].replace_all_text("a\nb\nc\nd");
        frame(&mut app, &ctx, vec![]);
        let rows = app.diff_rows().unwrap().unwrap();
        assert_eq!(crate::git::change_count(&rows), 0);
        frame(&mut app, &ctx, vec![]);

        // 不在 Git 內的檔案
        app.git_head.insert(id, None);
        frame(&mut app, &ctx, vec![]);
        assert!(app.diff_rows().is_none());

        // Esc 關閉
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        assert!(!app.show_diff_dialog);

        // 差異太大：不建立龐大的比對表格，視窗顯示說明
        let old: Vec<String> = (0..3000).map(|i| format!("old {i}")).collect();
        let new: String = (0..3000).map(|i| format!("new {i}\n")).collect();
        app.git_head.insert(id, Some(Arc::new(old)));
        app.tabs[0].replace_all_text(&new);
        app.run_cmd(Cmd::DiffHead);
        frame(&mut app, &ctx, vec![]);
        assert!(matches!(app.diff_rows(), Some(None)));
        frame(&mut app, &ctx, vec![]);
    }

    #[test]
    fn diff_view_scrolls_to_changes_far_down_the_file() {
        let ctx = ctx();
        let head: Vec<String> = (0..600).map(|i| format!("line {i}")).collect();
        let mut text = head.clone();
        text[300] = "changed 300".to_string();
        text[550] = "changed 550".to_string();
        let mut app = app_with(&ctx, &text.join("\n"));
        let id = app.tabs[0].id;
        app.git_head.insert(id, Some(Arc::new(head)));

        // 開啟時第一段變更（第 301 行）已經捲到畫面上，點它會跳到那一行
        app.run_cmd(Cmd::DiffHead);
        settle(&mut app, &ctx);
        let window = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0));
        let row = app.probe["diff.focus_row"];
        assert!(window.contains_rect(row), "the selected change is on screen: {row:?}");
        click(&mut app, &ctx, row.right_center() - egui::vec2(40.0, 0.0));
        settle(&mut app, &ctx);
        assert_eq!(cursor_line(&app), 300);

        // 下一段變更在更下面：同樣捲過去
        let next = app.probe["diff.next"].center();
        click(&mut app, &ctx, next);
        settle(&mut app, &ctx);
        let row = app.probe["diff.focus_row"];
        assert!(window.contains_rect(row), "{row:?}");
        click(&mut app, &ctx, row.right_center() - egui::vec2(40.0, 0.0));
        settle(&mut app, &ctx);
        assert_eq!(cursor_line(&app), 550);
    }

    /// 送出「關閉視窗」並跑一幀；回傳程式是否攔下了這次關閉（跳出詢問）
    fn close_window(app: &mut AlexCodeApp, ctx: &egui::Context) -> bool {
        let out = frame_with(app, ctx, |i| {
            i.viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
        });
        out.viewport_output[&ViewportId::ROOT].commands.contains(&ViewportCommand::CancelClose)
    }

    /// 「重新啟動」：新的 App 從同一個設定資料夾還原
    fn restart(state_file: &Path) -> AlexCodeApp {
        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.to_path_buf());
        app.restore_saved_session();
        app
    }

    fn tab_of<'a>(app: &'a AlexCodeApp, path: &Path) -> &'a EditorBuffer {
        let full = path.to_string_lossy();
        app.tabs.iter().find(|b| b.file_path.as_deref() == Some(full.as_ref())).expect("the file's tab is open")
    }

    #[test]
    fn hot_exit_keeps_unsaved_tabs_across_restarts() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let a = write(&dir, "a.rs", "fn a() {}\r\n");
        let clean = write(&dir, "clean.rs", "fn clean() {}");
        let state_file = dir.path().join("cfg").join("state.json");
        let backup = dir.path().join("cfg").join("hot_exit.json");

        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&a);
        app.open_path(&clean);
        app.tabs[0].replace_all_text("fn a() { edited }\n");
        app.new_tab();
        let untitled = app.active_tab_idx;
        app.tabs[untitled].replace_all_text("scratch notes\n第二行");
        app.tabs[untitled].selections = vec![Selection::point(Position { line: 1, col: 2 })];

        // 關閉視窗：不詢問，未存檔的內容寫進備份；磁碟上的檔案不動
        assert!(!close_window(&mut app, &ctx), "exit is not interrupted");
        assert!(app.pending_close.is_none());
        app.save_hot_exit(true); // eframe 結束時呼叫的 on_exit
        app.persist_state();
        assert!(backup.exists());
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "fn a() {}\r\n");

        let mut app2 = restart(&state_file);
        assert_eq!(app2.tabs.len(), 3, "two files and the untitled tab");
        let ra = tab_of(&app2, &a);
        assert_eq!(ra.edit_text(), "fn a() { edited }\n");
        assert!(ra.modified, "the restored content is still unsaved");
        assert_eq!(ra.line_ending, "\r\n");
        assert!(!tab_of(&app2, &clean).modified);
        let ru = &app2.tabs[app2.active_tab_idx];
        assert_eq!(ru.file_path.as_deref(), Some("untitled_3.txt"), "the untitled tab was the active one");
        assert_eq!(ru.edit_text(), "scratch notes\n第二行");
        assert!(ru.modified);
        assert_eq!((ru.selections[0].head.line, ru.selections[0].head.col), (1, 2));
        assert!(app2.status_message.contains("已還原") && !app2.status_message.contains("注意"), "{}", app2.status_message);

        // 復原會回到磁碟上的內容，並且不再算是有修改
        let ia = app2.tabs.iter().position(|b| b.file_path.as_deref() == Some(a.to_string_lossy().as_ref())).unwrap();
        assert!(app2.tabs[ia].undo());
        assert_eq!(app2.tabs[ia].edit_text(), "fn a() {}\n");
        assert!(!app2.tabs[ia].modified);
        assert!(app2.tabs[ia].redo());

        // 存檔之後那個分頁就不在備份裡了；全部存檔 / 關掉之後備份檔消失
        assert!(app2.save_tab(ia, false));
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "fn a() { edited }\r\n");
        app2.persist_state();
        let left = HotExit::load(&backup).unwrap();
        assert_eq!(left.tabs.len(), 1);
        assert_eq!(left.tabs[0].path.as_deref(), Some("untitled_3.txt"));
        let id = app2.tabs[app2.active_tab_idx].id;
        app2.finish_close(&[id], false); // 相當於在詢問視窗選「不儲存」
        app2.persist_state();
        assert!(!backup.exists(), "nothing unsaved is left");
        assert_eq!(restart(&state_file).tabs.len(), 2);
    }

    #[test]
    fn exit_still_asks_when_unsaved_content_would_not_be_restored() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let state_file = dir.path().join("cfg").join("state.json");
        let backup = dir.path().join("cfg").join("hot_exit.json");

        // 關閉「啟動時還原」：下次不會還原，所以結束時照舊詢問，也不留下備份
        let mut app = app_with(&ctx, "fn main() {}");
        app.state_path = Some(state_file.clone());
        app.tabs[0].replace_all_text("changed");
        app.persist_state();
        assert!(backup.exists());
        app.config.restore_session = false;
        app.persist_state();
        assert!(!backup.exists(), "the backup written earlier in this session is removed");
        assert!(close_window(&mut app, &ctx), "close must be cancelled");
        assert!(app.pending_close.as_ref().is_some_and(|p| p.exit));

        // 備份寫不進去（設定資料夾的位置被一個檔案佔住）：同樣詢問，並在狀態列說明
        let blocked = dir.path().join("blocked");
        std::fs::write(&blocked, "not a folder").unwrap();
        let mut app = app_with(&ctx, "fn main() {}");
        app.state_path = Some(blocked.join("state.json"));
        app.tabs[0].replace_all_text("changed");
        assert!(close_window(&mut app, &ctx));
        assert!(app.status_message.contains("備份寫入失敗"), "{}", app.status_message);
    }

    #[test]
    fn hot_exit_restores_into_background_loads_and_missing_files() {
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let state_file = dir.path().join("cfg").join("state.json");
        // 約 2.6 MB：會在背景載入
        let big = write(&dir, "big.log", &"0123456789 abcdefghij 0123456789 abcdefghi\n".repeat(60_000));
        let gone = write(&dir, "gone.txt", "on disk");
        let touched = write(&dir, "touched.txt", "old");

        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&big);
        pump_until(&mut app, &ctx, "background load", |app| app.loading.is_empty());
        app.open_path(&gone);
        app.open_path(&touched);
        for b in &mut app.tabs {
            b.insert_text("EDIT ");
        }
        app.save_hot_exit(true);
        app.persist_state();

        // 兩次啟動之間：一個檔案被刪掉，一個被其他程式改過
        std::fs::remove_file(&gone).unwrap();
        std::fs::write(&touched, "changed by another program").unwrap();

        let mut app2 = restart(&state_file);
        assert_eq!(app2.tabs.len(), 3);
        // 不在磁碟上的檔案：分頁與內容都還在，存檔會重新建立它
        let g = tab_of(&app2, &gone);
        assert_eq!(g.edit_text(), "EDIT on disk");
        assert!(g.modified);
        assert_eq!(tab_of(&app2, &touched).edit_text(), "EDIT old");
        assert!(app2.status_message.contains("注意") && app2.status_message.contains("touched.txt"), "{}", app2.status_message);

        // 大檔案還在背景載入：這時結束，備份裡仍然有它
        assert!(!app2.loading.is_empty());
        assert!(!tab_of(&app2, &big).modified);
        app2.save_hot_exit(true);
        let kept = HotExit::load(&state_file.with_file_name("hot_exit.json")).unwrap();
        assert!(kept.tabs.iter().any(|t| t.path.as_deref() == Some(big.to_string_lossy().as_ref()) && t.text.starts_with("EDIT 0123")));

        // 載入完成後內容套回去
        pump_until(&mut app2, &ctx, "background load", |app| app.loading.is_empty());
        let b = tab_of(&app2, &big);
        assert!(b.edit_text().starts_with("EDIT 0123456789"));
        assert_eq!(b.lines.len(), 60_001);
        assert!(b.modified);
        assert!(app2.hot_restore.is_empty());
    }

    /// 開一個超過大檔案門檻（測試中調低）的分頁：每行都是 "line NNNNN some text here"，最後有一個空行
    fn large_app(ctx: &egui::Context, lines: usize) -> AlexCodeApp {
        let text: String = (0..lines).map(|i| format!("line {i:05} some text here\n")).collect();
        large_app_with(ctx, &text)
    }

    /// 同 `large_app`，內容由呼叫端提供
    fn large_app_with(ctx: &egui::Context, text: &str) -> AlexCodeApp {
        let mut app = AlexCodeApp::new();
        app.large_view_bytes = 1000;
        let mut b = EditorBuffer::from_text(text);
        b.file_path = Some("big.log".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        app.pending_select = Some((0, 0));
        frame(&mut app, ctx, vec![]);
        frame(&mut app, ctx, vec![]);
        app
    }

    fn row_height(app: &AlexCodeApp, ctx: &egui::Context) -> f32 {
        ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)))
    }

    /// 大檔案檢視中某一行目前在畫面上的 y 座標（行的上緣）
    fn large_row_y(app: &AlexCodeApp, ctx: &egui::Context, line: usize) -> f32 {
        app.probe["large-text"].top() + line as f32 * row_height(app, ctx)
    }

    #[test]
    fn large_files_use_a_view_that_only_lays_out_visible_lines() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 5000);
        assert!(app.highlight_cache.is_none(), "the whole text is never laid out");
        assert!(app.text_cache.1.is_empty(), "no joined copy of the text is kept");
        assert!(app.editor_focused, "the view takes keyboard focus like the editor does");

        // 小於門檻的文件仍然用一般編輯區
        let small_ctx = self::ctx();
        let mut small = app_with(&small_ctx, "fn main() {}");
        frame(&mut small, &small_ctx, vec![]);
        assert!(small.highlight_cache.is_some());

        // 點一下把游標放到那一行
        let area = app.probe["large-text"];
        let row_h = row_height(&app, &ctx);
        click(&mut app, &ctx, Pos2::new(area.left() + 1.0, area.top() + row_h * 2.5));
        assert_eq!(cursor(&app), (2, 0));

        // 鍵盤移動游標；Shift 延伸選取，Ctrl+C 複製選取的文字
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE), key(Key::End, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (3, 25));
        frame(&mut app, &ctx, vec![key(Key::ArrowLeft, Modifiers::SHIFT), key(Key::ArrowLeft, Modifiers::SHIFT)]);
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "re");
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::NONE), key(Key::ArrowDown, Modifiers::SHIFT)]);
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "line 00003 some text here\n");

        // Ctrl+End 到檔尾，畫面跟著捲過去
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::COMMAND)]);
        settle(&mut app, &ctx);
        assert_eq!(cursor(&app), (5000, 0));
        let y = large_row_y(&app, &ctx, 5000);
        assert!(y > 0.0 && y < 800.0, "the last line is on screen (y = {y})");
        assert!(app.highlight_cache.is_none());

        // 全選後複製得到整份文字
        frame(&mut app, &ctx, vec![key(Key::A, Modifiers::COMMAND)]);
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), app.tabs[0].edit_text());
    }

    #[test]
    fn large_view_accepts_typing_paste_delete_and_ime() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 5000);
        let none = Modifiers::NONE;
        let line0 = |app: &AlexCodeApp| app.tabs[0].lines[0].clone();

        frame(&mut app, &ctx, vec![Event::Text("ab".into())]);
        assert_eq!(line0(&app), "abline 00000 some text here");
        assert_eq!(cursor(&app), (0, 2));
        assert!(app.tabs[0].modified);
        frame(&mut app, &ctx, vec![key(Key::Enter, none)]);
        assert_eq!((line0(&app).as_str(), app.tabs[0].lines[1].as_str()), ("ab", "line 00000 some text here"));
        assert_eq!((cursor(&app), app.tabs[0].lines.len()), ((1, 0), 5002));
        // Backspace 在行首把兩行接回來；一幀裡的多個按鍵依序處理
        frame(&mut app, &ctx, vec![key(Key::Backspace, none)]);
        assert_eq!((cursor(&app), app.tabs[0].lines.len()), ((0, 2), 5001));
        frame(&mut app, &ctx, vec![key(Key::Backspace, none), key(Key::Backspace, none), key(Key::Delete, none)]);
        assert_eq!(line0(&app), "ine 00000 some text here");

        // 有選取時輸入會取代選取的文字
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::SHIFT)]);
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("X", (0, 1)));

        // 貼上多行（CRLF 也可以）；Ctrl+Z 只復原這次貼上
        frame(&mut app, &ctx, vec![Event::Paste("p1\r\np2".into())]);
        assert_eq!((line0(&app).as_str(), app.tabs[0].lines[1].as_str(), cursor(&app)), ("Xp1", "p2", (1, 2)));
        std::thread::sleep(std::time::Duration::from_millis(5));
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!((line0(&app).as_str(), app.tabs[0].lines.len()), ("X", 5001));

        // Tab：預設插入 Tab 字元；設定成插入空白時對齊到下一個定位點
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::COMMAND), key(Key::End, none), key(Key::Tab, none)]);
        assert_eq!(line0(&app), "X\t");
        app.config.insert_spaces = true;
        frame(&mut app, &ctx, vec![key(Key::Tab, none)]);
        assert_eq!(line0(&app), "X\t  ");
        assert!(app.editor_focused, "Tab must not move keyboard focus away");
        // Ctrl+Backspace 刪掉前面的空白與一個字
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::COMMAND)]);
        assert_eq!(line0(&app), "");

        // 輸入法：組字期間文件不變、按鍵歸輸入法；選定後才插入
        frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: "zhong".into(), active_range_chars: None })]);
        frame(&mut app, &ctx, vec![key(Key::Backspace, none), key(Key::ArrowDown, none)]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("", (0, 0)));
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Commit("中文".into()))]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("中文", (0, 2)));
        assert!(out.platform_output.ime.is_some(), "the IME is told where the cursor is");

        // 剪下
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::SHIFT)]);
        let out = frame(&mut app, &ctx, vec![Event::Cut]);
        assert_eq!(copied_text(&out), "中文");
        assert_eq!(line0(&app), "");

        // Ctrl+左右鍵一次移動一個字
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, none), key(Key::ArrowRight, Modifiers::COMMAND)]);
        assert_eq!(cursor(&app), (1, 4));
        frame(&mut app, &ctx, vec![key(Key::ArrowRight, Modifiers::COMMAND)]);
        assert_eq!(cursor(&app), (1, 10));
        frame(&mut app, &ctx, vec![key(Key::ArrowLeft, Modifiers::COMMAND)]);
        assert_eq!(cursor(&app), (1, 5));

        // 從頭到尾都沒有排版整份文字，也沒有串接出全文
        assert!(app.highlight_cache.is_none());
        assert!(app.text_cache.1.is_empty());
        assert_eq!(app.tabs[0].lines.len(), 5001);
    }

    /// 這一幀畫出來的每一段文字
    fn painted_texts(out: &egui::FullOutput) -> Vec<String> {
        fn walk(shape: &Shape, found: &mut Vec<String>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, found)),
                Shape::Text(t) => found.push(t.galley.job.text.clone()),
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, &mut found));
        found
    }

    #[test]
    fn large_view_preedit_pushes_the_following_text_aside() {
        assert_eq!(with_preedit("abcd", 2, "中"), "ab中cd");
        assert_eq!(with_preedit("中文", 1, "x"), "中x文");
        assert_eq!(with_preedit("ab", 9, "x"), "abx");

        let ctx = ctx();
        let mut app = large_app(&ctx, 200);
        let line0 = |app: &AlexCodeApp| app.tabs[0].lines[0].clone();
        let has = |out: &egui::FullOutput, text: &str| painted_texts(out).iter().any(|t| t == text);
        let caret_x = |out: &egui::FullOutput| out.platform_output.ime.expect("IME output").cursor_rect.left();
        let preedit_w =
            ctx.fonts_mut(|f| f.layout_no_wrap("zhong".into(), FontId::monospace(app.config.font_size), Color32::WHITE).size().x);

        frame(&mut app, &ctx, vec![key(Key::ArrowRight, Modifiers::COMMAND)]);
        assert_eq!(cursor(&app), (0, 4));
        let out = frame(&mut app, &ctx, vec![]);
        assert!(has(&out, "line 00000 some text here"));
        let caret0 = caret_x(&out);

        // 組字中：那一行排成「游標前的文字 + 組字中的文字 + 游標後的文字」，不是另外蓋一塊在上面
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: "zhong".into(), active_range_chars: None })]);
        assert!(has(&out, "linezhong 00000 some text here"), "painted: {:?}", painted_texts(&out));
        assert!(!has(&out, "zhong"), "the preedit is no longer drawn as an overlay");
        assert!(!has(&out, "line 00000 some text here"));
        assert!(has(&out, "line 00001 some text here"), "other lines are untouched");
        // 文件與游標位置不變；游標（輸入法候選字視窗跟著它）畫在組字中的文字後面
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("line 00000 some text here", (0, 4)));
        assert!(!app.tabs[0].modified);
        // egui 0.33 起字形的位置是累加字寬後取整到像素，所以游標移動的距離與組字文字的寬度最多差一個像素；
        // 精確的值是 egui 排出來那一行裡，組字文字後面那個字與原本游標那個字的位置差
        let font_size = app.config.font_size;
        let x_at = |s: &str, col: usize| {
            ctx.fonts_mut(|f| f.layout_no_wrap(s.into(), FontId::monospace(font_size), Color32::WHITE))
                .pos_from_cursor(CCursor::new(col))
                .left()
        };
        let expect = x_at("linezhong 00000 some text here", 9) - x_at("line 00000 some text here", 4);
        let moved = caret_x(&out) - caret0;
        assert!((moved - expect).abs() < 0.01, "caret moved {moved}, the composed line puts it {expect} further");
        assert!((moved - preedit_w).abs() < 1.5, "caret moved {moved}, the preedit is {preedit_w} wide");
        // 之後沒有事件的幀也維持這個畫面
        let out = frame(&mut app, &ctx, vec![]);
        assert!(has(&out, "linezhong 00000 some text here"));
        // 輸入法報了組字中的游標位置（中文輸入法用方向鍵移回去選字）：游標與候選字視窗跟著它，不是固定在最後
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: "zhong".into(), active_range_chars: Some(2..2) })]);
        let expect = x_at("linezhong 00000 some text here", 6) - x_at("line 00000 some text here", 4);
        let moved = caret_x(&out) - caret0;
        assert!((moved - expect).abs() < 0.01, "caret moved {moved}, the IME cursor after 'zh' is {expect} further");
        let out = frame(&mut app, &ctx, vec![]);
        assert!((caret_x(&out) - caret0 - expect).abs() < 0.01, "it stays there without new events");

        // 取消組字：畫面回到原樣
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None })]);
        assert!(has(&out, "line 00000 some text here"));
        assert!((caret_x(&out) - caret0).abs() < 0.01);

        // 有選取時開始組字：選取的文字先刪掉，組字中的文字排在那個位置
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::SHIFT)]);
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: "z".into(), active_range_chars: None })]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), (" 00000 some text here", (0, 0)));
        assert!(has(&out, "z 00000 some text here"), "painted: {:?}", painted_texts(&out));
        let out = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Commit("中".into()))]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("中 00000 some text here", (0, 1)));
        assert!(has(&out, "中 00000 some text here"));
        assert!(app.highlight_cache.is_none() && app.text_cache.1.is_empty(), "still the large-file view");
    }

    /// 這一幀畫出來的每一段文字：(內容, 在畫面上的範圍, 折成幾列)
    fn painted_lines(out: &egui::FullOutput) -> Vec<(String, Rect, usize)> {
        fn walk(shape: &Shape, found: &mut Vec<(String, Rect, usize)>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, found)),
                Shape::Text(t) => {
                    found.push((t.galley.job.text.clone(), Rect::from_min_size(t.pos, t.galley.size()), t.galley.rows.len()))
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, &mut found));
        found
    }

    /// 300 行（加結尾的空行）：行號個位數是 5 的行很長（約 500 個字元），其餘都很短
    fn large_wrap_source() -> String {
        (0..300)
            .map(|i| if i % 10 == 5 { format!("L{i:03} {}\n", "word ".repeat(100)) } else { format!("L{i:03} short\n") })
            .collect()
    }

    #[test]
    fn large_view_wraps_long_lines_when_word_wrap_is_on() {
        let ctx = ctx();
        let mut app = large_app_with(&ctx, &large_wrap_source());
        let row_h = row_height(&app, &ctx);
        let clip = app.probe["large-clip"];
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        let line = |out: &egui::FullOutput, prefix: &str| {
            painted_lines(out).into_iter().find(|l| l.0.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is not on screen"))
        };
        // 以這一幀的可見範圍為準（egui 0.28 起有水平捲軸時，裁切範圍下緣比沒有時高一點）
        let on_screen = |app: &AlexCodeApp| {
            let (caret, clip) = (app.probe["large-caret"], app.probe["large-clip"]);
            caret.top() >= clip.top() - 1.0 && caret.bottom() <= clip.bottom() + 1.0
        };

        // 預設不換行：長行只有一列，超出畫面右邊
        let out = frame(&mut app, &ctx, vec![]);
        let l5 = line(&out, "L005 ");
        assert!(l5.2 == 1 && l5.1.right() > clip.right());

        app.config.word_wrap = true;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let (l4, l5, l6) = (line(&out, "L004 "), line(&out, "L005 "), line(&out, "L006 "));
        let rows = l5.2;
        assert!(rows >= 3, "the 500-character line takes several rows, got {rows}");
        assert!(l5.1.right() <= clip.right(), "wrapped text stays inside the view");
        assert!((l5.1.top() - l4.1.top() - row_h).abs() < 0.5);
        assert!(l5.1.height() > (rows as f32 - 0.5) * row_h, "{rows} rows are {} high", l5.1.height());
        assert!((l6.1.top() - l5.1.bottom()).abs() < 0.5, "the next line starts below every row of the wrapped one");
        // 行號仍是檔案的行號，標在第一列
        let number_y = |n: &str| {
            let l = painted_lines(&out).into_iter().find(|l| l.0 == n && l.1.right() < l5.1.left()).unwrap_or_else(|| panic!("no line number {n}"));
            l.1.center().y
        };
        assert!((number_y("6") - l5.1.top() - row_h * 0.5).abs() < 2.0);
        assert!((number_y("7") - l6.1.top() - row_h * 0.5).abs() < 2.0);
        assert!(app.highlight_cache.is_none() && app.text_cache.1.is_empty(), "still only the visible lines are laid out");

        // 點在長行的第二列：游標在那一行靠後的欄位，輸入的字寫進那一行
        click(&mut app, &ctx, Pos2::new(l5.1.left() + 20.0, l5.1.top() + row_h * 1.5));
        let (at_line, col) = cursor(&app);
        assert!(at_line == 5 && col > 50, "cursor at {:?}", cursor(&app));
        frame(&mut app, &ctx, vec![]);
        assert!((app.probe["large-caret"].top() - l5.1.top() - row_h).abs() < 2.0, "the caret is drawn on the second row");
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!(app.tabs[0].lines[5].chars().nth(col), Some('X'));
        assert_eq!(cursor(&app), (5, col + 1));

        // 選取整個長行：每一列各畫一塊選取底色
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::NONE), key(Key::End, Modifiers::SHIFT)]);
        let out = frame(&mut app, &ctx, vec![]);
        fn fills(shape: &Shape, fill: Color32, n: &mut usize) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| fills(s, fill, n)),
                Shape::Rect(r) if r.fill == fill => *n += 1,
                _ => {}
            }
        }
        let mut n = 0;
        out.shapes.iter().for_each(|s| fills(&s.shape, c.sel_bg, &mut n));
        assert_eq!(n, line(&out, "L005 ").2, "one selection rectangle per row");

        // 拖曳選取：從長行的第二列拖到下面第二行
        let (l5, l7) = (line(&out, "L005 "), line(&out, "L007 "));
        let to = Pos2::new(l7.1.left() + 20.0, l7.1.center().y);
        press_and_drag(&mut app, &ctx, Pos2::new(l5.1.left() + 20.0, l5.1.top() + row_h * 1.5), to);
        frame(&mut app, &ctx, vec![Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        let (anchor, head) = selection(&app);
        assert!(anchor.0 == 5 && anchor.1 > 50 && head.0 == 7, "dragged from {anchor:?} to {head:?}");
        let i = AlexCodeApp::char_index_of(&app.tabs[0], 5, 3);
        app.pending_select = Some((i, i));
        frame(&mut app, &ctx, vec![]);

        // 上下鍵以畫面上的列為單位：在長行裡往下是同一行的下一列，走完每一列才到下一行；往上亦然
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::NONE)]);
        let rows = line(&frame(&mut app, &ctx, vec![]), "L005 ").2;
        let mut cols = vec![0];
        for _ in 1..rows {
            frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
            assert_eq!(cursor(&app).0, 5, "still inside the wrapped line");
            cols.push(cursor(&app).1);
        }
        assert!(cols.windows(2).all(|w| w[1] > w[0] + 20), "each row is further into the line: {cols:?}");
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (6, 0));
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (5, cols[rows - 1]), "up from the next line lands on the last row");
        for _ in 1..rows {
            frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        }
        assert_eq!(cursor(&app), (5, 0));
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (4, 0));

        // 一路往下 / 往上移動：游標一直留在畫面上（捲動位置跟著走）
        for i in 0..120 {
            frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
            frame(&mut app, &ctx, vec![]);
            assert!(on_screen(&app), "after {i} x down: caret {:?}, view {clip:?}", app.probe["large-caret"]);
        }
        let far = cursor(&app).0;
        assert!(far > 60, "moved well past the first screen (line {far})");
        for i in 0..40 {
            frame(&mut app, &ctx, vec![key(Key::PageUp, Modifiers::NONE)]);
            frame(&mut app, &ctx, vec![]);
            assert!(on_screen(&app), "after {i} x page up: caret {:?}, view {clip:?}", app.probe["large-caret"]);
        }
        assert_eq!(cursor(&app).0, 0);
        assert!((line(&frame(&mut app, &ctx, vec![]), "L000 ").1.top() - clip.top()).abs() < 12.0, "back at the top");

        // 跳至行：那一行帶到畫面中央；Ctrl+End：最後一行在畫面上
        goto_line(&mut app, &ctx, 205);
        let out = frame(&mut app, &ctx, vec![]);
        let l205 = line(&out, "L205 ");
        assert!((l205.1.top() - clip.center().y).abs() < clip.height() * 0.25, "line 205 at y {} in {clip:?}", l205.1.top());
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (300, 0));
        assert!(on_screen(&app));
        assert!(line(&frame(&mut app, &ctx, vec![]), "L299 ").1.top() < clip.bottom());

        // 關掉之後回到一行一列
        app.config.word_wrap = false;
        goto_line(&mut app, &ctx, 5);
        assert_eq!(line(&frame(&mut app, &ctx, vec![]), "L005 ").2, 1);
    }

    #[test]
    fn large_view_hands_over_to_the_normal_editor_when_the_document_shrinks() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 200);
        // 全選後輸入：文件剩下一個字，小於門檻
        frame(&mut app, &ctx, vec![key(Key::A, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![Event::Text("a".into())]);
        assert_eq!(app.tabs[0].edit_text(), "a");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.highlight_cache.is_some(), "the normal editor is back");
        assert!(app.editor_focused, "and it has the keyboard focus");
        frame(&mut app, &ctx, vec![Event::Text("b".into())]);
        assert_eq!(app.tabs[0].edit_text(), "ab", "typing continues at the same place");

        // 反過來：在一般編輯區貼上一大段，超過門檻後換成大檔案檢視，可以接著打字
        let big: String = (0..200).map(|i| format!("pasted {i:05}\n")).collect();
        frame(&mut app, &ctx, vec![Event::Paste(big.clone())]);
        assert_eq!(app.tabs[0].edit_text(), format!("ab{big}"));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.highlight_cache.is_none(), "the large-file view took over");
        assert!(app.editor_focused);
        frame(&mut app, &ctx, vec![Event::Text("c".into())]);
        assert_eq!(app.tabs[0].lines[200], "c", "typing continues at the end of the pasted text");
    }

    #[test]
    fn large_view_follows_find_goto_and_bookmarks() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 5000);

        // 尋找：選取命中並把它捲到畫面上
        app.show_find_bar = true;
        app.find_bar_text = "line 02500".into();
        app.find_step(true);
        settle(&mut app, &ctx);
        let sel = app.tabs[0].selections[0];
        assert_eq!((sel.anchor.line, sel.anchor.col, sel.head.line, sel.head.col), (2500, 0, 2500, 10));
        let y = large_row_y(&app, &ctx, 2500);
        assert!(y > 100.0 && y < 700.0, "the match is scrolled into view (y = {y})");
        // 下一個命中從目前的命中之後找起，到底後繞回來
        app.find_step(true);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (2500, 10), "the only match is found again after wrapping around");

        // 跳至行（與「跳至行」對話框相同的路徑：字元索引 + 捲動）
        let i = AlexCodeApp::char_index_of(&app.tabs[0], 4000, 3);
        assert_eq!(AlexCodeApp::position_of(&app.tabs[0], i), Position { line: 4000, col: 3 });
        app.pending_select = Some((i, i));
        app.scroll_to_cursor = true;
        settle(&mut app, &ctx);
        assert_eq!(cursor(&app), (4000, 3));
        let y = large_row_y(&app, &ctx, 4000);
        assert!(y > 100.0 && y < 700.0, "the line is scrolled into view (y = {y})");

        // 點行號區切換書籤
        let area = app.probe["large-text"];
        click(&mut app, &ctx, Pos2::new(area.left() - 30.0, y + 2.0));
        assert_eq!(app.tabs[0].bookmarks, vec![4000]);

        // 選單的行操作仍然有效（可以復原）
        app.run_cmd(Cmd::DeleteLine);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines.len(), 5000);
        assert_eq!(app.tabs[0].lines[4000], "line 04001 some text here");
        app.run_cmd(Cmd::Undo);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[4000], "line 04000 some text here");
    }

    #[test]
    fn files_too_big_to_highlight_use_the_large_view() {
        // 門檻就是不再做語法高亮的大小：剛好 `HIGHLIGHT_MAX_BYTES` 的文件仍用一般編輯區（有語法高亮），多一個位元組就換
        let mut text = format!("{}\n", "x".repeat(99)).repeat(HIGHLIGHT_MAX_BYTES / 100);
        text.push_str(&"x".repeat(HIGHLIGHT_MAX_BYTES - text.len()));
        assert_eq!(text.len(), HIGHLIGHT_MAX_BYTES);
        let mut app = AlexCodeApp::new();
        app.tabs = vec![EditorBuffer::from_text(&text)];
        app.active_tab_idx = 0;
        assert!(!app.uses_large_view(0), "a file that is still highlighted stays in the normal editor");
        text.push('x');
        app.tabs = vec![EditorBuffer::from_text(&text)];
        app.large_stats.0 = (0, 0);
        assert!(app.uses_large_view(0), "one byte past the highlight limit uses the large-file view");
    }

    #[test]
    fn large_view_has_no_switch_bar_even_with_very_long_lines() {
        // 大檔案檢視什麼都不缺：沒有說明列也沒有「改用一般編輯區」按鈕，文字從編輯區最上面開始
        let ctx = ctx();
        let app = large_app(&ctx, 200);
        let top = app.probe["large-text"].top();

        // 有一行很長：不換行、自動換行都整行看得到，仍然沒有說明列
        let ctx2 = self::ctx();
        let long = "x".repeat(LARGE_VIEW_MAX_COLS + 500);
        let rest: String = (0..200).map(|i| format!("line {i:05} some text here\n")).collect();
        let mut app2 = large_app_with(&ctx2, &format!("{long}\n{rest}"));
        for wrap in [false, true] {
            app2.config.word_wrap = wrap;
            frame(&mut app2, &ctx2, vec![]);
            frame(&mut app2, &ctx2, vec![]);
            assert!(!app2.probe.contains_key("large-edit"), "no switch button (word wrap {wrap})");
            assert!((app2.probe["large-text"].top() - top).abs() < 0.5, "no bar above the text (word wrap {wrap})");
            assert!(app2.highlight_cache.is_none(), "still the large-file view");
        }

        // 在大檔案檢視裡把一行打到很長也一樣
        let ctx3 = self::ctx();
        let mut app3 = large_app(&ctx3, 200);
        app3.config.word_wrap = true;
        frame(&mut app3, &ctx3, vec![Event::Paste("y".repeat(LARGE_VIEW_MAX_COLS))]);
        frame(&mut app3, &ctx3, vec![Event::Text("z".into())]);
        frame(&mut app3, &ctx3, vec![]);
        assert!(!app3.probe.contains_key("large-edit"));
        assert_eq!(app3.tabs[0].lines[0].chars().count(), LARGE_VIEW_MAX_COLS + 1 + "line 00000 some text here".len());
    }

    #[test]
    fn long_line_rows_and_scroll_units() {
        assert_eq!(display_cols("ab\tc"), 7);
        assert_eq!(display_cols("中文"), 4);
        // ASCII：每列放得下的字元數相同；全形字 / Tab 依寬度排滿就換列，一列至少一個字元
        assert_eq!(wrap_rows(&"x".repeat(25), 80.0, 8.0, |_| 8.0), vec![(0, 0), (10, 10), (20, 20)]);
        let w = |ch: char| if ch == '中' { 16.0 } else if ch == '\t' { 32.0 } else { 8.0 };
        assert_eq!(wrap_rows("中中中a\t", 40.0, 8.0, w), vec![(0, 0), (2, 6), (4, 10)]);
        assert_eq!(wrap_rows("\t\t", 10.0, 8.0, w), vec![(0, 0), (1, 1)]);
        assert_eq!(row_of(&[(0, 0), (10, 10), (20, 20)], 10), 1, "the boundary belongs to the next row");
        assert_eq!(row_of(&[(0, 0), (10, 10), (20, 20)], 25), 2);

        // 第 2 行與第 5 行是長行（各 3 列），其他行一行一個單位
        let long = "x".repeat(LARGE_VIEW_MAX_COLS + 1);
        let k = (LARGE_VIEW_MAX_COLS + 1).div_ceil(3);
        let lines: Vec<String> = (0..7).map(|i| if i == 2 || i == 5 { long.clone() } else { format!("l{i}") }).collect();
        let rows = LongRows::build((0, 0, 0), &lines, k as f32 * 8.0, 8.0, |_| 8.0);
        assert_eq!(rows.extra(), 4);
        assert_eq!(rows.rows(2).map(|r| r.len()), Some(3));
        assert!(rows.rows(3).is_none());
        let units: Vec<(usize, usize)> = (0..11).map(|u| rows.at_unit(u)).collect();
        assert_eq!(units, vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (3, 0), (4, 0), (5, 0), (5, 1), (5, 2), (6, 0)]);
        assert_eq!((0..7).map(|l| rows.unit_of(l)).collect::<Vec<_>>(), vec![0, 1, 2, 5, 6, 7, 10]);
    }

    #[test]
    fn one_line_files_open_in_the_large_view() {
        // 一行就超過門檻的檔案也能開，而且只排版那一行看得到的部分
        let long = "x".repeat(LARGE_VIEW_MAX_COLS + 500);
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        app.large_view_bytes = 1000;
        let mut b = EditorBuffer::from_text(&format!("{long}\nsecond"));
        b.file_path = Some("one-line.json".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        assert!(app.highlight_cache.is_none());
    }

    #[test]
    fn large_view_shows_the_whole_of_very_long_lines() {
        // 一行 6 萬多個字元（像壓成一行的 JSON）：不換行時整行都看得到，但只排版畫面上的那一段
        let ctx = ctx();
        let long: String = (0..9000).map(|i| format!("t{i:05},")).collect();
        let total = long.chars().count();
        let rest: String = (0..200).map(|i| format!("line {i:05} some text here\n")).collect();
        let mut app = large_app_with(&ctx, &format!("{long}\n{rest}"));
        assert!(!app.probe.contains_key("large-edit"), "nothing is cut, so there is no switch button");
        let font = FontId::monospace(app.config.font_size);
        let cw = ctx.fonts_mut(|f| f.glyph_width(&font, '0'));
        let line0 = |out: &egui::FullOutput, probe: &str| {
            painted_lines(out).into_iter().find(|l| l.0.contains(probe)).unwrap_or_else(|| panic!("no painted text with {probe}"))
        };
        let out = frame(&mut app, &ctx, vec![]);
        let (text, _, _) = line0(&out, "t00000,");
        assert!(text.starts_with("t00000,") && text.chars().count() < 2000, "only the visible start is laid out ({} chars)", text.chars().count());

        // End：捲到行尾，畫出的是那一行的最後一段，放在整行排版時它該在的位置
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        settle(&mut app, &ctx);
        assert_eq!(cursor(&app), (0, total));
        let out = frame(&mut app, &ctx, vec![]);
        let (text, r, _) = line0(&out, "t08999,");
        assert!(long.ends_with(&text) && text.chars().count() < 2000, "the end of the line is laid out ({} chars)", text.chars().count());
        let start = total - text.chars().count();
        // egui 排版這一行的開頭（到這一段之前）有多寬：這一段就該從那裡開始
        let width_of = |ctx: &egui::Context, s: &str| ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)).size().x;
        let line_left = app.probe["large-text"].left();
        // egui 0.33 起以次像素累加字寬：這一行都是 ASCII（等寬字型），每個字往右移 cw。
        // 整行交給 egui 排版時，f32 在 48 萬像素附近每加一次就差一點，累積到幾百像素，所以這裡用 f64 算該在的位置；
        // 另外以 egui 排版前 2000 個字（誤差可以忽略）確認每個字往右移的距離就是 cw
        let g = ctx.fonts_mut(|f| f.layout_no_wrap(long[..2000].to_string(), font.clone(), Color32::WHITE));
        let x1999 = g.pos_from_cursor(CCursor::new(1999)).left();
        assert!((x1999 - 1999.0 * cw).abs() < 1.0, "egui moves each character by cw: {x1999} vs {}", 1999.0 * cw);
        let w = (start as f64 * cw as f64) as f32;
        assert!((r.left() - (line_left + w)).abs() < 1.0, "the piece sits where the whole line would put it: {} vs {w}", r.left() - line_left);
        let (clip, caret) = (app.probe["large-clip"], app.probe["large-caret"]);
        assert!(caret.left() >= clip.left() && caret.right() <= clip.right(), "the caret at the end is on screen: {caret:?} in {clip:?}");
        assert!((caret.left() - r.right()).abs() < 1.0);

        // 點在畫面中間：游標落在點到的字元，輸入的字寫進那個位置
        //（這一段的左邊多排了一點，在畫面外；每個字元往右移的距離是字寬 cw）
        let adv = cw;
        let k = ((clip.center().x - r.left()) / adv).floor() as usize;
        click(&mut app, &ctx, Pos2::new(r.left() + (k as f32 + 0.3) * adv, r.center().y));
        assert_eq!(cursor(&app), (0, start + k));
        frame(&mut app, &ctx, vec![Event::Text("Z".into())]);
        assert_eq!(app.tabs[0].lines[0].chars().nth(start + k), Some('Z'));
        assert_eq!(app.tabs[0].lines[0].chars().count(), total + 1);

        // Home 回到行首
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::NONE)]);
        settle(&mut app, &ctx);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(line0(&out, "t00000,").0.starts_with("t00000,"));
        assert!(!app.probe.contains_key("large-edit"));

        // 全形字與 Tab：每一段的位置以前面字元實際的寬度計算，與 egui 排版整行的結果相同
        let ctx2 = self::ctx();
        let wide = "中a\tb".repeat(15_000);
        let total = wide.chars().count();
        let mut app2 = large_app_with(&ctx2, &format!("{wide}\n{rest}"));
        frame(&mut app2, &ctx2, vec![key(Key::End, Modifiers::NONE)]);
        settle(&mut app2, &ctx2);
        let out = frame(&mut app2, &ctx2, vec![]);
        let (text, r, _) = painted_lines(&out).into_iter().find(|l| l.0.contains('中')).expect("line 0 is on screen");
        assert!(wide.ends_with(&text) && text.chars().count() < 2000);
        let start = total - text.chars().count();
        // 整行（88 萬像素寬）交給 egui 排版時 f32 累加的誤差有幾十像素（Linux / macOS CI 上差 61 px），
        // 所以以 f64 逐字累加每個字往右移的距離當標準答案；另外以 egui 排版前 2000 個字（f32 誤差可以忽略）確認逐字累加與 egui 相近：
        // 沒有中文字型時「中」畫成替代字元，2000 個字差 3.7 px（Linux / macOS CI；這台 Windows 有中文字型，差不到 1 px），
        // 可能是 egui 排版時替代字元與前後的字之間有字距調整（kerning）、逐字量不到（沒有查證），所以容許 0.02%
        let adv_of = |c: char| ctx2.fonts_mut(|f| advance_of(f, &font, c)) as f64;
        let w = wide.chars().take(start).map(adv_of).sum::<f64>() as f32;
        let short: String = wide.chars().take(2000).collect();
        let ws = short.chars().map(adv_of).sum::<f64>() as f32;
        assert!(
            (width_of(&ctx2, &short) - ws).abs() < 1.0 + ws * 2e-4,
            "egui lays out the first 2000 characters {} wide, the sum is {ws}",
            width_of(&ctx2, &short)
        );
        assert!((r.left() - (app2.probe["large-text"].left() + w)).abs() < 2.0, "{} vs {}", r.left() - app2.probe["large-text"].left(), w);
        // 點在畫面中間再輸入一個字：畫出來的那個字就在點的位置（差不到一個字元寬，Tab 是 4 個字寬）
        let at = Pos2::new(app2.probe["large-clip"].center().x, r.center().y);
        click(&mut app2, &ctx2, at);
        frame(&mut app2, &ctx2, vec![Event::Text("Z".into())]);
        let out = frame(&mut app2, &ctx2, vec![]);
        let (text, r, _) = painted_lines(&out).into_iter().find(|l| l.0.contains('Z')).expect("the typed character is on screen");
        let before: String = text.chars().take_while(|&ch| ch != 'Z').collect();
        let x = r.left() + width_of(&ctx2, &before);
        assert!((x - at.x).abs() <= cw * 4.0 + 1.0, "typed at {x}, clicked at {}", at.x);
        assert_eq!(app2.tabs[0].lines[0].chars().count(), total + 1);
    }

    #[test]
    fn large_view_wraps_very_long_lines_row_by_row() {
        // 自動換行時，6 萬多個字元的一行折成幾百列：只排版畫面上看得到的那幾列，捲動可以停在那一行的中間
        let ctx = ctx();
        let long: String = (0..9000).map(|i| format!("t{i:05},")).collect();
        let head: String = (0..5).map(|i| format!("head {i}\n")).collect();
        let rest: String = (0..200).map(|i| format!("line {i:05} some text here\n")).collect();
        let mut app = large_app_with(&ctx, &format!("{head}{long}\n{rest}"));
        app.config.word_wrap = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let row_h = row_height(&app, &ctx);
        let font = FontId::monospace(app.config.font_size);
        let width_of = |s: &str| ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)).size().x;
        let adv = width_of("00") - width_of("0");
        // 畫面上屬於那一長行的各列（由上到下）
        let pieces = |out: &egui::FullOutput| {
            let mut v: Vec<(String, Rect, usize)> =
                painted_lines(out).into_iter().filter(|l| l.0.contains(",t0") || l.0.starts_with("t0")).collect();
            v.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
            v
        };
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!app.probe.contains_key("large-edit"), "nothing is cut any more, so there is no switch button");
        let clip = app.probe["large-clip"];
        let p = pieces(&out);
        assert!(p.len() >= 5, "the long line shows several rows, got {}", p.len());
        assert!(p.iter().all(|l| l.2 == 1 && l.1.right() <= clip.right() + 0.5), "every piece is one row inside the view");
        let shown: usize = p.iter().map(|l| l.0.chars().count()).sum();
        assert!(shown < 20_000, "only the rows on screen are laid out ({shown} characters)");
        let joined: String = p.iter().map(|l| l.0.as_str()).collect();
        assert!(long.starts_with(&joined), "the rows continue each other");
        assert!(p.windows(2).all(|w| (w[1].1.top() - w[0].1.top() - row_h).abs() < 0.5), "one row below the other");
        assert!((p[0].1.left() - app.probe["large-text"].left()).abs() < 0.5);
        // 行號只標在第一列
        let numbers = painted_lines(&out).into_iter().filter(|l| l.0 == "6").count();
        assert_eq!(numbers, 1);

        // 點在第二列再輸入：字寫進那一列的位置；上鍵回到第一列的同一個位置
        let n0 = p[0].0.chars().count();
        click(&mut app, &ctx, Pos2::new(p[1].1.left() + 3.3 * adv, p[1].1.center().y));
        assert_eq!(cursor(&app), (5, n0 + 3));
        frame(&mut app, &ctx, vec![Event::Text("Z".into())]);
        assert_eq!(app.tabs[0].lines[5].chars().nth(n0 + 3), Some('Z'));
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (5, 4));
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE), key(Key::ArrowDown, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        let p = pieces(&out);
        let (n0, n1) = (p[0].0.chars().count(), p[1].0.chars().count());
        assert_eq!(cursor(&app), (5, n0 + n1 + 4), "down moves one row at a time inside the line");

        // 跳到那一行的中間：畫面捲到那一列，而不是整行一起跳過
        let i = AlexCodeApp::char_index_of(&app.tabs[0], 5, 30_000);
        app.pending_select = Some((i, i));
        app.scroll_to_cursor = true;
        settle(&mut app, &ctx);
        let out = frame(&mut app, &ctx, vec![]);
        let caret = app.probe["large-caret"];
        assert!(caret.top() >= clip.top() && caret.bottom() <= clip.bottom(), "the caret in the middle of the line is on screen");
        let p = pieces(&out);
        assert!(p.iter().any(|l| l.0.contains("t04285,")), "the rows around character 30000 are shown");
        assert!(p.iter().all(|l| !l.0.contains("t00000,")), "the start of the line has scrolled away");

        // End：到行尾，下一行緊接在最後一列下面
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        settle(&mut app, &ctx);
        let out = frame(&mut app, &ctx, vec![]);
        let p = pieces(&out);
        let last = p.last().expect("the end of the long line is on screen");
        assert!(last.0.ends_with("t08999,"));
        let caret = app.probe["large-caret"];
        assert!(caret.top() >= clip.top() && caret.bottom() <= clip.bottom(), "the caret at the end is on screen");
        let next = painted_lines(&out).into_iter().find(|l| l.0.starts_with("line 00000")).expect("the next line is on screen");
        assert!((next.1.top() - last.1.top() - row_h).abs() < 0.5, "the next line follows the last row");

        // 全形字與 Tab：每一列都不超過畫面寬度，列與列之間不漏字
        let ctx2 = self::ctx();
        let wide = "中a\tb".repeat(15_000);
        let mut app2 = large_app_with(&ctx2, &format!("{wide}\n{rest}"));
        app2.config.word_wrap = true;
        frame(&mut app2, &ctx2, vec![]);
        let out = frame(&mut app2, &ctx2, vec![]);
        let clip = app2.probe["large-clip"];
        let mut p: Vec<_> = painted_lines(&out).into_iter().filter(|l| l.0.contains('中')).collect();
        p.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        assert!(p.len() >= 5 && p.iter().all(|l| l.2 == 1 && l.1.right() <= clip.right() + 0.5));
        let joined: String = p.iter().map(|l| l.0.as_str()).collect();
        assert!(wide.starts_with(&joined));
    }

    #[test]
    fn terminal_runs_an_interactive_shell_and_streams_its_output() {
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        let screen = |app: &AlexCodeApp| {
            let t = &app.terminal;
            let mut s: Vec<String> = (0..t.scrollback_count()).map(|i| t.scrollback_line(i)).collect();
            s.extend((0..t.rows()).map(|r| t.line_text(r)));
            s.join("\n")
        };
        // 一直跑幀（不送任何輸入），直到畫面上的文字符合條件或逾時
        let wait = |app: &mut AlexCodeApp, cond: &dyn Fn(&str) -> bool| {
            let end = Instant::now() + Duration::from_secs(20);
            while Instant::now() < end {
                frame(app, &ctx, vec![]);
                if cond(&screen(app)) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        frame(&mut app, &ctx, vec![]);
        assert!(app.term_pty.is_none(), "the shell starts only when the terminal is opened");
        app.show_terminal = true;
        frame(&mut app, &ctx, vec![]);
        assert!(app.term_pty.is_some(), "opening the terminal starts a shell in a pseudo terminal");
        assert!(wait(&mut app, &|t| !t.trim().is_empty()), "the shell prints its prompt");

        // 送出的指令算出兩個數字、中間隔 2 秒：第一個數字出現時指令還在跑
        let cmd = if cfg!(windows) {
            "set /a 1000+234 & ping -n 3 127.0.0.1 >nul & set /a 5000+678"
        } else {
            "echo $((1000+234)); sleep 2; echo $((5000+678))"
        };
        app.submit_terminal_line(&ctx, cmd.to_string());
        assert!(wait(&mut app, &|t| t.contains("1234")), "the first result shows up:\n{}", screen(&app));
        assert!(!screen(&app).contains("5678"), "before the command has finished");
        assert!(wait(&mut app, &|t| t.contains("5678")), "then the second one:\n{}", screen(&app));

        // 離開 shell：畫面上說明，之後不會自己重新啟動；在輸入列按 Enter 才重新啟動並執行那一行
        app.submit_terminal_line(&ctx, "exit".to_string());
        assert!(wait(&mut app, &|t| t.contains("shell 已結束")), "the exit is reported:\n{}", screen(&app));
        assert!(app.term_pty.is_none());
        frame(&mut app, &ctx, vec![]);
        assert!(app.term_pty.is_none(), "an exited shell is not restarted by itself");
        let again = if cfg!(windows) { "set /a 4000+321" } else { "echo $((4000+321))" };
        app.submit_terminal_line(&ctx, again.to_string());
        assert!(app.term_pty.is_some());
        assert!(wait(&mut app, &|t| t.contains("4321")), "the restarted shell runs the line:\n{}", screen(&app));
    }

    #[test]
    fn language_servers_get_the_open_documents_and_show_in_the_status_bar() {
        use serde_json::{json, Value};
        use std::io::{Read, Write};
        use std::sync::Mutex;
        use std::time::{Duration, Instant};
        // 單元測試裡預設的啟動方式當作「未安裝」：狀態列照樣顯示，滑鼠停留提示怎麼安裝
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn main() {}");
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(painted_lines(&out).iter().any(|l| l.0 == "rust-analyzer：未安裝"), "the status bar tells the server is missing");
        assert!(app.probe.contains_key("lsp-status"));

        // 接上假的伺服器：記下收到的訊息，收到 initialize 就回覆
        let log = Arc::new(Mutex::new(Vec::<Value>::new()));
        let l = log.clone();
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            let log = l.clone();
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        log.lock().unwrap().push(m.clone());
                        if m["method"] == "initialize" {
                            let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": {} } });
                            let _ = server_write.write_all(&crate::lsp::frame(&reply));
                        }
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        let dir = tempfile::tempdir().unwrap();
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let sent = |method: &str| -> Vec<Value> { log.lock().unwrap().iter().filter(|m| m["method"] == method).cloned().collect() };
        let wait = |app: &mut AlexCodeApp, cond: &dyn Fn() -> bool| {
            let end = Instant::now() + Duration::from_secs(10);
            while Instant::now() < end {
                frame(app, &ctx, vec![]);
                if cond() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            false
        };
        // 開著的 .rs 分頁：啟動伺服器、握手、didOpen 送出全文
        assert!(wait(&mut app, &|| !sent("textDocument/didOpen").is_empty()), "the open file is sent to the server");
        let open = sent("textDocument/didOpen");
        assert_eq!(open[0]["params"]["textDocument"]["text"], "fn main() {}");
        assert_eq!(open[0]["params"]["textDocument"]["languageId"], "rust");
        let out = frame(&mut app, &ctx, vec![]);
        assert!(painted_lines(&out).iter().any(|l| l.0 == "rust-analyzer：已連線"), "the status bar shows the connection");
        // 在編輯區打字：didChange 送出新的全文
        frame(&mut app, &ctx, vec![Event::Text("x".into())]);
        assert!(wait(&mut app, &|| !sent("textDocument/didChange").is_empty()));
        let change = sent("textDocument/didChange");
        assert_eq!(change[0]["params"]["contentChanges"][0]["text"], app.tabs[0].edit_text());
        assert!(app.tabs[0].edit_text().contains('x'));
        // 換成沒有語言伺服器的分頁：狀態列不顯示，原來的文件 didClose
        let mut b = EditorBuffer::from_text("notes");
        b.file_path = Some("notes.txt".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        assert!(wait(&mut app, &|| !sent("textDocument/didClose").is_empty()));
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!painted_lines(&out).iter().any(|l| l.0.starts_with("rust-analyzer")), "no server status for a .txt tab");
        assert_eq!(sent("initialize").len(), 1);
    }

    /// 這一幀畫出的折線（診斷的波浪底線）：(顏色, 範圍)
    fn painted_paths(out: &egui::FullOutput) -> Vec<(Color32, Rect)> {
        fn walk(shape: &Shape, found: &mut Vec<(Color32, Rect)>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, found)),
                Shape::Path(p) => {
                    if let egui::epaint::ColorMode::Solid(col) = p.stroke.color {
                        found.push((col, Rect::from_points(&p.points)));
                    }
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, &mut found));
        found
    }

    #[test]
    fn diagnostics_mark_the_gutter_underline_the_range_and_show_on_hover() {
        use serde_json::json;
        use std::io::{Read, Write};
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.rs").to_string_lossy().into_owned();
        let mut app = app_with(&ctx, "fn nope() {}\nlet 😀x = 1;\nfn fine() {}\n");
        app.tabs[0].file_path = Some(path.clone());
        // 假的伺服器：收到 didOpen 就送出兩個診斷——第 1 行的 `nope`（錯誤）、第 2 行的 `x`（警告，前面有一個佔兩個 UTF-16 單位的表情符號）
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        if m["method"] == "initialize" {
                            let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": {} } });
                            let _ = server_write.write_all(&crate::lsp::frame(&reply));
                        }
                        if m["method"] == "textDocument/didOpen" {
                            let note = json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {
                                "uri": m["params"]["textDocument"]["uri"],
                                "diagnostics": [
                                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } },
                                      "severity": 1, "message": "cannot find function `nope`", "source": "rustc" },
                                    { "range": { "start": { "line": 1, "character": 6 }, "end": { "line": 1, "character": 7 } },
                                      "severity": 2, "message": "unused variable: `x`" }
                                ] } });
                            let _ = server_write.write_all(&crate::lsp::frame(&note));
                        }
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let end = Instant::now() + Duration::from_secs(10);
        while app.lsp.diagnostics(&path).len() < 2 && Instant::now() < end {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.lsp.diagnostics(&path).len(), 2, "the diagnostics arrived");
        assert!(app.lsp.has_waker(), "the UI asks to be woken when the server sends something");

        let out = frame(&mut app, &ctx, vec![]);
        let font = FontId::monospace(app.config.font_size);
        let row_h = ctx.fonts_mut(|f| f.row_height(&font));
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("fn nope()")).expect("the editor text").1;
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("fn nope()")).unwrap();
        // 字元在畫面上的 x（以 egui 實際排出來的位置為準）
        let x_at = |row: usize, col: usize| text.left() + g.rows[row].x_offset((col).into());
        // 行號區：有錯誤的行號是錯誤色、有警告的是警告色、沒有的照舊
        let num_color = |n: &str| {
            painted_galleys(&out).into_iter().find(|g| g.job.text == n).map(|g| g.job.sections[0].format.color)
        };
        assert_eq!(num_color("1"), Some(DIAG_ERROR));
        assert_eq!(num_color("2"), Some(DIAG_WARNING));
        assert!(!matches!(num_color("3"), Some(c) if c == DIAG_ERROR || c == DIAG_WARNING));
        // 波浪底線：從範圍的第一個字元到最後一個字元，貼在那一列的底下
        let paths = painted_paths(&out);
        let err: Vec<_> = paths.iter().filter(|p| p.0 == DIAG_ERROR).collect();
        assert_eq!(err.len(), 1, "one error squiggle: {paths:?}");
        let r = err[0].1;
        assert!((r.left() - x_at(0, 3)).abs() < 1.0 && (r.right() - x_at(0, 7)).abs() < 1.0, "{r:?} vs {text:?}");
        assert!(r.top() > text.top() + row_h * 0.6 && r.bottom() <= text.top() + row_h + 1.0, "under the first row: {r:?}");
        let warn: Vec<_> = paths.iter().filter(|p| p.0 == DIAG_WARNING).collect();
        assert_eq!(warn.len(), 1);
        let w = warn[0].1;
        // UTF-16 第 6 個單位是 `x`（`let ` 4 個 + 表情符號 2 個），也就是第 5 個字元；
        // 沒有換算的話底線會落在 `x` 後面的空白上（不論字型裡有沒有這個表情符號都分得出來）
        let x0 = x_at(1, 5);
        assert!((w.left() - x0).abs() < 1.0 && (w.right() - x_at(1, 6)).abs() < 1.0, "{w:?}, x starts at {x0}");
        assert!(w.top() > text.top() + row_h * 1.6, "on the second row");

        // 滑鼠停在底線上：顯示訊息
        let tip = |out: &egui::FullOutput, s: &str| painted_texts(out).iter().any(|t| t.contains(s));
        let at = Pos2::new(r.center().x, text.top() + row_h * 0.5);
        frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(tip(&out, "cannot find function `nope`"), "hovering the range shows the message: {:?}", painted_texts(&out));
        assert!(!tip(&out, "unused variable"));
        // 停在行號區的第 2 行：顯示那一行的診斷
        let num2 = painted_lines(&out).into_iter().find(|l| l.0 == "2").unwrap().1;
        frame(&mut app, &ctx, vec![Event::PointerMoved(num2.center())]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(tip(&out, "unused variable: `x`"), "hovering the line number shows its diagnostics");
        // 移到沒有診斷的地方：不顯示
        frame(&mut app, &ctx, vec![Event::PointerMoved(Pos2::new(x_at(2, 1), text.top() + row_h * 2.5))]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!tip(&out, "cannot find") && !tip(&out, "unused variable"));
    }

    #[test]
    fn f12_and_ctrl_click_jump_to_the_definition() {
        use serde_json::{json, Value};
        use std::io::{Read, Write};
        use std::sync::Mutex;
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let lib = write(&dir, "lib.rs", "// helpers\n\npub fn 😀helper() {}\n");
        let main = dir.path().join("main.rs").to_string_lossy().into_owned();
        let mut app = app_with(&ctx, "fn main() {\n    helper();\n}\n");
        app.tabs[0].file_path = Some(main.clone());
        // 假的伺服器：第 2 行（index 1）的定義在 lib.rs 第 3 行、UTF-16 第 9 個單位（`pub fn ` 7 個 + 表情符號 2 個）；
        // 其他位置回 null（沒有定義）
        let asked = Arc::new(Mutex::new(Vec::<Value>::new()));
        let a = asked.clone();
        let lib_uri = crate::lsp::uri_from_path(&lib.to_string_lossy());
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            let (a, lib_uri) = (a.clone(), lib_uri.clone());
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        let result = match m["method"].as_str() {
                            Some("initialize") => json!({ "capabilities": {} }),
                            Some("textDocument/definition") => {
                                a.lock().unwrap().push(m["params"].clone());
                                if m["params"]["position"]["line"] == 1 {
                                    json!([{ "uri": lib_uri, "range": { "start": { "line": 2, "character": 9 }, "end": { "line": 2, "character": 15 } } }])
                                } else {
                                    Value::Null
                                }
                            }
                            _ => continue,
                        };
                        let _ = server_write.write_all(&crate::lsp::frame(&json!({ "jsonrpc": "2.0", "id": m["id"], "result": result })));
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let pump = |app: &mut AlexCodeApp, what: &str, done: &dyn Fn(&AlexCodeApp) -> bool| {
            let end = Instant::now() + Duration::from_secs(10);
            while !done(app) && Instant::now() < end {
                frame(app, &ctx, vec![]);
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(done(app), "{what}; status: {}", app.status_message);
        };
        pump(&mut app, "the server is ready", &|app| app.lsp.status(ServerKind::RustAnalyzer) == Some(&LspStatus::Ready));
        frame(&mut app, &ctx, vec![]);
        let active_path = |app: &AlexCodeApp| app.tabs[app.active_tab_idx].file_path.clone().unwrap_or_default();
        let head = |app: &AlexCodeApp| {
            let h = app.tabs[app.active_tab_idx].selections[0].head;
            (h.line, h.col)
        };

        // F12：游標在 `helper` 裡（第 2 行第 6 個字元）→ 開啟 lib.rs，游標在表情符號後面的 `helper` 開頭（第 8 個字元）
        app.pending_select = Some((12 + 6, 12 + 6));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(head(&app), (1, 6));
        frame(&mut app, &ctx, vec![key(Key::F12, Modifiers::NONE)]);
        pump(&mut app, "F12 opens the file with the definition", &|app| same_file(&active_path(app), &lib.to_string_lossy()));
        pump(&mut app, "the cursor lands on the definition", &|app| head(app) == (2, 8));
        let q = asked.lock().unwrap()[0].clone();
        assert_eq!(q["textDocument"]["uri"], json!(crate::lsp::uri_from_path(&main)));
        assert_eq!(q["position"], json!({ "line": 1, "character": 6 }));
        assert_eq!(app.tabs.len(), 2);

        // Ctrl+點擊：回到 main.rs，按住 Ctrl 點 `helper` 的第 3 個字元
        app.active_tab_idx = 0;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("fn main()")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("fn main()")).unwrap().1;
        let row_h = ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)));
        let at = Pos2::new(text.left() + g.rows[1].x_offset((6).into()) + 1.0, text.top() + row_h * 1.5);
        let ctrl = Modifiers::COMMAND;
        let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: ctrl };
        frame_with(&mut app, &ctx, |i| {
            i.events = vec![Event::ModifiersChanged(ctrl), Event::PointerMoved(at)];
        });
        frame_with(&mut app, &ctx, |i| {
            i.events = vec![Event::ModifiersChanged(ctrl), button(true)];
        });
        frame_with(&mut app, &ctx, |i| {
            i.events = vec![Event::ModifiersChanged(ctrl), button(false)];
        });
        pump(&mut app, "Ctrl+click jumps too", &|app| same_file(&active_path(app), &lib.to_string_lossy()));
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert_eq!(asked.lock().unwrap()[1]["position"], json!({ "line": 1, "character": 6 }));
        assert_eq!(app.tabs.len(), 2, "the already open file is reused");
        pump(&mut app, "the cursor lands on the definition", &|app| head(app) == (2, 8));

        // 一般點擊（沒按 Ctrl）不會跳
        app.active_tab_idx = 0;
        frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, at);
        std::thread::sleep(Duration::from_millis(100));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert!(same_file(&active_path(&app), &main));

        // 沒有定義的位置：留在原地，狀態列說明
        app.pending_select = Some((3, 3));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::F12, Modifiers::NONE)]);
        pump(&mut app, "the status bar says nothing was found", &|app| app.status_message.contains("找不到定義"));
        assert!(same_file(&active_path(&app), &main));
        assert_eq!(head(&app), (0, 3));
        // 沒有語言伺服器的檔案
        let mut b = EditorBuffer::from_text("notes");
        b.file_path = Some(dir.path().join("notes.txt").to_string_lossy().into_owned());
        app.tabs.push(b);
        app.active_tab_idx = app.tabs.len() - 1;
        frame(&mut app, &ctx, vec![key(Key::F12, Modifiers::NONE)]);
        assert!(app.status_message.contains("沒有語言伺服器"), "{}", app.status_message);
    }

    #[test]
    fn resting_the_pointer_on_code_shows_the_hover_text() {
        use serde_json::{json, Value};
        use std::io::{Read, Write};
        use std::sync::Mutex;
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with(&ctx, "fn main() {\n    😀helper();\n}\n");
        app.tabs[0].file_path = Some(dir.path().join("main.rs").to_string_lossy().into_owned());
        // 假的伺服器：第 2 行有說明（Markdown，含程式碼區塊），其他位置回 null
        let asked = Arc::new(Mutex::new(Vec::<(Instant, Value)>::new()));
        let a = asked.clone();
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            let a = a.clone();
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        let result = match m["method"].as_str() {
                            Some("initialize") => json!({ "capabilities": {} }),
                            Some("textDocument/hover") => {
                                a.lock().unwrap().push((Instant::now(), m["params"].clone()));
                                if m["params"]["position"]["line"] == 1 {
                                    json!({ "contents": { "kind": "markdown", "value": "```rust\nfn helper() -> u32\n```\n---\nAdds things up." } })
                                } else {
                                    Value::Null
                                }
                            }
                            _ => continue,
                        };
                        let _ = server_write.write_all(&crate::lsp::frame(&json!({ "jsonrpc": "2.0", "id": m["id"], "result": result })));
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let end = Instant::now() + Duration::from_secs(10);
        while app.lsp.status(ServerKind::RustAnalyzer) != Some(&LspStatus::Ready) && Instant::now() < end {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(10));
        }
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("fn main()")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("fn main()")).unwrap().1;
        let row_h = ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)));
        let shown = |out: &egui::FullOutput, s: &str| painted_texts(out).iter().any(|t| t.contains(s));
        // 停在 `helper` 上（第 2 行，表情符號後面第 3 個字元）
        let at = Pos2::new(text.left() + g.rows[1].x_offset((8).into()) + 1.0, text.top() + row_h * 1.5);
        let moved = Instant::now();
        let out = frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
        assert!(!shown(&out, "Adds things up."));
        let mut out = out;
        while !shown(&out, "Adds things up.") && moved.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
            out = frame(&mut app, &ctx, vec![]);
        }
        assert!(shown(&out, "Adds things up."), "the hover text appears: {:?}", painted_texts(&out));
        assert!(shown(&out, "fn helper() -> u32"));
        assert!(!painted_texts(&out).iter().any(|t| t.contains("```")), "code fences are not shown");
        let q = asked.lock().unwrap().clone();
        assert_eq!(q.len(), 1, "asked once while the pointer rests");
        assert!(q[0].0 - moved >= Duration::from_millis(300), "asked only after the pointer rested a moment");
        // UTF-16：表情符號佔 2 個單位，第 8 個字元是第 9 個單位
        assert_eq!(q[0].1["position"], json!({ "line": 1, "character": 9 }));
        // 一直停著：不再重問，說明一直顯示
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(20));
            out = frame(&mut app, &ctx, vec![]);
        }
        assert!(shown(&out, "Adds things up."));
        assert_eq!(asked.lock().unwrap().len(), 1);

        // 移到沒有說明的地方（第 1 行）：說明消失，問過之後也不顯示任何東西
        let at0 = Pos2::new(text.left() + g.rows[0].x_offset((4).into()) + 1.0, text.top() + row_h * 0.5);
        let out = frame(&mut app, &ctx, vec![Event::PointerMoved(at0)]);
        assert!(!shown(&out, "Adds things up."), "moving away hides it");
        let t0 = Instant::now();
        while asked.lock().unwrap().len() < 2 && t0.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
            frame(&mut app, &ctx, vec![]);
        }
        assert_eq!(asked.lock().unwrap()[1].1["position"], json!({ "line": 0, "character": 4 }));
        std::thread::sleep(Duration::from_millis(100));
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!shown(&out, "Adds things up.") && !shown(&out, "fn helper"));
        // 移出編輯區（到行號區之外的工具列附近）：不問
        frame(&mut app, &ctx, vec![Event::PointerMoved(Pos2::new(640.0, 5.0))]);
        std::thread::sleep(Duration::from_millis(700));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(asked.lock().unwrap().len(), 2);
    }

    #[test]
    fn typing_shows_the_completion_menu_and_accepting_inserts_the_item() {
        use serde_json::{json, Value};
        use std::io::{Read, Write};
        use std::sync::Mutex;
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with(&ctx, "fn main() {\n    😀\n}\n");
        app.tabs[0].file_path = Some(dir.path().join("main.rs").to_string_lossy().into_owned());
        // 假的伺服器：`.` 是觸發字元；補全一律回同樣的四項（sortText 決定順序，最後一項用 textEdit + filterText）
        let asked = Arc::new(Mutex::new(Vec::<Value>::new()));
        let a = asked.clone();
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            let a = a.clone();
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        let result = match m["method"].as_str() {
                            Some("initialize") => {
                                assert_eq!(m["params"]["capabilities"]["textDocument"]["completion"]["completionItem"]["snippetSupport"], json!(false));
                                json!({ "capabilities": { "completionProvider": { "triggerCharacters": ["."] } } })
                            }
                            Some("textDocument/completion") => {
                                a.lock().unwrap().push(m["params"].clone());
                                let p = &m["params"]["position"];
                                let range = json!({ "start": p, "end": p });
                                json!({ "isIncomplete": false, "items": [
                                    { "label": "helper_two", "sortText": "b" },
                                    { "label": "other", "sortText": "c" },
                                    { "label": "helper_one", "detail": "fn() -> u32", "sortText": "a" },
                                    { "label": " helper_three", "filterText": "helper_three", "sortText": "d",
                                      "textEdit": { "range": range, "newText": "helper_three" } }
                                ]})
                            }
                            _ => continue,
                        };
                        let _ = server_write.write_all(&crate::lsp::frame(&json!({ "jsonrpc": "2.0", "id": m["id"], "result": result })));
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let pump = |app: &mut AlexCodeApp, what: &str, done: &dyn Fn(&egui::FullOutput) -> bool| {
            let end = Instant::now() + Duration::from_secs(10);
            loop {
                let out = frame(app, &ctx, vec![]);
                if done(&out) {
                    return out;
                }
                assert!(Instant::now() < end, "{what}; painted: {:?}", painted_texts(&out));
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let shown = |out: &egui::FullOutput, s: &str| painted_texts(out).iter().any(|t| t == s);
        let line1 = |app: &AlexCodeApp| app.tabs[0].lines[1].clone();
        let head = |app: &AlexCodeApp| {
            let h = app.tabs[0].selections[0].head;
            (h.line, h.col)
        };
        let end = Instant::now() + Duration::from_secs(10);
        while app.lsp.status(ServerKind::RustAnalyzer) != Some(&LspStatus::Ready) && Instant::now() < end {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(10));
        }
        // 游標放在表情符號後面（第 2 行第 5 個字元）
        app.pending_select = Some((17, 17));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(head(&app), (1, 5));

        // 打一個字：送出補全請求（UTF-16：4 個空白 + 表情符號 2 個單位 + h = 7），選單只列出符合 "h" 的項目，依 sortText 排序
        frame(&mut app, &ctx, vec![Event::Text("h".into())]);
        let out = pump(&mut app, "the menu appears after typing", &|o| shown(o, "helper_one"));
        assert!(shown(&out, "helper_two") && shown(&out, "helper_three") && shown(&out, "fn() -> u32"));
        assert!(!shown(&out, "other"), "items that do not match the typed prefix are filtered out");
        let q = asked.lock().unwrap()[0].clone();
        assert_eq!(q["textDocument"]["uri"], json!(crate::lsp::uri_from_path(app.tabs[0].file_path.as_deref().unwrap())));
        assert_eq!(q["position"], json!({ "line": 1, "character": 7 }));
        assert_eq!(q["context"]["triggerKind"], json!(1));
        let y = |out: &egui::FullOutput, s: &str| painted_lines(out).into_iter().find(|l| l.0 == s).unwrap().1.top();
        assert!(y(&out, "helper_one") < y(&out, "helper_two") && y(&out, "helper_two") < y(&out, "helper_three"));

        // 再打一個字：在已經收到的清單裡篩選，不重問
        frame(&mut app, &ctx, vec![Event::Text("e".into())]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one") && shown(&out, "helper_two"));
        assert_eq!(asked.lock().unwrap().len(), 1);

        // 下鍵選第二項、Enter 接受：打過的 "he" 換成完整的名稱，不插入換行
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(line1(&app), "    😀helper_two");
        assert_eq!(app.tabs[0].lines.len(), 4, "Enter accepted the item instead of breaking the line");
        assert_eq!(head(&app), (1, 15));
        assert!(!shown(&out, "helper_one"), "the menu closes after accepting");
        // 接受與前面連續打的字算同一步復原（復原紀錄把連續輸入合併）：一次復原回到打字之前，重做再回來
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(line1(&app), "    😀", "one undo step");
        frame(&mut app, &ctx, vec![key(Key::Y, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(line1(&app), "    😀helper_two");
        app.pending_select = Some((27, 27));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(head(&app), (1, 15));

        // 觸發字元 "."：triggerKind 2，還沒打字所以全部列出；Esc 關閉，內容不變
        frame(&mut app, &ctx, vec![Event::Text(".".into())]);
        let out = pump(&mut app, "the menu appears after a trigger character", &|o| shown(o, "other"));
        assert!(shown(&out, "helper_one"));
        let q = asked.lock().unwrap()[1].clone();
        assert_eq!(q["context"], json!({ "triggerKind": 2, "triggerCharacter": "." }));
        assert_eq!(q["position"], json!({ "line": 1, "character": 17 }));
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!shown(&out, "other"), "Esc closes the menu");
        assert_eq!(line1(&app), "    😀helper_two.");
        assert!(app.editor_focused);

        // Ctrl+Space 手動叫出、Tab 接受第一項
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        pump(&mut app, "Ctrl+Space opens the menu", &|o| shown(o, "other"));
        assert_eq!(asked.lock().unwrap()[2]["context"]["triggerKind"], json!(1));
        frame(&mut app, &ctx, vec![key(Key::Tab, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(line1(&app), "    😀helper_two.helper_one");

        // 打了不是名稱的字元：選單關閉
        frame(&mut app, &ctx, vec![Event::Text(" ".into()), ]);
        frame(&mut app, &ctx, vec![Event::Text("h".into())]);
        pump(&mut app, "the menu appears again", &|o| shown(o, "helper_three"));
        frame(&mut app, &ctx, vec![Event::Text(";".into())]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!shown(&out, "helper_three"), "typing a non-identifier character closes the menu");
        // 選單開著時上下鍵在選單裡移動（游標不動）；游標移出那個字（Home）時選單關閉
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        pump(&mut app, "Ctrl+Space opens the menu", &|o| shown(o, "other"));
        let at = head(&app);
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "other"));
        assert_eq!(head(&app), at);
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!shown(&out, "other"), "moving the cursor out of the word closes the menu");
        assert!(head(&app).1 < at.1);

        // 用滑鼠點選：textEdit 的 newText 插入（標籤前面的空白不算）
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![Event::Text("h".into())]);
        let out = pump(&mut app, "the menu appears", &|o| shown(o, "helper_three"));
        let r = painted_lines(&out).into_iter().find(|l| l.0 == "helper_three").unwrap().1;
        click(&mut app, &ctx, r.center());
        frame(&mut app, &ctx, vec![]);
        assert_eq!(line1(&app), "    😀helper_two.helper_one h;helper_three");
        assert!(app.editor_focused, "the editor keeps the keyboard focus after clicking an item");

        // 沒有語言伺服器的檔案：Ctrl+Space 不送請求
        let n = asked.lock().unwrap().len();
        let mut b = EditorBuffer::from_text("notes");
        b.file_path = Some(dir.path().join("notes.txt").to_string_lossy().into_owned());
        app.tabs.push(b);
        app.active_tab_idx = 1;
        app.pending_select = Some((5, 5));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        frame(&mut app, &ctx, vec![Event::Text("h".into())]);
        std::thread::sleep(Duration::from_millis(100));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(asked.lock().unwrap().len(), n);
    }

    #[test]
    fn alt_drag_selects_a_rectangle_and_copies_it() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "abcdefgh\nab\nABCDEFGH\nzzzz\n");
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("abcdefgh")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("abcdefgh")).unwrap().1;
        let row_h = ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)));
        let at = |row: usize, col: usize| Pos2::new(text.left() + g.rows[row].x_offset((col).into()) + 1.0, text.top() + row_h * (row as f32 + 0.5));
        let sel_fill = ctx.global_style().visuals.selection.bg_fill;
        let sel_rects = |out: &egui::FullOutput| {
            fn walk(shape: &Shape, fill: Color32, found: &mut Vec<Rect>) {
                match shape {
                    Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, found)),
                    Shape::Rect(r) if r.fill == fill => found.push(r.rect),
                    _ => {}
                }
            }
            let mut found = Vec::new();
            out.shapes.iter().for_each(|s| walk(&s.shape, sel_fill, &mut found));
            found
        };
        let alt = Modifiers::ALT;
        let drag = |app: &mut AlexCodeApp, from: Pos2, to: Pos2, m: Modifiers| {
            let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: m };
            for events in [vec![Event::PointerMoved(from)], vec![button(from, true)], vec![Event::PointerMoved(to)], vec![button(to, false)]] {
                frame_with(app, &ctx, |i| {
                    i.events = [vec![Event::ModifiersChanged(m)], events].concat();
                });
            }
            frame_with(app, &ctx, |i| i.events = vec![Event::ModifiersChanged(m)])
        };

        // 按住 Alt 從第 1 行第 2 欄拖到第 3 行第 5 欄：每一行選第 2–5 欄，短的第 2 行只到行尾（零寬）
        let out = drag(&mut app, at(0, 2), at(2, 5), alt);
        assert_eq!(app.box_ranges(), vec![(0, 2, 5), (1, 2, 2), (2, 2, 5)]);
        let rects = sel_rects(&out);
        assert_eq!(rects.len(), 3, "one highlight per line: {rects:?}");
        let x2 = text.left() + g.rows[0].x_offset((2).into());
        let x5 = text.left() + g.rows[0].x_offset((5).into());
        assert!(rects.iter().all(|r| (r.left() - x2).abs() < 1.0), "{rects:?} start at {x2}");
        assert!((rects[0].right() - x5).abs() < 1.0 && (rects[2].right() - x5).abs() < 1.0);
        assert!(rects[1].width() <= 2.5, "the short line gets a thin caret: {:?}", rects[1]);
        assert_eq!(app.tabs[0].edit_text(), "abcdefgh\nab\nABCDEFGH\nzzzz\n", "selecting does not edit");
        assert_eq!(cursor(&app), (2, 5), "the cursor follows the drag");
        assert!(app.editor_focused);

        // 反方向拖（右下往左上）得到同樣的區塊
        let out = drag(&mut app, at(2, 5), at(0, 2), alt);
        assert_eq!(app.box_ranges(), vec![(0, 2, 5), (1, 2, 2), (2, 2, 5)]);
        assert_eq!(sel_rects(&out).len(), 3);

        // Ctrl+C 複製區塊：每一行選到的部分以換行連接
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "cde\n\nCDE");
        assert_eq!(app.box_ranges().len(), 3, "copying keeps the selection");

        // 一般點擊（沒按 Alt）取消矩形選取
        click(&mut app, &ctx, at(3, 1));
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.box_ranges().is_empty());
        assert!(sel_rects(&out).is_empty());
        assert_eq!(cursor(&app), (3, 1));

        // 沒按 Alt 的拖曳仍是一般選取
        drag(&mut app, at(0, 2), at(2, 5), Modifiers::NONE);
        assert!(app.box_ranges().is_empty());
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "cdefgh\nab\nABCDE");

        // 方向鍵取消矩形選取，照一般按鍵移動游標（打字改成每一行同時輸入，另一個測試）
        drag(&mut app, at(0, 2), at(2, 5), alt);
        assert_eq!(app.box_ranges().len(), 3);
        frame(&mut app, &ctx, vec![key(Key::ArrowRight, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.box_ranges().is_empty());
        assert!(sel_rects(&out).is_empty());
        assert_eq!(cursor(&app), (2, 6));
        assert_eq!(app.tabs[0].edit_text(), "abcdefgh\nab\nABCDEFGH\nzzzz\n");
        // Esc 也取消
        drag(&mut app, at(0, 2), at(2, 5), alt);
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.box_ranges().is_empty());

        // 自動換行時不提供（Alt+拖曳照一般選取處理）
        app.config.word_wrap = true;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("abcdefgh")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("abcdefgh")).unwrap().1;
        let at = |row: usize, col: usize| Pos2::new(text.left() + g.rows[row].x_offset((col).into()) + 1.0, text.top() + row_h * (row as f32 + 0.5));
        drag(&mut app, at(0, 2), at(2, 5), alt);
        assert!(app.box_ranges().is_empty());
    }

    #[test]
    fn typing_in_a_rectangle_edits_every_line() {
        let ctx = ctx();
        let original = "abcdefgh\nab\nABCDEFGH\nzzzz\n";
        let mut app = app_with(&ctx, original);
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("abcdefgh")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("abcdefgh")).unwrap().1;
        let row_h = ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)));
        let at = |row: usize, col: usize| Pos2::new(text.left() + g.rows[row].x_offset((col).into()) + 1.0, text.top() + row_h * (row as f32 + 0.5));
        let alt = Modifiers::ALT;
        let drag = |app: &mut AlexCodeApp, from: Pos2, to: Pos2| {
            let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: alt };
            for events in [vec![Event::PointerMoved(from)], vec![button(from, true)], vec![Event::PointerMoved(to)], vec![button(to, false)]] {
                frame_with(app, &ctx, |i| {
                    i.events = [vec![Event::ModifiersChanged(alt)], events].concat();
                });
            }
            frame(app, &ctx, vec![]);
        };
        let lines = |app: &AlexCodeApp| app.tabs[0].edit_text();

        // 選第 2–5 欄（第 2 行短，零寬）後打字：每一行選到的部分都換成打的字，零寬的行也插入
        drag(&mut app, at(0, 2), at(2, 5));
        assert_eq!(app.box_ranges(), vec![(0, 2, 5), (1, 2, 2), (2, 2, 5)]);
        frame(&mut app, &ctx, vec![Event::Text("XY".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abXYfgh\nabXY\nABXYFGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 4, 4), (1, 4, 4), (2, 4, 4)], "the rectangle becomes a column after the text");
        assert_eq!(cursor(&app), (2, 4), "the cursor stays on the line where the drag ended");
        assert!(app.editor_focused);
        // 繼續打字接在同一欄
        frame(&mut app, &ctx, vec![Event::Text("Z".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abXYZfgh\nabXYZ\nABXYZFGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 5, 5), (1, 5, 5), (2, 5, 5)]);
        // 零寬時 Backspace 刪每一行游標前一個字、Delete 刪後一個字（行尾沒東西可刪就不動）
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abXYfgh\nabXY\nABXYFGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 4, 4), (1, 4, 4), (2, 4, 4)]);
        frame(&mut app, &ctx, vec![key(Key::Delete, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abXYgh\nabXY\nABXYGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 4, 4), (1, 4, 4), (2, 4, 4)]);
        // 復原：一次回到這一連串輸入之前（每一行一起，連續輸入合併成一步，與一般打字相同），矩形選取取消
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original);
        assert!(app.box_ranges().is_empty());

        // 有寬度時 Backspace / Delete 刪掉選到的部分
        drag(&mut app, at(0, 2), at(2, 5));
        frame(&mut app, &ctx, vec![key(Key::Delete, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abfgh\nab\nABFGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 2, 2), (1, 2, 2), (2, 2, 2)]);
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original);
        drag(&mut app, at(2, 5), at(0, 2));
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abfgh\nab\nABFGH\nzzzz\n");
        assert_eq!(cursor(&app), (0, 2), "dragged upwards: the cursor is on the first line");

        // 零寬的矩形（同一個 x 往下拖）直接打字就是多行插入
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        drag(&mut app, at(0, 1), at(3, 1));
        assert_eq!(app.box_ranges(), vec![(0, 1, 1), (1, 1, 1), (2, 1, 1), (3, 1, 1)]);
        frame(&mut app, &ctx, vec![Event::Text("-".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "a-bcdefgh\na-b\nA-BCDEFGH\nz-zzz\n");
    }

    #[test]
    fn rectangle_cut_paste_and_keyboard_extend() {
        let ctx = ctx();
        let original = "abcdefgh\nab\nABCDEFGH\nzzzz\n";
        let mut app = app_with(&ctx, original);
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.starts_with("abcdefgh")).unwrap();
        let text = painted_lines(&out).into_iter().find(|l| l.0.starts_with("abcdefgh")).unwrap().1;
        let row_h = ctx.fonts_mut(|f| f.row_height(&FontId::monospace(app.config.font_size)));
        let at = |row: usize, col: usize| Pos2::new(text.left() + g.rows[row].x_offset((col).into()) + 1.0, text.top() + row_h * (row as f32 + 0.5));
        let alt = Modifiers::ALT;
        let drag = |app: &mut AlexCodeApp, from: Pos2, to: Pos2| {
            let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: alt };
            for events in [vec![Event::PointerMoved(from)], vec![button(from, true)], vec![Event::PointerMoved(to)], vec![button(to, false)]] {
                frame_with(app, &ctx, |i| {
                    i.events = [vec![Event::ModifiersChanged(alt)], events].concat();
                });
            }
            frame(app, &ctx, vec![]);
        };
        let lines = |app: &AlexCodeApp| app.tabs[0].edit_text();
        let sa = Modifiers::SHIFT | Modifiers::ALT;
        let press = |app: &mut AlexCodeApp, k: Key| {
            frame(app, &ctx, vec![key(k, sa)]);
        };

        // Shift+Alt+方向鍵：從游標開始一個矩形，往下兩行、往右三個字；內容不變（不是 Alt+下的移動行）
        click(&mut app, &ctx, at(0, 1));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (0, 1));
        press(&mut app, Key::ArrowDown);
        press(&mut app, Key::ArrowDown);
        for _ in 0..3 {
            press(&mut app, Key::ArrowRight);
        }
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original, "Shift+Alt+Down does not move the line");
        assert_eq!(app.box_ranges(), vec![(0, 1, 4), (1, 1, 2), (2, 1, 4)]);
        assert_eq!(cursor(&app), (2, 4), "the cursor follows the end of the rectangle");
        press(&mut app, Key::ArrowLeft);
        press(&mut app, Key::ArrowUp);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.box_ranges(), vec![(0, 1, 3), (1, 1, 2)]);
        assert_eq!(cursor(&app), (1, 2));
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "bc\nb");
        // 從短的行的行尾往右拉：超過行尾以字寬延伸，往上一行時那一行選到更多欄
        click(&mut app, &ctx, at(1, 2));
        frame(&mut app, &ctx, vec![]);
        assert!(app.box_ranges().is_empty(), "a plain click cancels the rectangle");
        press(&mut app, Key::ArrowRight);
        press(&mut app, Key::ArrowRight);
        press(&mut app, Key::ArrowUp);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.box_ranges(), vec![(0, 2, 4), (1, 2, 2)]);
        press(&mut app, Key::ArrowLeft);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.box_ranges(), vec![(0, 2, 3), (1, 2, 2)]);
        assert_eq!(lines(&app), original);
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);

        // Ctrl+X：複製區塊並刪掉選到的部分，一次復原
        drag(&mut app, at(0, 2), at(2, 5));
        let out = frame(&mut app, &ctx, vec![Event::Cut]);
        assert_eq!(copied_text(&out), "cde\n\nCDE");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abfgh\nab\nABFGH\nzzzz\n");
        assert_eq!(app.box_ranges(), vec![(0, 2, 2), (1, 2, 2), (2, 2, 2)]);
        // 剪貼簿的行數與區塊相同：逐行貼回去
        frame(&mut app, &ctx, vec![Event::Paste("cde\r\n\r\nCDE".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original);
        assert_eq!(app.box_ranges(), vec![(0, 5, 5), (1, 2, 2), (2, 5, 5)]);
        // 只有一行：每一行都貼它（結尾的換行不算一行）
        frame(&mut app, &ctx, vec![Event::Paste("12\n".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abcde12fgh\nab12\nABCDE12FGH\nzzzz\n");
        assert_eq!(cursor(&app), (2, 7));
        // 貼上各自一步復原：復原一次回到貼上之前、再一次回到剪下之前
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original);
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), "abfgh\nab\nABFGH\nzzzz\n");
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(lines(&app), original);

        // 行數不合（兩行貼進三行的區塊）：取消矩形選取，照一般貼上貼在游標的位置
        drag(&mut app, at(0, 2), at(2, 5));
        frame(&mut app, &ctx, vec![Event::Paste("x\ny".into())]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.box_ranges().is_empty());
        assert_eq!(app.tabs[0].lines[2], "ABCDEx");
        assert_eq!(app.tabs[0].lines[3], "yFGH");
    }

    #[test]
    fn split_view_shows_two_documents_side_by_side() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        let mid = 640.0;
        let find = |out: &egui::FullOutput, start: &str| painted_lines(out).into_iter().find(|l| l.0.starts_with(start)).map(|l| l.1);

        // Ctrl+\ 分割：右欄是編輯區（同一份文件），左欄唯讀顯示同一份
        frame(&mut app, &ctx, vec![key(Key::Backslash, Modifiers::COMMAND)]);
        let out = frame(&mut app, &ctx, vec![]);
        let shown: Vec<Rect> = painted_lines(&out).into_iter().filter(|l| l.0.starts_with("alpha one")).map(|l| l.1).collect();
        assert_eq!(shown.len(), 2, "the same document in both panes: {shown:?}");
        assert!(shown.iter().any(|r| r.right() < mid) && shown.iter().any(|r| r.left() > mid));

        // 分頁列選另一個分頁：顯示在有焦點的右欄，左欄留著原本的文件
        app.active_tab_idx = 1;
        app.focus_editor = true;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let alpha = find(&out, "alpha one").expect("the left pane still shows the first tab");
        let beta = find(&out, "beta one").expect("the right pane shows the selected tab");
        assert!(alpha.right() < mid && beta.left() > mid, "{alpha:?} {beta:?}");
        // 打字進右欄的文件
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[1].edit_text(), "beta one\nbeta two\nX", "typed at the cursor of the right pane");
        assert_eq!(app.tabs[0].edit_text(), "alpha one\nalpha two\n");

        // 點左欄：切換成編輯左欄的文件，右欄改成唯讀顯示剛才的文件
        let at = find(&out, "alpha two").unwrap();
        click(&mut app, &ctx, Pos2::new(at.left() + 20.0, at.center().y));
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.active_tab_idx, 0);
        let alpha = find(&out, "alpha one").unwrap();
        let beta = find(&out, "beta one").expect("the right pane keeps the other document");
        assert!(alpha.right() < mid && beta.left() > mid, "the panes stay where they were: {alpha:?} {beta:?}");
        frame(&mut app, &ctx, vec![Event::Text("Y".into())]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.tabs[0].lines.iter().any(|l| l.contains('Y')), "{:?}", app.tabs[0].lines);
        assert_eq!(app.tabs[1].edit_text(), "beta one\nbeta two\nX");

        // 關掉另一欄的分頁：分割結束
        let other = app.tabs[1].id;
        app.tabs[1].mark_saved();
        app.finish_close(&[other], false);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.split.is_none());
        assert!(find(&out, "beta one").is_none());

        // 再分割一次後 Ctrl+\ 關閉：只剩一個編輯區
        frame(&mut app, &ctx, vec![key(Key::Backslash, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.split.is_some());
        frame(&mut app, &ctx, vec![key(Key::Backslash, Modifiers::COMMAND)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.split.is_none());
        let n = painted_lines(&out).into_iter().filter(|l| l.0.contains("ha two")).count();
        assert_eq!(n, 1);
    }

    #[test]
    fn split_view_click_places_the_cursor_and_mirrors_typing() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        let mid = 640.0;
        let galley = |out: &egui::FullOutput, text: &str| {
            let g = painted_galleys(out).into_iter().find(|g| g.job.text == text)?;
            let r = painted_lines(out).into_iter().find(|l| l.0 == text)?.1;
            Some((g, r))
        };

        // 同一份文件在兩欄：右欄打的字左欄馬上看得到
        frame(&mut app, &ctx, vec![key(Key::Backslash, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        app.pending_select = Some((0, 0));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![Event::Text("Q".into())]);
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[0], "Qalpha one");
        let (_, r) = galley(&out, "Qalpha one").expect("the read-only pane shows the typed text");
        assert!(r.right() < mid, "{r:?}");

        // 右欄換成另一個分頁，點左欄第 2 行的第 3 個字之後：游標放在那裡，打的字插在那裡
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let (g, r) = galley(&out, "alpha two").expect("the left pane shows the first tab");
        let p = Pos2::new(r.left() + g.rows[0].x_offset((3).into()) + 1.0, r.center().y);
        click(&mut app, &ctx, p);
        assert_eq!(app.active_tab_idx, 0);
        assert_eq!(cursor(&app), (1, 3), "the cursor goes where the pane was clicked");
        frame(&mut app, &ctx, vec![Event::Text("Z".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[1], "alpZha two");
        assert_eq!(app.tabs[1].edit_text(), "beta one\nbeta two\n");
    }

    #[test]
    fn split_view_other_pane_shows_marks_hits_and_cursor() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\nalpha three\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        // 第一份：第 1 行與 HEAD 不同、第 2 行有書籤、游標在第 2 行第 3 個字之後；尋找列找 "two"
        let id = app.tabs[0].id;
        app.git_head.insert(id, Some(Arc::new(vec!["alpha ONE".into(), "alpha two".into(), "alpha three".into(), String::new()])));
        app.tabs[0].toggle_bookmark(1);
        app.show_find_bar = true;
        app.find_bar_text = "two".into();
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        // 第一份已經在另一欄（不再由編輯區改寫游標）
        app.tabs[0].selections = vec![Selection::point(Position { line: 1, col: 3 })];
        let out = frame(&mut app, &ctx, vec![]);
        let mid = 640.0;
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        let shapes_of = |fill: Color32| {
            fn walk(shape: &Shape, fill: Color32, found: &mut Vec<Rect>) {
                match shape {
                    Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, found)),
                    Shape::Rect(r) if r.fill == fill => found.push(r.rect),
                    Shape::Circle(ci) if ci.fill == fill => found.push(Rect::from_center_size(ci.center, egui::vec2(ci.radius * 2.0, ci.radius * 2.0))),
                    _ => {}
                }
            }
            let mut found = Vec::new();
            out.shapes.iter().for_each(|s| walk(&s.shape, fill, &mut found));
            found.into_iter().filter(|r| r.right() < mid).collect::<Vec<_>>()
        };
        let line = |text: &str| painted_lines(&out).into_iter().find(|l| l.0 == text).map(|l| l.1).unwrap();
        let (one, two) = (line("alpha one"), line("alpha two"));
        assert!(one.right() < mid && two.right() < mid);

        // Git：第 1 行是「修改」的黃色條
        let warn = shapes_of(c.warn);
        assert!(warn.iter().any(|r| r.center().y > one.top() && r.center().y < one.bottom()), "modified bar on line 1: {warn:?}");
        assert!(!warn.iter().any(|r| r.center().y > two.top() && r.center().y < two.bottom()));
        // 書籤：第 2 行的行號區有一個點
        let marks = shapes_of(c.accent2);
        assert!(marks.iter().any(|r| r.center().y > two.top() && r.center().y < two.bottom() && r.right() < two.left()), "{marks:?}");
        // 尋找命中：左欄的 "two" 有底色（右欄的文件也有一個）
        let hits = hit_texts(&out, c.search_bg);
        assert!(hits.iter().filter(|h| h.as_str() == "two").count() >= 2, "{hits:?}");
        // 游標：第 2 行第 3 個字之後一條細線
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text == "alpha two").unwrap();
        let x = two.left() + g.rows[0].x_offset((3).into());
        let carets = shapes_of(c.fg_dim);
        assert!(carets.iter().any(|r| (r.left() - x).abs() < 1.5 && r.width() < 3.0 && r.center().y > two.top() && r.center().y < two.bottom()), "caret at {x}: {carets:?}");
    }

    #[test]
    fn split_view_other_pane_shows_diagnostics() {
        use serde_json::json;
        use std::io::{Read, Write};
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.rs").to_string_lossy().into_owned();
        let mut app = app_with(&ctx, "fn nope() {}\nlet 😀x = 1;\nfn fine() {}\n");
        app.tabs[0].file_path = Some(path.clone());
        let mut b = EditorBuffer::from_text("plain one\nplain two\n");
        b.file_path = Some(dir.path().join("b.txt").to_string_lossy().into_owned());
        app.tabs.push(b);
        // 假的伺服器：收到 didOpen 就送出第 1 行 `nope` 的錯誤與第 2 行 `x` 的警告
        let launcher: Launcher = Box::new(move |_, _| {
            let (client_read, mut server_write) = std::io::pipe()?;
            let (mut server_read, client_write) = std::io::pipe()?;
            std::thread::spawn(move || {
                let mut frames = crate::lsp::Reader::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = server_read.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    frames.feed(&buf[..n]);
                    for m in frames.take_messages() {
                        if m["method"] == "initialize" {
                            let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": {} } });
                            let _ = server_write.write_all(&crate::lsp::frame(&reply));
                        }
                        if m["method"] == "textDocument/didOpen" {
                            let note = json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {
                                "uri": m["params"]["textDocument"]["uri"],
                                "diagnostics": [
                                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } },
                                      "severity": 1, "message": "cannot find function `nope`" },
                                    { "range": { "start": { "line": 1, "character": 6 }, "end": { "line": 1, "character": 7 } },
                                      "severity": 2, "message": "unused variable: `x`" }
                                ] } });
                            let _ = server_write.write_all(&crate::lsp::frame(&note));
                        }
                    }
                }
            });
            Ok(crate::lsp::LspClient::from_io(client_read, client_write))
        });
        app.lsp = LspManager::new(dir.path().to_path_buf(), launcher);
        let end = Instant::now() + Duration::from_secs(10);
        while app.lsp.diagnostics(&path).len() < 2 && Instant::now() < end {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.lsp.diagnostics(&path).len(), 2, "the diagnostics arrived");

        // 分割後編輯右欄的純文字檔：有診斷的那一份在左欄唯讀顯示
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let mid = 640.0;
        let left_num = |n: &str| {
            let rects: Vec<Rect> = painted_lines(&out).into_iter().filter(|l| l.0 == n && l.1.right() < mid).map(|l| l.1).collect();
            painted_galleys(&out)
                .into_iter()
                .zip(painted_lines(&out))
                .find(|(g, l)| g.job.text == n && rects.contains(&l.1))
                .map(|(g, _)| g.job.sections[0].format.color)
        };
        assert_eq!(left_num("1"), Some(DIAG_ERROR), "the line number of the error line");
        assert_eq!(left_num("2"), Some(DIAG_WARNING));
        assert!(!matches!(left_num("3"), Some(c) if c == DIAG_ERROR || c == DIAG_WARNING));
        let (g, text) = painted_galleys(&out)
            .into_iter()
            .zip(painted_lines(&out))
            .find(|(g, l)| g.job.text == "fn nope() {}" && l.1.right() < mid)
            .map(|(g, l)| (g, l.1))
            .expect("the first line in the left pane");
        let paths = painted_paths(&out);
        let err: Vec<_> = paths.iter().filter(|p| p.0 == DIAG_ERROR).collect();
        assert_eq!(err.len(), 1, "one error squiggle: {paths:?}");
        let r = err[0].1;
        let (x3, x7) = (text.left() + g.rows[0].x_offset((3).into()), text.left() + g.rows[0].x_offset((7).into()));
        assert!((r.left() - x3).abs() < 1.0 && (r.right() - x7).abs() < 1.0, "{r:?} vs {x3}..{x7}");
        assert!(r.top() > text.top() + text.height() * 0.6 && r.bottom() <= text.bottom() + 1.0, "under the line: {r:?} {text:?}");
        assert_eq!(paths.iter().filter(|p| p.0 == DIAG_WARNING).count(), 1);
    }

    #[test]
    fn split_view_switching_panes_keeps_the_scroll_position() {
        let ctx = ctx();
        let text: String = (0..300).map(|i| format!("line {i:03}\n")).collect();
        let mut app = app_with(&ctx, &text);
        let mut b = EditorBuffer::from_text(&(0..300).map(|i| format!("other {i:03}\n")).collect::<String>());
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let mid = 640.0;

        // 捲動左欄（唯讀的第一份）
        frame(&mut app, &ctx, vec![Event::PointerMoved(Pos2::new(300.0, 400.0)), Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -1500.0), phase: egui::TouchPhase::Move, source: egui::MouseWheelSource::Wheel, modifiers: Modifiers::NONE }]);
        for _ in 0..40 {
            frame(&mut app, &ctx, vec![]);
        }
        let out = frame(&mut app, &ctx, vec![]);
        // 左欄畫面中間附近的一行
        let (name, at) = painted_lines(&out)
            .into_iter()
            .filter(|l| l.0.starts_with("line ") && l.0.len() == 8 && l.1.right() < mid && l.1.top() > 300.0)
            .min_by(|a, b| a.1.top().total_cmp(&b.1.top()))
            .map(|l| (l.0, l.1))
            .expect("a line in the middle of the left pane");
        let ln: usize = name[5..].parse().unwrap();
        assert!(ln > 20, "the left pane scrolled: {name}");

        // 點那一行：換成編輯第一份，那一行留在畫面上同一個位置，游標在那一行
        click(&mut app, &ctx, Pos2::new(at.left() + 20.0, at.center().y));
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.active_tab_idx, 0);
        assert_eq!(cursor(&app).0, ln);
        let (g, r) = painted_galleys(&out)
            .into_iter()
            .zip(painted_lines(&out))
            .find(|(g, _)| g.job.text.starts_with("line 000\n"))
            .map(|(g, l)| (g, l.1))
            .expect("the editor of the first tab");
        assert!(r.left() < mid);
        let y = r.top() + g.rows[ln].rect().top();
        assert!((y - at.top()).abs() < 1.5, "line {ln} stays at {} (now {y})", at.top());
        // 第二份移到右欄唯讀顯示，捲動位置也和它剛才在編輯區時相同（在最上面）
        let first_right = painted_lines(&out)
            .into_iter()
            .filter(|l| l.0.starts_with("other ") && l.1.left() > mid)
            .min_by(|a, b| a.1.top().total_cmp(&b.1.top()))
            .map(|l| l.0)
            .unwrap();
        assert_eq!(first_right, "other 000");
    }

    #[test]
    fn split_view_drag_in_the_other_pane_selects_text() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\nalpha three\nalpha four\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let at = |text: &str, col: usize| {
            let (g, r) = painted_galleys(&out)
                .into_iter()
                .zip(painted_lines(&out))
                .find(|(g, l)| g.job.text == text && l.1.right() < 640.0)
                .map(|(g, l)| (g, l.1))
                .unwrap();
            Pos2::new(r.left() + g.rows[0].x_offset((col).into()) + 1.0, r.center().y)
        };
        let (from, to) = (at("alpha two", 2), at("alpha four", 3));
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        // 在左欄（唯讀的第一份）按下、拖到兩行以下再放開：一次就換過去並選起來
        for events in [
            vec![Event::PointerMoved(from)],
            vec![button(from, true)],
            vec![Event::PointerMoved(Pos2::new(from.x + 30.0, from.y + 10.0))],
            vec![Event::PointerMoved(to)],
            vec![button(to, false)],
        ] {
            frame(&mut app, &ctx, events);
        }
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.active_tab_idx, 0, "pressing in the other pane switches to it");
        let out = frame(&mut app, &ctx, vec![Event::Copy]);
        assert_eq!(copied_text(&out), "pha two\nalpha three\nalp");
        assert_eq!(cursor(&app), (3, 3), "the cursor is where the drag ended");
        // 之後在同一欄照常點擊（一般編輯區；點在別的位置，免得被當成連點）：選取取消，打的字插在點的位置
        let elsewhere = at("alpha three", 4);
        click(&mut app, &ctx, elsewhere);
        assert_eq!(cursor(&app), (2, 4));
        frame(&mut app, &ctx, vec![Event::Text("Q".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].edit_text(), "alpha one\nalpha two\nalphQa three\nalpha four\n");
    }

    #[test]
    fn markdown_reading_mode_centres_the_text_column() {
        // 視窗很寬時版心（最寬 820 px）置中，左右各留下超過 127 px 的空白（egui 0.31 起 Margin 只到 127）
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        let mut md = EditorBuffer::from_text("# Big Title\n\nplain paragraph words\n");
        md.file_path = Some("notes.md".into());
        app.tabs = vec![md];
        app.active_tab_idx = 0;
        app.md_view.mode = MdViewMode::Typora;
        // 2000 px 寬的視窗：文件區扣掉大綱側邊欄後仍遠比 820 px 寬，置中的留白一定超過 127 px
        let wide = |input: &mut RawInput| input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(2000.0, 800.0)));
        frame_with(&mut app, &ctx, wide);
        let out = frame_with(&mut app, &ctx, wide);
        let para = painted_lines(&out).into_iter().find(|l| l.0 == "plain paragraph words").expect("the paragraph is drawn").1;
        let title = painted_lines(&out).into_iter().find(|l| l.0 == "Big Title").expect("the title is drawn").1;
        assert!((para.left() - title.left()).abs() < 1.0, "title and paragraph share the left edge: {title:?} {para:?}");
        // 文件區從 x = 220 開始、寬 1780：置中時左右各留 480 px，版心從 700 開始
        // （留白若被限制在 127 px，會從 347 開始）
        assert!((para.left() - 700.0).abs() < 2.0, "the 820 px column is centred: {para:?}");
    }

    #[test]
    fn split_view_other_pane_shows_markdown_in_reading_mode() {
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        let mut md = EditorBuffer::from_text("# Big Title\n\nSome **bold** words\n");
        md.file_path = Some("notes.md".into());
        let mut b = EditorBuffer::from_text("fn main() {}\n");
        b.file_path = Some("b.rs".into());
        app.tabs = vec![md, b];
        app.active_tab_idx = 0;
        app.md_view.mode = MdViewMode::Typora;
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let left: Vec<String> = painted_lines(&out).into_iter().filter(|l| l.1.right() < 640.0).map(|l| l.0).collect();
        assert!(left.iter().any(|t| t.contains("Big Title")), "{left:?}");
        assert!(!left.iter().any(|t| t.contains("# Big") || t.contains("**")), "rendered, not the source: {left:?}");
        // 原始碼模式時另一欄也顯示原始碼
        app.md_view.mode = MdViewMode::Source;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let left: Vec<String> = painted_lines(&out).into_iter().filter(|l| l.1.right() < 640.0).map(|l| l.0).collect();
        assert!(left.iter().any(|t| t == "# Big Title"), "{left:?}");
        // 閱讀模式的另一欄也是點一下就換過去編輯它（點在文字上）
        app.md_view.mode = MdViewMode::Typora;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let words = painted_lines(&out).into_iter().find(|l| l.0.contains("words") && l.1.right() < 640.0).expect("the paragraph").1;
        click(&mut app, &ctx, words.center());
        assert_eq!(app.active_tab_idx, 0);
    }

    #[test]
    fn split_view_divider_can_be_dragged() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let pane_left = |out: &egui::FullOutput, start: &str| {
            painted_lines(out).into_iter().find(|l| l.0.starts_with(start)).map(|l| l.1.left()).unwrap()
        };
        let right0 = pane_left(&out, "beta one");
        // 分隔線在兩欄之間：右欄文字左邊再往左一點（行號區與邊界）
        let divider_x = app.split_divider_x().expect("the divider");
        assert!(divider_x < right0 && right0 - divider_x < 80.0, "{divider_x} vs {right0}");
        let y = 400.0;
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        let (from, to) = (Pos2::new(divider_x, y), Pos2::new(400.0, y));
        for events in [
            vec![Event::PointerMoved(from)],
            vec![button(from, true)],
            vec![Event::PointerMoved(Pos2::new(500.0, y))],
            vec![Event::PointerMoved(to)],
            vec![button(to, false)],
        ] {
            frame(&mut app, &ctx, events);
        }
        let out = frame(&mut app, &ctx, vec![]);
        let x = app.split_divider_x().unwrap();
        assert!((x - 400.0).abs() < 2.0, "the divider follows the drag: {x}");
        assert_eq!(app.active_tab_idx, 1, "dragging the divider does not switch panes");
        assert!(pane_left(&out, "beta one") < right0 - 200.0, "the right pane got wider");
        assert!(painted_lines(&out).into_iter().filter(|l| l.0 == "alpha one").all(|l| l.1.right() <= 401.0));
        // 拖到很邊邊時兩欄都留一點寬度
        let far = Pos2::new(5.0, y);
        for events in [vec![Event::PointerMoved(Pos2::new(x, y))], vec![button(Pos2::new(x, y), true)], vec![Event::PointerMoved(far)], vec![button(far, false)]] {
            frame(&mut app, &ctx, events);
        }
        frame(&mut app, &ctx, vec![]);
        assert!(app.split_divider_x().unwrap() > 100.0);
    }

    #[test]
    fn split_view_is_saved_and_restored() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| {
            let p = dir.path().join(name);
            std::fs::write(&p, text).unwrap();
            p
        };
        let (a, b) = (write("a.rs", "fn a() {}"), write("b.rs", "fn b() {}"));
        let state_file = dir.path().join("cfg").join("state.json");
        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&a);
        app.open_path(&b);
        let ia = app.tabs.iter().position(|t| t.file_path.as_deref() == Some(a.to_str().unwrap())).unwrap();
        let ib = app.tabs.iter().position(|t| t.file_path.as_deref() == Some(b.to_str().unwrap())).unwrap();
        // b 在左欄（有焦點），a 在右欄（另一欄），左欄佔 30%
        app.active_tab_idx = ia;
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = ib;
        if let Some(s) = app.split.as_mut() {
            s.right_active = false;
            s.ratio = 0.3;
            s.vertical = true;
        }
        app.persist_state();

        let mut app2 = AlexCodeApp::new();
        app2.apply_state(AppState::load(&state_file).unwrap());
        let s = app2.split.as_ref().expect("the split is restored");
        let other = app2.tabs.iter().find(|t| t.id == s.other).and_then(|t| t.file_path.clone());
        assert_eq!(other.as_deref(), a.to_str(), "the other pane shows a.rs again");
        assert_eq!(app2.tabs[app2.active_tab_idx].file_path.as_deref(), b.to_str());
        assert!(!s.right_active);
        assert!((s.ratio - 0.3).abs() < 1e-6);
        assert!(s.vertical, "top / bottom split is restored");

        // 不分割時存的檔不還原分割；另一欄的檔案已經不存在時也不還原
        app.run_cmd(Cmd::SplitEditor);
        assert!(app.split.is_none());
        app.persist_state();
        let mut app3 = AlexCodeApp::new();
        app3.apply_state(AppState::load(&state_file).unwrap());
        assert!(app3.split.is_none());
        app.run_cmd(Cmd::SplitEditor);
        app.persist_state();
        std::fs::remove_file(&b).unwrap();
        let mut app4 = AlexCodeApp::new();
        app4.apply_state(AppState::load(&state_file).unwrap());
        assert!(app4.split.is_none(), "the other pane's file is gone");
    }

    #[test]
    fn split_view_marks_the_other_panes_tab() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "fn a() {}\n");
        let mut b = EditorBuffer::from_text("fn b() {}\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        let marks = |out: &egui::FullOutput| {
            fn walk(shape: &Shape, col: Color32, found: &mut Vec<(f32, f32, f32)>) {
                match shape {
                    Shape::Vec(v) => v.iter().for_each(|s| walk(s, col, found)),
                    Shape::LineSegment { points, stroke } if stroke.color == col && (stroke.width - 1.0).abs() < 0.01 => {
                        found.push((points[0].x.min(points[1].x), points[0].x.max(points[1].x), points[0].y))
                    }
                    _ => {}
                }
            }
            let mut found = Vec::new();
            out.shapes.iter().for_each(|s| walk(&s.shape, c.accent, &mut found));
            found
        };
        let tab = |out: &egui::FullOutput, name: &str| painted_lines(out).into_iter().find(|l| l.0 == name).unwrap().1;
        let under = |m: &(f32, f32, f32), r: Rect| m.0 <= r.left() && m.1 >= r.right() && m.2 > r.bottom() && m.2 < r.bottom() + 12.0;

        let out = frame(&mut app, &ctx, vec![]);
        assert!(marks(&out).is_empty(), "no split, no mark");
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let m = marks(&out);
        assert!(m.iter().any(|m| under(m, tab(&out, "t.rs"))), "the other pane's tab is underlined: {m:?}");
        assert!(!m.iter().any(|m| under(m, tab(&out, "b.rs"))), "the focused tab keeps its own mark");
        app.run_cmd(Cmd::SplitEditor);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(marks(&out).is_empty());
    }

    #[test]
    fn split_view_can_stack_the_panes_vertically() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        frame(&mut app, &ctx, vec![]);
        app.run_cmd(Cmd::SplitEditor);
        app.active_tab_idx = 1;
        // 切成上下分割：上面是另一欄（第一份），下面是有焦點的編輯區（第二份），左右都是整個寬度
        app.run_cmd(Cmd::SplitOrientation);
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let mid = app.split_divider_x().expect("the divider");
        let alpha = painted_lines(&out).into_iter().find(|l| l.0 == "alpha one").unwrap().1;
        let beta = painted_lines(&out).into_iter().find(|l| l.0.starts_with("beta one")).unwrap().1;
        assert!(alpha.bottom() < mid && beta.top() > mid, "top / bottom: {alpha:?} {beta:?} at {mid}");
        assert!(alpha.left() < 200.0 && beta.left() < 200.0, "both start at the left edge");
        // 拖曳分隔線改高度
        let x = 600.0;
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        let (from, to) = (Pos2::new(x, mid), Pos2::new(x, mid - 120.0));
        for events in [vec![Event::PointerMoved(from)], vec![button(from, true)], vec![Event::PointerMoved(Pos2::new(x, mid - 60.0))], vec![Event::PointerMoved(to)], vec![button(to, false)]] {
            frame(&mut app, &ctx, events);
        }
        frame(&mut app, &ctx, vec![]);
        let m2 = app.split_divider_x().unwrap();
        assert!((m2 - (mid - 120.0)).abs() < 2.0, "the divider follows the drag: {m2}");
        assert_eq!(app.active_tab_idx, 1);
        // 點上面那一欄：換成編輯第一份，位置不變
        let out = frame(&mut app, &ctx, vec![]);
        let alpha = painted_lines(&out).into_iter().find(|l| l.0 == "alpha two").unwrap().1;
        click(&mut app, &ctx, Pos2::new(alpha.left() + 10.0, alpha.center().y));
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.active_tab_idx, 0);
        let alpha = painted_lines(&out).into_iter().find(|l| l.0.starts_with("alpha one\n")).unwrap().1;
        let beta = painted_lines(&out).into_iter().find(|l| l.0 == "beta one").unwrap().1;
        assert!(alpha.top() < m2 && beta.top() > m2);
        // 再切一次回到左右分割
        app.run_cmd(Cmd::SplitOrientation);
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let beta = painted_lines(&out).into_iter().find(|l| l.0 == "beta one").unwrap().1;
        // （寬度比例沿用剛才拖過的高度比例）
        let x = app.split_divider_x().unwrap();
        assert!(beta.left() > x && beta.top() < m2 - 100.0, "side by side again: {beta:?} at {x}");
    }

    #[test]
    fn a_tab_can_be_shown_in_its_own_window() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\nbeta two\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        app.active_tab_idx = 1;
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!painted_lines(&out).iter().any(|l| l.0 == "alpha one"), "tab 1 is not shown anywhere yet");
        // 分頁右鍵「在新視窗檢視」：沒有多視窗的環境（測試）裡畫成一個內嵌的視窗
        app.run_tab_action(&ctx, TabAction::Detach(0));
        app.run_tab_action(&ctx, TabAction::Detach(0));
        assert_eq!(app.detached.len(), 1, "one window per tab");
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let texts: Vec<String> = painted_lines(&out).into_iter().map(|l| l.0).collect();
        assert!(texts.iter().any(|t| t == "alpha one"), "the window shows the tab: {texts:?}");
        assert!(texts.iter().any(|t| t == "t.rs — AlexCode"), "the window title names the file");
        assert!(texts.iter().any(|t| t.starts_with("beta one")), "the main editor still shows its tab");
        // 內容改了視窗馬上跟著變
        let top = Position { line: 0, col: 0 };
        app.tabs[0].replace_range(top, top, "NEW ", false);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(painted_lines(&out).iter().any(|l| l.0 == "NEW alpha one"));
        // 分頁關掉時視窗也關掉
        let id = app.tabs[0].id;
        app.tabs[0].mark_saved();
        app.finish_close(&[id], false);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.detached.is_empty());
        assert!(!painted_lines(&out).iter().any(|l| l.0.contains("alpha one")));
    }

    #[test]
    fn the_detached_window_shows_git_marks() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha ONE\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        app.active_tab_idx = 1;
        let id = app.tabs[0].id;
        app.git_head.insert(id, Some(Arc::new(vec!["alpha one".into(), "alpha two".into(), String::new()])));
        app.run_tab_action(&ctx, TabAction::Detach(0));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        let line = |text: &str| painted_lines(&out).into_iter().find(|l| l.0 == text).map(|l| l.1).unwrap();
        let (one, two) = (line("alpha ONE"), line("alpha two"));
        fn walk(shape: &Shape, fill: Color32, found: &mut Vec<Rect>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, found)),
                Shape::Rect(r) if r.fill == fill => found.push(r.rect),
                _ => {}
            }
        }
        let mut warn = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, c.warn, &mut warn));
        let on = |r: &Rect, l: Rect| r.center().y > l.top() && r.center().y < l.bottom() && r.right() < l.left();
        assert!(warn.iter().any(|r| on(r, one)), "the changed line has a modified bar: {warn:?}");
        assert!(!warn.iter().any(|r| on(r, two)));
    }

    #[test]
    fn the_detached_window_can_edit_the_shared_buffer() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        app.active_tab_idx = 1;
        app.run_tab_action(&ctx, TabAction::Detach(0));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let two = painted_lines(&out).into_iter().find(|l| l.0 == "alpha two").map(|l| l.1).unwrap();
        // 在視窗裡點一下：游標放到點的位置，主視窗的編輯區交出焦點
        click(&mut app, &ctx, Pos2::new(two.left() + 1.0, two.center().y));
        assert_eq!(cursor(&app), (1, 0), "the click puts the cursor at the start of line 2");
        assert!(!app.editor_focused, "the main editor gives up the keyboard");
        // 打字改的是那個分頁（與主視窗共用的同一份緩衝區），主視窗的分頁不動
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!(app.tabs[0].lines[1], "Xalpha two");
        assert_eq!(cursor(&app), (1, 1));
        assert_eq!(app.tabs[1].lines[0], "beta one");
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].lines[1], "alpha two");
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].lines[1..3], ["", "alpha two"]);
        assert_eq!(cursor(&app), (2, 0));
        // 方向鍵與 Home / End 移動游標
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE), key(Key::ArrowUp, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (0, 0));
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::NONE)]);
        assert_eq!(cursor(&app), (0, 9));
        frame(&mut app, &ctx, vec![key(Key::ArrowLeft, Modifiers::NONE), key(Key::Delete, Modifiers::NONE)]);
        assert_eq!(app.tabs[0].lines[0], "alpha on");
        // 復原用那份緩衝區的紀錄
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        assert_ne!(app.tabs[0].lines[0], "alpha on", "Ctrl+Z undoes the edit made in the window");
        let out = frame(&mut app, &ctx, vec![]);
        let texts: Vec<String> = painted_lines(&out).into_iter().map(|l| l.0).collect();
        assert!(texts.iter().any(|t| t == &app.tabs[0].lines[0]), "the window shows the buffer: {texts:?}");
        assert_eq!(app.active_tab_idx, 1, "the main window stays on its own tab");
    }

    #[test]
    fn the_detached_window_selects_copies_and_pastes() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\nalpha two\n");
        let mut b = EditorBuffer::from_text("beta one\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        app.active_tab_idx = 1;
        app.run_tab_action(&ctx, TabAction::Detach(0));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let one = painted_lines(&out).into_iter().find(|l| l.0 == "alpha one").map(|l| l.1).unwrap();
        click(&mut app, &ctx, Pos2::new(one.left() + 1.0, one.center().y));
        assert_eq!(cursor(&app), (0, 0));
        // Shift+End / Shift+方向鍵擴充選取；Ctrl+C 複製選到的文字
        let shift = |k| key(k, Modifiers::SHIFT);
        frame(&mut app, &ctx, vec![shift(Key::End)]);
        assert_eq!(copied_text(&frame(&mut app, &ctx, vec![Event::Copy])), "alpha one");
        frame(&mut app, &ctx, vec![shift(Key::ArrowDown)]);
        assert_eq!(app.tabs[0].selections[0].anchor, Position { line: 0, col: 0 }, "the anchor stays put");
        assert_eq!(copied_text(&frame(&mut app, &ctx, vec![Event::Copy])), "alpha one\nalpha two");
        // 選取的範圍畫出底色
        let out = frame(&mut app, &ctx, vec![]);
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        fn walk(shape: &Shape, fill: Color32, found: &mut Vec<Rect>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, found)),
                Shape::Rect(r) if r.fill == fill => found.push(r.rect),
                _ => {}
            }
        }
        let mut sel = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, c.sel_bg, &mut sel));
        assert!(sel.iter().any(|r| r.center().y > one.top() && r.center().y < one.bottom()), "selection is shaded: {sel:?}");
        // 剪下、貼上
        let out = frame(&mut app, &ctx, vec![Event::Cut]);
        assert_eq!(copied_text(&out), "alpha one\nalpha two");
        assert_eq!(app.tabs[0].lines, ["", ""]);
        frame(&mut app, &ctx, vec![Event::Paste("hello".into())]);
        assert_eq!(app.tabs[0].lines[0], "hello");
        assert_eq!(cursor(&app), (0, 5));
        // 滑鼠拖曳選取
        let out = frame(&mut app, &ctx, vec![]);
        let hello = painted_lines(&out).into_iter().find(|l| l.0 == "hello").map(|l| l.1).unwrap();
        let (p0, p1) = (Pos2::new(hello.left() + 1.0, hello.center().y), Pos2::new(hello.right() + 30.0, hello.center().y));
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(&mut app, &ctx, vec![Event::PointerMoved(p0)]);
        frame(&mut app, &ctx, vec![button(p0, true)]);
        frame(&mut app, &ctx, vec![Event::PointerMoved(Pos2::new(p0.x + 20.0, p0.y))]);
        frame(&mut app, &ctx, vec![Event::PointerMoved(p1)]);
        frame(&mut app, &ctx, vec![button(p1, false)]);
        assert_eq!(copied_text(&frame(&mut app, &ctx, vec![Event::Copy])), "hello", "dragging selects");
        // 主視窗不會顯示拖出去的分頁（以前兩邊同時顯示時要讓主視窗的游標跟著走，現在不會發生）
        app.active_tab_idx = 0;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.active_tab_idx, 1, "the main window switches back to a tab that is not detached");
    }

    #[test]
    fn dragging_a_tab_out_of_the_tab_bar_opens_its_window() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\n");
        let mut b = EditorBuffer::from_text("beta one\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        let out = frame(&mut app, &ctx, vec![]);
        let tab = |out: &egui::FullOutput, name: &str| painted_lines(out).into_iter().find(|l| l.0 == name).map(|l| l.1).unwrap();
        let (t0, t1) = (tab(&out, "t.rs"), tab(&out, "b.rs"));
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        let drag = |app: &mut AlexCodeApp, from: Pos2, to: Pos2| {
            frame(app, &ctx, vec![Event::PointerMoved(from)]);
            frame(app, &ctx, vec![button(from, true)]);
            frame(app, &ctx, vec![Event::PointerMoved(from + (to - from) * 0.5)]);
            let out = frame(app, &ctx, vec![Event::PointerMoved(to)]);
            frame(app, &ctx, vec![button(to, false)]);
            frame(app, &ctx, vec![]);
            out
        };
        // 在分頁列裡拖一小段：不開視窗
        drag(&mut app, t1.center(), t1.center() + egui::vec2(30.0, 2.0));
        assert!(app.detached.is_empty(), "a short drag inside the tab bar does nothing");
        // 拖到分頁列外面放開：那個分頁開成獨立視窗；拖的時候指標旁邊跟著分頁名稱
        let out = drag(&mut app, t0.center(), Pos2::new(t0.center().x + 200.0, 400.0));
        assert!(
            painted_lines(&out).iter().any(|l| l.0 == "t.rs" && l.1.top() > t0.bottom() + 100.0),
            "the dragged tab's name follows the pointer"
        );
        assert_eq!(app.detached, vec![app.tabs[0].id]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(painted_lines(&out).iter().any(|l| l.0 == "t.rs — AlexCode"), "the window is shown");
        assert_eq!(app.tabs.len(), 2, "the tab stays in the main window too");
    }

    #[test]
    fn a_detached_tab_leaves_the_main_tab_bar_and_its_window_has_the_menu() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "alpha one\n");
        let mut b = EditorBuffer::from_text("beta one\n");
        b.file_path = Some("b.rs".into());
        app.tabs.push(b);
        app.active_tab_idx = 0;
        frame(&mut app, &ctx, vec![]);
        let names = |out: &egui::FullOutput| painted_lines(out).into_iter().map(|l| l.0).collect::<Vec<_>>();
        let (t_id, b_id) = (app.tabs[0].id, app.tabs[1].id);

        // 把正在看的分頁拖出去：主視窗的分頁列不再有它，主視窗改顯示另一個分頁
        app.run_tab_action(&ctx, TabAction::Detach(0));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let shown = names(&out);
        assert!(shown.iter().any(|t| t == "t.rs — AlexCode"), "the window is shown");
        assert!(!shown.iter().any(|t| t == "t.rs"), "the detached tab is gone from the main tab bar: {shown:?}");
        assert!(shown.iter().any(|t| t == "b.rs"));
        assert_eq!(app.tabs[app.active_tab_idx].id, b_id, "the main window shows the other tab");
        assert_eq!(app.tabs.len(), 2, "the buffer is shared, not closed");

        // 獨立視窗有和主視窗一樣的選單
        let menus: Vec<Rect> = painted_lines(&out).into_iter().filter(|l| l.0 == "檔案 (File)").map(|l| l.1).collect();
        assert_eq!(menus.len(), 2, "one menu bar in each window");
        // 選單的指令作用在獨立視窗的分頁：切換書籤加在 t.rs、主視窗仍顯示 b.rs
        app.run_cmd_on(Cmd::ToggleBookmark, Some(t_id));
        assert!(!app.tabs[0].bookmarks.is_empty() && app.tabs[1].bookmarks.is_empty());
        assert_eq!(app.tabs[app.active_tab_idx].id, b_id);
        // 用滑鼠從獨立視窗的選單關閉分頁：關掉的是 t.rs
        let file_menu = menus.iter().copied().max_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
        // 測試裡時間不會前進，彈出的選單淡入動畫永遠停在透明：關掉動畫
        let mut st = (*ctx.global_style()).clone();
        st.animation_time = 0.0;
        ctx.set_global_style(st);
        click(&mut app, &ctx, file_menu.center());
        let out = frame(&mut app, &ctx, vec![]);
        let close = painted_lines(&out).into_iter().find(|l| l.0 == "關閉分頁 (Close)").map(|l| l.1).expect("the File menu opened");
        assert!(close.top() > file_menu.top(), "the menu opened under the window's menu bar");
        click(&mut app, &ctx, close.center());
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs.iter().map(|t| t.id).collect::<Vec<_>>(), vec![b_id], "the detached tab was closed");
        assert!(app.detached.is_empty());

        // 主視窗的分頁全部拖出去：主視窗開一個新的空白分頁
        app.run_tab_action(&ctx, TabAction::Detach(0));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs.len(), 2);
        assert!(!app.detached.contains(&app.tabs[app.active_tab_idx].id), "the main window never shows a detached tab");

        // 關掉獨立視窗：分頁回到主視窗
        app.detached.clear();
        let out = frame(&mut app, &ctx, vec![]);
        assert!(names(&out).iter().any(|t| t == "b.rs"), "the tab is back in the main tab bar");
    }

    #[test]
    fn detached_windows_are_saved_and_restored() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| {
            let p = dir.path().join(name);
            std::fs::write(&p, text).unwrap();
            p
        };
        let (a, b) = (write("a.rs", "fn a() {}"), write("b.rs", "fn b() {}"));
        let state_file = dir.path().join("cfg").join("state.json");
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        app.state_path = Some(state_file.clone());
        app.open_path(&a);
        app.open_path(&b);
        let ib = app.tabs.iter().position(|t| t.file_path.as_deref() == Some(b.to_str().unwrap())).unwrap();
        app.run_tab_action(&ctx, TabAction::Detach(ib));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let title = |out: &egui::FullOutput| painted_lines(out).into_iter().find(|l| l.0 == "b.rs — AlexCode").map(|l| l.1);
        let t0 = title(&out).expect("the window is shown");
        // 拖標題列把視窗移到別的地方
        let (from, to) = (t0.center(), t0.center() + egui::vec2(150.0, 120.0));
        let button = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(&mut app, &ctx, vec![Event::PointerMoved(from)]);
        frame(&mut app, &ctx, vec![button(from, true)]);
        frame(&mut app, &ctx, vec![Event::PointerMoved(from + egui::vec2(75.0, 60.0))]);
        frame(&mut app, &ctx, vec![Event::PointerMoved(to)]);
        frame(&mut app, &ctx, vec![button(to, false)]);
        let out = frame(&mut app, &ctx, vec![]);
        let t1 = title(&out).unwrap();
        assert!((t1.center() - to).length() < 2.0, "the window moved: {t0:?} -> {t1:?}");
        app.persist_state();
        let st = AppState::load(&state_file).unwrap();
        assert_eq!(st.detached.len(), 1);
        assert_eq!(st.detached[0].file, b.to_str().unwrap());

        // 重開：同一個檔案又開在獨立視窗、位置相同
        let ctx2 = self::ctx();
        let mut app2 = AlexCodeApp::new();
        app2.apply_state(st);
        let ib2 = app2.tabs.iter().position(|t| t.file_path.as_deref() == Some(b.to_str().unwrap())).unwrap();
        assert_eq!(app2.detached, vec![app2.tabs[ib2].id], "the window is reopened");
        frame(&mut app2, &ctx2, vec![]);
        let out = frame(&mut app2, &ctx2, vec![]);
        let t2 = title(&out).expect("the restored window is shown");
        assert!((t2.center() - t1.center()).length() < 2.0, "same place: {t1:?} vs {t2:?}");

        // 視窗關掉後存檔就不再記
        app.detached.clear();
        app.persist_state();
        assert!(AppState::load(&state_file).unwrap().detached.is_empty());
    }

    #[test]
    fn files_without_a_language_server_complete_words_from_the_document() {
        let ctx = ctx();
        let mut app = app_with(&ctx, "let helper_one = 1;\nlet helper_two = helper_one;\n\n");
        app.tabs[0].file_path = Some("t.js".into());
        let shown = |out: &egui::FullOutput, s: &str| painted_texts(out).iter().any(|t| t == s);
        // 游標放在第 3 行（空行）
        app.pending_select = Some((49, 49));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (2, 0));
        // 程式碼檔案：打到第 3 個字才自動列出文件裡的字（出現次數多的在前）
        frame(&mut app, &ctx, vec![Event::Text("h".into())]);
        let out = frame(&mut app, &ctx, vec![Event::Text("e".into())]);
        assert!(!shown(&out, "helper_one"), "two letters are not enough");
        frame(&mut app, &ctx, vec![Event::Text("l".into())]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one") && shown(&out, "helper_two"), "the menu lists words from the document: {:?}", painted_texts(&out));
        let y = |out: &egui::FullOutput, s: &str| painted_lines(out).into_iter().find(|l| l.0 == s).unwrap().1.top();
        assert!(y(&out, "helper_one") < y(&out, "helper_two"), "the word used more often comes first");
        // Enter 接受：打過的 "hel" 換成完整的字
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[2], "helper_one");
        assert_eq!(app.tabs[0].lines.len(), 4, "Enter accepted the item instead of breaking the line");
        assert!(!shown(&out, "helper_two"), "the menu closes after accepting");

        // 純文字檔：打字不自動跳出，Ctrl+Space 才列出
        // （換路徑後編輯區的 id 跟著變，用 pending_select 放游標順便把焦點交給它）
        app.tabs[0].file_path = Some("notes.txt".into());
        app.pending_select = Some((60, 60));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (3, 0));
        for ch in ["h", "e", "l"] {
            frame(&mut app, &ctx, vec![Event::Text(ch.into())]);
        }
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[3], "hel", "{:?} {:?}", app.tabs[0].lines, cursor(&app));
        assert!(!shown(&out, "helper_two"), "no automatic menu in plain text");
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one") && shown(&out, "helper_two"), "Ctrl+Space lists the words");
    }

    /// 127.0.0.1 上的假 OpenAI 相容端點：每個請求都回 `content`；回傳端點與收到的每個請求（標頭 + 內容）
    fn fake_ai_server(content: &'static str) -> (String, std::sync::mpsc::Receiver<String>) {
        use std::io::{Read, Write};
        use std::time::Duration;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { return };
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = s.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(end) = text.find("\r\n\r\n") {
                        let len = text[..end]
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
                            .unwrap_or(0);
                        if buf.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
                // 回覆前等一下（像真的模型一樣要花時間），測試才來得及在回覆前移動游標
                std::thread::sleep(Duration::from_millis(300));
                let body = serde_json::json!({ "choices": [ { "message": { "role": "assistant", "content": content } } ] }).to_string();
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = s.write_all(resp.as_bytes());
            }
        });
        (format!("http://127.0.0.1:{port}/v1/chat/completions"), rx)
    }

    #[test]
    fn remote_ai_suggestions_join_the_completion_menu() {
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let mut app = app_with(&ctx, "let helper_one = 1;\nlet total = hel\n");
        app.tabs[0].file_path = Some("t.js".into());
        let shown = |out: &egui::FullOutput, s: &str| painted_texts(out).iter().any(|t| t == s);
        let y = |out: &egui::FullOutput, s: &str| painted_lines(out).into_iter().find(|l| l.0 == s).unwrap().1.top();
        let pump = |app: &mut AlexCodeApp, what: &str, done: &dyn Fn(&AlexCodeApp, &egui::FullOutput) -> bool| {
            let end = Instant::now() + Duration::from_secs(10);
            loop {
                let out = frame(app, &ctx, vec![]);
                if done(app, &out) {
                    return out;
                }
                assert!(Instant::now() < end, "{what}; status {:?}; painted: {:?}", app.status_message, painted_texts(&out));
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let (endpoint, rx) = fake_ai_server("per_one + 41;\nconsole.log(total);");
        // 游標放在第 2 行行尾
        app.pending_select = Some((35, 35));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (1, 15));

        // 預設不啟用：Ctrl+Space 只列本機補全，不送任何請求
        assert!(!app.config.ai.enabled);
        app.config.ai.endpoint = endpoint.clone();
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one"), "local completion still works");
        assert!(rx.recv_timeout(Duration::from_millis(400)).is_err(), "nothing is sent while the remote provider is disabled");
        frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);

        // 啟用後：本機的選單馬上出現（不等回覆），回覆到了以後 AI 的建議排在第一項
        app.config.ai.enabled = true;
        app.config.ai.model = "test-model".into();
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one"), "the local items are listed right away");
        let out = pump(&mut app, "the AI suggestion appears", &|_, o| shown(o, "helper_one + 41; …"));
        assert!(y(&out, "helper_one + 41; …") < y(&out, "helper_one"), "the AI suggestion is the first item");
        let req = rx.recv_timeout(Duration::from_secs(5)).expect("a request was sent");
        assert!(req.contains("\"test-model\""), "{req}");
        assert!(req.contains("let total = hel<CURSOR>"), "the text around the cursor is sent: {req}");
        // Enter 接受：接在打過的字後面，多行的建議照樣插入
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[1], "let total = helper_one + 41;");
        assert_eq!(app.tabs[0].lines[2], "console.log(total);");
        assert_eq!(cursor(&app), (2, 19));

        // 回覆到之前游標移走了：丟掉這個建議
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        for ch in ["h", "e", "l"] {
            frame(&mut app, &ctx, vec![Event::Text(ch.into())]);
        }
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        frame(&mut app, &ctx, vec![Event::Text("x".into())]);
        rx.recv_timeout(Duration::from_secs(5)).expect("the second request was sent");
        let end = Instant::now() + Duration::from_millis(600);
        while Instant::now() < end {
            let out = frame(&mut app, &ctx, vec![]);
            assert!(!shown(&out, "helper_one + 41; …"), "a stale suggestion is not shown");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.tabs[0].lines[3], "helx");

        // 連不上：照舊只有本機補全，狀態列說明原因
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        app.config.ai.endpoint = format!("http://127.0.0.1:{port}/v1/chat/completions");
        frame(&mut app, &ctx, vec![key(Key::Backspace, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![key(Key::Space, Modifiers::CTRL)]);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(shown(&out, "helper_one"));
        let out = pump(&mut app, "the error is reported", &|a, _| a.status_message.contains("連不上"));
        assert!(app.status_message.starts_with("AI 補全"), "{}", app.status_message);
        assert!(shown(&out, "helper_one"), "the local menu stays open");
        frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[3], "helper_one");
    }

    #[test]
    fn the_settings_dialog_configures_the_remote_ai() {
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        app.show_settings = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        for name in ["settings.ai_enabled", "settings.ai_endpoint", "settings.ai_model", "settings.ai_key"] {
            assert!(app.probe.contains_key(name), "{name} is in the dialog");
        }
        let at = app.probe["settings.ai_enabled"].center();
        let click = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(&mut app, &ctx, vec![Event::PointerMoved(at), click(true)]);
        frame(&mut app, &ctx, vec![click(false)]);
        assert!(app.config.ai.enabled, "the checkbox turns the remote provider on");
        // 金鑰欄位不顯示金鑰本身
        app.config.ai.api_key = "sk-secret-123".into();
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!painted_texts(&out).iter().any(|t| t.contains("sk-secret")), "the key is masked");
    }

    /// 這一幀畫出的所有文字排版（含各段的顏色）
    fn painted_galleys(out: &egui::FullOutput) -> Vec<Arc<egui::Galley>> {
        fn walk(shape: &Shape, found: &mut Vec<Arc<egui::Galley>>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, found)),
                Shape::Text(t) => found.push(t.galley.clone()),
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, &mut found));
        found
    }

    #[test]
    fn terminal_shows_ansi_colors() {
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        app.term_pty_failed = true; // 不啟動 shell，直接把帶顏色的輸出餵給畫面模型
        app.show_terminal = true;
        frame(&mut app, &ctx, vec![]);
        app.terminal.feed(b"\x1b[31mRED\x1b[0m plain \x1b[1;32mBOLD\x1b[0m \x1b[44mONBLUE\x1b[0m\r\n");
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.contains("RED")).expect("the colored line is drawn");
        let section = |word: &str| {
            let at = g.job.text.find(word).unwrap();
            g.job.sections.iter().find(|s| s.byte_range.contains(&at.into())).unwrap().format.clone()
        };
        assert_eq!(section("RED").color, TERM_COLORS[1]);
        let fg = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&fg, app.theme_mgr.is_neon_hud());
        assert_eq!(section("plain").color, c.fg, "uncolored text uses the theme's text color");
        assert_eq!(section("BOLD").color, TERM_COLORS[10], "bold green shows as bright green");
        assert_eq!(section("ONBLUE").background, TERM_COLORS[4]);
        assert_eq!(section("plain").background, Color32::TRANSPARENT);
        // 捲出畫面的列也保留顏色
        for _ in 0..app.terminal.rows() {
            app.terminal.feed(b"\r\n");
        }
        let out = frame(&mut app, &ctx, vec![]);
        let g = painted_galleys(&out).into_iter().find(|g| g.job.text.contains("RED")).expect("the line is in the scrollback");
        assert!(app.terminal.scrollback_count() > 0);
        assert_eq!(g.job.sections[0].format.color, TERM_COLORS[1]);
    }

    #[test]
    fn terminal_follows_the_dock_size_and_draws_its_cursor() {
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = AlexCodeApp::new();
        app.term_cwd = dir.path().to_path_buf();
        let screen = |app: &AlexCodeApp| {
            let t = &app.terminal;
            let mut s: Vec<String> = (0..t.scrollback_count()).map(|i| t.scrollback_line(i)).collect();
            s.extend((0..t.rows()).map(|r| t.line_text(r)));
            s.join("\n")
        };
        let size = std::cell::Cell::new(egui::vec2(1280.0, 800.0));
        let run = |app: &mut AlexCodeApp, events: Vec<Event>| {
            frame_with(app, &ctx, |i| {
                i.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, size.get()));
                i.events = events;
            })
        };
        let wait = |app: &mut AlexCodeApp, cond: &dyn Fn(&str) -> bool| {
            let end = Instant::now() + Duration::from_secs(30);
            while Instant::now() < end {
                run(app, vec![]);
                if cond(&screen(app)) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        app.show_terminal = true;
        let mut seen = Vec::new();
        for _ in 0..12 {
            run(&mut app, vec![]);
            seen.push((app.probe["terminal-panel"].height(), app.terminal.rows(), app.terminal.cols()));
            std::thread::sleep(Duration::from_millis(50));
        }
        // 面板與終端機的大小每一幀都一樣（之前面板每一幀縮 2 像素）
        assert!(seen.windows(2).skip(1).all(|w| w[0] == w[1]), "the dock keeps its size: {seen:?}");
        let font = FontId::monospace(13.0);
        // 每個字往右移的距離就是字寬（egui 0.33 起以次像素累加；「兩個字與一個字的寬度差」會因為字形位置取整到像素而少算）
        let adv = ctx.fonts_mut(|f| f.glyph_width(&font, '0'));
        // 大小跟著輸出區：寬度放得下的欄數（扣掉捲軸），而不是固定的 200 × 160
        let (rows, cols) = (app.terminal.rows(), app.terminal.cols());
        let w = app.probe["terminal-output"].width();
        assert!((2..40).contains(&rows), "rows follow the dock height, got {rows}");
        assert!((cols as f32) * adv <= w && (cols as f32 + 4.0) * adv > w - 30.0, "{cols} columns for {w} px");
        assert!(wait(&mut app, &|t| !t.trim().is_empty()), "the shell prints its prompt");

        // shell 收到的大小與畫面相同
        let ask = if cfg!(windows) {
            "powershell -NoProfile -Command \"[Console]::WindowWidth*1000+[Console]::WindowHeight\""
        } else {
            "stty size"
        };
        let reply = |rows: usize, cols: usize| if cfg!(windows) { format!("{}", cols * 1000 + rows) } else { format!("{rows} {cols}") };
        app.submit_terminal_line(&ctx, ask.to_string());
        let want = reply(rows, cols);
        assert!(wait(&mut app, &|t| t.lines().any(|l| l.trim() == want)), "the shell sees {rows} x {cols}:\n{}", screen(&app));

        // 視窗變窄：欄數跟著變少，shell 也收到新的大小
        size.set(egui::vec2(900.0, 800.0));
        run(&mut app, vec![]);
        run(&mut app, vec![]);
        let (rows2, cols2) = (app.terminal.rows(), app.terminal.cols());
        assert!(cols2 < cols, "narrower window, fewer columns ({cols} -> {cols2})");
        app.submit_terminal_line(&ctx, ask.to_string());
        let want = reply(rows2, cols2);
        assert!(wait(&mut app, &|t| t.lines().any(|l| l.trim() == want)), "the shell sees {rows2} x {cols2}:\n{}", screen(&app));

        // 游標畫在畫面上，輸入的字讓它往右移。shell 的輸出是分批到的（提示字元、重畫），
        // 量位置前先等畫面與游標 0.5 秒沒有變化，不要量到印到一半的樣子
        let settle = |app: &mut AlexCodeApp| {
            let state = |app: &AlexCodeApp| (screen(app), app.probe.get("terminal-cursor").map(|r| (r.left(), r.top())));
            let mut last = (state(app), Instant::now());
            let end = Instant::now() + Duration::from_secs(20);
            while last.1.elapsed() < Duration::from_millis(500) && Instant::now() < end {
                run(app, vec![]);
                let now = state(app);
                if now != last.0 {
                    last = (now, Instant::now());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        settle(&mut app);
        let cursor = app.probe["terminal-cursor"];
        assert!(app.probe["terminal-output"].contains(cursor.center()), "the cursor is inside the output area");
        // 用同樣的視窗大小點擊：共用的 click() 用預設的 1280 寬，會讓終端機先變寬再變回來（shell 收到兩次 SIGWINCH 而重畫提示字元），
        // 之前 macOS 上偶爾因此量到重畫到一半的游標（「the cursor moved -79.75797 px for 3 characters」）
        let out = app.probe["terminal-output"].center();
        let button = |pressed| Event::PointerButton { pos: out, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        run(&mut app, vec![Event::PointerMoved(out)]);
        run(&mut app, vec![button(true)]);
        run(&mut app, vec![button(false)]);
        run(&mut app, vec![]);
        assert_eq!((app.terminal.rows(), app.terminal.cols()), (rows2, cols2), "clicking does not resize the terminal");
        run(&mut app, vec![Event::Text("abc".into())]);
        assert!(wait(&mut app, &|t| t.contains("abc")));
        settle(&mut app);
        let moved = app.probe["terminal-cursor"].left() - cursor.left();
        assert!((moved - 3.0 * adv).abs() < 1.5, "the cursor moved {moved} px for 3 characters");
    }

    #[test]
    fn terminal_ime_commits_go_to_the_shell_and_keys_while_composing_do_not() {
        let k = |key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
        let preedit = |t: &str| Event::Ime(egui::ImeEvent::Preedit { text: t.into(), active_range_chars: None });
        let mut composing = String::new();
        // 組字中：組字文字記下來（畫在游標處），按鍵屬於輸入法，不送進 shell
        let out = terminal_input_bytes_ime(&[preedit("ㄓㄨㄥ"), k(Key::Backspace), k(Key::ArrowLeft), k(Key::Enter)], &mut composing);
        assert!(out.is_empty(), "keys while composing belong to the IME: {out:?}");
        assert_eq!(composing, "ㄓㄨㄥ");
        // 選定的字送進 shell（UTF-8）
        let out = terminal_input_bytes_ime(&[Event::Ime(egui::ImeEvent::Commit("中文".into()))], &mut composing);
        assert_eq!(out, "中文".as_bytes());
        assert!(composing.is_empty());
        // 組字結束後按鍵照常送出
        assert_eq!(terminal_input_bytes_ime(&[k(Key::Enter)], &mut composing), b"\r");
        // 取消組字（空的組字文字）
        terminal_input_bytes_ime(&[preedit("ㄓ")], &mut composing);
        assert_eq!(terminal_input_bytes_ime(&[preedit(""), k(Key::Enter)], &mut composing), b"\r");
        assert!(composing.is_empty());
    }

    #[test]
    fn terminal_tells_the_ime_where_its_cursor_is_and_shows_the_composition() {
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = AlexCodeApp::new();
        app.term_cwd = dir.path().to_path_buf();
        app.show_terminal = true;
        let screen = |app: &AlexCodeApp| (0..app.terminal.rows()).map(|r| app.terminal.line_text(r)).collect::<Vec<_>>().join("\n");
        let end = Instant::now() + Duration::from_secs(30);
        while screen(&app).trim().is_empty() && Instant::now() < end {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!screen(&app).trim().is_empty(), "the shell prints its prompt");
        let out = app.probe["terminal-output"].center();
        click(&mut app, &ctx, out);
        // 輸出區有焦點：告訴作業系統游標在哪裡（沒有這個 Windows 不會在這裡啟用輸入法，候選字視窗也不知道放哪）
        let o = frame(&mut app, &ctx, vec![]);
        let cursor = app.probe["terminal-cursor"];
        let ime = o.platform_output.ime.expect("the IME is enabled while the terminal has focus");
        assert!((ime.cursor_rect.left() - cursor.left()).abs() < 0.5, "{:?} vs {cursor:?}", ime.cursor_rect);
        // 組字中：組字文字畫在游標處，候選字視窗在它後面
        let o = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Preedit { text: "ㄓㄨㄥ".into(), active_range_chars: None })]);
        let pre = app.probe["terminal-preedit"];
        let w = ctx.fonts_mut(|f| f.layout_no_wrap("ㄓㄨㄥ".into(), FontId::monospace(13.0), Color32::WHITE).size().x);
        assert!((pre.left() - cursor.left()).abs() < 0.5 && (pre.width() - w).abs() < 0.5, "{pre:?}, preedit {w} wide");
        assert!(painted_texts(&o).iter().any(|t| t == "ㄓㄨㄥ"), "the composition is drawn");
        let ime = o.platform_output.ime.unwrap();
        assert!((ime.cursor_rect.left() - pre.right()).abs() < 0.5);
        // 選定後組字文字消失（送進 shell 的字由 shell 回顯）
        let o = frame(&mut app, &ctx, vec![Event::Ime(egui::ImeEvent::Commit("中".into()))]);
        assert!(!painted_texts(&o).iter().any(|t| t == "ㄓㄨㄥ"));
        assert!(app.term_preedit.is_empty());
    }

    #[test]
    fn terminal_keys_become_the_bytes_a_terminal_sends() {
        let ctrl = Modifiers { ctrl: true, command: cfg!(not(target_os = "macos")), ..Modifiers::NONE };
        let k = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        let bytes = terminal_input_bytes(&[
            Event::Text("ls -a".into()),
            k(Key::Enter, Modifiers::NONE),
            k(Key::ArrowUp, Modifiers::NONE),
            k(Key::ArrowLeft, Modifiers::NONE),
            k(Key::Tab, Modifiers::NONE),
            k(Key::Backspace, Modifiers::NONE),
            k(Key::D, ctrl),
            k(Key::A, Modifiers::NONE), // 一般的字由 Text 送，按鍵本身不送
        ]);
        assert_eq!(bytes, b"ls -a\r\x1b[A\x1b[D\t\x7f\x04".to_vec());
        // egui 把 Ctrl+C / Ctrl+V 變成複製 / 貼上事件：分別送中斷字元、貼上剪貼簿的文字（換行改成 \r），按鍵不重送
        assert_eq!(terminal_input_bytes(&[Event::Copy, k(Key::C, ctrl)]), vec![0x03]);
        assert_eq!(terminal_input_bytes(&[Event::Paste("a\nb\r\nc".into()), k(Key::V, ctrl)]), b"a\rb\rc".to_vec());
        // 沒有複製事件時（例如平台沒有送）按鍵本身也是 Ctrl+C
        assert_eq!(terminal_input_bytes(&[k(Key::C, ctrl)]), vec![0x03]);
        assert_eq!(terminal_key_bytes(Key::Tab, Modifiers::SHIFT), Some(b"\x1b[Z".to_vec()));
        assert_eq!(terminal_key_bytes(Key::Delete, Modifiers::NONE), Some(b"\x1b[3~".to_vec()));
        assert_eq!(terminal_key_bytes(Key::F5, Modifiers::NONE), None);
    }

    #[test]
    fn terminal_keyboard_goes_straight_to_the_shell() {
        use std::time::{Duration, Instant};
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("zzcomplete_me.txt"), "hello-from-tab\n").unwrap();
        let mut app = AlexCodeApp::new();
        app.term_cwd = dir.path().to_path_buf();
        let screen = |app: &AlexCodeApp| {
            let t = &app.terminal;
            let mut s: Vec<String> = (0..t.scrollback_count()).map(|i| t.scrollback_line(i)).collect();
            s.extend((0..t.rows()).map(|r| t.line_text(r)));
            s.join("\n")
        };
        let wait = |app: &mut AlexCodeApp, secs: u64, cond: &dyn Fn(&str) -> bool| {
            let end = Instant::now() + Duration::from_secs(secs);
            while Instant::now() < end {
                frame(app, &ctx, vec![]);
                if cond(&screen(app)) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        let enter = || key(Key::Enter, Modifiers::NONE);
        app.show_terminal = true;
        frame(&mut app, &ctx, vec![]);
        assert!(wait(&mut app, 20, &|t| !t.trim().is_empty()), "the shell prints its prompt");
        frame(&mut app, &ctx, vec![]);
        assert!(app.probe.contains_key("terminal-shell"), "the shell can be chosen in the dock");
        let tabs_before: Vec<String> = app.tabs.iter().map(|b| b.edit_text()).collect();

        // 點一下輸出區取得焦點，之後打的字與 Enter 直接送進 shell
        let out = app.probe["terminal-output"].center();
        click(&mut app, &ctx, out);
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("alexcode-terminal-output"))), "the output area takes the keyboard");
        let (sum, again, long, tab) = if cfg!(windows) {
            ("set /a 1000+1", "set /a 5000+555", "ping -n 30 127.0.0.1 >nul", "type zzcompl")
        } else {
            ("echo $((1000+1))", "echo $((5000+555))", "sleep 30", "cat zzcompl")
        };
        frame(&mut app, &ctx, vec![Event::Text(sum.into()), enter()]);
        assert!(wait(&mut app, 20, &|t| t.lines().any(|l| l.trim() == "1001")), "typed command runs:\n{}", screen(&app));
        // 上鍵叫出上一個指令，再按 Enter 又跑一次
        frame(&mut app, &ctx, vec![key(Key::ArrowUp, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![enter()]);
        assert!(
            wait(&mut app, 20, &|t| t.lines().filter(|l| l.trim() == "1001").count() >= 2),
            "Up recalls the previous command:\n{}",
            screen(&app)
        );
        // Ctrl+C 中斷還在跑的指令（30 秒），之後馬上能執行下一個
        frame(&mut app, &ctx, vec![Event::Text(long.into()), enter()]);
        wait(&mut app, 2, &|_| false);
        let t0 = Instant::now();
        let ctrl = Modifiers { ctrl: true, command: cfg!(not(target_os = "macos")), ..Modifiers::NONE };
        frame(&mut app, &ctx, vec![Event::Copy, key(Key::C, ctrl)]);
        wait(&mut app, 1, &|_| false);
        frame(&mut app, &ctx, vec![Event::Text(again.into()), enter()]);
        assert!(wait(&mut app, 20, &|t| t.contains("5555")), "Ctrl+C stops the running command:\n{}", screen(&app));
        assert!(t0.elapsed() < Duration::from_secs(15), "it did not wait for the 30-second command ({:?})", t0.elapsed());
        // Tab 補完檔名
        frame(&mut app, &ctx, vec![Event::Text(tab.into()), key(Key::Tab, Modifiers::NONE)]);
        wait(&mut app, 1, &|_| false);
        frame(&mut app, &ctx, vec![enter()]);
        assert!(wait(&mut app, 20, &|t| t.contains("hello-from-tab")), "Tab completes the file name:\n{}", screen(&app));
        let tabs_after: Vec<String> = app.tabs.iter().map(|b| b.edit_text()).collect();
        assert_eq!(tabs_after, tabs_before, "nothing typed into the terminal reached the editor");

        // 換一個 shell：結束舊的、啟動選的那一個，並記在設定裡
        let shells = app.shells().to_vec();
        let other = shells.last().unwrap().clone();
        app.switch_shell(&ctx, &other.name);
        assert_eq!(app.config.terminal_shell, other.name);
        assert_eq!(app.chosen_shell(), other);
        assert!(app.term_pty.is_some());
        let cmd = match other.name.as_str() {
            "cmd" => "set /a 4000+321",
            "PowerShell" => "4000+321",
            _ => "echo $((4000+321))",
        };
        assert!(wait(&mut app, 30, &|t| t.contains(&format!("切換到 {}", other.name))));
        app.submit_terminal_line(&ctx, cmd.to_string());
        assert!(wait(&mut app, 30, &|t| t.contains("4321")), "{} runs commands:\n{}", other.name, screen(&app));
    }

    #[test]
    fn large_view_auto_pairs_brackets_and_quotes() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 200);
        let none = Modifiers::NONE;
        let line0 = |app: &AlexCodeApp| app.tabs[0].lines[0].clone();
        let type_ = |app: &mut AlexCodeApp, t: &str| {
            frame(app, &ctx, vec![Event::Text(t.into())]);
        };

        // 純文字檔（.log）與一般編輯區一樣不配對
        frame(&mut app, &ctx, vec![key(Key::End, none)]);
        type_(&mut app, "(");
        assert_eq!(line0(&app), "line 00000 some text here(");
        frame(&mut app, &ctx, vec![key(Key::Backspace, none)]);

        // 程式碼檔：補上右括號，游標留在中間
        app.tabs[0].file_path = Some("big.rs".into());
        type_(&mut app, "(");
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("line 00000 some text here()", (0, 26)));
        type_(&mut app, "x");
        assert_eq!(line0(&app), "line 00000 some text here(x)");
        // 輸入右括號時跳過已經有的那個
        type_(&mut app, ")");
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("line 00000 some text here(x)", (0, 28)));

        // 空的一對：退格連右邊一起刪
        type_(&mut app, "[");
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("line 00000 some text here(x)[]", (0, 29)));
        frame(&mut app, &ctx, vec![key(Key::Backspace, none)]);
        assert_eq!((line0(&app).as_str(), cursor(&app)), ("line 00000 some text here(x)", (0, 28)));

        // 後面緊接著字母時不補；英文字裡的撇號也不補
        frame(&mut app, &ctx, vec![key(Key::Home, none)]);
        type_(&mut app, "{");
        assert_eq!(line0(&app), "{line 00000 some text here(x)");
        frame(&mut app, &ctx, vec![key(Key::ArrowRight, Modifiers::COMMAND)]);
        type_(&mut app, "'");
        assert_eq!(line0(&app), "{line' 00000 some text here(x)");

        // 有選取時用一對符號把選取的文字包起來（跨行也可以）
        frame(&mut app, &ctx, vec![key(Key::ArrowDown, none), key(Key::Home, none), key(Key::ArrowRight, Modifiers::COMMAND | Modifiers::SHIFT)]);
        type_(&mut app, "\"");
        assert_eq!(app.tabs[0].lines[1], "\"line\" 00001 some text here");
        frame(&mut app, &ctx, vec![key(Key::Home, none), key(Key::ArrowDown, Modifiers::SHIFT)]);
        type_(&mut app, "(");
        assert_eq!(
            (app.tabs[0].lines[1].as_str(), app.tabs[0].lines[2].as_str()),
            ("(\"line\" 00001 some text here", ")line 00002 some text here")
        );
        // 一次復原就回到包起來之前
        std::thread::sleep(std::time::Duration::from_millis(5));
        frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[2], "line 00002 some text here");

        // 關閉設定後照常輸入
        app.config.auto_pair = false;
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::COMMAND)]);
        type_(&mut app, "(");
        assert_eq!(app.tabs[0].lines[200], "(");
        assert!(app.highlight_cache.is_none() && app.text_cache.1.is_empty(), "still the large-file view");
    }

    /// 這一幀畫出來的、寬 3 像素且顏色為 `fill` 的實心矩形（行號區的 Git 標記）的上緣 y 座標
    fn mark_bars(out: &egui::FullOutput, fill: Color32) -> Vec<f32> {
        fn walk(shape: &Shape, fill: Color32, found: &mut Vec<f32>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, found)),
                Shape::Rect(r) if r.fill == fill && (r.rect.width() - 3.0).abs() < 0.01 => found.push(r.rect.top()),
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, fill, &mut found));
        found
    }

    #[test]
    fn large_view_shows_git_marks_in_the_gutter() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 5000);
        let id = app.tabs[0].id;
        let pal = app.theme_mgr.current_palette().clone();
        let c = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud());
        let wait = || std::thread::sleep(LARGE_GIT_DELAY + std::time::Duration::from_millis(30));

        // 與 HEAD 相同：沒有標記
        app.git_head.insert(id, Some(Arc::new(app.tabs[0].lines.clone())));
        let out = frame(&mut app, &ctx, vec![]);
        assert!(app.git_marks.1.is_empty());
        assert!(mark_bars(&out, c.warn).is_empty() && mark_bars(&out, c.accent).is_empty());

        // 改第 0 行、在它後面加一行：停止輸入一小段時間後標記出現在那兩行的行號區
        frame(&mut app, &ctx, vec![Event::Text("x".into()), key(Key::End, Modifiers::NONE), key(Key::Enter, Modifiers::NONE)]);
        frame(&mut app, &ctx, vec![Event::Text("new".into())]);
        assert!(app.git_marks.1.is_empty(), "not recomputed on every keystroke");
        wait();
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.git_marks.1.get(&0), Some(&MODIFIED));
        assert_eq!(app.git_marks.1.get(&1), Some(&ADDED));
        assert_eq!(app.git_marks.1.len(), 2);
        let (row0, row_h) = (large_row_y(&app, &ctx, 0), row_height(&app, &ctx));
        let near = |ys: Vec<f32>, y: f32| ys.len() == 1 && (ys[0] - y).abs() < 0.5;
        assert!(near(mark_bars(&out, c.warn), row0), "modified bar on line 1");
        assert!(near(mark_bars(&out, c.accent), row0 + row_h), "added bar on line 2");

        // 捲到看不到那兩行的地方：標記還在，但不會畫出來
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::COMMAND)]);
        settle(&mut app, &ctx);
        let out = frame(&mut app, &ctx, vec![]);
        assert_eq!(app.git_marks.1.len(), 2);
        assert!(mark_bars(&out, c.warn).is_empty());

        // 不在 Git 內的檔案沒有標記
        app.git_head.insert(id, None);
        frame(&mut app, &ctx, vec![]);
        assert!(app.git_marks.1.is_empty());
        assert!(app.highlight_cache.is_none() && app.text_cache.1.is_empty());
    }

    /// 在 `pos` 連按 `n` 下左鍵（2 = 雙擊、3 = 三擊）
    fn multi_click(app: &mut AlexCodeApp, ctx: &egui::Context, pos: Pos2, n: usize) {
        let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        // 先等過雙擊的間隔（每幀 1/60 秒），免得和上一次的點擊連成三擊
        for _ in 0..30 {
            frame(app, ctx, vec![]);
        }
        frame(app, ctx, vec![Event::PointerMoved(pos)]);
        for _ in 0..n {
            frame(app, ctx, vec![button(true)]);
            frame(app, ctx, vec![button(false)]);
        }
        frame(app, ctx, vec![]);
    }

    fn selection(app: &AlexCodeApp) -> ((usize, usize), (usize, usize)) {
        let s = app.tabs[0].selections[0];
        ((s.anchor.line, s.anchor.col), (s.head.line, s.head.col))
    }

    #[test]
    fn large_view_double_click_selects_a_word_and_triple_click_the_line() {
        let ctx = ctx();
        let mut text: String = (0..5000).map(|i| format!("line {i:05} some text here\n")).collect();
        text.insert_str(0, "let some_value = foo(bar);  // 中文註解\n");
        let mut app = large_app_with(&ctx, &text);
        let area = app.probe["large-text"];
        let row_h = row_height(&app, &ctx);
        // 第 0 行第 `col` 個字元（小數部分 = 在那個字元裡的哪裡）在畫面上的位置，用實際排版量出來
        let first = app.tabs[0].lines[0].clone();
        let font = FontId::monospace(app.config.font_size);
        let x_of = |n: usize| ctx.fonts_mut(|f| f.layout_no_wrap(first.chars().take(n).collect(), font.clone(), Color32::WHITE).size().x);
        let at = |col: f32| {
            let n = col.floor() as usize;
            let x = x_of(n) + (x_of(n + 1) - x_of(n)).max(1.0) * col.fract();
            Pos2::new(area.left() + x, area.top() + row_h * 0.5)
        };
        let copy = |app: &mut AlexCodeApp| copied_text(&frame(app, &ctx, vec![Event::Copy]));

        // 雙擊選取一個字（英數字與底線算同一個字）
        multi_click(&mut app, &ctx, at(6.5), 2);
        assert_eq!(selection(&app), ((0, 4), (0, 14)));
        assert_eq!(copy(&mut app), "some_value");
        multi_click(&mut app, &ctx, at(18.5), 2);
        assert_eq!(copy(&mut app), "foo");
        // 中文字連在一起算一個字
        multi_click(&mut app, &ctx, at(31.3), 2);
        assert_eq!(copy(&mut app), "中文註解");
        // 點在空白上選取那一串空白；點在行尾右邊選取最後一個字
        multi_click(&mut app, &ctx, at(26.3), 2);
        assert_eq!(selection(&app), ((0, 26), (0, 28)));
        multi_click(&mut app, &ctx, at(60.0), 2);
        assert_eq!(selection(&app), ((0, 31), (0, 35)));

        // 三擊選取整行（不含換行，與一般編輯區相同）；接著輸入會取代那一行的內容
        multi_click(&mut app, &ctx, at(3.0), 3);
        assert_eq!(selection(&app), ((0, 0), (0, 35)));
        assert_eq!(copy(&mut app), "let some_value = foo(bar);  // 中文註解");
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!((app.tabs[0].lines[0].as_str(), app.tabs[0].lines.len()), ("X", 5002));

        // 雙擊行號區只會切換書籤，不會選字
        multi_click(&mut app, &ctx, Pos2::new(area.left() - 30.0, area.top() + row_h * 1.5), 2);
        assert_eq!(selection(&app), ((0, 1), (0, 1)));
        assert!(app.highlight_cache.is_none() && app.text_cache.1.is_empty());
    }

    /// 在 `from` 按下左鍵並拖到 `to`（不放開）
    fn press_and_drag(app: &mut AlexCodeApp, ctx: &egui::Context, from: Pos2, to: Pos2) {
        frame(app, ctx, vec![Event::PointerMoved(from)]);
        frame(app, ctx, vec![Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        frame(app, ctx, vec![Event::PointerMoved(Pos2::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5))]);
        frame(app, ctx, vec![Event::PointerMoved(to)]);
    }

    /// 把（仍按著的）指標移到 `to` 後不動，再跑 `frames` 幀；回傳每一幀之後游標的位置
    fn hold_at(app: &mut AlexCodeApp, ctx: &egui::Context, to: Pos2, frames: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in 0..frames {
            frame(app, ctx, if i == 0 { vec![Event::PointerMoved(to)] } else { vec![] });
            out.push(cursor(app));
        }
        out
    }

    #[test]
    fn large_view_keeps_scrolling_while_a_drag_is_held_past_the_edge() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 5000);
        let clip = app.probe["large-clip"];
        let row_h = row_height(&app, &ctx);
        let x = app.probe["large-text"].left() + 20.0;
        let visible = (clip.height() / row_h) as usize;
        let lines = |v: &[(usize, usize)]| v.iter().map(|c| c.0).collect::<Vec<_>>();
        press_and_drag(&mut app, &ctx, Pos2::new(x, clip.top() + row_h * 2.5), Pos2::new(x, clip.top() + row_h * 5.5));
        assert_eq!(selection(&app).0 .0, 2);

        // 拖到編輯區下緣再往下 1 行的地方不動：一秒內每一幀都繼續往下捲，選取跟著延伸
        let near = lines(&hold_at(&mut app, &ctx, Pos2::new(x, clip.bottom() + row_h), 60));
        assert!(near.windows(2).all(|w| w[1] >= w[0]), "{near:?}");
        assert!(near[59] > near[40] && near[40] > near[20] && near[20] > near[0], "keeps scrolling: {near:?}");
        assert!(near[59] > visible + 5, "went past the first screen: {near:?}");
        // 游標所在的行一直在畫面上，起點沒變
        let head = cursor(&app).0;
        let y = large_row_y(&app, &ctx, head);
        assert!(y > clip.top() - 1.0 && y < clip.bottom(), "head line {head} on screen (y = {y})");
        assert_eq!(selection(&app).0, (2, selection(&app).0 .1));

        // 拖得越遠捲得越快
        let far = lines(&hold_at(&mut app, &ctx, Pos2::new(x, clip.bottom() + row_h * 10.0), 60));
        assert!(far[59] - far[0] > (near[59] - near[0]) * 2, "near {near:?} far {far:?}");
        // 但拖到很遠（例如視窗外）也不會一下子跳過一大段：每一幀最多捲幾行
        let very_far = lines(&hold_at(&mut app, &ctx, Pos2::new(x, clip.bottom() + 3000.0), 20));
        assert!(very_far.windows(2).all(|w| w[1] >= w[0] && w[1] - w[0] <= 8), "{very_far:?}");
        assert!(very_far[19] > very_far[0]);

        // 往上拖到上緣以外：往回捲
        let up = lines(&hold_at(&mut app, &ctx, Pos2::new(x, clip.top() - row_h * 3.0), 30));
        assert!(up.windows(2).all(|w| w[1] <= w[0]) && up[29] < up[0], "{up:?}");

        // 放開之後就停下來
        let release = Event::PointerButton { pos: Pos2::new(x, clip.top() - row_h * 3.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE };
        frame(&mut app, &ctx, vec![release]);
        let stopped = cursor(&app);
        settle(&mut app, &ctx);
        let y = large_row_y(&app, &ctx, 0);
        for _ in 0..10 {
            frame(&mut app, &ctx, vec![]);
        }
        assert_eq!((cursor(&app), large_row_y(&app, &ctx, 0)), (stopped, y), "no scrolling after release");

        // 橫向也一樣：長行拖到右緣以外時繼續往右捲
        let long: String = (0..50).map(|i| format!("{}\n", "abcdefghij".repeat(40 + i))).collect();
        let mut app = large_app_with(&ctx, &long);
        let clip = app.probe["large-clip"];
        let y = clip.top() + row_h * 1.5;
        press_and_drag(&mut app, &ctx, Pos2::new(clip.left() + 200.0, y), Pos2::new(clip.left() + 400.0, y));
        let cols: Vec<usize> = hold_at(&mut app, &ctx, Pos2::new(clip.right() + 30.0, y), 60).iter().map(|c| c.1).collect();
        let shown = ((clip.width() - 60.0) / ctx.fonts_mut(|f| f.glyph_width(&FontId::monospace(app.config.font_size), '0'))) as usize;
        assert!(cols.windows(2).all(|w| w[1] >= w[0]) && cols[59] > cols[30] && cols[59] > shown, "{cols:?}");
    }

    /// 這一幀畫出的文字裡，底色是搜尋命中色的那些片段（已排序）
    fn hit_texts(out: &egui::FullOutput, bg: Color32) -> Vec<String> {
        fn walk(shape: &Shape, bg: Color32, found: &mut Vec<String>) {
            match shape {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, bg, found)),
                Shape::Text(t) => {
                    let job = &t.galley.job;
                    for s in job.sections.iter().filter(|s| s.format.background == bg) {
                        found.push(job.text[s.byte_range.start.0..s.byte_range.end.0].to_string());
                    }
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        out.shapes.iter().for_each(|s| walk(&s.shape, bg, &mut found));
        found.sort();
        found
    }

    #[test]
    fn markdown_view_highlights_with_the_find_bar_options() {
        let ctx = ctx();
        let mut app = AlexCodeApp::new();
        let mut b = EditorBuffer::from_text("# Cat notes\n\nconcat here\n\nA cat sat\n\n```rust\nlet cat = Cat::new();\n```\n");
        b.file_path = Some("notes.md".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        app.md_view.mode = MdViewMode::Typora;
        let pal = app.theme_mgr.current_palette().clone();
        let bg = Colors::from_palette(&pal, app.theme_mgr.is_neon_hud()).search_bg;
        let hits = |app: &mut AlexCodeApp| {
            frame(app, &ctx, vec![]);
            hit_texts(&frame(app, &ctx, vec![]), bg)
        };

        // 尋找列沒開：沒有高亮
        app.find_bar_text = "cat".into();
        assert!(hits(&mut app).is_empty());

        // 預設：不分大小寫的純文字。內文以整段為單位標示，程式碼區塊只標命中的字
        app.show_find_bar = true;
        assert_eq!(hits(&mut app), ["A cat sat", "Cat", "Cat notes", "cat", "concat here"]);

        // 區分大小寫
        app.find_opts.case_sensitive = true;
        assert_eq!(hits(&mut app), ["A cat sat", "cat", "concat here"]);

        // 全字
        app.find_opts = FindOptions { whole_word: true, ..Default::default() };
        assert_eq!(hits(&mut app), ["A cat sat", "Cat", "Cat notes", "cat"]);

        // 正規表示式；一般模式下同樣的字串只是普通文字
        app.find_opts = FindOptions { regex: true, ..Default::default() };
        app.find_bar_text = "s[ae]t".into();
        assert_eq!(hits(&mut app), ["A cat sat"]);
        app.find_opts.regex = false;
        assert!(hits(&mut app).is_empty());

        // 寫到一半、還不合法的正規表示式：沒有高亮，也不會出錯
        app.find_opts.regex = true;
        app.find_bar_text = "(".into();
        assert!(hits(&mut app).is_empty());

        // 並排模式右邊的預覽用的是同一套選項
        app.md_view.mode = MdViewMode::Split;
        app.find_opts = FindOptions { case_sensitive: true, ..Default::default() };
        app.find_bar_text = "Cat".into();
        let split = hits(&mut app);
        assert!(split.contains(&"Cat notes".to_string()) && !split.contains(&"A cat sat".to_string()), "{split:?}");
    }

    #[test]
    fn find_bar_count_in_large_files_waits_for_typing_to_pause() {
        let ctx = ctx();
        let mut app = large_app(&ctx, 200);
        app.show_find_bar = true;
        app.find_bar_text = "line".into();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 200);
        assert!(app.text_cache.1.is_empty(), "the joined copy of the text is dropped once the count is known");

        // 在文件裡打字：命中數先沿用舊的，不是每個字元都重新串接全文再搜尋
        frame(&mut app, &ctx, vec![Event::Text("line ".into())]);
        frame(&mut app, &ctx, vec![Event::Text("line ".into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.tabs[0].lines[0], "line line line 00000 some text here");
        assert_eq!(app.find_cache.1.len(), 200, "not recounted while typing");
        assert!(app.text_cache.1.is_empty());
        // 停下來之後才重算
        std::thread::sleep(LARGE_FIND_DELAY + std::time::Duration::from_millis(30));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 202);
        assert!(app.text_cache.1.is_empty());

        // 改了查詢字串或選項：立刻重算
        app.find_bar_text = "00001".into();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 1);
        app.find_bar_text = "LINE".into();
        app.find_opts.case_sensitive = true;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.find_cache.1.len(), 0);

        // 尋找下一個不看這份快取：剛打完字就按也找得到新的內容
        app.find_opts.case_sensitive = false;
        app.find_bar_text = "zzz".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(Key::End, Modifiers::COMMAND), Event::Text("zzz".into())]);
        frame(&mut app, &ctx, vec![key(Key::Home, Modifiers::COMMAND)]);
        app.find_step(true);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(cursor(&app), (200, 3));

        // 小檔案（一般編輯區）：內容一變就重算
        let mut small = app_with(&ctx, "aa\nbb\n");
        small.show_find_bar = true;
        small.find_bar_text = "aa".into();
        frame(&mut small, &ctx, vec![]);
        assert_eq!(small.find_cache.1.len(), 1);
        frame(&mut small, &ctx, vec![Event::Text("aa".into())]);
        frame(&mut small, &ctx, vec![]);
        assert_eq!(small.find_cache.1.len(), 2);
    }

    #[test]
    fn large_hot_exit_backups_are_written_in_the_background() {
        let dir = tempfile::tempdir().unwrap();
        let backup = dir.path().join("cfg").join("hot_exit.json");
        let mut app = AlexCodeApp::new();
        app.state_path = Some(dir.path().join("cfg").join("state.json"));
        app.new_tab();
        let i = app.active_tab_idx;
        let big = format!("{}\n", "x".repeat(99)).repeat(HOT_EXIT_SLOW_BYTES / 100 + 100);
        app.tabs[i].replace_all_text(&big);
        let saved_text = |path: &Path| HotExit::load(path).expect("a readable backup").tabs[0].text.clone();
        let top = Position { line: 0, col: 0 };
        let overdue = |app: &mut AlexCodeApp| app.hot_exit_written.0 -= HOT_EXIT_SLOW_INTERVAL * 2;

        // 定期的寫入：內容很大時交給背景執行緒，呼叫立刻回來，寫完之前算「還沒備份好」
        assert!(!app.save_hot_exit(false));
        assert!(app.hot_exit_job.is_some(), "the write runs on a background thread");
        let t = std::time::Instant::now();
        while !app.save_hot_exit(false) {
            assert!(t.elapsed() < std::time::Duration::from_secs(20), "the background write never finished");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.hot_exit_job.is_none());
        assert_eq!(saved_text(&backup).len(), app.tabs[i].edit_text().len());

        // 內容又變了：30 秒內不重寫；結束程式時（force）一定寫完最新的內容才回來
        app.tabs[i].replace_range(top, top, "a", true);
        assert!(!app.save_hot_exit(false) && app.hot_exit_job.is_none());
        assert!(app.save_hot_exit(true));
        assert!(saved_text(&backup).starts_with("ax"));

        // 背景還在寫的時候結束程式：等它寫完，再把之後的修改也寫進去
        app.tabs[i].replace_range(top, top, "b", true);
        overdue(&mut app);
        assert!(!app.save_hot_exit(false) && app.hot_exit_job.is_some());
        app.tabs[i].replace_range(top, top, "c", true);
        assert!(app.save_hot_exit(true));
        assert!(app.hot_exit_job.is_none());
        assert!(saved_text(&backup).starts_with("cbax"));

        // 背景寫完之後內容沒有再變：結束時不必重寫一次
        app.tabs[i].replace_range(top, top, "d", true);
        overdue(&mut app);
        assert!(!app.save_hot_exit(false));
        while !app.hot_exit_job.as_ref().is_some_and(|j| j.is_finished()) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let written = std::fs::metadata(&backup).unwrap().modified().unwrap();
        assert!(app.save_hot_exit(true));
        assert_eq!(std::fs::metadata(&backup).unwrap().modified().unwrap(), written, "not written twice");
        assert!(saved_text(&backup).starts_with("dcbax"));

        // 小的內容照舊當場寫完
        app.tabs[i].replace_all_text("small");
        overdue(&mut app);
        assert!(app.save_hot_exit(false));
        assert!(app.hot_exit_job.is_none());
        assert_eq!(saved_text(&backup), "small");
    }

    #[test]
    fn big_markdown_files_are_shown_as_source_instead_of_the_reading_view() {
        let ctx = ctx();
        let md: String = (0..400).map(|i| format!("## Section {i}\n\nParagraph {i} with **bold** text.\n\n")).collect();
        let open = |text: &str, limit: usize| {
            let mut app = AlexCodeApp::new();
            app.md_preview_bytes = limit;
            let mut b = EditorBuffer::from_text(text);
            b.file_path = Some("notes.md".into());
            app.tabs = vec![b];
            app.active_tab_idx = 0;
            app.md_view.mode = MdViewMode::Typora;
            frame(&mut app, &ctx, vec![]);
            app
        };
        let has = |out: &egui::FullOutput, what: &str| painted_texts(out).iter().any(|t| t.contains(what));

        // 小於門檻：照舊是閱讀模式（標題不帶 `##`），沒有說明列
        let mut small = open(&md, md.len() + 100);
        let out = frame(&mut small, &ctx, vec![]);
        assert!(has(&out, "Section 0") && !has(&out, "## Section 0"), "rendered, not source");
        assert!(!has(&out, "以原始碼顯示") && small.highlight_cache.is_none());

        // 達到門檻：以原始碼顯示，並說明原因；模式設定本身不變
        let mut app = open(&md, 1000);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(has(&out, "## Section 0"), "the Markdown source is shown");
        assert!(has(&out, "以原始碼顯示"), "and a bar says why");
        assert!(app.highlight_cache.is_some(), "it is the code editor");
        assert_eq!(app.md_view.mode, MdViewMode::Typora);
        // 可以直接打字
        app.pending_select = Some((0, 0));
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![Event::Text("X".into())]);
        assert_eq!(app.tabs[0].lines[0], "X## Section 0");

        // 按「仍要使用閱讀模式」：只有這個分頁換回閱讀模式
        let pos = app.probe["md-big-preview"].center();
        click(&mut app, &ctx, pos);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(!has(&out, "以原始碼顯示") && !has(&out, "## Section 1"), "back in the reading view");
        assert!(has(&out, "Section 1"));

        // 其他格式的檔案不受影響
        let mut code = open(&md, 1000);
        code.tabs[0].file_path = Some("notes.txt".into());
        let out = frame(&mut code, &ctx, vec![]);
        assert!(!has(&out, "以原始碼顯示"));
    }

    /// 大檔案其餘部分的耗時量測（待辦 3e）：Git 標記重算、尋找列、熱結束備份、Typora 閱讀模式。
    /// `cargo test --release --lib large_file_cost_report -- --ignored --nocapture`
    /// 檔案大小以 `ALEXCODE_MEM_REPORT_MB` 指定（預設 29）；記憶體用量在 `tests/memory_report.rs` 量。
    #[test]
    #[ignore]
    fn large_file_cost_report() {
        let size_mb: usize = std::env::var("ALEXCODE_MEM_REPORT_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(29);
        let mut text = String::new();
        let mut i = 0usize;
        while text.len() < size_mb * 1024 * 1024 {
            let id = i.wrapping_mul(2_654_435_761) as u32;
            text.push_str(&format!("{i:07} 2026-11-12 10:00:00.123 INFO  worker-{:02} request {id:08x} status=ok\n", i % 16));
            i += 1;
        }
        let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1000.0;
        let now = std::time::Instant::now;
        let ctx = ctx();
        let dir = tempfile::tempdir().unwrap();
        let mut app = AlexCodeApp::new();
        app.state_path = Some(dir.path().join("state.json"));
        let mut b = EditorBuffer::from_text(&text);
        b.file_path = Some("big.log".into());
        app.tabs = vec![b];
        app.active_tab_idx = 0;
        app.pending_select = Some((0, 0));
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let lines = app.tabs[0].lines.len();
        println!("COST file: {size_mb} MB, {lines} lines");

        // 1. 大檔案檢視重算 Git 行號區標記（每次內容變動後停 0.3 秒就會算一次）
        let head = app.tabs[0].lines.clone();
        let t = now();
        let n = diff_line_states(&head, &app.tabs[0].lines, 800).len();
        println!("COST git marks, identical to HEAD            = {:8.1} ms ({n} marks)", ms(t));
        let mid = Position { line: lines / 2, col: 0 };
        app.tabs[0].replace_range(mid, mid, "x", true);
        let t = now();
        let n = diff_line_states(&head, &app.tabs[0].lines, 800).len();
        println!("COST git marks, one line modified in middle  = {:8.1} ms ({n} marks)", ms(t));
        let top = Position { line: 0, col: 0 };
        app.tabs[0].replace_range(top, top, "new line\n", false);
        let t = now();
        let n = diff_line_states(&head, &app.tabs[0].lines, 800).len();
        println!("COST git marks, plus one line inserted at top = {:8.1} ms ({n} marks)", ms(t));
        drop(head);

        // 2. 尋找列：內容每變一次就重新串接全文並重新搜尋
        let t = now();
        let joined = app.tabs[0].edit_text();
        println!("COST join the full text                      = {:8.1} ms", ms(t));
        for q in ["status=ok", "worker-07", "zzz"] {
            let t = now();
            let n = find_all(&joined, q, &FindOptions::default()).map_or(0, |v| v.len());
            println!("COST find_all {q:<12}                    = {:8.1} ms ({n} hits)", ms(t));
        }
        drop(joined);
        let typing = |app: &mut AlexCodeApp| {
            let t = now();
            for _ in 0..5 {
                frame(app, &ctx, vec![Event::Text("x".into())]);
            }
            ms(t) / 5.0
        };
        println!("COST typing one character, find bar closed   = {:8.1} ms", typing(&mut app));
        app.show_find_bar = true;
        app.find_bar_text = "worker-07".into();
        let t = now();
        frame(&mut app, &ctx, vec![]);
        println!("COST first frame after opening the find bar  = {:8.1} ms ({} hits)", ms(t), app.find_cache.1.len());
        let t = now();
        for _ in 0..5 {
            frame(&mut app, &ctx, vec![]);
        }
        println!("COST idle frame, find bar open               = {:8.1} ms", ms(t) / 5.0);
        println!("COST typing one character, find bar open     = {:8.1} ms", typing(&mut app));
        app.show_find_bar = false;
        frame(&mut app, &ctx, vec![]);

        // 3. 熱結束：未存檔的大檔案每 30 秒把整份內容寫進備份檔一次（在介面執行緒上）
        assert!(app.tabs[0].modified);
        let t = now();
        let done = app.save_hot_exit(false);
        println!("COST hot-exit periodic write, on the UI thread = {:8.1} ms (finished={done}: the rest runs in the background)", ms(t));
        let t = now();
        let ok = app.save_hot_exit(true);
        let written = std::fs::metadata(dir.path().join("hot_exit.json")).map_or(0, |m| m.len());
        println!("COST hot-exit backup write                   = {:8.1} ms (ok={ok}, {:.1} MB on disk)", ms(t), written as f64 / 1048576.0);
        drop(app);

        // 4. 很大的 Markdown 檔在 Typora 閱讀模式（不走大檔案檢視）：每一幀的成本
        for md_mb in [1usize, 4] {
            let mut md = String::new();
            let mut i = 0usize;
            while md.len() < md_mb * 1024 * 1024 {
                md.push_str(&format!(
                    "## Section {i}\n\nParagraph {i} with **bold**, `code` and a [link](https://example.com/{i}). Some more words to fill the line.\n\n- item one\n- item two\n\n"
                ));
                i += 1;
            }
            let mut app = AlexCodeApp::new();
            let mut b = EditorBuffer::from_text(&md);
            b.file_path = Some("big.md".into());
            app.tabs = vec![b];
            app.active_tab_idx = 0;
            app.md_view.mode = MdViewMode::Typora;
            // 預設：超過門檻的 Markdown 檔以原始碼顯示
            let t = now();
            frame(&mut app, &ctx, vec![]);
            let first = ms(t);
            let t = now();
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            println!("COST {md_mb} MB Markdown file shown as source        = {:8.1} ms per frame (first frame {first:.1} ms)", ms(t) / 3.0);
            // 使用者選擇仍要用閱讀模式
            let id = app.tabs[0].id;
            app.md_big_preview.insert(id);
            let t = now();
            frame(&mut app, &ctx, vec![]);
            let first = ms(t);
            let t = now();
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            println!("COST Typora view of a {md_mb} MB Markdown file      = {:8.1} ms per frame (first frame {first:.1} ms, {} lines)", ms(t) / 3.0, app.tabs[0].lines.len());
        }
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

    /// egui 0.32 起游標只剩字元索引：自己換算的（列, 欄）與（段落, 位移）要與排版的列對得上（含折行處）
    #[test]
    fn cursor_conversions_follow_rows_and_paragraphs() {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        let font = FontId::monospace(14.0);
        let w = ctx.fonts_mut(|f| f.glyph_width(&font, 'x'));
        let job = egui::text::LayoutJob::simple("abcdefghij\nxy\n".into(), font, Color32::WHITE, w * 4.5);
        let g = ctx.fonts_mut(|f| f.layout_job(job));
        let texts: Vec<String> = g.rows.iter().map(|r| r.text()).collect();
        assert_eq!(texts, ["abcd", "efgh", "ij", "xy", ""], "rows of the wrapped layout");
        // 段落（檔案的行）與段落裡的位移
        assert_eq!(para_of(&g, 0), (0, 0));
        assert_eq!(para_of(&g, 5), (0, 5), "the second row of a wrapped line is still the first line");
        assert_eq!(para_of(&g, 10), (0, 10));
        assert_eq!(para_of(&g, 11), (1, 0));
        assert_eq!(para_of(&g, 13), (1, 2));
        assert_eq!(para_of(&g, 14), (2, 0));
        assert_eq!(para_of(&g, 99), (2, 0), "past the end: the last line");
        // 排版的列與列裡的位置：折行處依 prefer_next_row 決定
        assert_eq!(rc_of(&g, CCursor::new(5)), (1, 1));
        assert_eq!(rc_of(&g, CCursor { index: (4).into(), prefer_next_row: true }), (1, 0));
        assert_eq!(rc_of(&g, CCursor { index: (4).into(), prefer_next_row: false }), (0, 4));
        assert_eq!(rc_of(&g, CCursor::new(12)), (3, 1));
        // 點在第二列的結尾右邊：游標留在那一列的結尾（不跳到下一列的開頭）
        let row1 = &g.rows[1];
        assert_eq!(wrapped_col_at(&g, egui::vec2(row1.rect().right() + 20.0, row1.rect().center().y)), 7);
        // x 是相對於排版的位置：列的位置 + 字形在列裡的位置
        assert!((row_x(row1, 1) - (row1.pos.x + row1.glyphs[1].pos.x)).abs() < 0.01);
        assert!((g.pos_from_cursor(CCursor::new(5)).left() - row_x(row1, 1)).abs() < 0.01);
    }
}
