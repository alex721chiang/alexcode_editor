//! 語言伺服器管理：依檔案類型啟動伺服器、完成 initialize 握手、同步開著的文件。
//! 用一個在測試裡跑的假伺服器（經由 `std::io::pipe` 接上），每個平台都能跑；
//! 真的 rust-analyzer / clangd 在 `real_servers_accept_the_handshake`（預設不跑，`--ignored` 才跑）。

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alexcode::lsp::{frame, spawn_server, uri_from_path, Launcher, LspClient, LspManager, LspStatus, OpenDoc, Reader, ServerKind};
use serde_json::{json, Value};

type Log = Arc<Mutex<Vec<Value>>>;

/// 假的伺服器：記下收到的每一則訊息；收到 initialize 時回覆，並送一個 `workspace/configuration` 請求（id 900）
fn fake_launcher() -> (Launcher, Log) {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let l = log.clone();
    let launcher: Launcher = Box::new(move |_kind, _root| {
        let (client_read, mut server_write) = std::io::pipe()?;
        let (mut server_read, client_write) = std::io::pipe()?;
        let log = l.clone();
        std::thread::spawn(move || {
            let mut frames = Reader::new();
            let mut buf = [0u8; 4096];
            while let Ok(n) = server_read.read(&mut buf) {
                if n == 0 {
                    break;
                }
                frames.feed(&buf[..n]);
                for m in frames.take_messages() {
                    log.lock().unwrap().push(m.clone());
                    if m["method"] == "initialize" {
                        let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": { "textDocumentSync": 1 } } });
                        let ask = json!({ "jsonrpc": "2.0", "id": 900, "method": "workspace/configuration", "params": { "items": [{}, {}] } });
                        let _ = server_write.write_all(&frame(&reply));
                        let _ = server_write.write_all(&frame(&ask));
                    }
                }
            }
        });
        Ok(LspClient::from_io(client_read, client_write))
    });
    (launcher, log)
}

fn sent(log: &Log, method: &str) -> Vec<Value> {
    log.lock().unwrap().iter().filter(|m| m["method"] == method).cloned().collect()
}

/// 一直 `sync` 到 `done` 成立或逾時
fn pump(m: &mut LspManager, docs: &[OpenDoc], texts: &[&str], secs: u64, done: impl Fn(&LspManager) -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        m.sync(docs, |i| texts[i].to_string());
        if done(m) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn servers_are_chosen_by_file_type() {
    assert_eq!(ServerKind::for_path("src/main.rs"), Some(ServerKind::RustAnalyzer));
    assert_eq!(ServerKind::for_path("a/B.CPP"), Some(ServerKind::Clangd));
    assert_eq!(ServerKind::for_path("x.h"), Some(ServerKind::Clangd));
    assert_eq!(ServerKind::for_path("tool.py"), Some(ServerKind::Pylsp));
    assert_eq!(ServerKind::for_path("notes.md"), None);
    assert_eq!(ServerKind::for_path("Makefile"), None);
    assert_eq!((ServerKind::language_id("a.c"), ServerKind::language_id("a.hpp")), ("c", "cpp"));
    assert_eq!(ServerKind::RustAnalyzer.program(), "rust-analyzer");
}

#[test]
fn handshake_then_open_change_and_close_documents() {
    let dir = tempfile::tempdir().unwrap();
    let (launcher, log) = fake_launcher();
    let mut m = LspManager::new(dir.path().to_path_buf(), launcher);
    let rs = dir.path().join("main.rs").to_string_lossy().into_owned();
    let txt = dir.path().join("notes.txt").to_string_lossy().into_owned();
    let docs = vec![OpenDoc { path: rs.clone(), revision: 1 }, OpenDoc { path: txt, revision: 1 }];
    let texts = ["fn main() {}", "plain"];

    m.sync(&docs, |i| texts[i].to_string());
    assert_eq!(m.status(ServerKind::RustAnalyzer), Some(&LspStatus::Starting), "the .rs file starts rust-analyzer");
    assert!(m.status(ServerKind::Clangd).is_none() && m.status(ServerKind::Pylsp).is_none(), "nothing else is started");
    assert!(pump(&mut m, &docs, &texts, 10, |m| m.status(ServerKind::RustAnalyzer) == Some(&LspStatus::Ready)));
    assert!(pump(&mut m, &docs, &texts, 10, |_| !sent(&log, "textDocument/didOpen").is_empty()));

    let init = sent(&log, "initialize");
    assert_eq!(init.len(), 1, "one handshake");
    assert_eq!(init[0]["params"]["rootUri"], uri_from_path(&dir.path().to_string_lossy()));
    assert_eq!(init[0]["params"]["processId"], std::process::id());
    assert_eq!(sent(&log, "initialized").len(), 1);
    let open = sent(&log, "textDocument/didOpen");
    assert_eq!(open.len(), 1, "only the .rs file goes to rust-analyzer");
    let doc = &open[0]["params"]["textDocument"];
    assert_eq!((doc["uri"].clone(), doc["languageId"].clone(), doc["version"].clone(), doc["text"].clone()),
        (json!(uri_from_path(&rs)), json!("rust"), json!(1), json!("fn main() {}")));
    // 伺服器的請求有被回覆（兩個設定項目都回 null）
    assert!(pump(&mut m, &docs, &texts, 10, |_| log.lock().unwrap().iter().any(|v| v["id"] == 900)));
    let reply = log.lock().unwrap().iter().find(|v| v["id"] == 900).cloned().unwrap();
    assert_eq!(reply["result"], json!([null, null]));

    // 內容變了：didChange 送整份新內容，版本號加一；沒變就不送
    let docs2 = vec![OpenDoc { path: rs.clone(), revision: 2 }];
    let texts2 = ["fn main() { println!(); }"];
    m.sync(&docs2, |i| texts2[i].to_string());
    m.sync(&docs2, |i| texts2[i].to_string());
    assert!(pump(&mut m, &docs2, &texts2, 10, |_| !sent(&log, "textDocument/didChange").is_empty()));
    std::thread::sleep(Duration::from_millis(100));
    let change = sent(&log, "textDocument/didChange");
    assert_eq!(change.len(), 1, "one change, sent once");
    assert_eq!(change[0]["params"]["textDocument"]["version"], 2);
    assert_eq!(change[0]["params"]["contentChanges"], json!([{ "text": "fn main() { println!(); }" }]));

    // 分頁關掉：didClose
    m.sync(&[], |_| String::new());
    assert!(pump(&mut m, &[], &[], 10, |_| !sent(&log, "textDocument/didClose").is_empty()));
    assert_eq!(sent(&log, "textDocument/didClose")[0]["params"]["textDocument"]["uri"], json!(uri_from_path(&rs)));
    assert_eq!(sent(&log, "initialize").len(), 1, "the server is not restarted");
}

#[test]
fn missing_and_exited_servers_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let docs = vec![OpenDoc { path: dir.path().join("a.py").to_string_lossy().into_owned(), revision: 1 }];
    // 找不到程式：未安裝
    let launcher: Launcher = Box::new(|_, _| LspClient::spawn("alexcode-no-such-language-server", &[]));
    let mut m = LspManager::new(dir.path().to_path_buf(), launcher);
    m.sync(&docs, |_| String::new());
    assert_eq!(m.status(ServerKind::Pylsp), Some(&LspStatus::NotInstalled));
    assert_eq!(LspStatus::NotInstalled.label(), "未安裝");
    // 啟動之後馬上結束（例如 rustup 的 rust-analyzer 沒有裝那個元件）：已結束，不會一直重新啟動
    let starts = Arc::new(Mutex::new(0));
    let s = starts.clone();
    let launcher: Launcher = Box::new(move |_, _| {
        *s.lock().unwrap() += 1;
        let (client_read, server_write) = std::io::pipe()?;
        let (mut server_read, client_write) = std::io::pipe()?;
        // 伺服器收得到訊息，但馬上關掉輸出端（像是印了錯誤就結束）
        drop(server_write);
        std::thread::spawn(move || std::io::copy(&mut server_read, &mut std::io::sink()));
        Ok(LspClient::from_io(client_read, client_write))
    });
    let mut m = LspManager::new(dir.path().to_path_buf(), launcher);
    assert!(pump(&mut m, &docs, &[""], 10, |m| m.status(ServerKind::Pylsp) == Some(&LspStatus::Exited)));
    for _ in 0..20 {
        m.sync(&docs, |_| String::new());
    }
    assert_eq!(*starts.lock().unwrap(), 1);
}

