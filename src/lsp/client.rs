//! LSP 行程客戶端 (LspClient)
//!
//! 透過子行程 stdio 與語言伺服器 (如 clangd, rust-analyzer, pylsp) 進行非同步通訊。
//! 也可以接在任何一對讀寫端上（`from_io`，測試用假的伺服器）。

use super::protocol::{frame, Reader};
use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

pub struct LspClient {
    child: Option<Child>,
    stdin: Box<dyn Write + Send>,
    incoming_rx: Receiver<Value>,
    next_id: i64,
    /// 伺服器的輸出端已經關閉（伺服器結束了）
    closed: bool,
}

impl LspClient {
    pub fn spawn(program: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut cmd = Command::new(program);
        cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：不要跳出主控台視窗
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("Failed to open child stdin");
        let stdout = child.stdout.take().expect("Failed to open child stdout");
        let mut client = Self::from_io(stdout, stdin);
        client.child = Some(child);
        Ok(client)
    }

    /// 接在一對讀寫端上：`reader` 是伺服器送來的資料，`writer` 是要送給伺服器的
    pub fn from_io(mut reader: impl Read + Send + 'static, writer: impl Write + Send + 'static) -> Self {
        let (tx, rx): (Sender<Value>, Receiver<Value>) = channel();

        // 背景讀取執行緒
        thread::spawn(move || {
            let mut frames = Reader::new();
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                frames.feed(&buf[..n]);
                for msg in frames.take_messages() {
                    let _ = tx.send(msg);
                }
            }
        });

        Self { child: None, stdin: Box::new(writer), incoming_rx: rx, next_id: 1, closed: false }
    }

    pub fn send_message(&mut self, msg: &Value) -> std::io::Result<()> {
        let framed = frame(msg);
        self.stdin.write_all(&framed)?;
        self.stdin.flush()?;
        Ok(())
    }

    pub fn next_request_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn try_recv_message(&mut self) -> Option<Value> {
        match self.incoming_rx.try_recv() {
            Ok(v) => Some(v),
            Err(TryRecvError::Disconnected) => {
                self.closed = true;
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    /// 伺服器已經結束（輸出端關閉、訊息都取完了）
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // 不再需要這個伺服器：結束它，免得留下沒人管的行程
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
