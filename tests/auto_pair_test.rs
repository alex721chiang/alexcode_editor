use alexcode::editor::{closing_for, decide, should_delete_pair, Action};

#[test]
fn test_closing_for_map() {
    assert_eq!(closing_for('('), Some(')'));
    assert_eq!(closing_for('['), Some(']'));
    assert_eq!(closing_for('{'), Some('}'));
    assert_eq!(closing_for('"'), Some('"'));
    assert_eq!(closing_for('x'), None);
}

#[test]
fn test_open_bracket_auto_closes() {
    let d = decide('(', Some(' '), None, false);
    assert_eq!(d.action, Action::AutoClose);
    assert_eq!(d.open, Some('('));
    assert_eq!(d.close, Some(')'));
}

#[test]
fn test_open_bracket_before_word_does_not_auto_close() {
    let d = decide('(', Some(' '), Some('x'), false);
    assert_eq!(d.action, Action::Insert);
}

#[test]
fn test_closer_skips_over_matching() {
    let d = decide(')', None, Some(')'), false);
    assert_eq!(d.action, Action::SkipOver);
}

#[test]
fn test_closer_inserts_when_no_match_ahead() {
    let d = decide(')', None, Some('x'), false);
    assert_eq!(d.action, Action::Insert);
}

#[test]
fn test_quote_auto_closes_in_empty_context() {
    let d = decide('"', Some(' '), None, false);
    assert_eq!(d.action, Action::AutoClose);
    assert_eq!(d.close, Some('"'));
}

#[test]
fn test_quote_skips_over_matching() {
    let d = decide('"', None, Some('"'), false);
    assert_eq!(d.action, Action::SkipOver);
}

#[test]
fn test_quote_in_word_does_not_auto_close() {
    // apostrophe inside "don't"
    let d = decide('\'', Some('n'), Some('t'), false);
    assert_eq!(d.action, Action::Insert);
}

#[test]
fn test_surround_on_selection() {
    let d = decide('(', None, None, true);
    assert_eq!(d.action, Action::Surround);
    assert_eq!(d.open, Some('('));
    assert_eq!(d.close, Some(')'));
}

#[test]
fn test_surround_with_quote() {
    let d = decide('"', None, None, true);
    assert_eq!(d.action, Action::Surround);
    assert_eq!(d.open, Some('"'));
    assert_eq!(d.close, Some('"'));
}

#[test]
fn test_non_pair_char_inserts() {
    assert_eq!(decide('a', None, None, false).action, Action::Insert);
}

#[test]
fn test_delete_pair_between_empty_pair() {
    assert!(should_delete_pair(Some('('), Some(')')));
    assert!(should_delete_pair(Some('"'), Some('"')));
    assert!(!should_delete_pair(Some('('), Some(']')));
    assert!(!should_delete_pair(Some('x'), Some('y')));
}
