//! 輕量語法高亮（使用主題調色盤的 syn_* 色票，對應原版 Tree-sitter 分類配色）

use super::style::hex;
use crate::theme::Palette;
use eframe::egui::{text::LayoutJob, Color32, FontId, TextFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Rust,
    CLike,
    Python,
    JavaScript,
    Shell,
    Markdown,
    Json,
    Plain,
}

impl Lang {
    pub fn from_path(path: &str) -> Self {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "rs" => Lang::Rust,
            "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "java" | "cs" | "go" | "kt" | "swift" => Lang::CLike,
            "py" | "pyw" => Lang::Python,
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => Lang::JavaScript,
            "sh" | "bash" | "zsh" | "ps1" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" => Lang::Shell,
            "md" | "markdown" => Lang::Markdown,
            "json" => Lang::Json,
            _ => Lang::Plain,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::Rust => "Rust",
            Lang::CLike => "C / C++",
            Lang::Python => "Python",
            Lang::JavaScript => "JavaScript",
            Lang::Shell => "Shell / Config",
            Lang::Markdown => "Markdown",
            Lang::Json => "JSON",
            Lang::Plain => "Plain Text",
        }
    }

    pub fn comment_prefix(&self) -> &'static str {
        match self {
            Lang::Python | Lang::Shell => "#",
            _ => "//",
        }
    }

    fn keywords(&self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
                "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
                "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
                "use", "where", "while",
            ],
            Lang::CLike => &[
                "auto", "break", "case", "catch", "class", "const", "constexpr", "continue", "default", "delete",
                "do", "else", "enum", "explicit", "extern", "false", "for", "friend", "goto", "if", "inline",
                "namespace", "new", "nullptr", "operator", "override", "private", "protected", "public", "return",
                "sizeof", "static", "struct", "switch", "template", "this", "throw", "true", "try", "typedef",
                "typename", "union", "using", "virtual", "volatile", "while", "import", "package", "func", "var",
                "void", "int", "char", "bool", "float", "double", "long", "short", "unsigned", "signed",
            ],
            Lang::Python => &[
                "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
                "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in",
                "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
                "yield", "self",
            ],
            Lang::JavaScript => &[
                "async", "await", "break", "case", "catch", "class", "const", "continue", "default", "delete",
                "do", "else", "export", "extends", "false", "finally", "for", "from", "function", "if", "import",
                "in", "instanceof", "interface", "let", "new", "null", "of", "return", "static", "super", "switch",
                "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
            ],
            Lang::Shell => &[
                "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "function",
                "in", "return", "export", "local", "true", "false", "echo",
            ],
            Lang::Json => &["true", "false", "null"],
            _ => &[],
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tok {
    Normal,
    Keyword,
    Type,
    Comment,
    Str,
    Func,
    Pre,
    Num,
    Punct,
}

struct SynColors {
    fg: Color32,
    kw: Color32,
    ty: Color32,
    comment: Color32,
    string: Color32,
    func: Color32,
    pre: Color32,
    num: Color32,
    punct: Color32,
    search_bg: Color32,
    search_fg: Color32,
}

impl SynColors {
    fn of(&self, t: Tok) -> Color32 {
        match t {
            Tok::Normal => self.fg,
            Tok::Keyword => self.kw,
            Tok::Type => self.ty,
            Tok::Comment => self.comment,
            Tok::Str => self.string,
            Tok::Func => self.func,
            Tok::Pre => self.pre,
            Tok::Num => self.num,
            Tok::Punct => self.punct,
        }
    }
}

