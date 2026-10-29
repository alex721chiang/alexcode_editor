//! Markdown 連結解析 (MdLink)
//!
//! 支援：
//! - Obsidian 式雙中括號筆記連結：[[Note]]、[[Note|alias]]、[[Note#heading]]
//! - 標準 Markdown 相對連結：[text](path.md)
//! - 自動忽略程式碼區塊 (``` ... ```, ~~~ ... ~~~) 與行內代碼 (`...`)
//! - 忽略 http(s)://、mailto: 與純錨點 #heading 外部連結
//! - 連結正規化索引鍵 (normalize_key)

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    pub target: String, // 連結目標（已移除 alias 與 heading）
    pub wiki: bool,     // true = wikilink; false = [text](path)
}

/// 去除圍欄程式碼區塊（``` … ```, ~~~ ... ~~~）與行內程式碼（`…`），避免誤抓中括號
pub fn strip_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let lines = text.split('\n');
    let mut in_fence = false;

    for line in lines {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push('\n');
            continue;
        }

        // 移除行內程式碼 `...`
        let mut cleaned = String::new();
        let mut in_code = false;
        for c in line.chars() {
            if c == '`' {
                in_code = !in_code;
                continue;
            }
            cleaned.push(if in_code { ' ' } else { c });
        }
        out.push_str(&cleaned);
        out.push('\n');
    }

    out
}

/// 目標標準化為比對鍵：取檔名（去路徑）、去 .md 副檔名、轉小寫、去除頭尾空白
pub fn normalize_key(target: &str) -> String {
    let mut t = target.trim();
    if let Some(pos) = t.rfind(|c| c == '/' || c == '\\') {
        t = &t[pos + 1..];
    }
    if t.to_lowercase().ends_with(".md") {
        t = &t[..t.len() - 3];
    }
    t.trim().to_lowercase()
}

/// 從 Markdown 內文擷取所有有效連結參照
pub fn extract_refs(text: &str) -> Vec<Ref> {
    let src = strip_code(text);
    let mut refs = Vec::new();

    // 1. wikilink: [[ ... ]]
    let wiki_re = Regex::new(r"\[\[([^\]\[]+)\]\]").unwrap();
    for cap in wiki_re.captures_iter(&src) {
        let mut inner = cap[1].to_string();
        if let Some(pos) = inner.find('|') {
            inner.truncate(pos);
        }
        if let Some(pos) = inner.find('#') {
            inner.truncate(pos);
        }
        let inner_trimmed = inner.trim().to_string();
        if !inner_trimmed.is_empty() {
            refs.push(Ref {
                target: inner_trimmed,
                wiki: true,
            });
        }
    }

    // 2. [text](url)
    let md_re = Regex::new(r"\[[^\]]*\]\(([^)]+)\)").unwrap();
    for cap in md_re.captures_iter(&src) {
        let mut url = cap[1].trim().to_string();
        if let Some(pos) = url.find('#') {
            url.truncate(pos);
        }
        let url_trimmed = url.trim().to_string();
        if url_trimmed.is_empty() {
            continue;
        }
        if url_trimmed.contains("://") {
            continue;
        }
        if url_trimmed.to_lowercase().starts_with("mailto:") {
            continue;
        }
        refs.push(Ref {
            target: url_trimmed,
            wiki: false,
        });
    }

    refs
}
