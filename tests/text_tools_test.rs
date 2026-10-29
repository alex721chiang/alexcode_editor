use alexcode::editor::{
    parse_integer, remove_blank_lines, remove_duplicate_lines, reverse_lines, sort_lines,
    to_full_width, to_half_width, trim_trailing_whitespace, unicode_escape, unicode_unescape,
};

#[test]
fn test_sort_lines_ascending_descending() {
    assert_eq!(sort_lines("b\na\nc", false), "a\nb\nc");
    assert_eq!(sort_lines("b\na\nc", true), "c\nb\na");
}

#[test]
fn test_remove_duplicate_and_blank_lines() {
    assert_eq!(remove_duplicate_lines("a\nb\na\nc\nb"), "a\nb\nc");
    assert_eq!(remove_blank_lines("a\n\n  \nb"), "a\nb");
}

#[test]
fn test_reverse_and_trim() {
    assert_eq!(reverse_lines("a\nb\nc"), "c\nb\na");
    assert_eq!(trim_trailing_whitespace("a  \nb\t\nc"), "a\nb\nc");
}

#[test]
fn test_full_half_width_round_trip() {
    let full = "ＡＢＣ１２３";
    assert_eq!(to_half_width(full), "ABC123");
    assert_eq!(to_full_width("ABC123"), full);
    assert_eq!(to_half_width("\u{3000}"), " ");
    assert_eq!(to_full_width(" "), "\u{3000}");
}

#[test]
fn test_unicode_escape_unescape() {
    let zh = "Hi 中文";
    let esc = unicode_escape(zh);
    assert!(esc.starts_with("Hi "));
    assert!(esc.contains(r"\u"));
    assert_eq!(unicode_unescape(&esc), zh);
    assert_eq!(unicode_escape("ascii"), "ascii");
}

#[test]
fn test_parse_integer() {
    assert_eq!(parse_integer("255"), Some(255));
    assert_eq!(parse_integer("0xFF"), Some(255));
    assert_eq!(parse_integer("0b1010"), Some(10));
    assert_eq!(parse_integer("0755"), Some(493)); // 八進位
    assert_eq!(parse_integer("notanumber"), None);
}
