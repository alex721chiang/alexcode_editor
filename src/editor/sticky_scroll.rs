//! Sticky Scroll 頂端固定標頭計算
//!
//! 當程式碼向上捲動時，將目前所處的類別或函式宣告標頭固定在編輯器頂端，
//! 方便隨時辨識目前所在的程式碼區塊層級。

/// 與符號索引共用同一個型別：`line` / `end_line` 為 0-based 起訖行，`depth` 為巢狀深度
pub use crate::index::symbols::Symbol;

/// 取得要固定在頂端的符號鏈（依起始行由小到大排序）
pub fn headers(syms: &[Symbol], first_visible_line: usize, max_depth: usize) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = syms
        .iter()
        .filter(|s| s.line < first_visible_line && first_visible_line <= s.end_line)
        .cloned()
        .collect();

    out.sort_by_key(|s| s.line);

    if max_depth > 0 && out.len() > max_depth {
        let skip = out.len() - max_depth;
        out.drain(0..skip);
    }

    out
}
