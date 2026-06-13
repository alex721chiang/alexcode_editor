# AlexCode — Neon Edition (v4.3)

Qt 6 程式碼編輯器，目標是接近 Notepad++ 的日常編輯體驗，並整合 LSP 語言伺服器、Git、
強大的 log 行篩選與多套未來感主題。

## ✨ 功能總覽

### Notepad++ 風格編輯
| 功能 | 快捷鍵 |
|---|---|
| 新檔 / 開檔 / 儲存 / 另存 / 全部儲存 | Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| 關閉分頁 / 全部關閉（未儲存會提示） | Ctrl+W / Ctrl+Shift+W |
| 複製當前行 / 刪除當前行 | Ctrl+D / Ctrl+Shift+L |
| 上 / 下移動行 | Alt+↑ / Alt+↓ |
| 切換註解（依語言 `//` 或 `#`） | Ctrl+/ |
| 轉大寫 / 轉小寫 | Ctrl+Shift+U / Ctrl+U |
| Find / Replace（大小寫、全字、Regex、即時全部標示） | Ctrl+F, F3 / Shift+F3 |
| 跳至行 | Ctrl+G |
| 快速開檔（模糊比對） | Ctrl+P |
| Find / Replace in Files | Ctrl+Shift+F |
| 文件符號清單 | Ctrl+Shift+M |
| 縮放 | Ctrl+滾輪、Ctrl+= / Ctrl+- / Ctrl+0 |

其他：自動縮排、Tab/Shift+Tab 多行縮排、括號配對高亮、分頁未儲存 ● 標記、拖放開檔、
可拖曳排序分頁、書籤（Ctrl+F2 / F2 / Shift+F2）、巨集錄製重播、導覽歷史（Alt+←/→）、
Session 工作階段還原、自動快照當機復原、Big5↔UTF-8 / CRLF↔LF 偵測切換、大檔案模式（>50MB 自動降級）。

### 進階編輯（v4.3）
| 功能 | 操作 |
|---|---|
| 程式碼摺疊 | 行號區 ▾/▸ 點擊、Ctrl+Shift+[ / ]（大括號 + 縮排混合判斷） |
| 分割視窗 | Ctrl+\（同文件雙視圖，編輯與 Undo 即時同步） |
| 多游標 | 選字後 Ctrl+Shift+D 逐一加選相同字串，輸入同步套用，Esc 結束 |
| Snippet 樣板 | trigger + Tab 展開（`alexcode-snippets.json`，支援 `${1:預設}` / `$0`、跟隨縮排） |

### LSP 語言伺服器（v1 + v2）
開啟 C/C++ 或 Python 檔案時自動連線對應語言伺服器：
| 功能 | 操作 |
|---|---|
| 即時診斷 | 錯誤（洋紅）/ 警告（黃）波浪底線，滑鼠停留顯示訊息 |
| 程式碼補全 | Ctrl+Space，或輸入伺服器觸發字元（如 `.`、`->`）自動彈出 |
| 跳至定義 | F12（跨檔案，接導覽歷史 Alt+←） |
| 全部引用 | Shift+F12（底部 REFERENCES 面板，雙擊跳轉） |
| 重新命名符號 | Ctrl+Alt+R（跨檔案套用，每檔單一 Undo） |
| 格式化文件 | Shift+Alt+F |
| Hover 資訊 | 滑鼠停留於符號顯示文件 |
| 狀態列 | `LSP: clangd ✓ E2 W1`（伺服器、錯誤/警告數） |

文件同步在伺服器支援時採用增量同步（只送變更範圍）。設定：Tools → 外部工具 →
「編輯 LSP 設定檔」（`alexcode-lsp.json`），預設 `clangd`（C/C++）與 `pylsp`（Python），
可自行增改副檔名對應與啟動參數。未安裝伺服器時靜默停用並於狀態列提示一次；大檔案模式自動停用。

