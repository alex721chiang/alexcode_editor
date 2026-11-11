//! 取得檔案在 Git `HEAD` 的內容，供行號區的差異標記（Git gutter）比對

use std::path::Path;
use std::process::Command;

/// 檔案不在 Git 儲存庫內、未被追蹤、或系統沒有安裝 git 時回傳 None
pub fn head_lines(file: &Path) -> Option<Vec<String>> {
    let dir = file.parent()?;
    let name = file.file_name()?.to_str()?;
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).args(["show", &format!("HEAD:./{name}")]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：不要閃出主控台視窗
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let (text, _) = crate::editor::detect_and_decode(&out.stdout);
    Some(text.replace("\r\n", "\n").split('\n').map(|s| s.to_string()).collect())
}
