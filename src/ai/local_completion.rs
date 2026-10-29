//! 本地智慧補全 (LocalCompletion)
//!
//! 純本機、離線、不需 API 金鑰：從目前編輯中文件擷取識別字，
//! 依「前綴 (Kind 2) > 模糊子序列 (Kind 1)」比對等級、出現頻率 (Count)、
//! 以及與游標之距離就近度 (Proximity) 綜合評分排序候選項目。

use regex::Regex;
use std::collections::HashMap;

fn fuzzy(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let hay_chars: Vec<char> = hay.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    let needle_chars: Vec<char> = needle.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();

    let mut j = 0;
    for c in hay_chars {
        if j < needle_chars.len() && c == needle_chars[j] {
            j += 1;
        }
    }
    j == needle_chars.len()
}

pub fn suggest(
    text: &str,
    cursor_offset: usize,
    prefix: &str,
    max_items: usize,
) -> Vec<String> {
    let id_re = Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").unwrap();

    #[derive(Debug, Default)]
    struct Info {
        count: usize,
        min_dist: usize,
    }

    let mut ids: HashMap<String, Info> = HashMap::new();

    for m in id_re.find_iter(text) {
        let id = m.as_str().to_string();
        let start_pos = m.start();
        let dist = if start_pos >= cursor_offset {
            start_pos - cursor_offset
        } else {
            cursor_offset - start_pos
        };

        let entry = ids.entry(id).or_insert(Info {
            count: 0,
            min_dist: usize::MAX,
        });
        entry.count += 1;
        entry.min_dist = entry.min_dist.min(dist);
    }

    #[derive(Debug)]
    struct Cand {
        text: String,
        score: i64,
    }

    let mut cands: Vec<Cand> = Vec::new();
    let prefix_lower = prefix.to_lowercase();

    for (id, info) in &ids {
        if id == prefix {
            continue; // 不建議補成自己
        }

        let id_lower = id.to_lowercase();
        let kind = if prefix.is_empty() {
            2
        } else if id_lower.starts_with(&prefix_lower) {
            2
        } else if fuzzy(id, prefix) {
            1
        } else {
            0
        };

        if kind == 0 {
            continue;
        }

        let proximity = 1000 - info.min_dist.min(1000) as i64;
        let score = (kind as i64) * 1_000_000_000
            + (info.count as i64) * 100_000
            + proximity;

        cands.push(Cand {
            text: id.clone(),
            score,
        });
    }

    cands.sort_by(|a, b| {
        if a.score != b.score {
            b.score.cmp(&a.score)
        } else {
            a.text.cmp(&b.text) // 分數相同時依字典序，確保排序穩定
        }
    });

    cands.into_iter().take(max_items).map(|c| c.text).collect()
}
