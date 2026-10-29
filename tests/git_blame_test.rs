use alexcode::git::{parse_porcelain, status_text};

const SAMPLE: &[u8] = b"1234567890abcdef1234567890abcdef12345678 1 1 1\n\
author alex\n\
author-mail <a@b.c>\n\
author-time 1751980800\n\
author-tz -0700\n\
summary Fix the frobnicator\n\
filename foo.cpp\n\
\tint main() {\n\
0000000000000000000000000000000000000000 2 2 1\n\
author Not Committed Yet\n\
author-time 1751980800\n\
summary Version of foo.cpp from foo.cpp\n\
filename foo.cpp\n\
\t  return 0;\n";

#[test]
fn test_parses_sha_author_date_summary() {
    let map = parse_porcelain(SAMPLE);
    assert!(map.contains_key(&0));
    let l0 = &map[&0];
    assert_eq!(l0.short_sha, "12345678");
    assert_eq!(l0.author, "alex");
    assert_eq!(l0.summary, "Fix the frobnicator");
    assert!(!l0.uncommitted);
    assert!(!l0.date.is_empty());
}

#[test]
fn test_marks_uncommitted_lines() {
    let map = parse_porcelain(SAMPLE);
    assert!(map.contains_key(&1));
    assert!(map[&1].uncommitted);
}

#[test]
fn test_status_text_formats() {
    let map = parse_porcelain(SAMPLE);
    let s0 = status_text(&map[&0]);
    assert!(s0.contains("12345678"));
    assert!(s0.contains("alex"));
    assert!(s0.contains("Fix the frobnicator"));
    assert!(status_text(&map[&1]).contains("未提交"));
}

#[test]
fn test_empty_input_yields_empty_map() {
    assert!(parse_porcelain(b"").is_empty());
}
