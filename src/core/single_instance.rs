//! 單一實例：程式已在執行時，再次啟動（例如在檔案總管雙擊檔案）只把檔案路徑交給
//! 已開啟的視窗，然後結束，不另開新視窗。
//!
//! 第一個實例在 127.0.0.1 的隨機埠監聽，並把「埠號 + 隨機金鑰」寫到設定資料夾的
//! `instance.json`。後來的實例讀到後連線、送出金鑰與檔案路徑；金鑰不符的連線會被忽略，
//! 因此本機其他程式無法任意叫編輯器開檔。連不上（上次異常結束留下的舊檔）就自己成為第一個實例。

use crossbeam_channel::{unbounded, Receiver};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Endpoint {
    port: u16,
    key: String,
}

pub enum Instance {
    /// 這是第一個實例；之後其他實例交來的檔案會從 `Receiver` 送達
    Primary(Receiver<Vec<PathBuf>>, InstanceGuard),
    /// 已交給現有視窗處理，呼叫端應直接結束
    Forwarded,
}

/// 結束時刪除 `instance.json`
pub struct InstanceGuard(PathBuf);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn random_key() -> String {
    use std::hash::{BuildHasher, Hasher};
    // RandomState 每次建立都帶作業系統提供的隨機種子
    let a = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let b = std::collections::hash_map::RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

fn try_forward(endpoint_file: &Path, files: &[PathBuf]) -> bool {
    let Some(ep) = std::fs::read(endpoint_file).ok().and_then(|b| serde_json::from_slice::<Endpoint>(&b).ok()) else {
        return false;
    };
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, ep.port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let mut msg = format!("{}\n", ep.key);
    for f in files {
        msg.push_str(&f.to_string_lossy());
        msg.push('\n');
    }
    msg.push('\n');
    if stream.write_all(msg.as_bytes()).is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).is_ok() && reply.trim() == "OK"
}

/// 嘗試把 `files` 交給已在執行的實例；沒有的話成為第一個實例並開始監聽。
/// `wake` 在收到其他實例交來的檔案時呼叫（用來喚醒 UI）。
pub fn acquire(dir: &Path, files: &[PathBuf], wake: impl Fn() + Send + 'static) -> std::io::Result<Instance> {
    let endpoint_file = dir.join("instance.json");
    if try_forward(&endpoint_file, files) {
        return Ok(Instance::Forwarded);
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let ep = Endpoint { port: listener.local_addr()?.port(), key: random_key() };
    std::fs::create_dir_all(dir)?;
    std::fs::write(&endpoint_file, serde_json::to_vec(&ep).map_err(std::io::Error::other)?)?;
    let (tx, rx) = unbounded();
    let key = ep.key;
    std::thread::Builder::new().name("single-instance".into()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() || line.trim_end_matches(['\r', '\n']) != key {
                continue; // 金鑰不符：忽略
            }
            let mut files = Vec::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(n) if n > 0 && !line.trim_end_matches(['\r', '\n']).is_empty() => {
                        files.push(PathBuf::from(line.trim_end_matches(['\r', '\n'])));
                    }
                    _ => break,
                }
            }
            let _ = (&stream).write_all(b"OK\n");
            if tx.send(files).is_err() {
                return;
            }
            wake();
        }
    })?;
    Ok(Instance::Primary(rx, InstanceGuard(endpoint_file)))
}
