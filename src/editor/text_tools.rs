//! 文字工具箱轉換邏輯 (TextTools)
//!
//! 提供：
//! - 行排序（遞增 / 遞減）
//! - 去除重複行、去除空白行、反轉行、修剪行尾空白
//! - 全形 ↔ 半形轉換
//! - Unicode Escape / Unescape (\uXXXX)
//! - 數字底數解析（0x 十六進位、0b 二進位、0 開頭八進位、十進位）

use regex::Regex;
use std::collections::HashSet;

pub fn sort_lines(text: &str, descending: bool) -> String {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if descending {
        lines.sort_by(|a, b| b.cmp(a));
    } else {
        lines.sort();
    }
    lines.join("\n")
}

pub fn remove_duplicate_lines(text: &str) -> String {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for l in text.split('\n') {
        if seen.insert(l) {
            out.push(l);
        }
    }
    out.join("\n")
}

pub fn remove_blank_lines(text: &str) -> String {
    let mut out = Vec::new();
    for l in text.split('\n') {
        if !l.trim().is_empty() {
            out.push(l);
        }
    }
    out.join("\n")
}

pub fn reverse_lines(text: &str) -> String {
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.reverse();
    lines.join("\n")
}

pub fn trim_trailing_whitespace(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let trimmed_lines: Vec<&str> = lines.iter().map(|l| l.trim_end_matches(|c| c == ' ' || c == '\t')).collect();
    trimmed_lines.join("\n")
}

/// 全形字元轉半形字元
pub fn to_half_width(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let u = c as u32;
        if u == 0x3000 {
            out.push(' ');
        } else if (0xFF01..=0xFF5E).contains(&u) {
            if let Some(ch) = char::from_u32(u - 0xFEE0) {
                out.push(ch);
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 半形字元轉全形字元
pub fn to_full_width(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        let u = c as u32;
        if c == ' ' {
            out.push('\u{3000}');
        } else if (0x21..=0x7E).contains(&u) {
            if let Some(ch) = char::from_u32(u + 0xFEE0) {
                out.push(ch);
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 非 ASCII 字元轉為 \uXXXX
pub fn unicode_escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let u = c as u32;
        if u < 0x80 {
            out.push(c);
        } else {
            out.push_str(&format!("\\u{:04x}", u));
        }
    }
    out
}

/// \uXXXX 轉回原文字元
pub fn unicode_unescape(text: &str) -> String {
    let re = Regex::new(r"\\u([0-9a-fA-F]{4})").unwrap();
    let mut result = text.to_string();

    while let Some(caps) = re.captures(&result) {
        let full_match = caps.get(0).unwrap();
        let hex_str = caps.get(1).unwrap().as_str();
        if let Ok(code) = u32::from_str_radix(hex_str, 16) {
            if let Some(ch) = char::from_u32(code) {
                let mut repl = String::new();
                repl.push(ch);
                result.replace_range(full_match.range(), &repl);
                continue;
            }
        }
        break;
    }

    result
}

/// 解析整數字面值（支援 0x 十六進位、0b 二進位、0 開頭八進位、十進位）
pub fn parse_integer(s0: &str) -> Option<i64> {
    let s = s0.trim();
    if s.is_empty() {
        return None;
    }

    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else if s.starts_with("0b") || s.starts_with("0B") {
        i64::from_str_radix(&s[2..], 2).ok()
    } else if s.starts_with('0') && s.len() > 1 && !s.contains('.') {
        i64::from_str_radix(&s[1..], 8).ok()
    } else {
        s.parse::<i64>().ok()
    }
}
