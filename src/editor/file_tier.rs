//! 大檔分級降級層級 (FileTier)
//!
//! 依檔案大小自動切換功能層級：
//! - Full: 全功能開啟
//! - AssistOff: 關閉自動完成 / LSP / 即時 Git，保留語法高亮
//! - HighlightOff: 連語法高亮一併關閉（完整大檔案純文字模式）

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Full,
    AssistOff,
    HighlightOff,
}

pub fn for_size_mb(mb: f64, assist_max_mb: u32, highlight_max_mb: u32) -> Level {
    if mb > highlight_max_mb as f64 {
        Level::HighlightOff
    } else if mb > assist_max_mb as f64 {
        Level::AssistOff
    } else {
        Level::Full
    }
}
