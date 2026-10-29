//! AlexCode Editor — Neon Edition (Rust Port)
//!
//! 一款高效、具未來感的 Notepad++ 風格程式碼編輯器，以 Rust 重寫。
//! 整合了 LSP 語言伺服器、Git 版本控制、強大的 Log 行篩選、Obsidian 風格 Markdown 雙鏈筆記圖形、
//! VT100/ANSI 終端機、以及多套未來感調色盤主題。

pub mod ai;
pub mod core;
pub mod editor;
pub mod filter;
pub mod git;
pub mod index;
pub mod lsp;
pub mod markdown;
pub mod plugins;
pub mod search;
pub mod snippets;
pub mod terminal;
pub mod theme;
pub mod ui;

pub use ui::AlexCodeApp;
