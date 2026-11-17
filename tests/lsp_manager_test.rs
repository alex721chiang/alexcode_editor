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
    // 假伺服器回得很快時，同一次 sync 裡就可能已經收到 initialize 的回覆（CI 的 Linux 上看過），所以 Ready 也算
    assert!(
        matches!(m.status(ServerKind::RustAnalyzer), Some(LspStatus::Starting | LspStatus::Ready)),
        "the .rs file starts rust-analyzer: {:?}",
        m.status(ServerKind::RustAnalyzer)
    );
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

/// 把 uri 改成另一種合法的寫法（Windows 的磁碟機代號改小寫並把 `:` 編碼成 `%3A`，像 VS Code 那樣）
fn other_spelling(uri: &str) -> String {
    let b = uri.as_bytes();
    if uri.len() > 10 && b[9] == b':' && b[8].is_ascii_alphabetic() {
        format!("file:///{}%3A{}", (b[8] as char).to_ascii_lowercase(), &uri[10..])
    } else {
        uri.to_string()
    }
}

#[test]
fn diagnostics_are_kept_per_file_and_wake_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let l = log.clone();
    // 假伺服器：didOpen / didChange 之後送出那份文件的診斷（uri 換一種寫法），內容含 "ok" 時送空的清單
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
                    let method = m["method"].as_str().unwrap_or("");
                    if method == "initialize" {
                        let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": {} } });
                        let _ = server_write.write_all(&frame(&reply));
                    }
                    if method == "textDocument/didOpen" || method == "textDocument/didChange" {
                        let uri = m["params"]["textDocument"]["uri"].as_str().unwrap().to_string();
                        let text = m["params"]["textDocument"]["text"].as_str()
                            .or(m["params"]["contentChanges"][0]["text"].as_str()).unwrap_or("").to_string();
                        let diags = if text.contains("ok") { json!([]) } else { json!([
                            { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } },
                              "severity": 1, "message": "cannot find value `nope`", "source": "rustc" },
                            { "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 2 } },
                              "severity": 2, "message": "unused variable" }
                        ]) };
                        let note = json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                            "params": { "uri": other_spelling(&uri), "diagnostics": diags } });
                        let _ = server_write.write_all(&frame(&note));
                    }
                }
            }
        });
        Ok(LspClient::from_io(client_read, client_write))
    });
    let mut m = LspManager::new(dir.path().to_path_buf(), launcher);
    let woken = Arc::new(Mutex::new(0));
    let w = woken.clone();
    m.set_waker(Arc::new(move || *w.lock().unwrap() += 1));
    let rs = dir.path().join("main.rs").to_string_lossy().into_owned();
    let other = dir.path().join("other.rs").to_string_lossy().into_owned();
    let docs = vec![OpenDoc { path: rs.clone(), revision: 1 }];
    let texts = ["fn nope() {}\nlet x = 1;"];
    assert!(m.diagnostics(&rs).is_empty());
    assert!(pump(&mut m, &docs, &texts, 10, |m| m.diagnostics(&rs).len() == 2), "diagnostics arrive for the open file");
    let d = m.diagnostics(&rs);
    assert_eq!((d[0].start_line, d[0].start_char, d[0].end_line, d[0].end_char, d[0].severity), (0, 3, 0, 7, 1));
    assert_eq!((d[0].message.as_str(), d[0].source.as_str()), ("cannot find value `nope`", "rustc"));
    assert_eq!((d[1].severity, d[1].message.as_str()), (2, "unused variable"));
    assert!(m.diagnostics(&other).is_empty(), "other files have none");
    assert!(*woken.lock().unwrap() > 0, "the waker runs when messages arrive, so the UI repaints");
    // 修好之後伺服器送空的清單：診斷清掉
    let docs2 = vec![OpenDoc { path: rs.clone(), revision: 2 }];
    let texts2 = ["fn ok() {}"];
    assert!(pump(&mut m, &docs2, &texts2, 10, |m| m.diagnostics(&rs).is_empty()), "an empty list clears them");
    // 再改壞，然後關掉分頁：didClose 之後不再留著那份文件的診斷
    let docs3 = vec![OpenDoc { path: rs.clone(), revision: 3 }];
    assert!(pump(&mut m, &docs3, &texts, 10, |m| m.diagnostics(&rs).len() == 2));
    m.sync(&[], |_| String::new());
    assert!(m.diagnostics(&rs).is_empty(), "closing the tab drops its diagnostics");
}

