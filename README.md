# AlexCode — Neon Edition (v3)

Qt6 程式碼編輯器，目標是接近 Notepad++ 的日常編輯體驗，搭配強大的行篩選功能與未來感深色介面。

## ✨ v3 新功能

### Notepad++ 風格編輯功能
| 功能 | 快捷鍵 |
|---|---|
| 新檔 / 開檔 / 儲存 / 另存 / 全部儲存 | Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| 關閉分頁 / 全部關閉（未儲存會提示） | Ctrl+W / Ctrl+Shift+W |
| 複製當前行 | Ctrl+D |
| 刪除當前行 | Ctrl+Shift+L |
| 上 / 下移動行 | Alt+↑ / Alt+↓ |
| 切換註解（依語言 `//` 或 `#`） | Ctrl+/ |
| 轉大寫 / 轉小寫 | Ctrl+Shift+U / Ctrl+U |
| Find / Replace（大小寫、全字、Regex、即時全部標示） | Ctrl+F, F3 / Shift+F3 |
| 跳至行 | Ctrl+G |
| Find in Files（支援 `\|\|` 多關鍵字） | Ctrl+Shift+F |
| 縮放 | Ctrl+滾輪、Ctrl+= / Ctrl+- / Ctrl+0 |

其他：自動縮排（`{`、`:` 結尾自動加一層）、Tab/Shift+Tab 多行縮排、括號配對高亮、
分頁未儲存 ● 標記與關閉提醒、拖放開檔、可拖曳排序分頁、
狀態列（行/列、選取長度、總行數、語言、編碼）。

### LSP 語言伺服器（v4.1 新增）
開啟 C/C++ 或 Python 檔案時自動連線對應語言伺服器：
| 功能 | 操作 |
|---|---|
| 即時診斷 | 錯誤（洋紅）/ 警告（黃）波浪底線，滑鼠停留顯示訊息 |
| 程式碼補全 | Ctrl+Space（取代游標下的字詞前綴） |
| 跳至定義 | F12（跨檔案，接導覽歷史 Alt+←） |
| 符號資訊 | 滑鼠停留於符號上顯示 hover 文件 |
| 狀態列 | `LSP: clangd ✓ E2 W1`（伺服器、錯誤/警告數） |

伺服器設定：Tools → 外部工具 → 「編輯 LSP 設定檔」（`alexcode-lsp.json`），
預設 `clangd`（C/C++）與 `pylsp`（Python），可自行增改副檔名對應與啟動參數。
未安裝伺服器時靜默停用並於狀態列提示一次。大檔案模式下自動停用。

### Git Gutter 行標示（v4.2 新增）
檔案在 git 版控中時，行號區左緣即時顯示與 HEAD 的差異：
新增行（青色條）、修改行（黃色條）、刪除位置（洋紅三角）。
編輯停頓 0.6 秒自動重算；commit 後隨狀態列週期（30 秒）自動消除標示。
未版控檔案與大檔案模式自動停用。

### 行篩選（Filter）
- 關鍵字以 `||` 分隔（向下相容舊式 `|`），例如：`error || timeout || 失敗`
- AND / OR 邏輯切換
- **Fuzzy 模糊比對**：子序列匹配，例如 `mwin` 可比對 `MainWindow`
- 顯示符合行數，雙擊結果跳至該行

### Neon Grid 未來感主題
深空藍 `#0a0e17` 底色 + 霓虹青 `#00e5ff` / 霓虹洋紅 `#ff2d95` 強調色，
全套自訂 QSS（選單、分頁、工具列、捲軸、狀態列），行號區當前行霓虹高亮。
主題定義集中於 `src/Theme.h`，方便調整配色。

## 建置
```bash
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/src/AlexCode
```

## 測試
```bash
cd build && ctest   # 或直接執行 ./tests/AlexCodeTests
```
涵蓋 OR/AND 邏輯、`||` 解析（含舊式 `|` 相容）、模糊比對、10 萬行效能測試。
