//! 並排 Diff 的行對齊演算法 (DiffCalc)
//!
//! 以 LCS (最長共同子序列) 對齊兩份文字行，產生左右成對的「列」。
//! 連續的刪除與新增會盡量配對成 Modified（左右各佔一列），
//! 落單者以 -1 表示該側為填充列（空白墊高）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowType {
    Same,
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub left: i32,        // a 的 0-based 行號；-1 = 左側填充
    pub right: i32,       // b 的 0-based 行號；-1 = 右側填充
    pub row_type: RowType,
}

fn flush_pending(rows: &mut Vec<Row>, dels: &mut Vec<i32>, adds: &mut Vec<i32>) {
    let paired = dels.len().min(adds.len());
    for t in 0..paired {
        rows.push(Row {
            left: dels[t],
            right: adds[t],
            row_type: RowType::Modified,
        });
    }
    for t in paired..dels.len() {
        rows.push(Row {
            left: dels[t],
            right: -1,
            row_type: RowType::Removed,
        });
    }
    for t in paired..adds.len() {
        rows.push(Row {
            left: -1,
            right: adds[t],
            row_type: RowType::Added,
        });
    }
    dels.clear();
    adds.clear();
}

pub fn align(a: &[String], b: &[String]) -> Vec<Row> {
    let n = a.len();
    let m = b.len();

    // LCS 動態規劃矩陣
    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if a[i] == b[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut rows = Vec::new();
    let mut dels = Vec::new();
    let mut adds = Vec::new();

    let mut i = 0;
    let mut j = 0;

    while i < n && j < m {
        if a[i] == b[j] {
            flush_pending(&mut rows, &mut dels, &mut adds);
            rows.push(Row {
                left: i as i32,
                right: j as i32,
                row_type: RowType::Same,
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            dels.push(i as i32);
            i += 1;
        } else {
            adds.push(j as i32);
            j += 1;
        }
    }

    while i < n {
        dels.push(i as i32);
        i += 1;
    }
    while j < m {
        adds.push(j as i32);
        j += 1;
    }

    flush_pending(&mut rows, &mut dels, &mut adds);
    rows
}

/// 同 [`align`]，但先去掉兩邊相同的開頭與結尾，只對中間不同的部分做 LCS：
/// 大檔案裡改幾行不需要為整份檔案建表。
/// 中間部分的 (左側行數 × 右側行數) 超過 `max_cells` 時回傳 None（LCS 的表格會佔用太多記憶體）。
pub fn align_trimmed(a: &[String], b: &[String], max_cells: usize) -> Option<Vec<Row>> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (mid_a, mid_b) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    if mid_a.len().saturating_mul(mid_b.len()) > max_cells {
        return None;
    }
    let same = |left: usize, right: usize| Row { left: left as i32, right: right as i32, row_type: RowType::Same };
    let mut rows = Vec::with_capacity(a.len().max(b.len()));
    rows.extend((0..prefix).map(|i| same(i, i)));
    rows.extend(align(mid_a, mid_b).into_iter().map(|mut row| {
        if row.left >= 0 {
            row.left += prefix as i32;
        }
        if row.right >= 0 {
            row.right += prefix as i32;
        }
        row
    }));
    rows.extend((0..suffix).map(|k| same(a.len() - suffix + k, b.len() - suffix + k)));
    Some(rows)
}

/// 每一段連續變更（非 Same 的列）第一列的索引
pub fn change_starts(rows: &[Row]) -> Vec<usize> {
    (0..rows.len())
        .filter(|&i| rows[i].row_type != RowType::Same && (i == 0 || rows[i - 1].row_type == RowType::Same))
        .collect()
}

/// 計算非 Same 的變更列數
pub fn change_count(rows: &[Row]) -> usize {
    rows.iter().filter(|r| r.row_type != RowType::Same).count()
}
