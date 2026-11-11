//! 跨次啟動保存的狀態：主題、字型大小、上次開啟的檔案、最近開啟清單、專案資料夾
//!
//! 存成設定資料夾下的 `state.json`（Windows：`%APPDATA%\AlexCode`、
//! macOS：`~/Library/Application Support/AlexCode`、Linux：`$XDG_CONFIG_HOME/alexcode`）。

use super::config::EditorConfig;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

pub const MAX_RECENT_FILES: usize = 15;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub theme: String,
    /// 0 表示尚未設定（使用預設值）
    pub font_size: f32,
    /// 上次結束時開啟的檔案（僅已存檔的檔案）
    pub open_files: Vec<String>,
    pub active_file: Option<String>,
    /// 最近開啟的檔案，最新的在最前面
    pub recent_files: Vec<String>,
    /// 使用者以「開啟資料夾」指定的專案資料夾
    pub folder: Option<String>,
    pub show_explorer: bool,
    pub auto_pair: bool,
    /// 設定對話框的內容；None = 由沒有設定對話框的舊版本存的檔（沿用上面的 `font_size` / `auto_pair`）
    pub config: Option<EditorConfig>,
}

impl AppState {
    /// 讀取失敗（檔案不存在、內容損毀）一律視為沒有保存過
    pub fn load(path: &Path) -> Option<Self> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    /// 先寫到暫存檔再改名，避免寫到一半當機留下損毀的設定檔
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(io::Error::other)?)?;
        std::fs::rename(&tmp, path)
    }

    pub fn push_recent(&mut self, path: &str) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_string());
        self.recent_files.truncate(MAX_RECENT_FILES);
    }
}

/// 本程式的設定資料夾；環境變數都不存在時回傳 None（不保存狀態）
pub fn config_dir() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        var("APPDATA").map(|d| d.join("AlexCode"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|d| d.join("Library/Application Support/AlexCode"))
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|d| d.join(".config"))).map(|d| d.join("alexcode"))
    }
}
