use alexcode::ai::suggest_local_completions;

#[test]
fn test_prefix_matches_rank_above_fuzzy() {
    let text = "counter counter colorPick";
    let r = suggest_local_completions(text, 0, "co", 10);
    assert!(!r.is_empty());
    assert_eq!(r[0], "counter");
    assert!(r.contains(&"colorPick".to_string()));
    let idx_counter = r.iter().position(|x| x == "counter").unwrap();
    let idx_color = r.iter().position(|x| x == "colorPick").unwrap();
    assert!(idx_counter < idx_color);
}

#[test]
fn test_higher_frequency_ranks_higher() {
    let text = "apple apple apple apply";
    let r = suggest_local_completions(text, 0, "app", 10);
    let idx_apple = r.iter().position(|x| x == "apple").unwrap();
    let idx_apply = r.iter().position(|x| x == "apply").unwrap();
    assert!(idx_apple < idx_apply);
}

#[test]
fn test_closer_occurrence_ranks_higher() {
    let text = "farVar ............................................ nearVar";
    let cursor = text.len();
    let r = suggest_local_completions(text, cursor, "", 10);
    let idx_near = r.iter().position(|x| x == "nearVar").unwrap();
    let idx_far = r.iter().position(|x| x == "farVar").unwrap();
    assert!(idx_near < idx_far);
}

#[test]
fn test_deduplicates_identifiers() {
    let text = "value value value";
    let r = suggest_local_completions(text, 0, "val", 10);
    assert_eq!(r.iter().filter(|&x| x == "value").count(), 1);
}

#[test]
fn test_empty_prefix_returns_ranked_identifiers() {
    let r = suggest_local_completions("alpha beta beta gamma", 0, "", 10);
    assert!(!r.is_empty());
    assert_eq!(r[0], "beta"); // 出現兩次 → 最前
}

#[test]
fn test_excludes_exact_prefix_token() {
    let text = "len length lengthy";
    let r = suggest_local_completions(text, 0, "len", 10);
    assert!(!r.contains(&"len".to_string()));
    assert!(r.contains(&"length".to_string()));
    assert!(r.contains(&"lengthy".to_string()));
}

#[test]
fn test_respects_max_items() {
    let r = suggest_local_completions("aa1 aa2 aa3 aa4 aa5", 0, "aa", 3);
    assert_eq!(r.len(), 3);
}
