# AlexCode（Rust 版）待辦清單

每日排程會從上到下挑 **2 個尚未完成（`[ ]`）的項目**實作、驗證、推送，完成後改成 `[x]` 並在該行後面註明日期與 commit。
項目太大無法在一次工作中做完並驗證時：把它拆成數個較小的子項目（寫回本檔），當次只完成其中可以完整驗證的部分。

完成的定義：`cargo test` 與 `cargo clippy --all-targets` 在 Windows 上通過、有對應的自動測試、行為實際確認過，且已推送到 GitHub。

## 待辦（依優先順序）

- [ ] 全專案取代（Find in Files 加上取代）：使用 `src/search/find_in_files.rs` 已有的取代預檢與寫入，先預覽影響的檔案數再執行
- [ ] 背景開檔：大檔案 / 網路磁碟上的檔案改在背景執行緒讀取，讀取中分頁顯示「載入中」，視窗不凍結
- [ ] Git blame：狀態列顯示游標所在行的作者與提交訊息（接上 `src/git/git_blame.rs`，在背景執行 `git blame --porcelain`）
- [ ] Git 差異檢視：與 HEAD 並排比較目前檔案（接上 `src/git/diff_calc.rs`；`show_diff_dialog` 欄位目前未使用）
- [ ] 熱結束（hot exit）：結束時保留未存檔 / 未命名分頁的內容，下次啟動還原
- [ ] Markdown 檢視的尋找高亮改用與編輯區相同的選項（區分大小寫 / 全字 / 正規表示式）
- [ ] 互動式終端機 1/3：以 ConPTY（Windows）/ PTY（Unix）啟動 shell，輸出即時串流到 `VtParser`（不再等指令結束）
- [ ] 互動式終端機 2/3：鍵盤輸入直接送進 shell（方向鍵、Ctrl+C、Tab），可選擇 cmd / PowerShell / Git Bash
- [ ] 互動式終端機 3/3：依 `VtParser` 的屬性顯示 ANSI 顏色、游標，視窗大小改變時通知 shell
- [ ] LSP 1/4：依語言啟動語言伺服器（rust-analyzer / clangd / pylsp），送出 initialize、didOpen、didChange；狀態列顯示連線狀態
- [ ] LSP 2/4：診斷（diagnostics）：行號區標記與錯誤 / 警告底線，滑鼠停留顯示訊息
- [ ] LSP 3/4：跳到定義（F12 / Ctrl+點擊）與滑鼠停留說明（hover）
- [ ] LSP 4/4：自動完成選單
- [ ] 矩形選取（Alt+拖曳）：接上 `src/editor/box_select.rs`
- [ ] 分割檢視：同一視窗並排顯示兩份文件
- [ ] 分頁拖出成獨立視窗
- [ ] AI 補全：接上 `src/ai`（本機補全先做，遠端 provider 需使用者自行設定金鑰）
- [ ] 升級 egui / eframe 到最新穩定版，升級後實測中文輸入法
- [ ] macOS：包成 `.app`（含圖示與檔案關聯）並產生 universal（arm64 + x86_64）版本
- [ ] 清理 clippy 警告（目前約 40 個），之後 CI 改為 `-D warnings`

## 已完成

- [x] 設定對話框（`Ctrl+,`）：Tab 寬度、自動換行、字型大小、是否還原工作階段；接上 `EditorConfig` 並存入 `state.json` 的 `config` 區塊。對話框、Tab 鍵插入空白、還原工作階段於 2026-11-11 完成（`f4c8ef2`）；自動換行（程式碼編輯區依視窗寬度折行，行號 / 書籤 / Git 標記 / Sticky Scroll 仍以檔案的行計算；預設關閉，Markdown 閱讀模式與 Log 篩選檢視不受影響）於 2026-11-12 完成 — 2026-11-12
- [x] Sticky Scroll：捲動時把目前所在的函式 / 區塊標題固定在編輯區頂端（接上 `src/editor/sticky_scroll.rs`，符號來源用 `src/index/symbols.rs` 新增的 `extract_symbols`：以文字掃描擷取 Rust / C 家族 / JavaScript / Python 的區塊，非語法樹）— 2026-11-11
- [x] v5.5.0-rust（2026-11-11）：編碼偵測、未儲存提示、單一套復原紀錄、分頁右鍵選單、效能快取、檔案總管、尋找與取代、工作階段還原、單一實例、外部變更偵測、Git 行號區標記、自動配對、三平台 CI 與發佈流程
