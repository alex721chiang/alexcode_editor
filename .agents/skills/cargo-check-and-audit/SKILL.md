---
name: cargo-check-and-audit
description: 專門用於檢查 Rust 專案代碼質量、安全性漏洞與格式規範。涵蓋 cargo clippy、cargo audit 與 cargo fmt。當使用者要求檢查代碼品質、安全審查或格式化時使用。
---

# Cargo Check and Audit

此技能用於全面檢查 Rust 程式碼的代碼風格、潛在 Bug、安全性漏洞與格式。

## 檢查流程與步驟

1. **程式碼格式檢查 (`cargo fmt`)**：
   ```powershell
   cargo fmt --check
   ```
   *若格式有誤，可使用 `cargo fmt` 自動修復。*

2. **靜態分析與 Linter (`cargo clippy`)**：
   ```powershell
   cargo clippy --all-targets -- -D warnings
   ```
   *檢視所有 targets（包含 lib、bin、tests、benches）的警告與最佳實踐建議。*

3. **相依套件安全性弱點掃描 (`cargo audit`)**：
   - 檢查系統是否已安裝 `cargo-audit`，若尚未安裝則引導或執行安裝：
     ```powershell
     if (-not (Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
         Write-Host "Installing cargo-audit..."
         cargo install cargo-audit
     }
     cargo audit
     ```
   - 若發現有漏洞的依賴庫（Advisories），分析是否有可升級版本並通知使用者修復。

4. **單元與整合測試 (`cargo test`)**：
   ```powershell
   cargo test --all-targets
   ```