#[test]
#[ignore]
fn real_servers_accept_the_handshake() {
    // cargo test --test lsp_manager_test -- --ignored --nocapture：裝了 rust-analyzer / clangd 才有意義
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let rs = dir.path().join("src").join("main.rs");
    std::fs::write(&rs, "fn main() {}\n").unwrap();
    let c = dir.path().join("hello.c");
    std::fs::write(&c, "int main(void) { return 0; }\n").unwrap();
    let mut m = LspManager::new(dir.path().to_path_buf(), Box::new(spawn_server));
    let docs = vec![
        OpenDoc { path: rs.to_string_lossy().into_owned(), revision: 1 },
        OpenDoc { path: c.to_string_lossy().into_owned(), revision: 1 },
    ];
    let texts = ["fn main() {}\n", "int main(void) { return 0; }\n"];
    let t0 = Instant::now();
    let ok = pump(&mut m, &docs, &texts, 60, |m| !m.is_starting());
    println!("after {:?}: rust-analyzer {:?}, clangd {:?}", t0.elapsed(), m.status(ServerKind::RustAnalyzer), m.status(ServerKind::Clangd));
    assert!(ok, "the servers answered initialize");
    // 送一次修改後，伺服器仍在（沒有因為收到的訊息而結束）
    let docs2 = vec![OpenDoc { path: rs.to_string_lossy().into_owned(), revision: 2 }, OpenDoc { path: c.to_string_lossy().into_owned(), revision: 2 }];
    let texts2 = ["fn main() { let x = 1; }\n", "int main(void) { return 1; }\n"];
    pump(&mut m, &docs2, &texts2, 3, |_| false);
    println!("after a change: rust-analyzer {:?}, clangd {:?}", m.status(ServerKind::RustAnalyzer), m.status(ServerKind::Clangd));
    // 能執行（`--version` 成功）的伺服器要就緒；不能執行的（沒裝，或 rustup 有代理程式但沒裝元件）是未安裝 / 已結束
    for k in [ServerKind::RustAnalyzer, ServerKind::Clangd] {
        let works = std::process::Command::new(k.program()).arg("--version").output().is_ok_and(|o| o.status.success());
        println!("{k:?}: --version works: {works}, status {:?}", m.status(k));
        if works {
            assert_eq!(m.status(k), Some(&LspStatus::Ready), "{k:?}");
        } else {
            assert!(matches!(m.status(k), Some(LspStatus::NotInstalled) | Some(LspStatus::Exited)), "{k:?}: {:?}", m.status(k));
        }
    }
}
