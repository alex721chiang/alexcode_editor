# 🧠 方案三：強制標準化的 Linting / Formatting 引擎實作計畫

## 1. 目標
在儲存檔案時自動格式化程式碼，並在編寫過程中即時提示潛在語法錯誤或警告，確保團隊代碼風格一致。

## 2. Formatting (存檔即校正) 設計
### 2.1 核心機制
- **工具選擇**：支援呼叫外部 `clang-format` (針對 C/C++) 或 `prettier` (針對 Web)。
- **觸發時機**：攔截 `MainWindow::saveFile()` 事件。
- **執行流程**：
  1. 擷取當前 `CodeEditor` 內的全部文本。
  2. 將文本透過 stdin 傳遞給 `QProcess` 執行的 `clang-format`。
  3. 捕捉 stdout 返回的格式化後文本。
  4. 如果有差異，使用 `QTextCursor::beginEditBlock()` 替換文本，以確保 Undo 歷史只記錄為一次操作。
  5. 執行實際的檔案寫入。

## 3. Linting (即時語法檢查) 設計
### 3.1 核心機制
- **工具選擇**：支援 `clang-tidy` 或 LSP (Language Server Protocol) 的診斷訊息 (Diagnostics)。
- **非同步掃描**：使用者停止輸入 (Idle) 約 500ms 後觸發。使用 `QTimer::singleShot` 實作防抖 (Debounce)。

### 3.2 UI 呈現
- **錯誤底線**：利用 `QTextCharFormat::setUnderlineStyle(QTextCharFormat::WaveUnderline)`，針對有問題的程式碼區段畫上紅色或黃色的波浪底線。
- **懸浮提示 (Tooltip)**：攔截 `QPlainTextEdit::mouseMoveEvent`，當游標懸停在波浪線上時，顯示具體的錯誤或警告訊息。
