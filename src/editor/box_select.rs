//! 矩形（欄位 / Box / Column）選取計算邏輯 (BoxSelect)
//!
//! 給定錨點與目前游標位置 (line, col) 以及各行字元數，
//! 算出每一行要選取的欄位範圍 [start, end)。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub line: usize,
    pub start: usize,
    pub end: usize, // 半開區間 [start, end)；start == end 表示零寬（純插入點）
}

/// 計算矩形選取區塊，超出長度的欄位會 clamp 到行尾（短行處理）
pub fn compute(
    anchor_line: usize,
    anchor_col: usize,
    cur_line: usize,
    cur_col: usize,
    line_lengths: &[usize],
) -> Vec<Range> {
    let l0 = anchor_line.min(cur_line);
    let l1 = anchor_line.max(cur_line);
    let c0 = anchor_col.min(cur_col);
    let c1 = anchor_col.max(cur_col);

    let mut out = Vec::new();
    for l in l0..=l1 {
        let len = if l < line_lengths.len() {
            line_lengths[l]
        } else {
            0
        };
        let s = c0.min(len);
        let e = c1.min(len);
        out.push(Range {
            line: l,
            start: s,
            end: e,
        });
    }

    out
}
