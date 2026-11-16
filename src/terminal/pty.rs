//! 互動式終端機的 shell：以虛擬終端（Windows 的 ConPTY、其他平台的 PTY，經由 `portable-pty`）啟動，
//! 輸出由背景執行緒一讀到就送進 channel（不等指令結束），介面每一幀取出來餵給 `VtParser`。
//! shell 結束時另一條背景執行緒會收到結束代碼。

use std::io::{Read, Write};
use std::path::Path;

use crossbeam_channel::{unbounded, Receiver};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

/// 預設的 shell：Windows 是 `%COMSPEC%`（通常是 cmd.exe），其他平台是 `$SHELL`（沒有時 /bin/sh）
pub fn default_shell() -> String {
    #[cfg(windows)]
    let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    #[cfg(not(windows))]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    shell
}

/// 一個在虛擬終端裡執行的 shell
pub struct PtySession {
    // 虛擬終端本身：一直留著，關掉它 shell 的輸出端也會跟著關閉
    _master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    output: Receiver<Vec<u8>>,
    exit: Receiver<u32>,
    exit_code: Option<u32>,
}

impl PtySession {
    /// 在 `cwd` 啟動 `program`，虛擬終端的大小是 `rows` × `cols`。
    /// `notify` 在每次讀到輸出、以及 shell 結束時被呼叫（在背景執行緒上；介面用它要求重畫）。
    pub fn spawn(
        program: &str,
        cwd: &Path,
        rows: u16,
        cols: u16,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let pair = native_pty_system()
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("無法建立虛擬終端：{e}"))?;
        let mut cmd = CommandBuilder::new(program);
        cmd.cwd(cwd);
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| format!("無法啟動 {program}：{e}"))?;
        // 子程序已經拿到它那一端了；這裡留著的話，Windows 上 shell 結束後讀取端也不會結束
        drop(pair.slave);
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader().map_err(|e| format!("無法讀取虛擬終端：{e}"))?;
        let writer = pair.master.take_writer().map_err(|e| format!("無法寫入虛擬終端：{e}"))?;

        let notify = std::sync::Arc::new(notify);
        let (tx, output) = unbounded();
        let n = notify.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(len) => {
                        if tx.send(buf[..len].to_vec()).is_err() {
                            break;
                        }
                        n();
                    }
                }
            }
        });
        let (exit_tx, exit) = unbounded();
        std::thread::spawn(move || {
            let code = child.wait().map(|s| s.exit_code()).unwrap_or(u32::MAX);
            let _ = exit_tx.send(code);
            notify();
        });
        Ok(PtySession { _master: pair.master, writer, killer, output, exit, exit_code: None })
    }

    /// 取出目前為止讀到、還沒取走的輸出（沒有時是空的）
    pub fn read_available(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Ok(chunk) = self.output.try_recv() {
            out.extend_from_slice(&chunk);
        }
        out
    }

    /// 送進 shell（就像在終端機裡打字；Enter 是 `\r`）
    pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// shell 已經結束時回傳結束代碼
    pub fn exit_code(&mut self) -> Option<u32> {
        if self.exit_code.is_none() {
            self.exit_code = self.exit.try_recv().ok();
        }
        self.exit_code
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.exit_code().is_none() {
            let _ = self.killer.kill();
        }
    }
}
