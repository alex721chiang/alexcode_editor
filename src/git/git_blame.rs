//! git blame --line-porcelain 輸出解析
//!
//! 每行格式由 header「<sha> <orig> <final> [count]」＋ 屬性行（author, author-time, summary 等）
//! ＋ 一行 tab 開頭的原始碼內容組成。

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineInfo {
    pub short_sha: String,     // 前 8 碼
    pub author: String,
    pub date: String,          // yyyy-MM-dd
    pub summary: String,
    pub uncommitted: bool,     // 全零 SHA 表示未提交的本地修改
}

/// 解析 git blame --line-porcelain 輸出
pub fn parse_porcelain(bytes: &[u8]) -> HashMap<usize, LineInfo> {
    let mut result = HashMap::new();
    let text = String::from_utf8_lossy(bytes);
    let lines = text.split('\n');

    let mut cur = LineInfo::default();
    let mut final_line: i32 = -1; // 1-based (porcelain header 第三欄)

    for line in lines {
        if line.starts_with('\t') {
            // 內容行 = 這一筆 record 結束
            if final_line > 0 {
                result.insert((final_line - 1) as usize, cur);
            }
            cur = LineInfo::default();
            final_line = -1;
            continue;
        }

        if final_line < 0 {
            // Header: <sha> <orig> <final> [count]
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0].len() == 40 {
                cur.short_sha = parts[0][0..8].to_string();
                cur.uncommitted = parts[0].chars().all(|c| c == '0');
                if let Ok(fl) = parts[2].parse::<i32>() {
                    final_line = fl;
                }
            }
            continue;
        }

        if line.starts_with("author ") {
            cur.author = line[7..].trim().to_string();
        } else if line.starts_with("author-time ") {
            if let Ok(secs) = line[12..].trim().parse::<i64>() {
                if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, 0) {
                    cur.date = dt.format("%Y-%m-%d").to_string();
                }
            }
        } else if line.starts_with("summary ") {
            cur.summary = line[8..].trim().to_string();
        }
    }

    result
}

/// 對 `contents`（檔案目前的內容，可以是尚未存檔的版本）執行 `git blame`，回傳每一行（0-based）的資訊。
/// 尚未提交的行會標為 `uncommitted`。會等 git 執行完才回傳：請在背景執行緒呼叫。
/// 檔案不在 Git 儲存庫內、未被追蹤、或系統沒有安裝 git 時回傳 None。
pub fn blame_contents(file: &Path, contents: &[u8]) -> Option<HashMap<usize, LineInfo>> {
    let dir = file.parent()?;
    let name = file.file_name()?.to_str()?;
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .args(["blame", "--line-porcelain", "--contents", "-", "--", name])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：不要閃出主控台視窗
    }
    let mut child = cmd.spawn().ok()?;
    // 內容由另一條執行緒寫入 stdin：git 的輸出把管線塞滿時，兩邊才不會互相等待
    let mut stdin = child.stdin.take()?;
    let data = contents.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().ok()?;
    let _ = writer.join();
    if !out.status.success() {
        return None;
    }
    Some(parse_porcelain(&out.stdout))
}

/// 產生狀態列顯示的行內 blame 字串："⎇ abcd1234 作者 2026-07-09 · 摘要"；未提交顯示「⎇ 未提交變更」
pub fn status_text(info: &LineInfo) -> String {
    if info.short_sha.is_empty() {
        return String::new();
    }
    if info.uncommitted {
        return "⎇ 未提交變更".to_string();
    }
    format!(
        "⎇ {} {} {} · {}",
        info.short_sha, info.author, info.date, info.summary
    )
}
