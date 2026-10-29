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
