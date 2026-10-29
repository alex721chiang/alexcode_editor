//! LSP (Language Server Protocol) JSON-RPC 2.0 純邏輯協定層
//!
//! 包含：
//! - 框架編碼與串流解碼 (Content-Length header + JSON body)
//! - 請求 / 通知 / 回應建構
//! - URI 與本機檔案路徑轉換
//! - 診斷 (Diagnostics) 解析
//! - 程式碼補全 (Completion) 與 Snippet 佔位符過濾
//! - 跳至定義 (Definition) 與引用 (References) 解析
//! - Hover 文件提示解析
//! - 格式化與重新命名 (WorkspaceEdit / TextEdit) 及全文套用
//! - 增量同步 (Incremental Sync) diff 計算
//! - 伺服器能力 (ServerCapabilities) 解析

use serde_json::{json, Value};
use std::collections::HashMap;
use url::Url;

/// ---- 框架編碼 ----
pub fn frame(msg: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(msg).unwrap_or_default();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = header.into_bytes();
    out.extend_from_slice(&body);
    out
}

/// ---- 串流讀取器 ----
#[derive(Debug, Default)]
pub struct Reader {
    buf: Vec<u8>,
    messages: Vec<Value>,
}

impl Reader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        self.parse_buffer();
    }

    pub fn take_messages(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.messages)
    }

    fn parse_buffer(&mut self) {
        loop {
            // 尋找 header 結尾 "\r\n\r\n"
            let header_sep = b"\r\n\r\n";
            let header_end = match self.buf.windows(4).position(|w| w == header_sep) {
                Some(pos) => pos,
                None => return,
            };

            // 解析 Headers
            let header_str = String::from_utf8_lossy(&self.buf[..header_end]);
            let mut content_length: Option<usize> = None;

            for line in header_str.split('\n') {
                let line = line.trim();
                if let Some(colon) = line.find(':') {
                    let key = line[..colon].trim().to_lowercase();
                    if key == "content-length" {
                        let val_str = line[colon + 1..].trim();
                        if let Ok(len) = val_str.parse::<usize>() {
                            content_length = Some(len);
                        }
                    }
                }
            }

            let content_len = match content_length {
                Some(len) => len,
                None => {
                    // 無效框架：丟棄此 header 避免卡死
                    self.buf.drain(..header_end + 4);
                    continue;
                }
            };

            let body_start = header_end + 4;
            if self.buf.len() < body_start + content_len {
                return; // 封包尚未完整接收
            }

            let body_bytes = &self.buf[body_start..body_start + content_len];
            if let Ok(val) = serde_json::from_slice::<Value>(body_bytes) {
                self.messages.push(val);
            }

            self.buf.drain(..body_start + content_len);
        }
    }
}

/// ---- 訊息建構 ----
pub fn make_request(id: i64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

pub fn make_notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    })
}

pub fn make_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(h1), Some(h2)) = (h1, h2) {
                let hex_str = [h1, h2];
                if let Ok(s) = std::str::from_utf8(&hex_str) {
                    if let Ok(val) = u8::from_str_radix(s, 16) {
                        bytes.push(val);
                        continue;
                    }
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn percent_encode_path(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        if c == ' ' {
            out.push_str("%20");
        } else {
            out.push(c);
        }
    }
    out
}

/// ---- URI / 位置 ----
pub fn uri_from_path(path: &str) -> String {
    if let Ok(url) = Url::from_file_path(path) {
        url.to_string()
    } else {
        let normalized = path.replace('\\', "/");
        let encoded = percent_encode_path(&normalized);
        if encoded.starts_with('/') {
            format!("file://{}", encoded)
        } else {
            format!("file:///{}", encoded)
        }
    }
}

pub fn path_from_uri(uri: &str) -> Option<String> {
    if let Ok(url) = Url::parse(uri) {
        if url.scheme() == "file" {
            if let Ok(p) = url.to_file_path() {
                if let Some(s) = p.to_str() {
                    return Some(s.to_string());
                }
            }
            // 跨平台相容：處理 Windows 下沒有磁碟機代號的 Unix 測試路徑 (如 file:///tmp/a.h)
            let path_str = url.path();
            let decoded = percent_decode(path_str);
            if decoded.len() >= 3
                && decoded.starts_with('/')
                && decoded.as_bytes()[1].is_ascii_alphabetic()
                && decoded.as_bytes()[2] == b':'
            {
                return Some(decoded[1..].to_string());
            }
            return Some(decoded);
        }
    }
    None
}

pub fn position_json(line: i32, character: i32) -> Value {
    json!({ "line": line, "character": character })
}

pub fn text_document_position_params(path: &str, line: i32, character: i32) -> Value {
    json!({
        "textDocument": { "uri": uri_from_path(path) },
        "position": position_json(line, character)
    })
}

/// ---- 診斷 ----
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub start_line: i32,
    pub start_char: i32,
    pub end_line: i32,
    pub end_char: i32,
    pub severity: i32, // 1=Error, 2=Warning, 3=Info, 4=Hint
    pub message: String,
    pub source: String,
}

