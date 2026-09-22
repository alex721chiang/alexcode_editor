# AlexCode — Neon Edition (v5.2)

Qt 6 程式碼編輯器，目標是接近 Notepad++ 的日常編輯體驗，並整合 LSP 語言伺服器、Git、
強大的 log 行篩選與多套未來感主題。

## ⬇️ 下載（Windows）
到 [Releases](https://github.com/alex721chiang/alexcode_editor/releases) 下載：
- **AlexCode-Setup-x.y.z.exe** — 一鍵安裝程式（含開始選單捷徑、解除安裝）
- **AlexCode-Windows-vx.y.z.zip** — 免安裝可攜版（解壓即執行 `AlexCode.exe`）

兩種都已用 windeployqt 內含 Qt 與 MSVC 執行期，免另外安裝。
（未做程式碼簽章，首次執行 Windows SmartScreen 可能提示，點「其他資訊 → 仍要執行」即可。）

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
Session 工作階段還原、自動快照當機復原、Big5↔UTF-8 / CRLF↔LF 偵測切換。
**多游標**（Sublime 式）：Ctrl+Shift+D 加選下一個相同字串、Alt+F3 全選所有相同字串、
Ctrl+點擊新增/移除游標、Alt+拖曳欄選；輸入/刪除/導覽/貼上同步套用（行數=游標數時逐行分配貼上）。
**大檔分級降級**：>2MB 關自動完成/LSP/即時 Git 標示（保留高亮）、>10MB 連語法高亮一併關閉；閾值可在偏好設定調整。
顯示空白字元/行尾符號可由「檢視」選單切換。

### 語法高亮
**tree-sitter 語法樹高亮**（C/C++、Python、JavaScript/TypeScript、JSON）：解析整份文件的語法樹，
依節點型別精準上色（理解巢狀結構、字串/註解邊界、模板/泛型），比正規表示式更準確；
編輯時採**增量解析**（保留語法樹、只重解析變更處）+ 120ms 防抖，並只重畫受影響的行，大檔輸入不卡；
其餘語言（XML/HTML、Markdown、CMake、Shell…）沿用內建 regex 高亮器。
色票跟隨目前主題（Neon Grid / Paper Light / Matrix），切換主題即時重繪。

### 進階編輯
| 功能 | 操作 |
|---|---|
| 命令面板 | Ctrl+Shift+P：模糊搜尋並執行所有指令，顯示對應快捷鍵 |
| 專案符號搜尋 | Ctrl+T：開資料夾時於背景用 tree-sitter 索引整個專案的類別/函式（不卡 UI、狀態列顯示進度），跨檔模糊搜尋並跳轉；存檔/外部變更增量更新；索引快取於 `.alexcode/symbols.json`（依 mtime，再開秒載）（Source Insight 風） |
| 專案找引用 | 右鍵「在專案中尋找引用」：跨檔文字版引用清單（REFERENCES 面板，雙擊跳轉）；有 LSP 時 Shift+F12 為精準語意版 |
| 跳至符號 | Ctrl+Shift+O：模糊搜尋目前檔的類別/函式並跳轉（tree-sitter 擷取符號） |
| Sticky scroll | 捲動時固定目前函式/類別標頭於頂端，點擊跳轉（檢視選單可開關） |
| Minimap | 右側程式碼縮圖，點擊/拖曳快速定位（檢視選單可開關） |
| 麵包屑導覽 | 編輯器頂部顯示「檔 › 類別 › 函式」，點擊任一段跳轉 |
| 括號自動配對 | 輸入 ( [ { " ' ` 自動補對側、選取後包圍、輸入閉合可跳過、退格刪空配對 |
| 字詞出現處高亮 | 游標停在識別字，可視範圍內同名字詞自動標示 |
| 縮排輔助線 | 各縮排層級淡色直線 |
| 程式碼摺疊 | 行號區 ▾/▸ 點擊、Ctrl+Shift+[ / ]（大括號 + 縮排混合判斷） |
| 分割視窗 | Ctrl+\（同文件雙視圖，編輯與 Undo 即時同步） |
| 多游標 | 選字後 Ctrl+Shift+D 逐一加選相同字串，輸入同步套用，Esc 結束 |
| 欄位／矩形編輯 | Alt + 滑鼠拖曳選取跨行同欄區塊，輸入/刪除同步套用至每一行 |
| 本地智慧補全 | 未啟用 LSP 時 Ctrl+Space：從文件擷取識別字，依前綴/模糊 × 出現頻率 × 與游標就近度排序（離線、免金鑰） |
| Snippet 樣板 | trigger + Tab 展開（`alexcode-snippets.json`，支援 `${1:預設}` / `$0`、跟隨縮排） |

### AI 輔助（本機/雲端 LLM）
工具→AI 輔助（OpenAI 相容端點；預設指向本機 LM Studio `127.0.0.1:1234`，設定檔可改雲端 + API key）：
- **AI 補全（Ctrl+Alt+A）**：游標前文脈送模型，建議以灰字 ghost text 顯示——Tab 接受、Esc 拒絕、
  繼續打字自動消失；多行建議顯示「⏎ +N 行」。設定檔開 `autoTrigger` 可改為編輯停頓自動觸發。
- **AI：解釋選取**：繁中解釋開新分頁。
- **AI：重構選取**：AI 建議與原文以**並排 diff** 對照，確認後一鍵套用（可復原）。
設定檔 `alexcode-ai.json`（端點/金鑰/模型/溫度/自動觸發），存檔即重載。

### 腳本外掛（JavaScript）
工具→腳本外掛。把 `.js` 放進外掛資料夾（選單可直接開啟），透過全域 `alexcode` 物件擴充編輯器：
```js
alexcode.registerCommand("插入日期時間", function() {
    alexcode.insertText(new Date().toLocaleString());
});
```
API：`text()/setText()`（可 Ctrl+Z 復原）、`selectedText()/insertText()`、
`currentLine()/lineCount()/line(n)/gotoLine(n)`、`currentFilePath()/openFile()`、
`statusMessage()/prompt()`。首次啟動自動附上範例（`examples.js`）；改完選「重新載入外掛」即生效。
選 JS 而非 C++ DLL：無編譯器 ABI 相容問題，存檔即用。

### 整合終端機
Ctrl+\` 開啟互動式終端機（停靠面板）。Windows 透過 ConPTY 接 PowerShell、
Linux/macOS 透過 forkpty 接 `$SHELL`（v4.7 起），支援 VT100/ANSI
色彩與粗體、游標控制、鍵盤轉送（含 Ctrl+C、方向鍵、Home/End）、視窗縮放同步、滾輪捲動回看、
UTF-8 多位元組字元（中文/emoji，跨封包邊界正確累積解碼）。
另保留第一版「輸出面板＋」（F5 建置任務輸出、雙擊錯誤跳行）。

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

### Markdown 筆記（Obsidian 風）
以資料夾為 vault，把 `.md` 之間的連結關係視覺化：
| 功能 | 說明 |
|---|---|
| **Wikilink** | `[[筆記名]]`、`[[名\|別名]]`、`[[名#標題]]`；編輯器內 Ctrl+點擊跳轉，vault 內找不到時自動建立新筆記 |
| **Backlinks 面板** | 檢視 → Backlinks：列出「有哪些檔連到目前這篇」，點擊開啟 |
| **關係圖** | 檢視 → 關係圖：節點＝筆記（大小依連結數）、邊＝連結，力導向佈局自動排開，點節點開檔，目前筆記高亮 |
| **高擬真預覽** | Markdown 預覽採 GitHub 風深色 CSS（標題層級、程式碼區塊、表格、引言），`[[wikilink]]` 渲染成可點擊的內部連結 |

連結解析會略過程式碼區塊與行內 `code`，並忽略 http(s)/mailto 外部連結。

### Git 整合
- **行標示（gutter）**：行號區左緣即時顯示與 HEAD 的差異 — 新增行（青）、修改行（黃）、刪除位置（洋紅三角）。編輯停頓 0.6 秒重算；commit 後隨 30 秒週期自動消除。
- **並排 diff 檢視器**（v4.7 起）：工具→比較目前分頁→「與 Git HEAD 並排比較」/「與磁碟版本並排比較」——
  左右雙欄逐列對齊（LCS）、增刪改整行底色、同步捲動、開啟自動跳到第一處差異。
- **行內 blame**（v4.7 起）：狀態列即時顯示游標行的 commit / 作者 / 日期 / 摘要（未提交變更會標示）。
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
內建四套主題，可於設定中心即時切換：
- **Neon Grid**（預設）：深空藍底 + 霓虹青/洋紅
- **Neon HUD**：Neon Grid 的未來科技變體 — 更深的底色、更強的強調色光暈與發光邊界，
  外加編輯區四角的 HUD 角標覆蓋層（航太儀表框感；非互動、滑鼠穿透，僅此主題顯示）
- **Paper Light**：亮色護眼
- **Matrix**：綠調終端風

調色盤集中於 `src/core/Theme.h`，以單一 `Palette` 結構驅動全套 QSS 與編輯器色票。
「關於」對話框以內嵌的 **Orbitron** 字型（OFL）呈現 ALEXCODE 未來科技字標。

### 自訂與可攜
- **介面語言（i18n）**：設定中心可選 系統預設 / English / 繁體中文，選單、對話框、狀態列訊息全面雙語化（Qt `tr()` + 內嵌 `.qm`），重新啟動後生效。
- **快捷鍵自訂**：Tools → 外部工具 →「編輯快捷鍵設定檔」（`alexcode-keys.json`，首次自動輸出現況模板），含衝突偵測，存檔即生效。
- **可攜模式**：執行檔旁放 `portable.ini`，所有設定 / Session / 組態改存 `./data`，解壓即用、不碰系統登錄檔。一般模式則存於使用者 AppData（`settings.ini`）。

翻譯來源為 `src/i18n/app_en.ts`、`app_zh_TW.ts`，以 `lupdate` 抽取；`.qm` 由 CMake `qt_add_translations` 於建置期自動編譯並內嵌至 `:/i18n`（不需手動 `lrelease`，`.qm` 不入版控）。

### 快速啟動
- **視窗即時顯示**：主視窗先繪出，上次工作階段的分頁再於背景**串流還原**（依原順序逐一載入），不再等所有檔案解析完才出現畫面。
- **延遲啟用重量級服務**：LSP（`clangd` 等）的 `didOpen` 與 Git gutter／blame 改為「分頁實際切到時才啟用」，前景開檔立即生效、背景還原的分頁則於首次切換時載入 — 避免多分頁還原時大量語言伺服器同步與 Git 子行程一次湧入拖慢開啟（Windows 尤其明顯）。
- 大檔仍沿用分級降級（`editor/highlightMaxMB`、`editor/assistMaxMB`）自動關閉高亮／LSP／即時 Git。
- **網路分享（SMB/NFS）友善**：還原工作階段時不在主執行緒碰原檔——非作用中分頁只建佔位，切到才於背景讀檔（讀不到時保留分頁，切回即重試）；未存檔分頁由本機快照還原並沿用原編碼／換行；專案資料夾存在檢查、Markdown 連結索引（僅在 Backlinks／Graph 開啟時建立）、資料夾監看清單皆改為背景執行。
- 網路路徑相關設定（`settings.ini`）：`index/watchNetworkFolders`（預設 `false`，網路資料夾不掛目錄監看）、`network/disableGit`（預設 `false`；設為 `true` 則網路路徑不跑 git gutter／blame／狀態）。網路專案的 git 狀態輪詢自動由 30 秒放寬為 120 秒。

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

## 打包與發佈

版本號統一在 `CMakeLists.txt` 的 `project(AlexCode VERSION x.y.z)`，建置時產生 `Version.h`，
顯示於視窗標題與「關於 AlexCode」對話框（Help 選單）。

一鍵打包可攜 zip：
```powershell
pwsh scripts/package.ps1 -QtDir C:\Qt\6.8.3\mingw_64
# → dist\AlexCode-Windows-vX.Y.Z.zip（已含全部 DLL 與 portable.ini）
```

**自動發佈**：推送 `v*` 標籤（如 `git tag v4.4.0 && git push origin v4.4.0`）會觸發
GitHub Actions 建置、打包 zip，並建立 GitHub Release 附上該 zip。

**程式碼簽章（未內建）**：要消除「未知發行者」警告需 Authenticode 憑證，取得後可用
`signtool sign /fd SHA256 /tr <時間戳記伺服器> /td SHA256 /f cert.pfx /p <密碼> AlexCode.exe` 簽署；
CI 中則把憑證放 GitHub Secrets 後於 release 前加一步簽章。本專案預設不簽。

## 測試
```bash
cd build && ctest   # 或直接執行 ./tests/AlexCodeTests
```
75 項單元測試，涵蓋篩選引擎（OR/AND、`||` 解析與舊式 `|` 相容、模糊比對、`re:` regex）、
LSP 協定層（框架切割/重組、診斷/補全/定義/hover/references/rename/formatting 解析、增量同步 diff、
伺服器能力解析）、Git gutter 行級 diff、VT100/ANSI 解析器（游標/清除/SGR/跨封包切割）、
矩形選取邏輯、本地補全排序、與 10 萬行效能測試。
另有無頭整合測試：真實 clangd（`tests/lsp_smoke.cpp`）與 PTY（`tests/pty_smoke.cpp`；
Linux CI 以 ctest 執行 forkpty 驗證，Windows ConPTY 版供手動執行）。

開發輔助：`AlexCode --screenshot out.png [檔案…]` 讓程式以 Qt 自我渲染存圖（不需實體螢幕、桌面鎖定亦可），方便自動化驗證 GUI。
