//! 檔案總管側邊欄用的資料夾列舉

use std::io;
use std::path::{Path, PathBuf};

/// 單一資料夾最多列出的項目數（避免數十萬個檔案的資料夾拖垮介面）
pub const MAX_DIR_ENTRIES: usize = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// 列出資料夾內容：資料夾在前、檔案在後，各自依名稱（不分大小寫）排序
pub fn list_dir(dir: &Path) -> io::Result<Vec<DirEntryInfo>> {
    let mut out: Vec<DirEntryInfo> = std::fs::read_dir(dir)?
        .flatten()
        .take(MAX_DIR_ENTRIES)
        .map(|e| DirEntryInfo {
            name: e.file_name().to_string_lossy().into_owned(),
            is_dir: e.file_type().map_or(false, |t| t.is_dir()) || e.path().is_dir(),
            path: e.path(),
        })
        .collect();
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}
