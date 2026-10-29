//! AlexCode 主題與調色盤系統
//!
//! 內建四套風格主題：
//! - Neon Grid（預設）：深空藍底 + 霓虹青 / 洋紅
//! - Neon HUD：Neon Grid 的未來科技變體 — 更深底色、強烈光暈邊界與 HUD 角標覆蓋
//! - Paper Light：亮色護眼
//! - Matrix：綠調終端風

use std::collections::HashMap;

/// 代表一組主題調色盤
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub name: String,
    pub bg: &'static str,
    pub panel: &'static str,
    pub border: &'static str,
    pub accent: &'static str,
    pub accent_dim: &'static str,
    pub accent2: &'static str,
    pub fg: &'static str,
    pub fg_dim: &'static str,
    pub fg_dim2: &'static str,
    pub fg_list: &'static str,
    pub faint: &'static str,
    pub hover: &'static str,
    pub pressed: &'static str,
    pub current_line: &'static str,
    pub sel_bg: &'static str,
    pub search_bg: &'static str,
    pub search_fg: &'static str,
    pub bracket_bg: &'static str,
    pub warn: &'static str,
    // 語法高亮 (keyword, type, comment, string, func, preprocessor, number)
    pub syn_kw: &'static str,
    pub syn_type: &'static str,
    pub syn_comment: &'static str,
    pub syn_string: &'static str,
    pub syn_func: &'static str,
    pub syn_pre: &'static str,
    pub syn_num: &'static str,
}

impl Palette {
    /// 將十六進位色碼 (如 "#00e5ff") 轉換為 RGB u8 組
    pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            (r, g, b)
        } else {
            (255, 255, 255)
        }
    }

    /// 將十六進位色碼轉為 RGBA (alpha: 0..=255)
    pub fn hex_to_rgba(hex: &str, alpha: u8) -> (u8, u8, u8, u8) {
        let (r, g, b) = Self::hex_to_rgb(hex);
        (r, g, b, alpha)
    }
}

/// 取得所有內建調色盤定義
pub fn builtin_palettes() -> HashMap<&'static str, Palette> {
    let mut map = HashMap::new();

    map.insert(
        "Neon Grid",
        Palette {
            name: "Neon Grid".to_string(),
            bg: "#0a0e17",
            panel: "#0d1420",
            border: "#1c2940",
            accent: "#00e5ff",
            accent_dim: "#00b8d4",
            accent2: "#ff2d95",
            fg: "#d6e4ff",
            fg_dim: "#9db4d8",
            fg_dim2: "#7d93b8",
            fg_list: "#b8cdf0",
            faint: "#3d5a80",
            hover: "#15233a",
            pressed: "#1a2c47",
            current_line: "#101a2e",
            sel_bg: "#234a7d",
            search_bg: "#3a2a55",
            search_fg: "#ffd166",
            bracket_bg: "#1a3a4a",
            warn: "#ffd166",
            syn_kw: "#00e5ff",
            syn_type: "#ff79c6",
            syn_comment: "#5c6f8a",
            syn_string: "#c3f73a",
            syn_func: "#ffd166",
            syn_pre: "#ff9e64",
            syn_num: "#bd93f9",
        },
    );

    map.insert(
        "Neon HUD",
        Palette {
            name: "Neon HUD".to_string(),
            bg: "#070b12",
            panel: "#0b111c",
            border: "#1b2c47",
            accent: "#00e5ff",
            accent_dim: "#00b8d4",
            accent2: "#ff2d95",
            fg: "#e2ecff",
            fg_dim: "#9db4d8",
            fg_dim2: "#7d93b8",
            fg_list: "#c3d6f5",
            faint: "#3d5a80",
            hover: "#14243c",
            pressed: "#182f4d",
            current_line: "#0e1a2e",
            sel_bg: "#245089",
            search_bg: "#3a2a55",
            search_fg: "#ffd166",
            bracket_bg: "#16394a",
            warn: "#ffd166",
            syn_kw: "#00e5ff",
            syn_type: "#ff79c6",
            syn_comment: "#5c6f8a",
            syn_string: "#c3f73a",
            syn_func: "#ffd166",
            syn_pre: "#ff9e64",
            syn_num: "#bd93f9",
        },
    );

    map.insert(
        "Paper Light",
        Palette {
            name: "Paper Light".to_string(),
            bg: "#fafafa",
            panel: "#f0f0f0",
            border: "#d0d0d0",
            accent: "#0066cc",
            accent_dim: "#0052a3",
            accent2: "#d81b60",
            fg: "#1a1a1a",
            fg_dim: "#444444",
            fg_dim2: "#555555",
            fg_list: "#333333",
            faint: "#999999",
            hover: "#e3eefc",
            pressed: "#d0e2f7",
            current_line: "#eef4fb",
            sel_bg: "#bcd6f5",
            search_bg: "#ffe9a8",
            search_fg: "#5a4500",
            bracket_bg: "#cfe6ff",
            warn: "#b8860b",
            syn_kw: "#0000cc",
            syn_type: "#267f99",
            syn_comment: "#008000",
            syn_string: "#a31515",
            syn_func: "#795e26",
            syn_pre: "#af00db",
            syn_num: "#098658",
        },
    );

    map.insert(
        "Matrix",
        Palette {
            name: "Matrix".to_string(),
            bg: "#050905",
            panel: "#0a120a",
            border: "#143814",
            accent: "#00ff66",
            accent_dim: "#00cc52",
            accent2: "#ffaa00",
            fg: "#c8efc8",
            fg_dim: "#8fbf8f",
            fg_dim2: "#7aa97a",
            fg_list: "#b0dcb0",
            faint: "#3f6f3f",
            hover: "#0f2410",
            pressed: "#143514",
            current_line: "#0c1a0c",
            sel_bg: "#1d4d28",
            search_bg: "#2c4d10",
            search_fg: "#eaff90",
            bracket_bg: "#15402a",
            warn: "#ffd166",
            syn_kw: "#00ff66",
            syn_type: "#7af0a0",
            syn_comment: "#3f6f3f",
            syn_string: "#eaff90",
            syn_func: "#b0dcb0",
            syn_pre: "#ffaa00",
            syn_num: "#00cc52",
        },
    );

    map
}

/// 主題管理器
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current_name: String,
    palettes: HashMap<&'static str, Palette>,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current_name: "Neon Grid".to_string(),
            palettes: builtin_palettes(),
        }
    }

    pub fn theme_names() -> &'static [&'static str] {
        &["Neon Grid", "Neon HUD", "Paper Light", "Matrix"]
    }

    pub fn current_theme_name(&self) -> &str {
        &self.current_name
    }

    pub fn set_theme(&mut self, name: &str) {
        if self.palettes.contains_key(name) {
            self.current_name = name.to_string();
        } else {
            self.current_name = "Neon Grid".to_string();
        }
    }

    pub fn current_palette(&self) -> &Palette {
        self.palettes
            .get(self.current_name.as_str())
            .unwrap_or_else(|| self.palettes.get("Neon Grid").unwrap())
    }

    pub fn is_neon_hud(&self) -> bool {
        self.current_name == "Neon HUD"
    }
}
