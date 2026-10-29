---
name: flamegraph-profiler
description: 用於整合 cargo-flamegraph 產生 Rust 專案的效能分析火焰圖（Flamegraph）。當使用者要求效能調校、Profile、分析 CPU 瓶頸時使用。
---

# Flamegraph Profiler

此技能用於對 Rust 應用程式進行效能分析（Profiling）並產出火焰圖（Flamegraph SVG）。

## 前置條件與安裝

1. **安裝 `cargo-flamegraph`**：
   ```powershell
   if (-not (Get-Command cargo-flamegraph -ErrorAction SilentlyContinue)) {
       Write-Host "Installing cargo-flamegraph..."
       cargo install flamegraph
   }
   ```

2. **Windows 平台注意事項**：
   在 Windows 上，`cargo-flamegraph` 依賴 DTrace 或專門的採樣驅動。
   - 若 Windows 支援 DTrace，需以系統管理員權限啟用：
     ```powershell
     bcdedit /set testsigning on
     ```
   - 若在 Windows 本機遇到 DTrace 權限或相容性問題，替代方案：
     - 使用 WSL2 (Linux) 環境執行（搭配 Linux perf，最穩定推薦）
     - 或搭配 `samply`：
       ```powershell
       cargo install samply
       samply record ./target/release/<binary>.exe
       ```

## 產生火焰圖步驟

1. **在 Cargo.toml 啟用除錯符號（若需精準分析 release 版本）**：
   ```toml
   [profile.release]
   debug = true
   ```

2. **執行 Profiling 產出火焰圖**：
   針對 Binary：
   ```powershell
   cargo flamegraph --bin <bin_name> -- <arguments>
   ```
   針對 Benchmark / Test：
   ```powershell
   cargo flamegraph --test <test_name>
   ```

3. **檢視結果**：
   預設會在專案根目錄產生 `flamegraph.svg`，可直接在瀏覽器或 IDE 預覽。
