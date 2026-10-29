use alexcode::search::command_match_score;

#[test]
fn test_empty_query_keeps_all() {
    assert!(command_match_score("Anything", "") >= 0);
}

#[test]
fn test_prefix_beats_contains() {
    let pre = command_match_score("Open File", "Open");
    let con = command_match_score("Recent Open", "Open");
    assert!(pre > 0);
    assert!(con > 0);
    assert!(pre > con);
}

#[test]
fn test_contains_beats_fuzzy() {
    let con = command_match_score("Toggle Comment", "Comment");
    let fz = command_match_score("Close Document", "Comment"); // 子序列 C-o-m...
    assert!(con > 0);
    if fz >= 0 {
        assert!(con > fz);
    }
}

#[test]
fn test_fuzzy_subsequence_matches() {
    assert!(command_match_score("Go to Symbol", "gts") >= 0); // g-t-s 子序列
}

#[test]
fn test_no_match_returns_negative() {
    assert!(command_match_score("Save File", "xyz") < 0);
}

#[test]
fn test_shorter_name_ranks_higher_for_same_prefix() {
    assert!(command_match_score("Save", "Sa") > command_match_score("Save All Files", "Sa"));
}
