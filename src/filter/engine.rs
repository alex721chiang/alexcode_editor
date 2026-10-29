//! AlexCode Log 行篩選引擎 (FilterEngine)
//!
//! 支援：
//! - 關鍵字以 "||" 分隔（向下相容舊式 "|"）
//! - AND / OR 邏輯運算
//! - 子序列模糊比對（如 "mwin" 匹配 "MainWindow"）
//! - 進階語法："error && !heartbeat || timeout"
//! - "re:" 正規表示式前綴（不分大小寫），如 "re:err(or)?s? && !heartbeat"
//! - 正向關鍵字提取（供多色霓虹標示）

use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterLogic {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct Term {
    pub text: String,
    pub negate: bool,
    pub is_regex: bool,
    pub regex: Option<Regex>,
}

#[derive(Debug, Clone)]
pub struct FilterEngine {
    keywords: Vec<String>,
    logic: FilterLogic,
    fuzzy: bool,
    groups: Vec<Vec<Term>>,
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterEngine {
    pub fn new() -> Self {
        Self {
            keywords: Vec::new(),
            logic: FilterLogic::Or,
            fuzzy: false,
            groups: Vec::new(),
        }
    }

    pub fn set_keywords(&mut self, keywords: Vec<String>) {
        self.keywords = keywords;
    }

    pub fn set_logic(&mut self, logic: FilterLogic) {
        self.logic = logic;
    }

    pub fn set_fuzzy(&mut self, fuzzy: bool) {
        self.fuzzy = fuzzy;
    }

    /// 將使用者輸入解析為關鍵字清單："a || b || c"（也接受舊式 "a|b"）
    pub fn parse_query(query: &str) -> Vec<String> {
        let sep = if query.contains("||") { "||" } else { "|" };
        query
            .split(sep)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// 子序列模糊比對（不分大小寫）
    pub fn fuzzy_contains(haystack: &str, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }

        let hay_chars: Vec<char> = haystack.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
        let needle_chars: Vec<char> = needle.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();

        let mut hi = 0;
        let h_len = hay_chars.len();

        for nc in needle_chars {
            let mut found = false;
            while hi < h_len {
                if hay_chars[hi] == nc {
                    found = true;
                    hi += 1;
                    break;
                }
                hi += 1;
            }
            if !found {
                return false;
            }
        }
        true
    }

    /// 根據目前設定的關鍵字與邏輯檢查一行文字
    pub fn match_line(&self, line: &str) -> bool {
        if self.keywords.is_empty() {
            return true;
        }

        let matches = |kw: &str| -> bool {
            if self.fuzzy {
                Self::fuzzy_contains(line, kw)
            } else {
                let kw_lower = kw.to_lowercase();
                let line_lower = line.to_lowercase();
                line_lower.contains(&kw_lower)
            }
        };

        match self.logic {
            FilterLogic::Or => {
                for kw in &self.keywords {
                    if matches(kw) {
                        return true;
                    }
                }
                false
            }
            FilterLogic::And => {
                for kw in &self.keywords {
                    if !matches(kw) {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// 編譯進階篩選語法：
    /// "error && !heartbeat || timeout"
    /// "re:err(or)?s? && !heartbeat"
    pub fn compile(&mut self, query: &str) {
        self.groups.clear();
        let advanced = query.contains("&&") || query.contains('!');
        let or_parts: Vec<&str> = if query.contains("||") {
            query.split("||").filter(|s| !s.is_empty()).collect()
        } else if !advanced && query.contains('|') {
            query.split('|').filter(|s| !s.is_empty()).collect()
        } else {
            vec![query]
        };

        for g in or_parts {
            let mut terms = Vec::new();
            let and_parts = g.split("&&").filter(|s| !s.is_empty());
            for raw_t in and_parts {
                let mut t = raw_t.trim();
                let neg = t.starts_with('!');
                if neg {
                    t = t[1..].trim();
                }
                if t.is_empty() {
                    continue;
                }

                let mut is_regex = false;
                let mut regex = None;

                if t.starts_with("re:") {
                    let pattern = t[3..].trim();
                    if !pattern.is_empty() {
                        if let Ok(re) = RegexBuilder::new(pattern).case_insensitive(true).build() {
                            is_regex = true;
                            regex = Some(re);
                        }
                    }
                    // 若 pattern 無效或錯誤，則保留 term.text = t 原樣（例如 "re:[invalid("），
                    // 退回字面比對不崩潰。
                }

                terms.push(Term {
                    text: t.to_string(),
                    negate: neg,
                    is_regex,
                    regex,
                });
            }
            if !terms.is_empty() {
                self.groups.push(terms);
            }
        }
    }

    /// 比對編譯後的進階規則
    pub fn match_compiled(&self, line: &str) -> bool {
        if self.groups.is_empty() {
            return true;
        }

        let line_lower = line.to_lowercase();

        for group in &self.groups {
            let mut all = true;
            for term in group {
                let hit = if term.is_regex {
                    term.regex.as_ref().map_or(false, |re| re.is_match(line))
                } else if self.fuzzy {
                    Self::fuzzy_contains(line, &term.text)
                } else {
                    line_lower.contains(&term.text.to_lowercase())
                };

                if hit == term.negate {
                    all = false;
                    break;
                }
            }
            if all {
                return true;
            }
        }

        false
    }

    /// 正向關鍵字清單（排除 regex 與否定項），供多色標籤/高亮使用
    pub fn positive_keywords(&self) -> Vec<String> {
        let mut kws = Vec::new();
        for group in &self.groups {
            for term in group {
                if !term.negate && !term.is_regex {
                    let term_lower = term.text.to_lowercase();
                    let already_has = kws.iter().any(|k: &String| k.to_lowercase() == term_lower);
                    if !already_has {
                        kws.push(term.text.clone());
                    }
                }
            }
        }
        kws
    }
}
