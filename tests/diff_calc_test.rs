use alexcode::git::{align, change_count, RowType};

fn s(list: &[&str]) -> Vec<String> {
    list.iter().map(|&x| x.to_string()).collect()
}

#[test]
fn test_identical_texts_all_same() {
    let rows = align(&s(&["a", "b"]), &s(&["a", "b"]));
    assert_eq!(rows.len(), 2);
    assert_eq!(change_count(&rows), 0);
    assert_eq!(rows[0].left, 0);
    assert_eq!(rows[0].right, 0);
    assert_eq!(rows[1].row_type, RowType::Same);
}

#[test]
fn test_pure_addition() {
    let rows = align(&s(&["a", "c"]), &s(&["a", "b", "c"]));
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].row_type, RowType::Added);
    assert_eq!(rows[1].left, -1); // 左側填充
    assert_eq!(rows[1].right, 1);
    assert_eq!(change_count(&rows), 1);
}

#[test]
fn test_pure_removal() {
    let rows = align(&s(&["a", "b", "c"]), &s(&["a", "c"]));
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].row_type, RowType::Removed);
    assert_eq!(rows[1].left, 1);
    assert_eq!(rows[1].right, -1); // 右側填充
}

#[test]
fn test_changed_line_becomes_modified_pair() {
    let rows = align(&s(&["a", "OLD", "c"]), &s(&["a", "NEW", "c"]));
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].row_type, RowType::Modified); // 一刪一增配成同一列
    assert_eq!(rows[1].left, 1);
    assert_eq!(rows[1].right, 1);
    assert_eq!(change_count(&rows), 1);
}

#[test]
fn test_unbalanced_block_pairs_then_fills() {
    // 兩行換成一行：一列 Modified + 一列 Removed
    let rows = align(&s(&["a", "x", "y", "c"]), &s(&["a", "z", "c"]));
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[1].row_type, RowType::Modified);
    assert_eq!(rows[2].row_type, RowType::Removed);
    assert_eq!(rows[2].right, -1);
}

#[test]
fn test_empty_versus_content() {
    let rows = align(&s(&[""]), &s(&["a", "b"]));
    // "" 與 a/b 皆不同 → "" 配 a 成 Modified、b 為 Added
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].row_type, RowType::Modified);
    assert_eq!(rows[1].row_type, RowType::Added);
}

#[test]
fn test_trimmed_alignment_matches_the_full_one() {
    use alexcode::git::align_trimmed;
    let cases: [(&[&str], &[&str]); 7] = [
        (&["a", "b"], &["a", "b"]),
        (&["a", "c"], &["a", "b", "c"]),
        (&["a", "b", "c"], &["a", "c"]),
        (&["a", "OLD", "c", "d"], &["a", "NEW", "c", "d"]),
        (&["a", "x", "y", "c"], &["a", "z", "c"]),
        (&[""], &["a", "b"]),
        (&["a", "a", "a"], &["a", "a"]),
    ];
    for (a, b) in cases {
        let rows = align_trimmed(&s(a), &s(b), usize::MAX).unwrap();
        // 每一行都剛好出現一次、順序不變，Same 的列兩邊內容相同
        let left: Vec<i32> = rows.iter().map(|r| r.left).filter(|&i| i >= 0).collect();
        let right: Vec<i32> = rows.iter().map(|r| r.right).filter(|&i| i >= 0).collect();
        assert_eq!(left, (0..a.len() as i32).collect::<Vec<_>>(), "{a:?} vs {b:?}");
        assert_eq!(right, (0..b.len() as i32).collect::<Vec<_>>(), "{a:?} vs {b:?}");
        for r in &rows {
            if r.row_type == RowType::Same {
                assert_eq!(a[r.left as usize], b[r.right as usize]);
            }
        }
        assert_eq!(change_count(&rows), change_count(&align(&s(a), &s(b))), "{a:?} vs {b:?}");
    }
}

#[test]
fn test_trimmed_alignment_handles_large_files_with_small_changes() {
    use alexcode::git::{align_trimmed, change_starts};
    let a: Vec<String> = (0..50_000).map(|i| format!("line {i}")).collect();
    let mut b = a.clone();
    b[25_000] = "changed".to_string();
    b.insert(25_300, "inserted".to_string());

    // 完整的 LCS 表格會是 50000 × 50001 格；修剪後中間只剩 300 × 301 格
    assert!(align_trimmed(&a, &b, 50_000).is_none(), "too large for the given budget");
    let rows = align_trimmed(&a, &b, 100_000).unwrap();
    assert_eq!(rows.len(), 50_001);
    assert_eq!(change_starts(&rows), vec![25_000, 25_300]);
    assert_eq!(rows[25_000].row_type, RowType::Modified);
    assert_eq!((rows[25_300].left, rows[25_300].right, rows[25_300].row_type), (-1, 25_300, RowType::Added));
    assert_eq!((rows[50_000].left, rows[50_000].right), (49_999, 50_000));

    // 完全相同的大檔案不需要建表
    let rows = align_trimmed(&a, &a, 0).unwrap();
    assert_eq!(change_count(&rows), 0);
    assert!(change_starts(&rows).is_empty());
}

#[test]
fn test_change_starts_marks_the_first_row_of_each_block() {
    use alexcode::git::change_starts;
    let rows = align(&s(&["a", "x", "y", "c", "d"]), &s(&["NEW", "a", "z", "c", "d", "tail"]));
    let types: Vec<RowType> = rows.iter().map(|r| r.row_type).collect();
    assert_eq!(
        types,
        [RowType::Added, RowType::Same, RowType::Modified, RowType::Removed, RowType::Same, RowType::Same, RowType::Added]
    );
    assert_eq!(change_starts(&rows), vec![0, 2, 6]);
}