### Git 整合
- **行標示（gutter）**：行號區左緣即時顯示與 HEAD 的差異 — 新增行（青）、修改行（黃）、刪除位置（洋紅三角）。編輯停頓 0.6 秒重算；commit 後隨 30 秒週期自動消除。
- **狀態列**：目前分支與變更檔數。
- **檔案樹染色**：EXPLORER 中已修改（黃）/ 未追蹤（青）檔案上色。

### Log 行篩選
- 關鍵字以 `||` 分隔（向下相容舊式 `|`），例如 `error || timeout || 失敗`
- 進階語法：`error && !heartbeat || fatal`（`&&` 群組內 AND、`!` 排除）
- **`re:` 正規表示式**：`re:err(or)?s? && !debug`（無效 pattern 自動退回字面比對）
- **Fuzzy 模糊比對**：子序列匹配（`mwin` → `MainWindow`）
- **多色標示**：正向關鍵字六色霓虹循環，編輯器內直接染色
- **篩選預設集**：工具列下拉儲存／套用／刪除常用條件
- **Tail 聯動**：檔案被外部寫入時自動重載並重套篩選
- **時間軸密度條**：呈現命中於整份文件的分布，點擊跳至該區段
- 「→ Tab」把結果抽成新分頁，可再次篩選做漏斗分析

### 文字工具箱（Tools 選單）
行整理（排序/去重/移除空行/反轉/修剪）、JSON 格式化/壓縮/驗證、XML 格式化/驗證、
編解碼（Base64 / URL / HTML Entity / Unicode escape）、時間戳 ↔ 時間、雜湊（MD5 / SHA-1 / SHA-256）、
數字底數轉換、全形半形轉換、摘要統計、檔案比較 Diff、Markdown 預覽、匯出 HTML、
建置任務（F5 + `alexcode-tasks.json`，雙擊錯誤跳行）、外部工具選單。

### 主題系統
內建三套主題，可於設定中心即時切換：
- **Neon Grid**（預設）：深空藍底 + 霓虹青/洋紅
- **Paper Light**：亮色護眼
- **Matrix**：綠調終端風

調色盤集中於 `src/Theme.h`，以單一 `Palette` 結構驅動全套 QSS 與編輯器色票。

### 自訂與可攜
- **介面語言（i18n）**：設定中心可選 系統預設 / English / 繁體中文，選單、對話框、狀態列訊息全面雙語化（Qt `tr()` + 內嵌 `.qm`），重新啟動後生效。
- **快捷鍵自訂**：Tools → 外部工具 →「編輯快捷鍵設定檔」（`alexcode-keys.json`，首次自動輸出現況模板），含衝突偵測，存檔即生效。
- **可攜模式**：執行檔旁放 `portable.ini`，所有設定 / Session / 組態改存 `./data`，解壓即用、不碰系統登錄檔。一般模式則存於使用者 AppData（`settings.ini`）。

翻譯來源為 `src/i18n/app_en.ts`、`app_zh_TW.ts`，以 `lupdate` 抽取、`lrelease` 編譯為 `.qm` 後經 `resources.qrc` 內嵌。

## 建置

需求：Qt 6（含 `Qt5Compat` 模組）、CMake ≥ 3.16、C++17 編譯器、Ninja（建議）。

```bash
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/path/to/Qt/6.x/<kit>
cmake --build build
./build/src/AlexCode
```

Windows（MinGW + Qt 6.8.3）打包可攜版：
```powershell
windeployqt --release --compiler-runtime build\src\AlexCode.exe
```

## 測試
```bash
cd build && ctest   # 或直接執行 ./tests/AlexCodeTests
```
48 項單元測試，涵蓋篩選引擎（OR/AND、`||` 解析與舊式 `|` 相容、模糊比對、`re:` regex）、
LSP 協定層（框架切割/重組、診斷/補全/定義/hover/references/rename/formatting 解析、增量同步 diff、
伺服器能力解析）、Git gutter 行級 diff、與 10 萬行效能測試。
另有真實 clangd 無頭整合測試（`tests/lsp_smoke.cpp`，需系統已安裝 clangd）。
