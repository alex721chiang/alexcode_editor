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
        // 中段過大：粗略整段標 Modified，避免大檔 O(n*m) 計算與記憶體溢出
        emit_change_run(&mut states, prefix, on, om, m);
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