#[test]
fn requests_get_their_own_replies() {
    use alexcode::lsp::RequestId;
    let dir = tempfile::tempdir().unwrap();
    // 假伺服器：一般請求回 { "echo": params }，`boom` 回錯誤；先回後收到的那一個（回覆順序與送出順序不同）
    let launcher: Launcher = Box::new(move |_kind, _root| {
        let (client_read, mut server_write) = std::io::pipe()?;
        let (mut server_read, client_write) = std::io::pipe()?;
        std::thread::spawn(move || {
            let mut frames = Reader::new();
            let mut buf = [0u8; 4096];
            let mut held: Option<Value> = None;
            while let Ok(n) = server_read.read(&mut buf) {
                if n == 0 {
                    break;
                }
                frames.feed(&buf[..n]);
                for m in frames.take_messages() {
                    let (Some(method), Some(_)) = (m["method"].as_str(), m.get("id")) else { continue };
                    let reply = match method {
                        "initialize" => json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "capabilities": {} } }),
                        "boom" => json!({ "jsonrpc": "2.0", "id": m["id"], "error": { "code": -32603, "message": "it broke" } }),
                        _ => json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "echo": m["params"] } }),
                    };
                    if method == "first" {
                        held = Some(reply);
                        continue;
                    }
                    let _ = server_write.write_all(&frame(&reply));
                    if let Some(h) = held.take() {
                        let _ = server_write.write_all(&frame(&h));
                    }
                }
            }
        });
        Ok(LspClient::from_io(client_read, client_write))
    });
    let mut m = LspManager::new(dir.path().to_path_buf(), launcher);
    let rs = dir.path().join("main.rs").to_string_lossy().into_owned();
    let docs = vec![OpenDoc { path: rs.clone(), revision: 1 }];
    // 還沒同步（伺服器還沒啟動）：不送，說明原因
    assert!(m.request(&rs, "first", json!({})).is_err());
    assert_eq!(m.request("notes.txt", "x", json!({})), Err("這種檔案沒有語言伺服器".to_string()));
    assert!(pump(&mut m, &docs, &["fn main() {}"], 10, |m| m.status(ServerKind::RustAnalyzer) == Some(&LspStatus::Ready)));
    m.sync(&docs, |_| "fn main() {}".into());
    let a: RequestId = m.request(&rs, "first", json!({ "n": 1 })).unwrap();
    let b = m.request(&rs, "second", json!({ "n": 2 })).unwrap();
    let c = m.request(&rs, "boom", json!({})).unwrap();
    assert_ne!(a, b);
    let mut got = (None, None, None);
    let end = Instant::now() + Duration::from_secs(10);
    while (got.0.is_none() || got.1.is_none() || got.2.is_none()) && Instant::now() < end {
        m.sync(&docs, |_| "fn main() {}".into());
        got.0 = got.0.or_else(|| m.take_reply(a));
        got.1 = got.1.or_else(|| m.take_reply(b));
        got.2 = got.2.or_else(|| m.take_reply(c));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(got.0, Some(Ok(json!({ "echo": { "n": 1 } }))), "each reply goes to its own request");
    assert_eq!(got.1, Some(Ok(json!({ "echo": { "n": 2 } }))));
    assert_eq!(got.2, Some(Err("it broke".to_string())));
    assert_eq!(m.take_reply(a), None, "a reply is taken once");
    // 不再需要的請求：之後收到的回覆被丟掉
    let d = m.request(&rs, "second", json!({ "n": 4 })).unwrap();
    m.forget(d);
    pump(&mut m, &docs, &["fn main() {}"], 1, |_| false);
    assert_eq!(m.take_reply(d), None);
}

