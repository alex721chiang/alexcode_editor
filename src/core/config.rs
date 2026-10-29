//! 全域組態與偏好設定 (Config)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub theme: String,
    pub font_size: f32,
    pub tab_size: usize,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub show_whitespace: bool,
    pub minimap: bool,
    pub sticky_scroll: bool,
    pub auto_pair: bool,
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
            word_wrap: true,
            show_line_numbers: true,
            show_whitespace: false,
            minimap: true,
            sticky_scroll: true,
            auto_pair: true,
            highlight_max_mb: 10,
            assist_max_mb: 2,
            long_line_chars: 10000,
        }
    }
}
