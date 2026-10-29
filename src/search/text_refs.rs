//! 文字版全字引用搜尋 (TextRefs)
//!
//! 在文字中搜尋指定識別字的全字 (Whole Word) 出現處，
//! 用於專案內快速查找引用與定義（一檔案每一行只記錄一次）。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,   // 0-based
    pub text: String,  // 該行修剪後文字
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn find_whole_word(content: &str, name: &str) -> Vec<Hit> {
    let mut hits = Vec::new();
    if name.is_empty() {
        return hits;
    }

    let lines = content.split('\n');
    for (li, line) in lines.enumerate() {
        let mut from = 0;
        let mut added = false;

        while let Some(idx) = line[from..].find(name) {
            let abs_idx = from + idx;
            let left_ok = if abs_idx == 0 {
                true
            } else {
                let prev_char = line[..abs_idx].chars().last().unwrap();
                !is_word_char(prev_char)
            };

            let next_idx = abs_idx + name.len();
            let right_ok = if next_idx >= line.len() {
                true
            } else {
                let next_char = line[next_idx..].chars().next().unwrap();
                !is_word_char(next_char)
            };

            if left_ok && right_ok && !added {
                hits.push(Hit {
                    line: li,
                    text: line.trim().to_string(),
                });
                added = true;
            }

            from = next_idx;
            if from >= line.len() {
                break;
            }
        }
    }

    hits
}
