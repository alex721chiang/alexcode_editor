## AlexCode v6.0.0（Rust 版）

v5.5.0 之後的主要更新：介面框架 egui / eframe 從 0.27 升到 0.37；LSP、互動式終端機、自動補全、矩形選取都接上介面；新增分割檢視、獨立視窗與大檔案檢視。

### 新功能
- **分割檢視**（`Ctrl+\`、檢視 → 分割編輯器）：左右或上下兩欄，拖曳分隔線調整大小，點另一欄直接換過去編輯；另一欄也顯示 Git 標記、書籤、尋找命中與診斷，Markdown 以閱讀模式顯示；重新啟動後還原分割狀態。
- **獨立視窗**：把分頁拖到分頁列外面（或分頁右鍵「在新視窗開啟」）開成獨立視窗，可以編輯、選取、複製貼上，有 Git 標記和與主視窗相同的選單；拖出去的分頁不再出現在主視窗的分頁列，關掉視窗後回到主視窗；重新啟動後照原位置還原。
- **LSP**：依檔案類型啟動 rust-analyzer / clangd / pylsp（需自行安裝並放在 PATH 裡）：診斷（行號顏色與波浪底線）、跳到定義（`F12` / `Ctrl+點擊`）、滑鼠停留說明、自動完成選單（打字時或 `Ctrl+Space`）。
- **補全**：沒有語言伺服器的檔案也會列出文件裡出現過的字；設定對話框可啟用遠端 AI 補全（OpenAI 相容端點，例如 LM Studio、Ollama；預設不啟用）。
- **互動式終端機**：在虛擬終端（Windows ConPTY / Unix PTY）裡執行 shell，輸出即時顯示，支援 ANSI 16 色與游標，大小跟著面板；點一下輸出區後鍵盤（含方向鍵、Tab、`Ctrl+C`）直接送進 shell；可選擇 cmd / PowerShell / Git Bash（其他平台為 bash / zsh / sh）。
- **矩形選取**：`Alt+拖曳` 或 `Shift+Alt+方向鍵`；可複製、剪下、貼上，打字 / Backspace / Delete 同時修改每一行。
- **Sticky Scroll**：捲動時把目前所在的函式 / 類別標題固定在編輯區頂端。
- **設定對話框**（`Ctrl+,`）：字型大小、Tab 插入空白與寬度、自動換行、啟動時還原工作階段、自動配對、Sticky Scroll、遠端 AI 補全。
- **全專案取代**：先預覽會改寫的檔案與處數，確認後才寫入。
- **Git**：狀態列顯示游標所在行的 blame；目前內容與 Git HEAD 並排比較（檢視 → 與 Git HEAD 比較）。
- **熱結束**：有未存檔的修改時直接關閉視窗，下次啟動連同未存檔的內容（含未命名分頁）一起還原。
- **背景開檔**：2 MB 以上的檔案與網路路徑上的檔案在背景讀取，視窗不凍結。
- **macOS**：新增 universal（Apple Silicon + Intel）的 `AlexCode.app`（含圖示與檔案關聯）；從 Finder 雙擊或用「打開方式」開檔會開進編輯器。

### 大檔案
- 2 MB 以上的檔案改用大檔案檢視，只排版畫面上看得到的行：29 MB 的檔案開啟後的記憶體從約 5.9 GB 降到約 190 MB。可以直接輸入、復原、尋找、選取、自動換行，很長的行（例如壓成一行的 JSON）也整行看得到。
- 256 KB 以上的 Markdown 以原始碼顯示（說明列上的按鈕可換回閱讀模式）。

### 中文輸入法
- 選字時游標與候選字視窗跟著輸入法的位置移動（一般編輯區與大檔案檢視）。
- 終端機可以用中文輸入法輸入。

### 其他
- egui / eframe 0.27 → 0.37。
- CI 把 clippy 警告視為錯誤。

### 下載
| 平台 | 檔案 |
|---|---|
| Windows x64 | `alexcode.exe`（單一執行檔）或 `AlexCode-Rust-win64-portable.zip` |
| Linux x86_64 | `AlexCode-Rust-linux-x86_64.tar.gz` |
| macOS（Apple Silicon + Intel） | `AlexCode-Rust-macos-universal.zip`（`AlexCode.app`） |
| macOS（Apple Silicon） | `AlexCode-Rust-macos-arm64.tar.gz`（只有執行檔） |

### 已知限制
- 程式沒有簽章：Windows 可能出現 SmartScreen 警告；macOS 第一次開啟需按右鍵 →「打開」，或在「系統設定 → 隱私權與安全性」允許。
- macOS 的 `.app` 與 Finder 開檔只在 CI 上建置與測試，沒有在真的 Mac 上實際操作過。
- 從獨立視窗的選單執行「尋找 / 取代」時，尋找列開在主視窗、作用在主視窗的分頁。
