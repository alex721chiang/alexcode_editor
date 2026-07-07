# 🔍 專案級 API 快速搜尋索引 + 既有問題修復 實作計畫

## 0. 現況分析

專案已經有一套符號索引基礎設施，不是從零開始：

| 元件 | 檔案 | 現有能力 |
|---|---|---|
| 索引資料結構 | `ProjectSymbolIndex.h/.cpp` | `QHash<file, Entry[]>`，逐檔存符號 |
| 磁碟快取 | `ProjectSymbolIndexBuild.cpp` | `.alexcode/symbols.json`，依 `mtime` 增量重建（未變更的檔不重解析） |
| 搜尋 | `ProjectSymbolIndex::search()` / `exact()` | 目前是**線性掃描全部符號**，每次呼叫 `CommandMatch::score()` 逐一比對 |
| UI | `ProjectSymbolDialog` / `QuickOpenDialog` | 使用者輸入時即時查詢 |

**瓶頸診斷**：`search(query)` 對「每一個」已索引符號都算一次模糊分數，是 O(總符號數)。在中小型專案（數千符號）感覺不出來，但對照你在 Qualcomm 的工作型態（大型 C++ codebase、數萬到數十萬符號），每敲一個字元都重新掃全表，QuickOpen 打字會明顯卡頓。

**目標**：不要重寫現有架構，而是在 `ProjectSymbolIndex` 之上加一層「記憶體內索引」，把 O(N) 查詢降到 O(命中數)，同時保留現有的磁碟快取與 mtime 增量機制。

---

## 1. 目標與驗收標準

- 在 5 萬個符號規模下，QuickOpen/符號搜尋單次查詢延遲 < 5ms（目前線性掃描量級預估 30–100ms，取決於機器）
- 索引結構的新增/刪除單一檔案符號為 O(該檔符號數)，不需要整表重建
- 不破壞現有 `serialize()/deserialize()` 磁碟快取格式相容性（舊快取檔仍可讀）
- 保留現有 `CommandMatch::score()` 模糊比對邏輯（使用者體驗不變），只縮小候選集合

---

## 2. 架構設計

### 2.1 新增記憶體內索引層

在 `ProjectSymbolIndex` 內新增：

```cpp
// 名稱 → 該符號在 flat vector 中的位置（大小寫不敏感，用於精確/前綴查詢）
QMultiHash<QString, int> m_nameIndex;   // key = name.toLower()

// 首字元 bucket，讓模糊搜尋只需比對「可能相關」的候選，而非全表
QHash<QChar, QVector<int>> m_firstCharBucket;

QVector<Entry> m_flat;   // 取代目前每次重新展開 m_byFile 的做法
```

- `setFileSymbols()` / `removeFile()` 同步維護 `m_nameIndex`、`m_firstCharBucket`、`m_flat`（增量更新，不重建全表）
- `exact(name)`：直接查 `m_nameIndex[name.toLower()]`，O(1) 起跳
- `search(query)`：
  1. 先用 `m_firstCharBucket[query[0].toLower()]` 縮小候選（如果 query 非空）
  2. 只對候選集合跑 `CommandMatch::score()`，其餘邏輯不變
  3. query 為空或極短（如單一符號）時退回全表，避免 bucket 反而變大

### 2.2 是否需要 Trie / Suffix Array？

評估後**不建議**在此專案導入完整 Trie 或後綴陣列：
- 現有 `CommandMatch::score()` 是子序列模糊比對（fuzzy subsequence match，類似 Sublime/VSCode 的 QuickOpen），這種比對邏輯不容易直接映射到 Trie 的前綴查詢
- 首字元 bucket + 精確名稱雜湊，已經能把候選集合縮小到原本的 1/26～1/52（英文字母大小寫），對於「輸入 API 名稱前幾碼快速找到」的核心情境，效益已經足夠，複雜度成本卻低很多
- 如果之後要支援「子字串」而非「前綴」搜尋大量檔案內容（不是符號名），才需要考慮 n-gram 反向索引，屬於不同範疇，可另立計畫

### 2.3 磁碟快取維持不變、加一層「索引就緒」狀態

`build()` 流程不變（讀 `.alexcode/symbols.json` → 依 mtime 增量解析 → 寫回快取），只在 `deserialize()` 完成後呼叫一次 `rebuildMemoryIndex()`，把 `m_nameIndex`/`m_firstCharBucket`/`m_flat` 建好。之後的 `setFileSymbols()`/`removeFile()`（單檔更新、儲存觸發）改為增量維護，不再整表重建。

