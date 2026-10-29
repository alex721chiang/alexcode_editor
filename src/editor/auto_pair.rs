//! 括號與引號自動配對決策邏輯 (AutoPair)
//!
//! 支援：
//! - 輸入開括號 ( [ { 或引號 " ' ` 自動補齊對側
//! - 選取文字時輸入開括號或引號則以成對符號包圍選取 (Surround)
//! - 游標右側已存在相同閉括號或引號時跳過 (SkipOver)
//! - 退格刪除空配對 (DeletePair)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Insert,    // 照常插入
    AutoClose, // 插入「開+閉」，游標停中間
    SkipOver,  // 右側已是相同閉合字元 → 游標右移、不插入
    Surround,  // 有選取 → 以開/閉字元包圍選取
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub open: Option<char>,
    pub close: Option<char>,
}

pub fn closing_for(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        _ => None,
    }
}

pub fn is_quote(c: char) -> bool {
    c == '"' || c == '\'' || c == '`'
}

pub fn is_opener(c: char) -> bool {
    c == '(' || c == '[' || c == '{'
}

pub fn is_closer(c: char) -> bool {
    c == ')' || c == ']' || c == '}'
}

pub fn is_relevant(c: char) -> bool {
    is_opener(c) || is_closer(c) || is_quote(c)
}

pub fn decide(
    typed: char,
    before: Option<char>,
    after: Option<char>,
    has_selection: bool,
) -> Decision {
    let close = closing_for(typed);

    // 有選取且輸入開括號/引號 → 包圍
    if has_selection && (is_opener(typed) || is_quote(typed)) {
        return Decision {
            action: Action::Surround,
            open: Some(typed),
            close: if is_quote(typed) { Some(typed) } else { close },
        };
    }

    if has_selection {
        return Decision {
            action: Action::Insert,
            open: None,
            close: None,
        };
    }

    // 輸入閉括號，且右側已是相同閉括號 → 跳過
    if is_closer(typed) && after == Some(typed) {
        return Decision {
            action: Action::SkipOver,
            open: None,
            close: Some(typed),
        };
    }

    // 引號：右側已是相同引號 → 跳過
    if is_quote(typed) && after == Some(typed) {
        return Decision {
            action: Action::SkipOver,
            open: None,
            close: Some(typed),
        };
    }

    // 引號自動配對：避免在英文字詞中（如縮寫 don't）誤配
    if is_quote(typed) {
        let is_letter_or_num = |c: Option<char>| c.map_or(false, |ch| ch.is_alphanumeric());
        if is_letter_or_num(before) || is_letter_or_num(after) {
            return Decision {
                action: Action::Insert,
                open: None,
                close: None,
            };
        }
        return Decision {
            action: Action::AutoClose,
            open: Some(typed),
            close: Some(typed),
        };
    }

    // 括號自動配對：右側若緊接字詞字元則不自動配（避免吃進後面的字）
    if is_opener(typed) {
        if after.map_or(false, |ch| ch.is_alphanumeric()) {
            return Decision {
                action: Action::Insert,
                open: None,
                close: None,
            };
        }
        return Decision {
            action: Action::AutoClose,
            open: Some(typed),
            close,
        };
    }

    Decision {
        action: Action::Insert,
        open: None,
        close: None,
    }
}

/// 退格時檢查是否為一對空配對（例如游標在 () 或 "" 之間）
pub fn should_delete_pair(before: Option<char>, after: Option<char>) -> bool {
    if let (Some(b), Some(a)) = (before, after) {
        closing_for(b) == Some(a)
    } else {
        false
    }
}
