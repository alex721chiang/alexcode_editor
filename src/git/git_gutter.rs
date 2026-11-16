//! Git Gutter 行狀態差異計算
//!
//! 比較 HEAD 版本與目前緩衝區，輸出每行的 Git 狀態標記：
//! - Added (1): 新增行（青色條）
//! - Modified (2): 修改行（黃色條）
//! - DeletedAbove (4): 此行上方有刪除（洋紅三角）

use std::collections::HashMap;

pub const ADDED: u8 = 1;
pub const MODIFIED: u8 = 2;
pub const DELETED_ABOVE: u8 = 4;

fn emit_change_run(
    states: &mut HashMap<usize, u8>,
    new_start: usize,
    del_count: usize,
    add_count: usize,
    new_total: usize,
) {
    let paired = del_count.min(add_count);
    for k in 0..paired {
        let entry = states.entry(new_start + k).or_insert(0);
        *entry |= MODIFIED;
    }
    for k in paired..add_count {
        let entry = states.entry(new_start + k).or_insert(0);
        *entry |= ADDED;
    }
    if del_count > add_count && new_total > 0 {
        // 淨刪除：標示於刪除點下一行（若在結尾則標於最後一行）
        let mark = (new_start + add_count).min(new_total - 1);
        let entry = states.entry(mark).or_insert(0);
        *entry |= DELETED_ABOVE;
    }
}

/// 中段太大時最多容許幾處增刪（一行算一處）：Myers 演算法要記下每一步的狀態才能回溯，
/// 佔用約 8 × 處數² 位元組（1000 處約 8 MB，算完就釋放），時間約為 處數² + 行數
const MYERS_MAX_EDITS: usize = 1000;

/// Myers O(ND) 差異演算法：`a` 變成 `b` 的最短編輯腳本，整理成變更區段 (在 `b` 中的起點, 刪除行數, 新增行數)，
/// 依位置由前到後；相鄰的增刪併成一段。需要超過 `max_d` 處增刪時回傳 None
fn myers_runs(a: &[String], b: &[String], max_d: usize) -> Option<Vec<(usize, usize, usize)>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = max_d as isize;
    let off = max + 1;
    // v[off + k]：在對角線 k（= x - y）上目前走到的最遠 x
    let mut v = vec![0isize; (2 * max + 3) as usize];
    // trace[d]：開始走第 d 步之前的 v，只留用得到的範圍 k ∈ [-d-1, d+1]
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found = None;
    'search: for d in 0..=max {
        trace.push(v[(off - d - 1) as usize..=(off + d + 1) as usize].to_vec());
        for k in (-d..=d).step_by(2) {
            let down = k == -d || (k != d && v[(off + k - 1) as usize] < v[(off + k + 1) as usize]);
            let mut x = if down { v[(off + k + 1) as usize] } else { v[(off + k - 1) as usize] + 1 };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[(off + k) as usize] = x;
            if x >= n && y >= m {
                found = Some(d);
                break 'search;
            }
        }
    }
    // 從終點往回走，找出每一步是刪除 a 的一行還是插入 b 的一行：(這一步之前的 x, y, 是否為插入)
    let mut steps: Vec<(isize, isize, bool)> = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (1..=found?).rev() {
        let before = &trace[d as usize];
        let at = |k: isize| before[(k + d + 1) as usize];
        let k = x - y;
        let down = k == -d || (k != d && at(k - 1) < at(k + 1));
        let prev_k = if down { k + 1 } else { k - 1 };
        let prev_x = at(prev_k);
        let prev_y = prev_x - prev_k;
        steps.push((prev_x, prev_y, down));
        (x, y) = (prev_x, prev_y);
    }
    steps.reverse();
    // 相鄰的步驟（中間沒有相同的行）併成一段
    let mut runs: Vec<(usize, usize, usize)> = Vec::new();
    let mut end = (-1, -1); // 上一步結束的位置
    for (x, y, insert) in steps {
        if (x, y) != end {
            runs.push((y as usize, 0, 0));
        }
        let run = runs.last_mut()?;
        if insert {
            run.2 += 1;
            end = (x, y + 1);
        } else {
            run.1 += 1;
            end = (x + 1, y);
        }
    }
    Some(runs)
}

pub fn diff_line_states(
    old_lines: &[String],
    new_lines: &[String],
    max_middle: usize,
) -> HashMap<usize, u8> {
    let mut states = HashMap::new();
    let n = old_lines.len();
    let m = new_lines.len();

    // 共同前綴修剪
    let mut prefix = 0;
    while prefix < n && prefix < m && old_lines[prefix] == new_lines[prefix] {
        prefix += 1;
    }

    // 共同後綴修剪
    let mut suffix = 0;
    while suffix < n - prefix && suffix < m - prefix && old_lines[n - 1 - suffix] == new_lines[m - 1 - suffix] {
        suffix += 1;
    }

    let on = n - prefix - suffix; // 中段長度
    let om = m - prefix - suffix;

    if on == 0 && om == 0 {
        return states; // 無差異
    }
    if on == 0 {
        emit_change_run(&mut states, prefix, 0, om, m);
        return states;
    }
    if om == 0 {
        emit_change_run(&mut states, prefix, on, 0, m);
        return states;
    }
    if on > max_middle || om > max_middle {
        // 中段過大，LCS 表格（O(n*m)）放不下。差異不多時改用 Myers 演算法逐段標出（花費只跟差異的數量有關）：
        // 大檔案開頭插入一行、後面再改一行，中段就幾乎是整個檔案，但真正不同的只有兩處。
        // 差異多到算不完才粗略地整段標成 Modified
        match myers_runs(&old_lines[prefix..n - suffix], &new_lines[prefix..m - suffix], MYERS_MAX_EDITS) {
            Some(runs) => {
                for (start, del, add) in runs {
                    emit_change_run(&mut states, prefix + start, del, add, m);
                }
            }
            None => emit_change_run(&mut states, prefix, on, om, m),
        }
        return states;
    }

    // 中段 LCS
    let mut dp = vec![vec![0; om + 1]; on + 1];
    for i in (0..on).rev() {
        for j in (0..om).rev() {
            if old_lines[prefix + i] == new_lines[prefix + j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut i = 0;
    let mut j = 0;
    let mut run_del = 0;
    let mut run_add = 0;
    let mut run_start = 0;

    let flush = |states: &mut HashMap<usize, u8>, run_del: &mut usize, run_add: &mut usize, run_start: usize| {
        if *run_del > 0 || *run_add > 0 {
            emit_change_run(states, prefix + run_start, *run_del, *run_add, m);
        }
        *run_del = 0;
        *run_add = 0;
    };

    while i < on && j < om {
        if old_lines[prefix + i] == new_lines[prefix + j] {
            flush(&mut states, &mut run_del, &mut run_add, run_start);
            i += 1;
            j += 1;
        } else {
            if run_del == 0 && run_add == 0 {
                run_start = j;
            }
            if dp[i + 1][j] >= dp[i][j + 1] {
                run_del += 1;
                i += 1;
            } else {
                run_add += 1;
                j += 1;
            }
        }
    }

    if i < on || j < om {
        if run_del == 0 && run_add == 0 {
            run_start = j;
        }
        run_del += on - i;
        run_add += om - j;
    }

    flush(&mut states, &mut run_del, &mut run_add, run_start);
    states
}
