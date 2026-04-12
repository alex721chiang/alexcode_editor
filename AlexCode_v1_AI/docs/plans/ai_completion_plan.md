# 🧠 方案一：深度整合 AI-Powered Code Completion 實作計畫

## 1. 目標
將 `alexcode_editor` 從單純的文字編輯器升級為具備 AI 補全與重構功能的開發工具。主要串接本地端 Gemma 4 模型 (透過 LM Studio API) 或雲端模型。

## 2. 核心架構
採用 **「非同步、非阻塞 (Non-blocking)」** 的架構，確保編輯器在 AI 思考時不會卡頓。

### 2.1 新增類別
- `AICompletionProvider`: 負責處理網路請求 (使用 `QNetworkAccessManager`)。
- `SuggestionWidget`: 繼承自 `QListWidget` 的浮動視窗，用於顯示補全選項。

### 2.2 修改現有類別
- `CodeEditor`:
    - 覆寫 `keyPressEvent` 以捕捉觸發鍵。
    - 實作 `triggerAICompletion()`，傳送當前游標前的 Context。
    - 連結 `AICompletionProvider` 的結果訊號。

## 3. 實作流程
1.  **Context 擷取**：當使用者停頓或輸入特定符號時，擷取游標前約 2000-5000 tokens 的文字。
2.  **API 請求**：發送非同步 POST 請求至 `http://localhost:1234/v1/chat/completions`。
3.  **UI 呈現**：在游標位置計算座標並彈出 `SuggestionWidget`。
4.  **選取插入**：使用者按 `Tab` 或 `Enter` 插入建議，或按 `Esc` 關閉。

## 4. 關鍵 Signals/Slots
- `AICompletionProvider::requestStarted()`
- `AICompletionProvider::suggestionsReady(QStringList suggestions)`
- `CodeEditor::onAIResponseReceived(QStringList suggestions)`

## 5. UI/UX 設計
- 補全視窗應具備半透明效果。
- 顯示 AI 正在思考的狀態圖示（例如：角落轉圈圈）。
- 支援 `Ctrl + Space` 強制觸發。
