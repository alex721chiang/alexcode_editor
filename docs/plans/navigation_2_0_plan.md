# 🧠 方案四：視覺化呼叫圖譜 (Navigation 2.0) 實作計畫

## 1. 目標
協助開發者快速掌握程式碼結構，透過追蹤函數呼叫，建立視覺化關聯圖，降低重構時的風險，特別適合用於追查大型 Legacy 系統。

## 2. 核心架構 (非同步 Call Graph)
### 2.1 QTextBlock 迭代器掃描
利用 `FilterEngine` 的高效能經驗，將文本掃描改造成解析呼叫關係。
- 建立一個 `CodeParser` 類別，在背景執行緒 (`QThread` 或 `QtConcurrent::run`) 讀取所有載入的專案文件。
- 使用 `QRegularExpression` 來分析函數宣告 (`Definition`) 與函數呼叫 (`Invocation`)，針對 C++ 可先找尋 `type name(args)` 與 `name(args)` 的模式。

### 2.2 呼叫關係建立
- 以 `QHash<QString, QSet<QString>>` 作為資料結構，紀錄 `Caller -> Callees` (誰呼叫了誰) 與 `Callee -> Callers` (被誰呼叫)。
- 設計一個 LRU Cache，只在檔案內容變動時重新更新。

## 3. UI/UX 設計
### 3.1 Context Menu (右鍵選單)
- 在 `CodeEditor::contextMenuEvent` 增加兩個選項：
  1. `Show Call Graph` (誰呼叫我)
  2. `Show Outgoing Calls` (我呼叫誰)

### 3.2 視覺化圖譜顯示
- 建立一個新的 `QGraphicsView` 子視窗 (`CallGraphWidget`)。
- 將 `Caller` 視為根節點，與 `Callees` 節點繪製連線 (`QGraphicsLineItem`)，並可利用 Force-directed graph 演算法自動排版節點位置。
- 支援雙擊節點自動跳轉至原始碼中對應的行號 (類似 `FindInFilesDialog` 的機制)。
