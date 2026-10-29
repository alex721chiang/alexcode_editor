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
