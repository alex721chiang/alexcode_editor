//! 單一文件內的尋找與取代（區分大小寫 / 全字拼寫 / 正規表示式）

use regex::{NoExpand, Regex, RegexBuilder};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FindOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

fn build(query: &str, opts: &FindOptions) -> Result<Regex, String> {
    let core = if opts.regex { query.to_string() } else { regex::escape(query) };
    let pattern = if opts.whole_word { format!(r"\b(?:{core})\b") } else { core };
    RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|e| format!("正規表示式錯誤：{}", e.to_string().lines().last().unwrap_or_default().trim()))
}

/// 所有命中的 byte 區間（空字串命中會略過）
pub fn find_all(text: &str, query: &str, opts: &FindOptions) -> Result<Vec<(usize, usize)>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let re = build(query, opts)?;
    Ok(re.find_iter(text).filter(|m| !m.is_empty()).map(|m| (m.start(), m.end())).collect())
}

/// 取代全部命中；回傳 (新文字, 取代次數)。
/// 正規表示式模式下 `replacement` 可用 `$1`、`${name}` 引用群組；一般模式一律視為純文字。
pub fn replace_all(text: &str, query: &str, replacement: &str, opts: &FindOptions) -> Result<(String, usize), String> {
    if query.is_empty() {
        return Ok((text.to_string(), 0));
    }
    let re = build(query, opts)?;
    let count = re.find_iter(text).filter(|m| !m.is_empty()).count();
    let out = if opts.regex { re.replace_all(text, replacement) } else { re.replace_all(text, NoExpand(replacement)) };
    Ok((out.into_owned(), count))
}

/// 只取代 `range` 這一個命中（須為 `find_all` 回傳的區間）；回傳 (新文字, 取代後文字的結尾位置)
pub fn replace_range(
    text: &str,
    range: (usize, usize),
    query: &str,
    replacement: &str,
    opts: &FindOptions,
) -> Result<(String, usize), String> {
    let (a, b) = range;
    let piece = if opts.regex {
        let re = build(query, opts)?;
        // 以整份文字比對，前後文相關的樣式（\b、^、$）才會正確
        match re.captures_at(text, a).filter(|c| c.get(0).map_or(false, |m| m.start() == a && m.end() == b)) {
            Some(caps) => {
                let mut s = String::new();
                caps.expand(replacement, &mut s);
                s
            }
            None => return Err("游標所在的命中已不存在，請重新尋找".to_string()),
        }
    } else {
        replacement.to_string()
    };
    let mut out = String::with_capacity(text.len() + piece.len());
    out.push_str(&text[..a]);
    out.push_str(&piece);
    let end = out.len();
    out.push_str(&text[b..]);
    Ok((out, end))
}
