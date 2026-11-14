# AlexCode — Neon Edition (Rust v5.5)

[![Rust](https://img.shields.io/badge/language-Rust_2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-neon.svg)](https://github.com/emilk/egui)
[![Release](https://img.shields.io/badge/Release-v5.5.0--rust-00e5ff.svg)](https://github.com/alex721chiang/alexcode_editor/releases/latest)

AlexCode 的全新 **Rust 重寫版本**。從原本的 C++20 / Qt 6 架構完全移植為高效、記憶體安全且跨平台的純 Rust 實現。
保留了 Notepad++ 風格的日常編輯體驗、強大的 Log 行篩選引擎、LSP 語言伺服器協定支援、Git 行級差異與 Blame 追蹤、**Typora 風格直觀 Markdown 即時預覽與原位編輯**、Obsidian 雙鏈筆記關聯圖、VT100 終端機，以及極具未來感的 **Neon Grid / Neon HUD** 霓虹調色盤主題。

---

## 📥 直接下載 Release 版 (Download)

免安裝、單一執行檔內嵌圖示與 Orbitron 字型，下載後直接雙擊即可執行（Windows 10 / 11 x64）：

- 🚀 **[Windows：`alexcode.exe`（單一獨立執行檔）](https://github.com/alex721chiang/alexcode_editor/releases/latest/download/alexcode.exe)**
- 📦 **[Windows：`AlexCode-Rust-win64-portable.zip`（免安裝可攜包）](https://github.com/alex721chiang/alexcode_editor/releases/latest/download/AlexCode-Rust-win64-portable.zip)**
- 🐧 **[Linux x86_64：`AlexCode-Rust-linux-x86_64.tar.gz`](https://github.com/alex721chiang/alexcode_editor/releases/latest/download/AlexCode-Rust-linux-x86_64.tar.gz)**
- 🍎 **[macOS (Apple Silicon)：`AlexCode-Rust-macos-arm64.tar.gz`](https://github.com/alex721chiang/alexcode_editor/releases/latest/download/AlexCode-Rust-macos-arm64.tar.gz)**（未簽章，第一次執行需在系統設定中允許）
- 🔗 **[前往 GitHub Releases 頁面查看版本說明](https://github.com/alex721chiang/alexcode_editor/releases/latest)**

---

## 🆕 v5.5 新增 (What's New)

| 功能 | 操作 |
|---|---|
| 分頁右鍵選單：開啟所在資料夾、在此開啟 cmd / PowerShell / Windows Terminal、複製路徑、關閉其他 / 右側 / 全部 | 在分頁上按右鍵 |
| 檔案總管側邊欄、開啟資料夾 | `Ctrl+B`、檔案 → 開啟資料夾 |
| 尋找與取代（區分大小寫 / 全字 / 正規表示式） | `Ctrl+F`、`Ctrl+H` |
| 編碼偵測與轉換（UTF-8 / BOM / UTF-16 / Big5 / GBK / Shift_JIS） | 點狀態列的編碼 |
| 未儲存提示、外部變更自動重新載入 | 自動 |
| 工作階段還原、最近開啟的檔案 | 自動、檔案 → 最近開啟的檔案 |
| 單一實例（雙擊檔案開在現有視窗） | 自動；`--new-instance` 另開視窗 |
| Git 行號區標記（新增 / 修改 / 刪除）、自動配對括號 | 自動；檢視選單可關閉自動配對 |

**v5.5.0 之後加入（尚未發佈，需自行建置）**

| 功能 | 操作 |
|---|---|
| Sticky Scroll：捲動時把目前所在的函式 / 類別標題固定在編輯區頂端（Rust、C / C++ / Java / C# / Go、JavaScript / TypeScript、Python；最多 5 層），點標題可跳到該行 | 自動；檢視選單可關閉 |
| 設定對話框：字型大小、Tab 鍵插入空白與 Tab 寬度、自動換行、啟動時還原工作階段、自動配對、Sticky Scroll；修改立即生效並存入 `state.json` | `Ctrl+,` |
| 自動換行：程式碼編輯區把過長的行折到下一列顯示（不改變檔案內容，行號仍以檔案的行計算；預設關閉） | 設定對話框、檢視 → 自動換行 |
| 全專案取代：在「全專案搜尋」視窗輸入取代文字，先預覽會改寫的檔案與處數，確認後才寫入（不分大小寫的純文字比對；只處理 UTF-8 文字檔；寫入後無法用 `Ctrl+Z` 復原） | `Ctrl+Shift+F` →「預覽取代」→「執行取代」 |
| 背景開檔：2 MB 以上的檔案與網路路徑（`\\server\share\…`）上的檔案在背景讀取，分頁先顯示「載入中…」，視窗不凍結 | 自動 |
| Git blame：狀態列右側顯示游標所在行最後一次修改的提交（短 SHA、作者、日期、摘要）；尚未存檔的修改也算在內，未提交的行顯示「未提交變更」 | 自動（檔案需在 Git 儲存庫內且已追蹤） |
| Git 差異檢視：目前內容（含尚未存檔的修改）與 HEAD 並排比較，可在各段變更之間移動，點右側的一列跳到該行 | 檢視 → 與 Git HEAD 比較、命令面板 |
| 大檔案檢視：4 MB 以上的檔案只排版畫面上看得到的行，29 MB 的檔案開啟後佔用的記憶體從約 5.9 GB 降到約 190 MB。可以捲動、選取、複製、尋找、跳至行、設書籤；不能直接打字，沒有自動換行 | 自動；要編輯時按編輯區上方的「改用一般編輯區」（會用回原本的記憶體量） |

> **目前狀態**：下方列出的 LSP、AI 補全、矩形選取，其邏輯模組與測試已在程式庫中，但尚未接上介面；
> 內建終端機為逐指令執行（非互動式）。完整說明見 [RELEASE_NOTES.md](RELEASE_NOTES.md)。

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
