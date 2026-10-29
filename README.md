# AlexCode — Neon Edition (Rust Port v5.3)

[![Rust](https://img.shields.io/badge/language-Rust_2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-neon.svg)](https://github.com/emilk/egui)

AlexCode 的全新 **Rust 重寫版本**。從原本的 C++20 / Qt 6 架構完全移植為高效、記憶體安全且跨平台的純 Rust 實現。
保留了 Notepad++ 風格的日常編輯體驗、強大的 Log 行篩選引擎、LSP 語言伺服器協定支援、Git 行級差異與 Blame 追蹤、Obsidian 風格 Markdown 雙鏈筆記關聯圖、VT100 終端機，以及極具未來感的 **Neon Grid / Neon HUD** 霓虹調色盤主題。

---

## 🚀 核心特性 (Features)

### 1. Notepad++ 風格高生產力編輯
- **多游標編輯 (Multi-Cursor)**：Sublime 式多游標，支援多處同時輸入、選取與行操作。
- **欄位 / 矩形選取 (Box / Column Selection)**：`Alt + 拖曳` 矩形區塊選取，跨行同欄批次輸入與刪除。
- **括號與引號智慧自動配對 (Auto-Pair)**：自動補閉合、選取包圍 (Surround)、輸入閉合跳過 (SkipOver)、退格成對刪除。
- **行操作捷徑**：
  - 複製當前行 (`Ctrl+D`)
  - 刪除當前行 (`Ctrl+Shift+L`)
  - 上下移動行 (`Alt+↑` / `Alt+↓`)
  - 智慧切換註解 (`Ctrl+/`)
  - 轉大寫 / 小寫 (`Ctrl+Shift+U` / `Ctrl+U`)
- **文字工具箱 (Text Tools)**：行排序、去重、反轉、全形半形互轉、Unicode Escape/Unescape、數字底數解析。
- **大檔自動分級降級 (File Tiering)**：
  - `< 2MB`：Full 全功能模式（自動補全、LSP、即時 Git、語法高亮）
  - `> 2MB`：AssistOff 模式（停用 LSP / 即時背景掃描，保留高亮）
  - `> 10MB`：HighlightOff 模式（極速純文字流暢滾動）

### 2. 強大 Log 行篩選引擎 (FilterEngine)
- 支援以 `||`（或相容舊式 `|`）分割 OR 關鍵字條件。
- 支援進階語法：`error && !heartbeat || timeout`（`&&` 群組內 AND，`!` 前綴排除）。
- 支援 `re:` 正規表示式前綴（不分大小寫），如 `re:err(or)?s? && !debug`。無效 Regex 自動降級為字面比對不崩潰。
- 子序列模糊比對（如 `mwin` 符合 `MainWindow`）。
- 自動提取正向關鍵字供多色霓虹標籤即時染色。

### 3. Git 版本控制整合
- **行標示 (Gutter Diff)**：基於 Myers / LCS 動態規劃演算法，即時計算行級變更 — 新增行（青色條）、修改行（黃色條）、刪除位置（洋紅三角）。支援共同前綴/後綴修剪與大檔保護。
- **並排 Diff 對齊檢視器 (Side-by-side Diff)**：左右雙欄對照與差異行配對。
- **行內 Git Blame**：解析 `git blame --line-porcelain`，狀態列即時顯示 commit SHA、作者、日期、摘要及未提交變更標示。

### 4. LSP (Language Server Protocol) 語言伺服器
- 完整 JSON-RPC 2.0 協定層（`Content-Length` 框架封裝與串流 Reader）。
- 即時診斷解析 (Diagnostics)。
- 程式碼智慧補全與 Snippet 佔位符過濾 (`extract_completions`)。
- 跳至定義 (Goto Definition)、尋找全部引用 (References)。
- 符號重新命名 (Rename) 與跨檔案工作區編輯套用 (`apply_text_edits`)。
- 增量同步 (Incremental Sync) diff 計算，UTF-16 代理對安全。

### 5. Markdown 筆記雙鏈與知識圖譜 (Obsidian Style)
- **Wikilink 雙鏈解析**：支援 `[[筆記名稱]]`、`[[筆記|別名]]`、`[[筆記#章節]]` 與標準 `[text](path.md)`。
- 自動過濾程式碼區塊與行內代碼。
- **全庫反向連結索引 (Backlinks Index)**：自動建立雙向參照關係與有向邊。
- **力導向知識圖譜 (Fruchterman-Reingold Graph Layout)**：決定性均勻圓周初始佈局、斥力與引力模擬、向心重力防止孤立節點漂移。

### 6. VT100 / ANSI 整合終端機
- 純邏輯 VT100/ANSI 狀態機與 2D 螢幕單元模型。
- 支援 CR/LF/BS/TAB (對齊 8 倍數)、EOL 自動折行、5000 行 Scrollback 捲動歷史。
- CSI 游標移動 (CUU, CUD, CUF, CUB, CHA, CUP)、ED (2J) 全螢幕清除、EL (K) 行尾清除。
- 完整 SGR 色彩支援（標準 8 色、粗體、高亮 16 色 90..97 / 100..107）。
- UTF-8 多位元組跨封包邊界累積解碼（繁體中文、Emoji 正常顯示不亂碼）。

### 7. 未來感主題系統 (Themes)
- **Neon Grid**（預設）：深空藍底 (`#0a0e17`) + 霓虹青 (`#00e5ff`) + 洋紅 (`#ff2d95`)。
- **Neon HUD**：更深底色與光暈邊界，搭配未來科技 HUD 角標覆蓋。
- **Paper Light**：明亮護眼淺色風格。
- **Matrix**：經典綠黑駭客終端風格。

---

## 📁 專案架構 (Architecture)

```text
alexcode_rust/
├── Cargo.toml                  # 套件設定與依賴庫
├── README.md                   # 說明文件
├── src/
│   ├── lib.rs                  # 函式庫根入口
│   ├── main.rs                 # 應用程式啟動入口
│   ├── theme/                  # 主題調色盤系統 (Neon Grid, HUD, Paper, Matrix)
│   ├── filter/                 # Log 行篩選引擎 (||, &&, !, re:, Fuzzy)
│   ├── git/                    # Git Gutter, Git Blame, 並排 Diff 計算
│   ├── editor/                 # 緩衝區、多游標、AutoPair、BoxSelect、TextTools
│   ├── lsp/                    # LSP JSON-RPC 2.0 協定、Reader、增量同步
│   ├── markdown/               # Wikilink 解析、Backlink 索引、力導向知識圖譜
│   ├── terminal/               # VT100/ANSI 解析器、螢幕 Cell 模型、選取擷取
│   ├── search/                 # CommandMatch 模糊評分、Find in Files 搜尋引擎
│   ├── index/                  # 符號型別判定、專案符號快取索引
│   ├── ai/                     # 本地離線智慧補全、OpenAI 相容 LLM 客戶端
│   ├── snippets/               # 樣板代碼展開控制器
│   ├── plugins/                # 外掛指令擴充系統
│   ├── core/                   # 網路路徑判斷、全域組態設定
│   └── ui/                     # Egui / Eframe 現代化 GPU 加速前端介面
└── tests/                      # 涵蓋全模組的完整單元測試套件
```

---

## 🛠️ 建置與執行 (Build & Run)

### 前置需求
- 安裝 [Rust](https://rustup.rs/) (建議 1.70 以上版本)

### 建置與啟動
```bash
# 檢查與編譯
cargo build --release

# 執行 AlexCode
cargo run --release

# 執行所有單元測試
cargo test
```

---

## 🧪 測試涵蓋 (Test Coverage)

專案包含對應於原 C++ 專案各個模組的完整單元測試：
- `filter_test.rs`：AND / OR 邏輯、`||` 與 `|` 相容、進階語法、Regex 前綴與錯誤容忍。
- `diff_calc_test.rs`：LCS 行對齊、純新增/純刪除、Modified 配對、不平衡區塊填充。
- `auto_pair_test.rs`：括號引號補全、選取包圍、閉合跳過、成對刪除。
- `box_select_test.rs`：矩形欄位計算、短行 clamp、反向拖曳正規化。
- `command_match_test.rs`：命令面板評分（前綴 > 包含 > 模糊子序列）。
- `md_link_test.rs`：Wikilink 雙鏈別名剝除、代碼區塊排除、正規化鍵。
- `graph_layout_test.rs`：力導向座標邊界檢查、決定性佈局驗證。
- `vt_parser_test.rs`：VT100 游標移動、螢幕清除、SGR 色彩、UTF-8 跨 Feed 解碼。
- `local_completion_test.rs`：前綴與模糊排序、詞頻加權、就近度評分。
- `text_tools_test.rs`：行排序、去重、反轉、全半形、Unicode、進位解析。
- `git_blame_test.rs`：Porcelain 格式解析、未提交狀態標示。
- `git_gutter_test.rs`：HEAD 差異標記、中段 LCS、大檔保護。
- `lsp_protocol_test.rs`：框架封裝、增量同步 diff、TextEdit 逆序套用。
- `file_tier_test.rs`：大檔大小門檻判定。

---

## 📄 授權 (License)

本專案採用 [MIT License](LICENSE) 授權開源。
