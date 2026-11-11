//! 語法樹符號分類 (TsSymbols)
//!
//! 供 Go to Symbol (Ctrl+Shift+O)、專案符號索引 (Ctrl+T) 與麵包屑 (Breadcrumb) 使用。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub end_line: usize,
    pub depth: usize,
}

pub fn symbol_kind(type_str: &str) -> Option<&'static str> {
    match type_str {
        "function_definition" | "function_declaration" | "generator_function_declaration" => {
            Some("function")
        }
        "method_definition" => Some("method"),
        "class_specifier" | "class_definition" | "class_declaration" => Some("class"),
        "struct_specifier" => Some("struct"),
        "union_specifier" => Some("union"),
        "enum_specifier" => Some("enum"),
        "namespace_definition" => Some("namespace"),
        _ => None,
    }
}

pub fn is_symbol_node(type_str: &str) -> bool {
    symbol_kind(type_str).is_some()
}

pub fn is_call_node(type_str: &str) -> bool {
    type_str == "call_expression" || type_str == "call"
}

// ---------------------------------------------------------------------------
// 由原始碼文字擷取符號（不依賴語法樹）：供 Sticky Scroll 使用
// ---------------------------------------------------------------------------

use regex::Regex;
use std::sync::OnceLock;

/// 擷取符號時採用的語法規則
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolLang {
    /// Rust：以 `fn` / `struct` / `impl` … 關鍵字辨識
    Rust,
    /// 以大括號分區塊的語言：C / C++ / Java / C# / Go / Kotlin / Swift / JavaScript / TypeScript
    CFamily,
    /// 以縮排分區塊：Python
    Python,
}

