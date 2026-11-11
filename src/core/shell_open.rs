//! 在系統檔案總管顯示檔案、在指定資料夾開啟外部終端機
//!（分頁右鍵選單「開啟檔案所在資料夾 / 在此開啟命令提示字元 / PowerShell」）

use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// Windows 命令提示字元 (cmd.exe)
    Cmd,
    /// PowerShell（優先 pwsh 7，沒有則用內建 Windows PowerShell）
    PowerShell,
    /// Windows Terminal (wt.exe)
    WindowsTerminal,
    /// macOS / Linux 的系統預設終端機
    SystemTerminal,
}

/// 此作業系統可用的終端機種類與選單文字
pub fn available_shells() -> &'static [(ShellKind, &'static str)] {
    if cfg!(windows) {
        &[
            (ShellKind::Cmd, "在此開啟命令提示字元 (cmd)"),
            (ShellKind::PowerShell, "在此開啟 PowerShell"),
            (ShellKind::WindowsTerminal, "在此開啟 Windows Terminal"),
        ]
    } else {
        &[(ShellKind::SystemTerminal, "在此開啟終端機")]
    }
}

/// `explorer.exe` 的 `/select,` 參數：路徑要另外加引號（含空白的路徑才不會被截斷）
pub fn explorer_select_arg(path: &Path) -> String {
    format!("/select,\"{}\"", path.display())
}

/// cmd.exe 不能把 UNC 路徑（`\\server\share`）當工作目錄；改用 `pushd` 自動掛載磁碟機代號
pub fn cmd_needs_pushd(dir: &Path) -> bool {
    dir.to_string_lossy().starts_with(r"\\")
}

/// 在檔案總管中顯示（並選取）指定檔案；`path` 為資料夾時直接開啟該資料夾
pub fn reveal_in_file_manager(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("explorer.exe");
        if path.is_dir() {
            c.arg(path);
        } else {
            c.raw_arg(explorer_select_arg(path));
        }
        c.spawn().map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        Command::new("xdg-open").arg(dir).spawn().map(|_| ())
    }
}

/// 開啟一個獨立的終端機視窗，工作目錄為 `dir`
pub fn open_terminal(dir: &Path, kind: ShellKind) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        match kind {
            ShellKind::WindowsTerminal => Command::new("wt.exe").arg("-d").arg(dir).spawn().map(|_| ()),
            ShellKind::PowerShell => {
                let run = |exe: &str| {
                    Command::new(exe)
                        .arg("-NoLogo")
                        .current_dir(dir)
                        .creation_flags(CREATE_NEW_CONSOLE)
                        .spawn()
                        .map(|_| ())
                };
                run("pwsh.exe").or_else(|_| run("powershell.exe"))
            }
            _ => {
                let mut c = Command::new("cmd.exe");
                if cmd_needs_pushd(dir) {
                    c.raw_arg(format!("/K pushd \"{}\"", dir.display()));
                } else {
                    c.current_dir(dir);
                }
                c.creation_flags(CREATE_NEW_CONSOLE).spawn().map(|_| ())
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = kind;
        Command::new("open").args(["-a", "Terminal"]).arg(dir).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = kind;
        let mut last = io::Error::new(io::ErrorKind::NotFound, "找不到終端機程式");
        for exe in ["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
            match Command::new(exe).current_dir(dir).spawn() {
                Ok(_) => return Ok(()),
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}