pub fn parse_diagnostics(arr: &[Value]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for v in arr {
        let range = &v["range"];
        let start = &range["start"];
        let end = &range["end"];

        out.push(Diagnostic {
            start_line: start["line"].as_i64().unwrap_or(0) as i32,
            start_char: start["character"].as_i64().unwrap_or(0) as i32,
            end_line: end["line"].as_i64().unwrap_or(0) as i32,
            end_char: end["character"].as_i64().unwrap_or(0) as i32,
            severity: v["severity"].as_i64().unwrap_or(1) as i32,
            message: v["message"].as_str().unwrap_or("").to_string(),
            source: v["source"].as_str().unwrap_or("").to_string(),
        });
    }
    out
}

/// Snippet 佔位符剝除："foo(${1:bar}, $0)" → "foo(bar, )"
pub fn strip_snippet_placeholders(s: &str) -> String {
    let re1 = regex::Regex::new(r"\$\{\d+:([^}]*)\}").unwrap();
    let re2 = regex::Regex::new(r"\$\{\d+\}").unwrap();
    let re3 = regex::Regex::new(r"\$\d+").unwrap();

    let s1 = re1.replace_all(s, "$1");
    let s2 = re2.replace_all(&s1, "");
    let s3 = re3.replace_all(&s2, "");
    s3.to_string()
}

/// ---- 程式碼補全解析 ----
pub fn extract_completions(result: &Value, max_items: usize) -> Vec<String> {
    let items = if let Some(arr) = result.as_array() {
        arr
    } else if let Some(arr) = result["items"].as_array() {
        arr
    } else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for v in items {
        if out.len() >= max_items {
            break;
        }

        let mut text = v["insertText"].as_str().unwrap_or("").to_string();
        if text.is_empty() {
            text = v["label"].as_str().unwrap_or("").trim().to_string();
        }

        if v["insertTextFormat"].as_i64().unwrap_or(1) == 2 {
            text = strip_snippet_placeholders(&text);
        }

        if !text.is_empty() && !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

/// ---- 定義解析 ----
pub fn extract_definition(result: &Value) -> Option<(String, i32, i32)> {
    let loc = if result.is_object() {
        result
    } else if let Some(arr) = result.as_array() {
        arr.first()?
    } else {
        return None;
    };

    let mut uri = loc["uri"].as_str().unwrap_or("");
    let mut range = &loc["range"];

    if uri.is_empty() {
        uri = loc["targetUri"].as_str().unwrap_or("");
        range = if !loc["targetSelectionRange"].is_null() {
            &loc["targetSelectionRange"]
        } else {
            &loc["targetRange"]
        };
    }

    if uri.is_empty() {
        return None;
    }

    let path = path_from_uri(uri)?;
    let start = &range["start"];
    let line = start["line"].as_i64().unwrap_or(0) as i32;
    let ch = start["character"].as_i64().unwrap_or(0) as i32;

    Some((path, line, ch))
}

/// ---- Hover 解析 ----
fn marked_string_to_text(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(val) = v["value"].as_str() {
        return val.to_string();
    }
    String::new()
}

pub fn extract_hover_text(result: &Value, max_len: usize) -> String {
    if !result.is_object() {
        return String::new();
    }
    let contents = &result["contents"];
    let text = if let Some(arr) = contents.as_array() {
        let parts: Vec<String> = arr
            .iter()
            .map(marked_string_to_text)
            .filter(|s| !s.is_empty())
            .collect();
        parts.join("\n\n")
    } else {
        marked_string_to_text(contents)
    };

    let text_trimmed = text.trim();
    if text_trimmed.chars().count() > max_len {
        let mut truncated: String = text_trimmed.chars().take(max_len).collect();
        truncated.push('…');
        truncated
    } else {
        text_trimmed.to_string()
    }
}

/// ---- References 解析 ----
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub line: i32,
    pub character: i32,
}

fn location_from_json(loc: &Value) -> Option<Location> {
    let mut uri = loc["uri"].as_str().unwrap_or("");
    let mut range = &loc["range"];

    if uri.is_empty() {
        uri = loc["targetUri"].as_str().unwrap_or("");
        range = if !loc["targetSelectionRange"].is_null() {
            &loc["targetSelectionRange"]
        } else {
            &loc["targetRange"]
        };
    }

    let path = path_from_uri(uri)?;
    let start = &range["start"];
    let line = start["line"].as_i64().unwrap_or(0) as i32;
    let character = start["character"].as_i64().unwrap_or(0) as i32;

    Some(Location {
        path,
        line,
        character,
    })
}

pub fn extract_locations(result: &Value) -> Vec<Location> {
    let mut out = Vec::new();
    if let Some(arr) = result.as_array() {
        for v in arr {
            if let Some(loc) = location_from_json(v) {
                out.push(loc);
            }
        }
    } else if result.is_object() {
        if let Some(loc) = location_from_json(result) {
            out.push(loc);
        }
    }
    out
}

/// ---- TextEdit / WorkspaceEdit 解析 ----
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start_line: i32,
    pub start_char: i32,
    pub end_line: i32,
    pub end_char: i32,
    pub new_text: String,
}