### 2.4（可選，第二階段）檔案變更自動更新索引

目前索引更新靠使用者手動觸發（`rebuildProjectSymbolIndex()`）或存檔時呼叫 `updateFileFromDisk()`。可以加一個 `QFileSystemWatcher` 監看專案資料夾，外部變更（例如 git pull、其他工具改檔）也能自動更新索引，而不必等使用者手動重建。此為錦上添花，不影響核心效能目標，排在第二階段。

---

## 3. 分階段時程

### Phase 0：修復既有問題（優先，風險較高，建議先做）

| 項目 | 檔案 | 內容 |
|---|---|---|
| 0-1 PTY 關閉競態 | `PtySession.cpp` | 改用 `CancelSynchronousIo(threadHandle)` 或改為 overlapped I/O + `CancelIoEx`，取代目前靠 `CloseHandle` 讓 `ReadFile` 中斷的做法 |
| 0-2 macOS/Linux 終端機 | `PtySession.cpp` | 短期：`start()` 失敗時明確顯示「此平台尚未支援內建終端機」而非通用錯誤訊息；長期：用 `forkpty()`(POSIX) 實作對應分支 |
| 0-3 MainWindow 拆分 | `MainWindow.cpp/.h` | 依職責拆出 `ProjectSymbolController`、`GitGutterController`、`TerminalController` 等子物件，MainWindow 保留窗口組裝與轉發，降低單檔案複雜度 |

> 0-3 屬於重構，建議在 0-1/0-2 穩定、且有更多測試覆蓋後再進行，避免與新功能開發衝突。

### Phase 1：記憶體索引核心（對應第 2 節設計）

1. `ProjectSymbolIndex` 新增 `m_nameIndex` / `m_firstCharBucket` / `m_flat`
2. 改寫 `setFileSymbols()` / `removeFile()` 為增量維護索引
3. 改寫 `exact()` / `search()` 走新索引
4. `deserialize()` 完成後呼叫 `rebuildMemoryIndex()` 一次性建索引

**單元測試（沿用現有 `tests/ProjectSymbolIndexTest.cpp` 風格）**：
- 索引 1000+ 假符號後，驗證 `exact()`/`search()` 結果與線性掃描版本完全一致（正確性優先於效能）
- 驗證 `removeFile()` 後該檔符號不再出現於 `m_nameIndex`／`m_firstCharBucket`
- 邊界：空 query、query 長度 1、符號名稱含中文（例如變數/函式帶中文註解場景）

### Phase 2：效能量測與調校

1. 寫一個一次性的 benchmark（可放 `tests/PerformanceTest.cpp` 現有檔案內擴充），產生 5 萬筆假符號，量測 `search()` 平均延遲
2. 依量測結果決定是否需要進一步優化（例如候選集合仍過大時，可再依符號 `kind` 分桶）

### Phase 3（可選）：檔案系統自動監看更新索引

依第 2.4 節設計，加入 `QFileSystemWatcher`，非必要但可以提升「多工具協作」情境下的索引新鮮度。

---

## 4. 風險與相容性

- **磁碟快取格式不變**：`serialize()`/`deserialize()` 的 JSON schema 不動，舊的 `.alexcode/symbols.json` 可直接沿用，使用者升級後不需要清快取
- **記憶體成本**：`m_nameIndex`/`m_firstCharBucket` 是索引，不是資料本體，額外記憶體約為符號數 × (指標/int 大小)，以 10 萬符號估算約數 MB 等級，可接受
- **Phase 0 與 Phase 1 可並行**：兩者觸碰的檔案不重疊（`PtySession.cpp`/`MainWindow.cpp` vs `ProjectSymbolIndex.cpp`），可以分別開分支同時進行，降低合併衝突風險

---

## 5. 建議順序

1. Phase 0-1（PTY 競態）— 風險最高，優先修
2. Phase 1（記憶體索引）— 你這次主要想要的功能
3. Phase 0-2（macOS 終端機訊息）— 小改動，可穿插
4. Phase 2（效能量測）— 驗證 Phase 1 成果
5. Phase 0-3（MainWindow 拆分）／Phase 3（自動監看）— 排在最後，屬於品質提升而非急迫需求
