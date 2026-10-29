//! 命令面板比對評分 (CommandMatch)
//!
//! 評分標準：
//! - -1: 不符合
//! - 空查詢: 0（全部保留，保持原順序）
//! - 前綴符合: 400 - name.len()
//! - 包含符合: 300 - name.len()
//! - 子序列模糊符合: 200 - name.len()

use crate::filter::FilterEngine;

pub fn score(name: &str, query: &str) -> i32 {
    let q = query.trim();
    if q.is_empty() {
        return 0;
    }

    let name_lower = name.to_lowercase();
    let q_lower = q.to_lowercase();

    if name_lower.starts_with(&q_lower) {
        return 400 - name.chars().count() as i32;
    }

    if name_lower.contains(&q_lower) {
        return 300 - name.chars().count() as i32;
    }

    if FilterEngine::fuzzy_contains(name, q) {
        return 200 - name.chars().count() as i32;
    }

    -1
}
