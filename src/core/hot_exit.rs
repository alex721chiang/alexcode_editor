//! 熱結束（hot exit）：把尚未存檔的分頁內容保存起來，下次啟動時還原
//!
//! 存成設定資料夾下的 `hot_exit.json`（與 `state.json` 同一個資料夾）。內容是純文字、只留在本機；
//! 沒有任何未存檔的分頁時檔案會被移除。

use super::file_watch::Stamp;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// 一個尚未存檔的分頁
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UnsavedTab {
    /// 分頁的檔案路徑；None = 未命名
    pub path: Option<String>,
    /// 以 `\n` 換行的全文
    pub text: String,
    /// 存檔時是否使用 CRLF 換行
    pub crlf: bool,
    /// 存檔編碼的名稱（狀態列上顯示的那個）
    pub encoding: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// 保存當時是否為目前的分頁
    pub active: bool,
    /// 保存當時磁碟上那個檔案的 (大小, 修改時間)；未命名或檔案不存在時為 None。
    /// 還原時用來察覺檔案在這段期間被其他程式改過。
    pub disk: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotExit {
    pub tabs: Vec<UnsavedTab>,
}

impl HotExit {
    /// 讀取失敗（檔案不存在、內容損毀）一律視為沒有保存過
    pub fn load(path: &Path) -> Option<Self> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    /// 沒有任何分頁時移除檔案；否則先寫到暫存檔再改名，避免寫到一半當機留下損毀的備份
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if self.tabs.is_empty() {
            return match std::fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(self).map_err(io::Error::other)?)?;
        std::fs::rename(&tmp, path)
    }
}

/// 把檔案在磁碟上的版本（`stamp_of` 的結果）換成可以寫進備份檔的 (大小, 修改時間：自 1970 年起的毫秒)
pub fn disk_signature(stamp: Option<Stamp>) -> Option<(u64, u64)> {
    let (modified, len) = stamp?;
    Some((len, modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64))
}
