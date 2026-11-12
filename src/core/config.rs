//! 全域組態與偏好設定 (Config)
//!
//! 由設定對話框修改，隨 `state.json` 的 `config` 區塊保存。
//! 目前介面實際使用的欄位：`font_size`、`tab_size`、`insert_spaces`、`word_wrap`、`sticky_scroll`、
//! `auto_pair`、`restore_session`；其餘欄位保留給尚未接上介面的功能。

use serde::{Deserialize, Serialize};

pub const MIN_FONT_SIZE: f32 = 8.0;
pub const MAX_FONT_SIZE: f32 = 36.0;
pub const MIN_TAB_SIZE: usize = 1;
pub const MAX_TAB_SIZE: usize = 8;

/// 設定檔缺少的欄位（舊版本存的檔）一律使用預設值
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub theme: String,
    pub font_size: f32,
    /// 按 Tab 鍵插入空白時的定位點間隔
    pub tab_size: usize,
    /// 按 Tab 鍵插入空白（對齊到下一個定位點）而不是 Tab 字元
    pub insert_spaces: bool,
    /// 程式碼編輯區把超過視窗寬度的行折到下一列顯示（不改變檔案內容）
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub show_whitespace: bool,
    pub minimap: bool,
    pub sticky_scroll: bool,
    pub auto_pair: bool,
    /// 啟動時重新開啟上次的檔案與資料夾
    pub restore_session: bool,
    pub highlight_max_mb: u32,
    pub assist_max_mb: u32,
    pub long_line_chars: usize,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            theme: "Neon Grid".to_string(),
            font_size: 14.0,
            tab_size: 4,
            insert_spaces: false,
            // 預設關閉：與接上介面之前的行為（長行以水平捲動檢視）相同
            word_wrap: false,
            show_line_numbers: true,
            show_whitespace: false,
            minimap: true,
            sticky_scroll: true,
            auto_pair: true,
            restore_session: true,
            highlight_max_mb: 10,
            assist_max_mb: 2,
            long_line_chars: 10000,
        }
    }
}

impl EditorConfig {
    /// 把超出範圍的值（例如手動改過的設定檔）修正到介面可用的範圍
    pub fn sanitized(mut self) -> Self {
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
        } else {
            Self::default().font_size
        };
        self.tab_size = self.tab_size.clamp(MIN_TAB_SIZE, MAX_TAB_SIZE);
        self
    }
}