fn text_edit_from_json(o: &Value) -> TextEdit {
    let range = &o["range"];
    let s = &range["start"];
    let e = &range["end"];

    TextEdit {
        start_line: s["line"].as_i64().unwrap_or(0) as i32,
        start_char: s["character"].as_i64().unwrap_or(0) as i32,
        end_line: e["line"].as_i64().unwrap_or(0) as i32,
        end_char: e["character"].as_i64().unwrap_or(0) as i32,
        new_text: o["newText"].as_str().unwrap_or("").to_string(),
    }
}

pub fn extract_text_edits(result: &Value) -> Vec<TextEdit> {
    if let Some(arr) = result.as_array() {
        arr.iter().map(text_edit_from_json).collect()
    } else {
        Vec::new()
    }
}

pub fn extract_workspace_edit(result: &Value) -> HashMap<String, Vec<TextEdit>> {
    let mut out: HashMap<String, Vec<TextEdit>> = HashMap::new();
    if !result.is_object() {
        return out;
    }

    if let Some(changes) = result["changes"].as_object() {
        for (uri, edits_val) in changes {
            if let Some(path) = path_from_uri(uri) {
                out.entry(path).or_default().extend(extract_text_edits(edits_val));
            }
        }
    }

    if let Some(doc_changes) = result["documentChanges"].as_array() {
        for v in doc_changes {
            if v["kind"].is_string() {
                // CreateFile / RenameFile / DeleteFile 忽略
                continue;
            }
            if let Some(uri) = v["textDocument"]["uri"].as_str() {
                if let Some(path) = path_from_uri(uri) {
                    out.entry(path).or_default().extend(extract_text_edits(&v["edits"]));
                }
            }
        }
    }

    out
}

/// line/char (0-based UTF-16 code unit offset) -> byte offset
fn offset_of_position(text: &str, line: i32, character: i32) -> usize {
    let mut current_line = 0;
    let mut line_start_offset = 0;

    for (byte_idx, ch) in text.char_indices() {
        if current_line == line {
            break;
        }
        if ch == '\n' {
            current_line += 1;
            line_start_offset = byte_idx + 1;
        }
    }

    if current_line < line {
        return text.len();
    }

    let line_content = &text[line_start_offset..];
    let next_nl = line_content.find('\n').unwrap_or(line_content.len());
    let max_line_bytes = next_nl;

    // 計算 UTF-16 字元偏移對應的 byte 偏移
    let mut utf16_count = 0;
    let mut char_byte_offset = 0;
    for ch in line_content[..max_line_bytes].chars() {
        if utf16_count >= character {
            break;
        }
        utf16_count += ch.len_utf16() as i32;
        char_byte_offset += ch.len_utf8();
    }

    line_start_offset + char_byte_offset.min(max_line_bytes)
}

