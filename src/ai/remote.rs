//! 遠端 AI 補全 (RemoteAi)
//!
//! 向 OpenAI 相容的 `/v1/chat/completions` 端點（LM Studio、Ollama、雲端服務…）要一段接在游標後面的程式碼。
//! 請求在背景執行緒送出（`ureq`，有連線與整體逾時），介面每一幀以 `RemoteAi::poll` 取回結果，所以不會卡住畫面。

use super::provider::{parse_content, AIConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde_json::{json, Value};
use std::time::Duration;

/// 送給模型的上下文：游標前、後最多幾個字元
pub const CONTEXT_BEFORE: usize = 4000;
pub const CONTEXT_AFTER: usize = 1000;
/// 連線逾時與整個請求的逾時
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// 游標所在位置的標記（放在送出的文字裡）
pub const CURSOR_MARK: &str = "<CURSOR>";

const SYSTEM_PROMPT: &str = "You are a code completion engine. The user sends a file with the marker <CURSOR> \
at the cursor position. Reply with ONLY the text that should be inserted at <CURSOR> (usually the rest of the \
current statement or a few lines). Do not repeat text that is already before the cursor, do not explain, \
and do not use Markdown code fences.";

/// 游標前、後的文字（`lines` 是文件的每一行、不含換行；`col` 是字元欄）。
/// 前面最多 `CONTEXT_BEFORE` 個字元、後面最多 `CONTEXT_AFTER` 個字元，從離游標最遠的地方截掉
pub fn context_around(lines: &[String], line: usize, col: usize) -> (String, String) {
    let Some(cur) = lines.get(line) else { return (String::new(), String::new()) };
    let split = cur.char_indices().nth(col).map_or(cur.len(), |(b, _)| b);
    // 往前收集到超過上限為止，再從開頭截掉多出來的字元
    let mut before = cur[..split].to_string();
    let mut l = line;
    while l > 0 && before.chars().count() < CONTEXT_BEFORE {
        l -= 1;
        before = format!("{}\n{}", lines[l], before);
    }
    let extra = before.chars().count().saturating_sub(CONTEXT_BEFORE);
    if extra > 0 {
        before = before.chars().skip(extra).collect();
    }
    let mut after = cur[split..].to_string();
    let mut l = line + 1;
    while l < lines.len() && after.chars().count() < CONTEXT_AFTER {
        after.push('\n');
        after.push_str(&lines[l]);
        l += 1;
    }
    if after.chars().count() > CONTEXT_AFTER {
        after = after.chars().take(CONTEXT_AFTER).collect();
    }
    (before, after)
}

/// chat/completions 的請求內容。`language` 是給模型參考的語言名稱（例如副檔名）
pub fn build_request(cfg: &AIConfig, language: &str, before: &str, after: &str) -> Value {
    let user = format!("Language: {language}\n\n{before}{CURSOR_MARK}{after}");
    json!({
        "model": cfg.model,
        "max_tokens": cfg.max_tokens,
        "temperature": cfg.temperature,
        "stream": false,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user },
        ],
    })
}

/// 把模型的回覆整理成要插入的文字：拿掉 Markdown 程式碼區塊的 ``` 行、游標標記，
/// 以及開頭重複的「游標前那一行已經打的字」（模型常把整行重寫一次）。沒有東西可插入時是 None
pub fn clean_completion(reply: &str, line_before_cursor: &str) -> Option<String> {
    let mut text = reply.replace("\r\n", "\n");
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let mut lines: Vec<&str> = trimmed.lines().skip(1).collect();
        if lines.last().is_some_and(|l| l.trim_start().starts_with("```")) {
            lines.pop();
        }
        text = lines.join("\n");
    }
    text = text.replace(CURSOR_MARK, "");
    let typed = line_before_cursor.trim_start();
    if !typed.is_empty() {
        let lead = text.len() - text.trim_start().len();
        if text[lead..].starts_with(typed) {
            text = text[lead + typed.len()..].to_string();
        }
    }
    let text = text.trim_end().to_string();
    (!text.trim().is_empty()).then_some(text)
}

/// 同步送出一個請求並取回第一個 choice 的內容（在背景執行緒呼叫）。錯誤訊息是給使用者看的
pub fn post(cfg: &AIConfig, body: &Value) -> Result<String, String> {
    let endpoint = cfg.endpoint.trim();
    if endpoint.is_empty() {
        return Err("沒有設定端點".into());
    }
    let agent = ureq::AgentBuilder::new().timeout_connect(CONNECT_TIMEOUT).timeout(REQUEST_TIMEOUT).build();
    let mut req = agent.post(endpoint).set("Content-Type", "application/json");
    let key = cfg.api_key.trim();
    if !key.is_empty() {
        req = req.set("Authorization", &format!("Bearer {key}"));
    }
    match req.send_string(&body.to_string()) {
        Ok(resp) => {
            let text = resp.into_string().map_err(|e| format!("讀取回覆失敗：{e}"))?;
            parse_content(text.as_bytes()).ok_or_else(|| "回覆不是 OpenAI 相容的格式（找不到 choices[0].message.content）".into())
        }
        Err(ureq::Error::Status(code, resp)) => {
            let detail: String = resp.into_string().unwrap_or_default().chars().take(200).collect();
            Err(format!("伺服器回應 HTTP {code} {}", detail.trim()))
        }
        Err(ureq::Error::Transport(t)) => Err(format!("連不上 {endpoint}：{t}")),
    }
}

/// 背景請求：每個請求一條執行緒，結果經由 channel 交回介面
pub struct RemoteAi {
    tx: Sender<(u64, Result<String, String>)>,
    rx: Receiver<(u64, Result<String, String>)>,
    next: u64,
}

impl Default for RemoteAi {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx, next: 1 }
    }
}

impl RemoteAi {
    /// 在背景送出請求，回傳它的編號；完成後呼叫 `wake`（讓介面重畫）
    pub fn send(&mut self, cfg: &AIConfig, body: Value, wake: impl FnOnce() + Send + 'static) -> u64 {
        let id = self.next;
        self.next += 1;
        let (cfg, tx) = (cfg.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let r = post(&cfg, &body);
            let _ = tx.send((id, r));
            wake();
        });
        id
    }

    /// 取出一個已完成的結果（沒有就是 None）
    pub fn poll(&self) -> Option<(u64, Result<String, String>)> {
        self.rx.try_recv().ok()
    }
}
