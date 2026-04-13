# 🧠 方案二：開發生命週期整合 (Git & Testing) 實作計畫

## 1. 目標
將 AlexCode 轉變為完整的 IDE，內建 Git 版本控制檢視、衝突解決，以及單元測試運行環境，減少開發者在終端機與編輯器間切換的時間。

## 2. Git 整合設計
### 2.1 Git Diff 視圖
- **架構**：使用 `QProcess` 在背景執行 `git diff`，捕捉 stdout。
- **UI 呈現**：
  - 在 `CodeEditor` 的左側行號區 (LineNumberArea) 新增顏色標記：綠色(新增)、紅色(刪除)、藍色(修改)。
  - 新增一個 `GitDiffWidget` 繼承自 `QSplitter`，實作雙欄 (Side-by-side) 對比視窗。

### 2.2 衝突解決嚮導 (Conflict Resolver)
- **架構**：解析 `<<<<<<< HEAD`、`=======` 與 `>>>>>>>` 標籤。
- **UI 呈現**：將衝突區域高亮顯示，並在上方提供「Accept Current Change」、「Accept Incoming Change」或「Accept Both」的點擊按鈕（利用 `QTextEdit::ExtraSelection` 與客製化按鈕覆蓋）。

## 3. Test Runner 整合
### 3.1 架構
- **掃描**：解析 `CMakeLists.txt` 或 `CTestTestfile.cmake` 找出所有測試目標 (Target)。
- **執行**：透過 `QProcess` 非同步執行測試指令 (如 `./tests/AlexCodeTests`)。

### 3.2 UI 呈現
- **Test Explorer**：新增一個 `QDockWidget`，以樹狀圖 (`QTreeView`) 顯示所有測試用例。
- **狀態顯示**：綠色打勾 (Pass) 或紅色打叉 (Fail)。雙擊失敗的測試可跳轉至原始碼行號。

## 4. 資源監控
- 利用 `QTimer` 定期讀取作業系統 API (Windows: `GetProcessMemoryInfo`, macOS: `task_info`)，在 StatusBar 顯示目前記憶體佔用。
