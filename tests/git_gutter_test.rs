use alexcode::git::{diff_line_states, ADDED, DELETED_ABOVE, MODIFIED};

fn l(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|&s| s.to_string()).collect()
}

#[test]
fn test_no_changes_returns_empty() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a", "b", "c"]), 3000);
    assert!(s.is_empty());
}

#[test]
fn test_pure_addition() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a", "b", "x", "y", "c"]), 3000);
    assert_eq!(s.get(&2), Some(&ADDED));
    assert_eq!(s.get(&3), Some(&ADDED));
    assert_eq!(s.len(), 2);
}

#[test]
fn test_pure_deletion_marks_line_below() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a", "c"]), 3000);
    assert_eq!(s.get(&1), Some(&DELETED_ABOVE));
    assert_eq!(s.len(), 1);
}

#[test]
fn test_deletion_at_end_marks_last_line() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a"]), 3000);
    assert_eq!(s.get(&0).copied().unwrap_or(0) & DELETED_ABOVE, DELETED_ABOVE);
    assert_eq!(s.len(), 1);
}

#[test]
fn test_modification_pairs_old_and_new() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a", "B!", "c"]), 3000);
    assert_eq!(s.get(&1), Some(&MODIFIED));
    assert_eq!(s.len(), 1);
}

#[test]
fn test_replace_one_line_with_two_is_modified_plus_added() {
    let s = diff_line_states(&l(&["a", "b", "c"]), &l(&["a", "x", "y", "c"]), 3000);
    assert_eq!(s.get(&1), Some(&MODIFIED));
    assert_eq!(s.get(&2), Some(&ADDED));
    assert_eq!(s.len(), 2);
}

#[test]
fn test_replace_two_lines_with_one_is_modified_plus_deleted_above() {
    let s = diff_line_states(&l(&["a", "b", "c", "d"]), &l(&["a", "x", "d"]), 3000);
    assert_eq!(s.get(&1).copied().unwrap_or(0) & MODIFIED, MODIFIED);
    assert_eq!(s.get(&2).copied().unwrap_or(0) & DELETED_ABOVE, DELETED_ABOVE);
}

#[test]
fn test_multiple_separated_change_runs() {
    let s = diff_line_states(
        &l(&["a", "b", "c", "d", "e"]),
        &l(&["a", "B", "c", "d", "E"]),
        3000,
    );
    assert_eq!(s.get(&1), Some(&MODIFIED));
    assert_eq!(s.get(&4), Some(&MODIFIED));
    assert_eq!(s.len(), 2);
}

#[test]
fn test_empty_old_means_all_added() {
    let s = diff_line_states(&[], &l(&["a", "b"]), 3000);
    assert_eq!(s.get(&0), Some(&ADDED));
    assert_eq!(s.get(&1), Some(&ADDED));
}

#[test]
fn test_oversized_middle_falls_back_to_coarse_modified() {
    let mut old_l = Vec::new();
    let mut new_l = Vec::new();
    for i in 0..50 {
        old_l.push(format!("old{}", i));
        new_l.push(format!("new{}", i));
    }
    let s = diff_line_states(&old_l, &new_l, 10);
    assert_eq!(s.len(), 50);
    for i in 0..50 {
        assert_eq!(s.get(&i).copied().unwrap_or(0) & MODIFIED, MODIFIED);
    }
}

#[test]
fn test_prefix_suffix_trim_keeps_line_numbers_correct() {
    let mut old_l = Vec::new();
    let mut new_l = Vec::new();
    for i in 0..1000 {
        old_l.push(i.to_string());
        new_l.push(i.to_string());
    }
    new_l[500] = "changed".to_string();
    let s = diff_line_states(&old_l, &new_l, 3000);
    assert_eq!(s.len(), 1);
    assert_eq!(s.get(&500), Some(&MODIFIED));
}

#[test]
fn test_oversized_middle_with_few_changes_is_still_precise() {
    // 2000 行、上限 100：開頭插入一行、中間改一行、後面刪掉兩行。差異的範圍幾乎涵蓋整個檔案，
    // 但真正不同的只有這三處
    let old_l: Vec<String> = (0..2000).map(|i| format!("line {i}")).collect();
    let mut new_l = old_l.clone();
    new_l.drain(1500..1502);
    new_l[1000] = "changed".into();
    new_l.insert(0, "inserted".into());
    let mut marks: Vec<(usize, u8)> = diff_line_states(&old_l, &new_l, 100).into_iter().collect();
    marks.sort();
    assert_eq!(marks, vec![(0, ADDED), (1001, MODIFIED), (1501, DELETED_ABOVE)]);

    // 重複的行（空行）很多時也一樣
    let old_l: Vec<String> = (0..3000).map(|i| if i % 3 == 0 { String::new() } else { format!("row {}", i / 3) }).collect();
    let mut new_l = old_l.clone();
    new_l.insert(10, "extra".into());
    new_l.push("tail".into());
    let mut marks: Vec<(usize, u8)> = diff_line_states(&old_l, &new_l, 100).into_iter().collect();
    marks.sort();
    assert_eq!(marks, vec![(10, ADDED), (3001, ADDED)]);
}

#[test]
fn test_oversized_middle_with_too_many_changes_is_marked_coarsely() {
    let old_l: Vec<String> = (0..5000).map(|i| format!("old {i}")).collect();
    let new_l: Vec<String> = (0..5000).map(|i| format!("new {i}")).collect();
    let s = diff_line_states(&old_l, &new_l, 100);
    assert_eq!(s.len(), 5000);
    assert!(s.values().all(|&st| st == MODIFIED));
}