/// 將文字切成 (起始 byte, 結束 byte, 類別) 片段
fn tokenize(text: &str, lang: Lang) -> Vec<(usize, usize, Tok)> {
    let mut out: Vec<(usize, usize, Tok)> = Vec::new();
    let bytes = text.as_bytes();
    let n = bytes.len();
    let kws = lang.keywords();
    let mut i = 0;
    let mut line_start = true;

    let push = |out: &mut Vec<(usize, usize, Tok)>, a: usize, b: usize, t: Tok| {
        if a < b {
            out.push((a, b, t));
        }
    };

    if lang == Lang::Plain {
        push(&mut out, 0, n, Tok::Normal);
        return out;
    }

    while i < n {
        let c = bytes[i];

        // Markdown：以行為單位
        if lang == Lang::Markdown {
            let end = text[i..].find('\n').map(|p| i + p).unwrap_or(n);
            let line = &text[i..end];
            let t = line.trim_start();
            let tok = if t.starts_with('#') {
                Tok::Keyword
            } else if t.starts_with("```") || t.starts_with("    ") {
                Tok::Str
            } else if t.starts_with('>') {
                Tok::Comment
            } else if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
                Tok::Func
            } else {
                Tok::Normal
            };
            if tok == Tok::Normal {
                // 行內 [[wiki link]] / `code`
                let mut j = i;
                let mut seg = i;
                while j < end {
                    if text[j..end].starts_with("[[") {
                        if let Some(p) = text[j..end].find("]]") {
                            push(&mut out, seg, j, Tok::Normal);
                            push(&mut out, j, j + p + 2, Tok::Type);
                            j += p + 2;
                            seg = j;
                            continue;
                        }
                    }
                    if bytes[j] == b'`' {
                        if let Some(p) = text[j + 1..end].find('`') {
                            push(&mut out, seg, j, Tok::Normal);
                            push(&mut out, j, j + p + 2, Tok::Str);
                            j += p + 2;
                            seg = j;
                            continue;
                        }
                    }
                    j += text[j..].chars().next().map_or(1, |ch| ch.len_utf8());
                }
                push(&mut out, seg, end, Tok::Normal);
            } else {
                push(&mut out, i, end, tok);
            }
            if end < n {
                push(&mut out, end, end + 1, Tok::Normal);
            }
            i = end + 1;
            continue;
        }

        if c == b'\n' {
            push(&mut out, i, i + 1, Tok::Normal);
            i += 1;
            line_start = true;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            let s = i;
            while i < n && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\r') {
                i += 1;
            }
            push(&mut out, s, i, Tok::Normal);
            continue;
        }

        let rest = &text[i..];
        let hash_comment = matches!(lang, Lang::Python | Lang::Shell);
        let slash_comment = matches!(lang, Lang::Rust | Lang::CLike | Lang::JavaScript);

        // 行註解
        if (slash_comment && rest.starts_with("//")) || (hash_comment && c == b'#') {
            let end = rest.find('\n').map(|p| i + p).unwrap_or(n);
            push(&mut out, i, end, Tok::Comment);
            i = end;
            continue;
        }
        // 區塊註解
        if slash_comment && rest.starts_with("/*") {
            let end = rest[2..].find("*/").map(|p| i + 2 + p + 2).unwrap_or(n);
            push(&mut out, i, end, Tok::Comment);
            i = end;
            continue;
        }
        // C 前置處理器
        if lang == Lang::CLike && c == b'#' && line_start {
            let end = rest.find('\n').map(|p| i + p).unwrap_or(n);
            push(&mut out, i, end, Tok::Pre);
            i = end;
            continue;
        }
        // Rust 屬性 #[...]
        if lang == Lang::Rust && rest.starts_with("#[") || rest.starts_with("#![") {
            let end = rest.find(']').map(|p| i + p + 1).unwrap_or(n);
            push(&mut out, i, end, Tok::Pre);
            i = end;
            line_start = false;
            continue;
        }
        line_start = false;

        // 字串
        if c == b'"' || (c == b'\'' && lang != Lang::Rust) || (c == b'`' && lang == Lang::JavaScript) {
            let q = c;
            let mut j = i + 1;
            while j < n && bytes[j] != q {
                if bytes[j] == b'\\' {
                    j += 1;
                } else if bytes[j] == b'\n' && q != b'`' && lang != Lang::Rust {
                    break;
                }
                j += 1;
            }
            let end = (j + 1).min(n);
            push(&mut out, i, end, Tok::Str);
            i = end;
            continue;
        }
        // Rust 字元常值 'a'（避開生命週期 'a）
        if c == b'\'' && lang == Lang::Rust {
            let mut chars = rest.char_indices().skip(1);
            if let Some((_, ch)) = chars.next() {
                let close = if ch == '\\' { rest[1..].find('\'').map(|p| p + 1) } else { chars.next().filter(|&(_, c2)| c2 == '\'').map(|(p, _)| p) };
                if let Some(p) = close {
                    if p > 0 {
                        push(&mut out, i, i + p + 1, Tok::Str);
                        i += p + 1;
                        continue;
                    }
                }
            }
            // 生命週期
            let mut j = i + 1;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            push(&mut out, i, j, Tok::Type);
            i = j;
            continue;
        }
        // 數字
        if c.is_ascii_digit() {
            let mut j = i;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'.' || bytes[j] == b'_') {
                j += 1;
            }
            push(&mut out, i, j, Tok::Num);
            i = j;
            continue;
        }
        // 識別字
        if c.is_ascii_alphabetic() || c == b'_' || c == b'@' || c == b'$' {
            let mut j = i + 1;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            let word = &text[i..j];
            let tok = if lang == Lang::Rust && j < n && bytes[j] == b'!' {
                j += 1; // macro!
                Tok::Pre
            } else if c == b'@' {
                Tok::Pre
            } else if kws.contains(&word) {
                Tok::Keyword
            } else if text[j..].trim_start_matches(' ').starts_with('(') {
                Tok::Func
            } else if word.chars().next().map_or(false, |ch| ch.is_ascii_uppercase()) {
                Tok::Type
            } else if lang == Lang::Json && text[j..].trim_start().starts_with(':') {
                Tok::Type
            } else {
                Tok::Normal
            };
            push(&mut out, i, j, tok);
            i = j;
            continue;
        }
        // 標點 / 其他字元（含多位元組 UTF-8）
        let ch_len = rest.chars().next().map_or(1, |ch| ch.len_utf8());
        let tok = if c.is_ascii_punctuation() { Tok::Punct } else { Tok::Normal };
        push(&mut out, i, i + ch_len, tok);
        i += ch_len;
    }
    out
}

