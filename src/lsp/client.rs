//! LSP 行程客戶端 (LspClient)
//!
//! 透過子行程 stdio 與語言伺服器 (如 clangd, rust-analyzer, pylsp) 進行非同步通訊。

use super::protocol::{frame, Reader};
use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct LspClient {
    _child: Child,
    stdin: ChildStdin,
    incoming_rx: Receiver<Value>,
    next_id: i64,
}

impl LspClient {
    pub fn spawn(program: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open child stdin");
        let mut stdout = child.stdout.take().expect("Failed to open child stdout");

        let (tx, rx): (Sender<Value>, Receiver<Value>) = channel();

        // 背景讀取執行緒
        thread::spawn(move || {
            let mut reader = Reader::new();
            let mut buf = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                reader.feed(&buf[..n]);
                for msg in reader.take_messages() {
                    let _ = tx.send(msg);
                }
            }
        });

        Ok(Self {
            _child: child,
            stdin,
            incoming_rx: rx,
            next_id: 1,
        })
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

    pub fn try_recv_message(&self) -> Option<Value> {
        self.incoming_rx.try_recv().ok()
    }
}