#[test]
fn utf16_positions_become_char_columns() {
    assert_eq!(alexcode::lsp::char_col_to_utf16("😀x中", 2), 3);
    assert_eq!(alexcode::lsp::char_col_to_utf16("ab", 9), 2);
    use alexcode::lsp::utf16_to_char_col;
    assert_eq!(utf16_to_char_col("abc", 2), 2);
    assert_eq!(utf16_to_char_col("中文x", 2), 2, "BMP characters are one UTF-16 unit");
    assert_eq!(utf16_to_char_col("😀x", 2), 1, "an emoji is two UTF-16 units but one char");
    assert_eq!(utf16_to_char_col("😀x", 3), 2);
    assert_eq!(utf16_to_char_col("ab", 99), 2, "past the end clamps to the line length");
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

#[test]
#[ignore]
fn real_clangd_reports_diagnostics() {
    // cargo test --test lsp_manager_test real_clangd -- --ignored --nocapture：裝了 clangd 才有意義
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("broken.c");
    let text = "int main(void) {\n    int unused = 1;\n    return missing_value;\n}\n";
    std::fs::write(&c, text).unwrap();
    let path = c.to_string_lossy().into_owned();
    let mut m = LspManager::new(dir.path().to_path_buf(), Box::new(spawn_server));
    let docs = vec![OpenDoc { path: path.clone(), revision: 1 }];
    let t0 = Instant::now();
    let ok = pump(&mut m, &docs, &[text], 60, |m| !m.diagnostics(&path).is_empty());
    println!("after {:?}: status {:?}", t0.elapsed(), m.status(ServerKind::Clangd));
    for d in m.diagnostics(&path).iter() {
        println!("  {}:{}-{}:{} severity {} {:?} ({})", d.start_line, d.start_char, d.end_line, d.end_char, d.severity, d.message, d.source);
    }
    assert!(ok, "clangd sent diagnostics for the broken file");
    assert!(m.diagnostics(&path).iter().any(|d| d.severity == 1 && d.start_line == 2), "the undeclared identifier is an error on line 3");
    // 修好之後清單清空
    let fixed = "int main(void) {\n    return 0;\n}\n";
    let docs2 = vec![OpenDoc { path: path.clone(), revision: 2 }];
    assert!(pump(&mut m, &docs2, &[fixed], 30, |m| m.diagnostics(&path).is_empty()), "fixing the file clears them");
}

#[test]
#[ignore]
fn real_clangd_finds_definitions() {
    // cargo test --test lsp_manager_test real_clangd -- --ignored --nocapture：裝了 clangd 才有意義
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("calls.c");
    let text = "static int twice(int x) { return x * 2; }\n\nint main(void) {\n    return twice(21);\n}\n";
    std::fs::write(&c, text).unwrap();
    let path = c.to_string_lossy().into_owned();
    let mut m = LspManager::new(dir.path().to_path_buf(), Box::new(spawn_server));
    let docs = vec![OpenDoc { path: path.clone(), revision: 1 }];
    assert!(pump(&mut m, &docs, &[text], 60, |m| m.status(ServerKind::Clangd) == Some(&LspStatus::Ready)));
    m.sync(&docs, |_| text.to_string());
    // `twice` 的呼叫在第 4 行（index 3）第 12 個字元
    let params = alexcode::lsp::text_document_position_params(&path, 3, 12);
    let r = m.request(&path, "textDocument/definition", params).unwrap();
    let t0 = Instant::now();
    let mut reply = None;
    while reply.is_none() && t0.elapsed() < Duration::from_secs(30) {
        m.sync(&docs, |_| text.to_string());
        reply = m.take_reply(r);
        std::thread::sleep(Duration::from_millis(10));
    }
    println!("after {:?}: {reply:?}", t0.elapsed());
    let def = alexcode::lsp::extract_definition(&reply.expect("a reply").expect("not an error")).expect("a location");
    println!("definition: {def:?}");
    assert!(std::path::Path::new(&def.0) == c.as_path() || def.0.eq_ignore_ascii_case(&path), "{} vs {}", def.0, path);
    assert_eq!((def.1, def.2), (0, 11), "the function name on line 1");
}

#[test]
#[ignore]
fn real_clangd_describes_symbols_on_hover() {
    // cargo test --test lsp_manager_test real_clangd -- --ignored --nocapture：裝了 clangd 才有意義
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("hover.c");
    let text = "/// Doubles the value.\nstatic int twice(int x) { return x * 2; }\n\nint main(void) {\n    return twice(21);\n}\n";
    std::fs::write(&c, text).unwrap();
    let path = c.to_string_lossy().into_owned();
    let mut m = LspManager::new(dir.path().to_path_buf(), Box::new(spawn_server));
    let docs = vec![OpenDoc { path: path.clone(), revision: 1 }];
    assert!(pump(&mut m, &docs, &[text], 60, |m| m.status(ServerKind::Clangd) == Some(&LspStatus::Ready)));
    m.sync(&docs, |_| text.to_string());
    let params = alexcode::lsp::text_document_position_params(&path, 4, 12);
    let r = m.request(&path, "textDocument/hover", params).unwrap();
    let t0 = Instant::now();
    let mut reply = None;
    while reply.is_none() && t0.elapsed() < Duration::from_secs(30) {
        m.sync(&docs, |_| text.to_string());
        reply = m.take_reply(r);
        std::thread::sleep(Duration::from_millis(10));
    }
    let v = reply.expect("a reply").expect("not an error");
    let shown = alexcode::lsp::extract_hover_text(&v, 4000);
    println!("after {:?}:\n{shown}", t0.elapsed());
    assert!(shown.contains("twice"), "the hover names the function");
}

#[test]
#[ignore]
fn real_clangd_completes_names() {
    // cargo test --test lsp_manager_test real_clangd -- --ignored --nocapture：裝了 clangd 才有意義
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("complete.c");
    let text = "struct point { int x_coord; int y_coord; };\nstatic int twice_value(int x) { return x * 2; }\n\nint main(void) {\n    struct point p = {0};\n    tw\n    p.\n}\n";
    std::fs::write(&c, text).unwrap();
    let path = c.to_string_lossy().into_owned();
    let mut m = LspManager::new(dir.path().to_path_buf(), Box::new(spawn_server));
    let docs = vec![OpenDoc { path: path.clone(), revision: 1 }];
    assert!(pump(&mut m, &docs, &[text], 60, |m| m.status(ServerKind::Clangd) == Some(&LspStatus::Ready)));
    m.sync(&docs, |_| text.to_string());
    println!("trigger characters: {:?}", m.completion_triggers(&path));
    assert!(m.completion_triggers(&path).contains('.'));
    let mut ask = |line: i32, ch: i32, trigger: Option<char>| {
        let params = alexcode::lsp::completion_params(&path, line, ch, trigger);
        let r = m.request(&path, "textDocument/completion", params).unwrap();
        let t0 = Instant::now();
        let mut reply = None;
        while reply.is_none() && t0.elapsed() < Duration::from_secs(30) {
            m.sync(&docs, |_| text.to_string());
            reply = m.take_reply(r);
            std::thread::sleep(Duration::from_millis(10));
        }
        let v = reply.expect("a reply").expect("not an error");
        let (items, incomplete) = alexcode::lsp::extract_completion_items(&v, 200);
        println!("after {:?}: {} items, incomplete {incomplete}", t0.elapsed(), items.len());
        for it in items.iter().take(5) {
            println!("  {:?} insert {:?} filter {:?} detail {:?}", it.label, it.insert, it.filter, it.detail);
        }
        items
    };
    // `tw` 後面（第 6 行第 6 個字元）：有 twice_value，插入的是純文字（沒有 snippet 的 ${…}）
    let items = ask(5, 6, None);
    let it = items.iter().find(|i| i.filter == "twice_value").expect("twice_value is offered");
    assert!(it.insert.starts_with("twice_value") && !it.insert.contains('$'), "{:?}", it.insert);
    // `p.` 後面：結構的欄位
    let items = ask(6, 6, Some('.'));
    assert!(items.iter().any(|i| i.filter == "x_coord") && items.iter().any(|i| i.filter == "y_coord"));
}
