---
name: rust-doc-generator
description: 用於自動檢查與產生 Rust 函式庫/專案的文件註解，確保所有公開（pub）函式、結構體、列舉皆有 Markdown 文件註解並通過文檔測試 (cargo test --doc)。
---

# Rust Doc Generator

此技能用於自動維護與檢核 Rust 專案的文件品質。

## 檢查與生成步驟

1. **啟用文件遺漏檢查**：
   在 Crate 根目錄（如 `lib.rs` 或 `main.rs`）檢查是否包含：
   ```rust
   #![warn(missing_docs)]
   ```
   或使用 Clippy 針對缺少文檔進行檢查：
   ```powershell
   cargo clippy -- -W missing_docs
   ```

2. **檢查並補充公開介面文件**：
   針對所有 `pub fn`、`pub struct`、`pub enum`、`pub trait`：
   - 必須包含 Markdown 格式的說明區塊 `///`。
   - 包含：
     - 功能簡介（Summary）
     - `# Arguments` 參數說明（若有參數）
     - `# Returns` 回傳值說明（若有非 void 回傳）
     - `# Errors` 可能發生的錯誤條件（若回傳 Result）
     - `# Examples` 包含可執行的範例代碼：
       ```rust
       /// # Examples
       /// ```
       /// use my_crate::my_function;
       /// assert_eq!(my_function(1), 2);
       /// ```
       ```

3. **執行文件測試 (Doc Tests)**：
   驗證文檔中的程式碼範例能否編譯並測試通過：
   ```powershell
   cargo test --doc
   ```

4. **產出本機 HTML 文件**：
   ```powershell
   cargo doc --no-deps --document-private-items
   ```
