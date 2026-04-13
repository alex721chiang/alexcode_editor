# 🧠 方案五：模組化插件架構與協作生態實作計畫

## 1. 目標
讓 alexcode_editor 具備類似 VS Code 般無限的擴充能力，將封閉的單體 (Monolith) 程式架構轉變為生態系平臺，並加入即時的 Pair Programming 體驗。

## 2. 插件系統架構
### 2.1 C++ DLL/SO 動態加載 (C++ Plugin API)
定義一套純虛擬介面 (`IEditorPlugin`) 讓第三方庫實作：
- `virtual void init(IPluginContext* context) = 0;`
- `virtual QString name() const = 0;`

透過 Qt 的 `QPluginLoader`，在啟動時自動掃描 `~/.alexcode/plugins/` 底下所有的 `.dll` (Windows) 或 `.so` (Linux/Mac) 並掛載它們，動態新增 `QAction` 或 `QWidget`。

### 2.2 腳本引擎整合 (Python / JavaScript)
- **QJSEngine**：由於 Qt 內建 Javascript 引擎，我們可以將核心 API (如 `openFile`, `insertText`) 暴露給 JS 腳本，讓使用者能輕易撰寫簡單的 Macro。

## 3. 雲端共同編寫 (Live Share)
### 3.1 核心機制 (CRDT 或 OT)
要在多人間同步編輯，不只要傳送按鍵事件，更要解決衝突。
- 採用 CRDT (Conflict-free Replicated Data Type) 演算法來維持文本一致性。
- 將每次編輯行為（插入/刪除字元及其位置）打包成 Operation。

### 3.2 網路傳輸
- 實作基於 `QWebSocket` 的通訊機制。
- 客戶端分為 Host (房主) 與 Guest (參與者)。Host 建立 `QTcpServer` 搭配 WebSocket 接收所有 Guest 的連線。
- 同步多方游標位置，在 `CodeEditor` 繪製不同顏色的彩色游標 (`QPainter::drawRect`)。