/// 套用文字編輯 (由後往前套用避免位置偏移)
pub fn apply_text_edits(text: &str, edits: &[TextEdit]) -> String {
    #[derive(Debug)]
    struct Span {
        start: usize,
        end: usize,
        new_text: String,
    }

    let mut spans: Vec<Span> = edits
        .iter()
        .map(|e| Span {
            start: offset_of_position(text, e.start_line, e.start_char),
            end: offset_of_position(text, e.end_line, e.end_char),
            new_text: e.new_text.clone(),
        })
        .collect();

    // 依起始位置降冪排序
    spans.sort_by(|a, b| b.start.cmp(&a.start));

    let mut out = text.to_string();
    for s in spans {
        let start = s.start.min(out.len());
        let end = s.end.min(out.len()).max(start);
        out.replace_range(start..end, &s.new_text);
    }
    out
}

/// 計算字元偏移對應的 (line, character in UTF-16)
fn position_of_offset(text: &str, offset: usize) -> (i32, i32) {
    let mut line = 0;
    let mut char_count = 0;

    for (byte_idx, ch) in text.char_indices() {
        if byte_idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            char_count = 0;
        } else {
            char_count += ch.len_utf16() as i32;
        }
    }

    (line, char_count)
}

/// 計算增量同步 diff (共同前綴 / 後綴修剪，UTF-16 代理對安全)
pub fn compute_incremental_edit(old_text: &str, new_text: &str) -> Option<TextEdit> {
    if old_text == new_text {
        return None;
    }

    let old_chars: Vec<char> = old_text.chars().collect();
    let new_chars: Vec<char> = new_text.chars().collect();

    let mut prefix = 0;
    let max_prefix = old_chars.len().min(new_chars.len());
    while prefix < max_prefix && old_chars[prefix] == new_chars[prefix] {
        prefix += 1;
    }

    let mut suffix = 0;
    let max_suffix = old_chars.len().min(new_chars.len()) - prefix;
    while suffix < max_suffix && old_chars[old_chars.len() - 1 - suffix] == new_chars[new_chars.len() - 1 - suffix] {
        suffix += 1;
    }

    let old_prefix_bytes: usize = old_chars[..prefix].iter().map(|c| c.len_utf8()).sum();
    let old_del_len_bytes: usize = old_chars[prefix..old_chars.len() - suffix].iter().map(|c| c.len_utf8()).sum();
    let new_inserted_str: String = new_chars[prefix..new_chars.len() - suffix].iter().collect();

    let (start_line, start_char) = position_of_offset(old_text, old_prefix_bytes);
    let (end_line, end_char) = position_of_offset(old_text, old_prefix_bytes + old_del_len_bytes);

    Some(TextEdit {
        start_line,
        start_char,
        end_line,
        end_char,
        new_text: new_inserted_str,
    })
}

/// ---- Server Capabilities 解析 ----
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerCaps {
    pub sync_kind: i32, // 0=None, 1=Full, 2=Incremental
    pub completion_triggers: String,
    pub references: bool,
    pub rename: bool,
    pub formatting: bool,
}

impl Default for ServerCaps {
    fn default() -> Self {
        Self {
            sync_kind: 1,
            completion_triggers: String::new(),
            references: false,
            rename: false,
            formatting: false,
        }
    }
}

pub fn parse_server_capabilities(init_result: &Value) -> ServerCaps {
    let mut caps = ServerCaps::default();
    let c = &init_result["capabilities"];
    if !c.is_object() {
        return caps;
    }

    let sync = &c["textDocumentSync"];
    if let Some(k) = sync.as_i64() {
        caps.sync_kind = k as i32;
    } else if let Some(k) = sync["change"].as_i64() {
        caps.sync_kind = k as i32;
    }

    if let Some(triggers) = c["completionProvider"]["triggerCharacters"].as_array() {
        for t in triggers {
            if let Some(s) = t.as_str() {
                caps.completion_triggers.push_str(s);
            }
        }
    }

    let provider_enabled = |v: &Value| v.is_object() || v.as_bool().unwrap_or(false);

    caps.references = provider_enabled(&c["referencesProvider"]);
    caps.rename = provider_enabled(&c["renameProvider"]);
    caps.formatting = provider_enabled(&c["documentFormattingProvider"]);

    caps
}
