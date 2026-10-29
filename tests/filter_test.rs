use alexcode::filter::{FilterEngine, FilterLogic};

#[test]
fn test_or_logic() {
    let mut e = FilterEngine::new();
    e.set_keywords(vec!["error".to_string(), "warn".to_string()]);
    e.set_logic(FilterLogic::Or);
    assert!(e.match_line("an ERROR occurred"));
    assert!(e.match_line("just a warning"));
    assert!(!e.match_line("all good"));
}

#[test]
fn test_and_logic() {
    let mut e = FilterEngine::new();
    e.set_keywords(vec!["error".to_string(), "timeout".to_string()]);
    e.set_logic(FilterLogic::And);
    assert!(e.match_line("error: connection timeout"));
    assert!(!e.match_line("error: refused"));
}

#[test]
fn test_empty_keywords_matches_all() {
    let e = FilterEngine::new();
    assert!(e.match_line("anything"));
}

#[test]
fn test_parse_query_double_pipe() {
    let kws = FilterEngine::parse_query("error || timeout ||  fail ");
    assert_eq!(kws, vec!["error", "timeout", "fail"]);
}

#[test]
fn test_parse_query_legacy_single_pipe() {
    let kws = FilterEngine::parse_query("a|b|c");
    assert_eq!(kws, vec!["a", "b", "c"]);
}

#[test]
fn test_fuzzy_match() {
    assert!(FilterEngine::fuzzy_contains("MainWindow.cpp", "mwin"));
    assert!(FilterEngine::fuzzy_contains("FilterEngine", "ftre"));
    assert!(!FilterEngine::fuzzy_contains("abc", "abd"));

    let mut e = FilterEngine::new();
    e.set_keywords(vec!["mwin".to_string()]);
    e.set_fuzzy(true);
    assert!(e.match_line("class MainWindow : public QMainWindow"));
    e.set_fuzzy(false);
    assert!(!e.match_line("class MainWindow : public QMainWindow"));
}

#[test]
fn test_advanced_syntax() {
    let mut e = FilterEngine::new();
    e.compile("error && !heartbeat");
    assert!(e.match_compiled("ERROR: payment failed"));
    assert!(!e.match_compiled("error heartbeat check"));
    assert!(!e.match_compiled("all good"));

    e.compile("error && timeout || fatal");
    assert!(e.match_compiled("error: timeout on db"));
    assert!(!e.match_compiled("error: refused"));
    assert!(e.match_compiled("FATAL crash"));

    e.compile("a||b"); // 舊式仍可用
    assert!(e.match_compiled("has b"));

    e.compile("error && !heartbeat");
    assert_eq!(e.positive_keywords(), vec!["error".to_string()]);
}

#[test]
fn test_regex_prefix() {
    let mut e = FilterEngine::new();
    e.compile("re:err(or)?s?");
    assert!(e.match_compiled("ERRORS detected"));
    assert!(e.match_compiled("an err in line 3"));
    assert!(!e.match_compiled("all good"));

    e.compile(r"re:timeout=\d{3,} && !debug"); // regex 與排除組合
    assert!(e.match_compiled("conn timeout=1500ms"));
    assert!(!e.match_compiled("conn timeout=15ms"));
    assert!(!e.match_compiled("debug: timeout=1500"));

    e.compile(r"re:^\[WARN\] || fatal"); // regex OR 字面詞
    assert!(e.match_compiled("[WARN] disk almost full"));
    assert!(e.match_compiled("FATAL: oom"));
    assert!(!e.match_compiled("note: [WARN] quoted mid-line"));

    e.compile("re:err.* && info");
    assert_eq!(e.positive_keywords(), vec!["info".to_string()]); // regex 詞不入多色標示

    e.compile("re:[invalid("); // 無效 regex：退回字面比對不崩潰
    assert!(!e.match_compiled("anything"));
    assert!(e.match_compiled("re:[invalid("));
}