/// 找出所有 (不分大小寫) 搜尋命中的 byte 區間
pub fn find_matches(text: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hay = text.to_lowercase();
    let nd = needle.to_lowercase();
    if hay.len() != text.len() {
        // 大小寫轉換改變長度（少見的 Unicode），退回區分大小寫
        return text.match_indices(needle).map(|(p, m)| (p, p + m.len())).collect();
    }
    hay.match_indices(&nd).map(|(p, m)| (p, p + m.len())).collect()
}

/// 產生帶語法色與搜尋命中底色的 LayoutJob
pub fn highlight(text: &str, lang: Lang, pal: &Palette, font: FontId, find: &str) -> LayoutJob {
    let sc = SynColors {
        fg: hex(pal.fg),
        kw: hex(pal.syn_kw),
        ty: hex(pal.syn_type),
        comment: hex(pal.syn_comment),
        string: hex(pal.syn_string),
        func: hex(pal.syn_func),
        pre: hex(pal.syn_pre),
        num: hex(pal.syn_num),
        punct: hex(pal.fg_dim),
        search_bg: hex(pal.search_bg),
        search_fg: hex(pal.search_fg),
    };
    let tokens = tokenize(text, lang);
    let matches = find_matches(text, find);

    let mut job = LayoutJob::default();
    let mut mi = 0;
    for (a, b, tok) in tokens {
        let mut s = a;
        while s < b {
            while mi < matches.len() && matches[mi].1 <= s {
                mi += 1;
            }
            let (e, hit) = if mi < matches.len() && matches[mi].0 <= s {
                (matches[mi].1.min(b), true)
            } else if mi < matches.len() && matches[mi].0 < b {
                (matches[mi].0, false)
            } else {
                (b, false)
            };
            let e = e.max(s + 1).min(b);
            // 確保切在字元邊界
            let mut e2 = e;
            while e2 < b && !text.is_char_boundary(e2) {
                e2 += 1;
            }
            let mut fmt = TextFormat::simple(font.clone(), if hit { sc.search_fg } else { sc.of(tok) });
            if hit {
                fmt.background = sc.search_bg;
            }
            if tok == Tok::Comment {
                fmt.italics = true;
            }
            job.append(&text[s..e2], 0.0, fmt);
            s = e2;
        }
    }
    job
}
