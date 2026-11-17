//! 遠端 AI 補全：請求內容、回覆整理，以及在本機假伺服器上實際送出 HTTP 請求

use alexcode::ai::remote::{build_request, clean_completion, context_around, post, CONTEXT_AFTER, CONTEXT_BEFORE, CURSOR_MARK};
use alexcode::ai::{AIConfig, RemoteAi};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// 在 127.0.0.1 開一個只回一種回覆的 HTTP 伺服器；回傳端點與收到的每一個完整請求（標頭 + 內容）
fn fake_server(status: &'static str, body: String) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { return };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text[..end]
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
                        .unwrap_or(0);
                    if buf.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
            let resp = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = s.write_all(resp.as_bytes());
        }
    });
    (format!("http://127.0.0.1:{port}/v1/chat/completions"), rx)
}

fn chat_reply(content: &str) -> String {
    serde_json::json!({ "choices": [ { "message": { "role": "assistant", "content": content } } ] }).to_string()
}

#[test]
fn the_config_is_off_by_default_and_old_files_still_load() {
    let cfg = AIConfig::default();
    assert!(!cfg.enabled, "remote completion must be opt-in");
    assert!(cfg.api_key.is_empty());
    // 舊版存的設定（沒有 enabled 等欄位）照樣讀得進來
    let old = AIConfig::from_json(r#"{"endpoint":"http://x/v1/chat/completions","model":"m"}"#);
    assert_eq!((old.endpoint.as_str(), old.model.as_str(), old.enabled), ("http://x/v1/chat/completions", "m", false));
    let ed: alexcode::core::config::EditorConfig = serde_json::from_str(r#"{"font_size":15.0}"#).unwrap();
    assert_eq!(ed.ai, AIConfig::default(), "settings saved before this feature get the default (disabled) AI config");
}

#[test]
fn context_is_cut_around_the_cursor() {
    let lines: Vec<String> = vec!["fn a() {".into(), "    let 值 = 1;".into(), "}".into()];
    let (before, after) = context_around(&lines, 1, 8);
    assert_eq!(before, "fn a() {\n    let ");
    assert_eq!(after, "值 = 1;\n}");
    // 很長的文件：只留離游標近的部分
    let many: Vec<String> = (0..5000).map(|i| format!("line {i}")).collect();
    let (before, after) = context_around(&many, 2500, 2);
    assert_eq!(before.chars().count(), CONTEXT_BEFORE);
    assert!(before.ends_with("line 2499\nli"));
    assert_eq!(after.chars().count(), CONTEXT_AFTER);
    assert!(after.starts_with("ne 2500\nline 2501"));
    assert_eq!(context_around(&many, 9999, 0), (String::new(), String::new()));
}

#[test]
fn the_request_has_the_model_and_the_text_around_the_cursor() {
    let cfg = AIConfig { model: "my-model".into(), max_tokens: 64, ..AIConfig::default() };
    let req = build_request(&cfg, "rs", "let x = ", ";\n");
    assert_eq!(req["model"], "my-model");
    assert_eq!(req["max_tokens"], 64);
    assert_eq!(req["stream"], false);
    let user = req["messages"][1]["content"].as_str().unwrap();
    assert!(user.contains(&format!("let x = {CURSOR_MARK};")), "{user}");
    assert!(user.contains("rs"));
    assert_eq!(req["messages"][0]["role"], "system");
}

#[test]
fn replies_are_cleaned_up() {
    assert_eq!(clean_completion("42;", "let x = ").as_deref(), Some("42;"));
    assert_eq!(clean_completion("```rust\nfoo(1);\nbar();\n```\n", "").as_deref(), Some("foo(1);\nbar();"));
    // 模型把游標前已經打的字又寫了一次
    assert_eq!(clean_completion("hello_world();", "    hel").as_deref(), Some("lo_world();"));
    assert_eq!(clean_completion("  \n", "x"), None);
    assert_eq!(clean_completion("```\n```", ""), None);
    assert_eq!(clean_completion("a<CURSOR>b\r\nc", "").as_deref(), Some("ab\nc"));
}

#[test]
fn post_sends_the_key_and_returns_the_content() {
    let (endpoint, rx) = fake_server("200 OK", chat_reply("value + 1"));
    let cfg = AIConfig { enabled: true, endpoint, api_key: "sk-test".into(), ..AIConfig::default() };
    let body = build_request(&cfg, "py", "x = ", "");
    assert_eq!(post(&cfg, &body).unwrap(), "value + 1");
    let req = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(req.starts_with("POST /v1/chat/completions "), "{req}");
    assert!(req.to_ascii_lowercase().contains("authorization: bearer sk-test"), "{req}");
    let json = &req[req.find("\r\n\r\n").unwrap() + 4..];
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(v["model"], cfg.model);

    // 沒有金鑰時不送 Authorization
    let (endpoint, rx) = fake_server("200 OK", chat_reply("1"));
    let cfg = AIConfig { enabled: true, endpoint, ..AIConfig::default() };
    post(&cfg, &body).unwrap();
    let req = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!req.to_ascii_lowercase().contains("authorization:"), "{req}");
}

#[test]
fn post_reports_errors() {
    let (endpoint, _rx) = fake_server("401 Unauthorized", r#"{"error":"bad key"}"#.into());
    let cfg = AIConfig { enabled: true, endpoint, ..AIConfig::default() };
    let err = post(&cfg, &serde_json::json!({})).unwrap_err();
    assert!(err.contains("401") && err.contains("bad key"), "{err}");

    let (endpoint, _rx) = fake_server("200 OK", "not json".into());
    let cfg = AIConfig { enabled: true, endpoint, ..AIConfig::default() };
    assert!(post(&cfg, &serde_json::json!({})).unwrap_err().contains("choices"));

    // 沒有人在聽的埠：很快就失敗，不會等到整個逾時
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let cfg = AIConfig { enabled: true, endpoint: format!("http://127.0.0.1:{port}/v1/chat/completions"), ..AIConfig::default() };
    let t = Instant::now();
    let err = post(&cfg, &serde_json::json!({})).unwrap_err();
    assert!(err.contains("連不上"), "{err}");
    assert!(t.elapsed() < Duration::from_secs(10), "took {:?}", t.elapsed());

    let cfg = AIConfig { enabled: true, endpoint: "  ".into(), ..AIConfig::default() };
    assert!(post(&cfg, &serde_json::json!({})).is_err());
}

#[test]
fn remote_ai_runs_requests_in_the_background() {
    let (endpoint, _rx) = fake_server("200 OK", chat_reply("done"));
    let cfg = AIConfig { enabled: true, endpoint, ..AIConfig::default() };
    let mut ai = RemoteAi::default();
    let (wtx, wrx) = mpsc::channel();
    let id = ai.send(&cfg, serde_json::json!({}), move || {
        let _ = wtx.send(());
    });
    wrx.recv_timeout(Duration::from_secs(10)).expect("the waker is called when the reply arrives");
    assert_eq!(ai.poll(), Some((id, Ok("done".to_string()))));
    assert_eq!(ai.poll(), None);
}