/// 擷取跨越多行的區塊（函式、類別、命名空間…）。結果依起始行排序；
/// `depth` 為外層符號的數目，`end_line` 為區塊最後一行（含）。
///
/// 這是逐字元掃描的啟發式作法：字串、字元常值與註解中的括號會略過，
/// 但不做完整的語法分析。
pub fn extract_symbols(lines: &[String], lang: SymbolLang) -> Vec<Symbol> {
    let mut syms = match lang {
        SymbolLang::Python => indent_symbols(lines),
        SymbolLang::Rust => brace_symbols(lines, true),
        SymbolLang::CFamily => brace_symbols(lines, false),
    };
    syms.retain(|s| s.end_line > s.line);
    syms
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid built-in regex"))
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// 外層是類別 / impl 之類時，函式視為方法
fn is_type_scope(kind: &str) -> bool {
    matches!(kind, "class" | "struct" | "interface" | "impl" | "trait" | "union" | "object")
}

enum StrState {
    /// 以該字元結尾、支援反斜線跳脫
    Normal(char),
    /// Rust 原始字串：以 `"` 加上 n 個 `#` 結尾
    Raw(usize),
}

struct Frame {
    sym: Option<usize>,
    parens: i32,
}

/// 陳述句文字中每一段對應的原始行號：(段落在文字中的起始 byte, 行號)
type Segments = Vec<(usize, usize)>;

/// 標頭（多行參數列）最多跨幾行
const MAX_HEADER_LINES: usize = 40;

/// 從陳述句起點到 `{` 之前的文字（多行以空白相接，去掉行尾 `//` 註解）
fn header_text(lines: &[String], start: (usize, usize), end: (usize, usize)) -> Option<(String, Segments)> {
    if end.0 - start.0 >= MAX_HEADER_LINES {
        return None;
    }
    let mut text = String::new();
    let mut segs = Segments::new();
    for (li, line) in lines.iter().enumerate().take(end.0 + 1).skip(start.0) {
        let cs: Vec<char> = line.chars().collect();
        let from = if li == start.0 { start.1 } else { 0 };
        let to = if li == end.0 { end.1 } else { cs.len() };
        let piece: String = cs[from.min(to)..to].iter().collect();
        let piece = piece.split("//").next().unwrap_or("").trim();
        if piece.is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        segs.push((text.len(), li));
        text.push_str(piece);
    }
    Some((text, segs))
}

fn line_of(segs: &Segments, offset: usize) -> Option<usize> {
    segs.iter().rev().find(|(start, _)| *start <= offset).map(|(_, line)| *line)
}

/// (種類, 名稱, 名稱或關鍵字在標頭文字中的 byte 位置)
type Header = (&'static str, String, usize);

fn classify_rust(h: &str) -> Option<Header> {
    static ITEM: OnceLock<Regex> = OnceLock::new();
    static MACRO: OnceLock<Regex> = OnceLock::new();
    let item = re(
        &ITEM,
        r#"^(?:pub(?:\s*\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+(?:"[^"]*"\s+)?)?(fn|struct|enum|trait|impl|mod|union)\b(.*)$"#,
    );
    if let Some(c) = item.captures(h) {
        let kw = c.get(1)?;
        let rest = c.get(2)?.as_str().trim();
        let (kind, name) = match kw.as_str() {
            "impl" => {
                let name = rest.split(" where ").next().unwrap_or(rest);
                ("impl", name.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            other => {
                let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
                if name.is_empty() {
                    return None;
                }
                let kind = match other {
                    "fn" => "function",
                    "struct" => "struct",
                    "enum" => "enum",
                    "trait" => "trait",
                    "mod" => "module",
                    _ => "union",
                };
                (kind, name)
            }
        };
        return Some((kind, name, kw.start()));
    }
    let mac = re(&MACRO, r"^macro_rules!\s*([A-Za-z_]\w*)$");
    mac.captures(h).map(|c| ("macro", c[1].to_string(), 0))
}

const CONTROL_WORDS: &[&str] = &[
    "if", "else", "for", "foreach", "while", "do", "switch", "case", "default", "try", "catch", "finally", "return",
    "throw", "new", "delete", "with", "using", "lock", "synchronized", "match", "select", "go", "defer", "when",
    "guard", "repeat", "await", "yield", "typeof", "sizeof", "in", "of", "and", "or", "not",
];

fn classify_c_family(h: &str) -> Option<Header> {
    static ANNOTATION: OnceLock<Regex> = OnceLock::new();
    static TYPE: OnceLock<Regex> = OnceLock::new();
    static FUNC_KW: OnceLock<Regex> = OnceLock::new();
    static ARROW: OnceLock<Regex> = OnceLock::new();
    static CALLEE: OnceLock<Regex> = OnceLock::new();

    // 去掉開頭的標註（@Override、@Component(...)）與存取標籤（public:）
    let mut skip = 0;
    let lead = re(&ANNOTATION, r"^(?:@[\w.]+(?:\([^)]*\))?\s*|(?:public|private|protected)\s*:\s*)");
    while let Some(m) = lead.find(&h[skip..]) {
        if m.end() == 0 {
            break;
        }
        skip += m.end();
    }
    let body = &h[skip..];
    let first_paren = body.find('(');

    let ty = re(
        &TYPE,
        r"^(?:(?:export|default|declare|public|private|protected|internal|static|final|abstract|sealed|partial|open|data|inline|typedef|template\s*<.*>)\s+)*(class|struct|enum|namespace|interface|union|object)\b\s*(?:(?:class|struct)\s+)?([A-Za-z_$][\w$:.]*)?",
    );
    if let Some(c) = ty.captures(body) {
        let kw = c.get(1)?;
        let c_style_type = matches!(kw.as_str(), "struct" | "union" | "enum");
        // `struct foo *make(void) {` 是函式；`struct foo x = {` 是初始化
        if !(body.contains('=') || c_style_type && first_paren.is_some()) {
            let kind = match kw.as_str() {
                "class" => "class",
                "struct" => "struct",
                "enum" => "enum",
                "namespace" => "namespace",
                "interface" => "interface",
                "union" => "union",
                _ => "object",
            };
            let name = c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            return Some((kind, name, skip + kw.start()));
        }
    }

    let func_kw = re(
        &FUNC_KW,
        r"^(?:(?:export|default|async|public|private|internal|protected|override|open|static|final|suspend|inline|mutating|fileprivate)\s+)*(?:function\b\s*\*?|func\b|fun\b)\s*(?:<[^>]*>\s*)?(?:\([^)]*\)\s*)?([A-Za-z_$][\w$.]*)?",
    );
    if let Some(c) = func_kw.captures(body) {
        let name = c.get(1);
        let at = name.map_or(c.get(0)?.start(), |m| m.start());
        return Some(("function", name.map(|m| m.as_str().to_string()).unwrap_or_default(), skip + at));
    }

    let arrow = re(&ARROW, r"^(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:async\s*)?(?:function\b|\(|[A-Za-z_$][\w$]*\s*=>)");
    if let Some(c) = arrow.captures(body) {
        if body.trim_end().ends_with("=>") || body.contains("function") {
            let name = c.get(1)?;
            return Some(("function", name.as_str().to_string(), skip + name.start()));
        }
    }

    // 一般的 C 風格函式定義：`回傳型別 名稱(參數)`，名稱前不能有指派
    let p = first_paren?;
    if body[..p].contains('=') {
        return None;
    }
    let first_word: String = body.chars().take_while(|c| is_ident(*c)).collect();
    if CONTROL_WORDS.contains(&first_word.as_str()) {
        return None;
    }
    let callee = re(&CALLEE, r"([A-Za-z_~$][\w$~]*(?:::[A-Za-z_~$][\w$~]*)*)\s*(?:<[^<>()]*>)?\s*$");
    let m = callee.captures(&body[..p])?.get(1)?;
    let name = m.as_str().rsplit("::").next().unwrap_or("");
    if name.is_empty() || CONTROL_WORDS.contains(&name) {
        return None;
    }
    Some(("function", name.to_string(), skip + m.start()))
}

fn brace_symbols(lines: &[String], rust: bool) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    // 目前陳述句的起點 (行, 字元欄)；None 表示上一個陳述句已結束
    let mut stmt: Option<(usize, usize)> = None;
    let mut parens: i32 = 0;
    let mut block_comment = false;
    let mut string: Option<StrState> = None;

    for (li, line) in lines.iter().enumerate() {
        let cs: Vec<char> = line.chars().collect();
        let at = |i: usize| cs.get(i).copied().unwrap_or('\0');
        let mut i = 0;
        let mut seen_code = false;
        while i < cs.len() {
            let ch = cs[i];
            if block_comment {
                if ch == '*' && at(i + 1) == '/' {
                    block_comment = false;
                    i += 1;
                }
                i += 1;
                continue;
            }
            match &string {
                Some(StrState::Normal(q)) => {
                    if ch == '\\' {
                        i += 1;
                    } else if ch == *q {
                        string = None;
                    }
                    i += 1;
                    continue;
                }
                Some(StrState::Raw(n)) => {
                    let n = *n;
                    if ch == '"' && (1..=n).all(|k| at(i + k) == '#') {
                        string = None;
                        i += n;
                    }
                    i += 1;
                    continue;
                }
                None => {}
            }
            if ch.is_whitespace() {
                i += 1;
                continue;
            }
            if ch == '/' && at(i + 1) == '/' {
                break;
            }
            if ch == '/' && at(i + 1) == '*' {
                block_comment = true;
                i += 2;
                continue;
            }
            if ch == '#' && rust && stmt.is_none() && (at(i + 1) == '[' || at(i + 1) == '!') {
                // 屬性 `#[...]`：不算陳述句的開頭
                let mut depth = 0;
                while i < cs.len() {
                    match cs[i] {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }
            if ch == '#' && !rust && !seen_code {
                break; // 前置處理器指令（#include / #define）
            }
            seen_code = true;
            if stmt.is_none() {
                stmt = Some((li, i));
                parens = 0;
            }
            match ch {
                '"' => {
                    let mut hashes = 0;
                    while hashes < i && cs[i - 1 - hashes] == '#' {
                        hashes += 1;
                    }
                    let r_at = i.checked_sub(hashes + 1);
                    let raw = rust
                        && r_at.is_some_and(|r| cs[r] == 'r' && (r == 0 || !is_ident(cs[r - 1]) || cs[r - 1] == 'b'));
                    string = Some(if raw { StrState::Raw(hashes) } else { StrState::Normal('"') });
                }
                '\'' if rust => {
                    // 字元常值 `'x'` / `'\n'`；其餘是生命週期標記 `'a`
                    if at(i + 1) == '\\' {
                        let mut j = i + 3;
                        while j < cs.len() && cs[j] != '\'' {
                            j += 1;
                        }
                        i = j;
                    } else if at(i + 2) == '\'' {
                        i += 2;
                    }
                }
                '\'' => string = Some(StrState::Normal('\'')),
                '`' if !rust => string = Some(StrState::Normal('`')),
                '(' | '[' => parens += 1,
                ')' | ']' => parens -= 1,
                '{' => {
                    let header = if parens <= 0 {
                        stmt.and_then(|s| header_text(lines, s, (li, i))).and_then(|(text, segs)| {
                            let found = if rust { classify_rust(&text) } else { classify_c_family(&text) };
                            found.map(|(kind, name, pos)| (kind, name, line_of(&segs, pos).unwrap_or(li)))
                        })
                    } else {
                        None
                    };
                    let sym = header.map(|(kind, name, line)| {
                        let enclosing: Vec<&Symbol> = stack.iter().filter_map(|f| f.sym).map(|id| &out[id]).collect();
                        let kind = match (kind, enclosing.last()) {
                            ("function", Some(parent)) if is_type_scope(&parent.kind) => "method",
                            _ => kind,
                        };
                        let depth = enclosing.len();
                        out.push(Symbol { name, kind: kind.to_string(), line, end_line: usize::MAX, depth });
                        out.len() - 1
                    });
                    stack.push(Frame { sym, parens });
                    stmt = None;
                    parens = 0;
                }
                '}' => {
                    if let Some(frame) = stack.pop() {
                        if let Some(id) = frame.sym {
                            out[id].end_line = li;
                        }
                        parens = frame.parens;
                    }
                    stmt = None;
                }
                ';' if parens <= 0 => stmt = None,
                _ => {}
            }
            i += 1;
        }
        // 一般字串不跨行（Rust 字串與 JavaScript 樣板字串除外）
        if matches!(string, Some(StrState::Normal(q)) if !rust && q != '`') {
            string = None;
        }
    }

    let last = lines.len().saturating_sub(1);
    for s in &mut out {
        if s.end_line == usize::MAX {
            s.end_line = last; // 尚未輸入右括號
        }
    }
    out
}

fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

fn indent_symbols(lines: &[String]) -> Vec<Symbol> {
    static DEF: OnceLock<Regex> = OnceLock::new();
    let def = re(&DEF, r"^(?:async\s+)?(def|class)\s+([A-Za-z_]\w*)");
    let is_code = |l: &str| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#')
    };

    let mut out: Vec<Symbol> = Vec::new();
    // 尚未結束的外層符號：(縮排, 在 out 中的索引)
    let mut open: Vec<(usize, usize)> = Vec::new();
    // 多行標頭（參數換行）結束前，不以縮排判斷區塊結束
    let mut header_until = 0;
    let mut last_code = 0;

    for (li, line) in lines.iter().enumerate() {
        if !is_code(line) {
            continue;
        }
        let indent = indent_of(line);
        if li >= header_until {
            while open.last().is_some_and(|(ind, _)| indent <= *ind) {
                let (_, id) = open.pop().unwrap_or_default();
                out[id].end_line = last_code;
            }
        }
        last_code = li;
        let Some(c) = def.captures(line.trim_start()) else { continue };
        let is_class = &c[1] == "class";
        let kind = match open.last() {
            _ if is_class => "class",
            Some((_, parent)) if out[*parent].kind == "class" => "method",
            _ => "function",
        };
        out.push(Symbol { name: c[2].to_string(), kind: kind.to_string(), line: li, end_line: li, depth: open.len() });
        open.push((indent, out.len() - 1));

        // 找出標頭結束的那一行（括號配對完成且以 `:` 結尾）
        let mut depth = 0i32;
        header_until = li + 1;
        for (k, l) in lines.iter().enumerate().skip(li).take(MAX_HEADER_LINES) {
            let code = l.split('#').next().unwrap_or("");
            depth += code.chars().filter(|c| "([{".contains(*c)).count() as i32;
            depth -= code.chars().filter(|c| ")]}".contains(*c)).count() as i32;
            if depth <= 0 {
                header_until = k + 1;
                break;
            }
        }
    }
    for (_, id) in open {
        out[id].end_line = last_code;
    }
    out
}
