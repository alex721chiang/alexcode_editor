use alexcode::search::{find_all, replace_all, replace_range, FindOptions, Finder};

const PLAIN: FindOptions = FindOptions { case_sensitive: false, whole_word: false, regex: false };

#[test]
fn default_search_is_case_insensitive_literal() {
    assert_eq!(find_all("Error error ERROR", "error", &PLAIN).unwrap().len(), 3);
    // 一般模式下正規表示式的特殊字元只是普通字元
    assert_eq!(find_all("a.b axb", "a.b", &PLAIN).unwrap(), vec![(0, 3)]);
    assert!(find_all("abc", "", &PLAIN).unwrap().is_empty());
}

#[test]
fn case_and_whole_word_options() {
    let cs = FindOptions { case_sensitive: true, ..PLAIN };
    assert_eq!(find_all("Error error", "error", &cs).unwrap(), vec![(6, 11)]);
    let ww = FindOptions { whole_word: true, ..PLAIN };
    assert_eq!(find_all("cat concat cat_x cat", "cat", &ww).unwrap(), vec![(0, 3), (17, 20)]);
}

#[test]
fn regex_search_and_invalid_pattern() {
    let re = FindOptions { regex: true, ..PLAIN };
    assert_eq!(find_all("x=1 y=22", r"\w=(\d+)", &re).unwrap(), vec![(0, 3), (4, 8)]);
    assert!(find_all("abc", "(", &re).is_err());
    // 可比對空字串的樣式不會產生無限多個空命中
    assert!(find_all("abc", "x*", &re).unwrap().is_empty());
}

#[test]
fn finder_matches_like_find_all_and_never_fails() {
    let text = "Cat concat cat_x cat";
    for opts in [
        PLAIN,
        FindOptions { case_sensitive: true, ..PLAIN },
        FindOptions { whole_word: true, ..PLAIN },
        FindOptions { regex: true, ..PLAIN },
    ] {
        let f = Finder::new("cat", &opts);
        assert_eq!(f.find_all(text), find_all(text, "cat", &opts).unwrap(), "{opts:?}");
    }
    let cs = Finder::new("cat", &FindOptions { case_sensitive: true, ..PLAIN });
    assert!(cs.is_match("a cat") && !cs.is_match("A Cat"));
    let ww = Finder::new("cat", &FindOptions { whole_word: true, ..PLAIN });
    assert!(ww.is_match("the Cat sat") && !ww.is_match("concat"));
    let re = FindOptions { regex: true, ..PLAIN };
    assert!(Finder::new(r"c[ao]t\b", &re).is_match("a cot") && !Finder::new("a.b", &PLAIN).is_match("axb"));

    // 空的查詢、不合法的正規表示式、只會比對到空字串的樣式：都不算命中，也不會出錯
    for f in [Finder::new("", &PLAIN), Finder::new("(", &re), Finder::new("x*", &re), Finder::default()] {
        assert!(!f.is_match("abc") && f.find_all("abc").is_empty());
    }
}

#[test]
fn replace_all_counts_and_keeps_replacement_literal() {
    let (out, n) = replace_all("a-b-c", "-", "$1", &PLAIN).unwrap();
    assert_eq!((out.as_str(), n), ("a$1b$1c", 2));
    let re = FindOptions { regex: true, ..PLAIN };
    let (out, n) = replace_all("x=1 y=22", r"(\w)=(\d+)", "$2=$1", &re).unwrap();
    assert_eq!((out.as_str(), n), ("1=x 22=y", 2));
}

#[test]
fn replace_single_match_only() {
    let hits = find_all("foo foo foo", "foo", &PLAIN).unwrap();
    let (out, end) = replace_range("foo foo foo", hits[1], "foo", "bar!", &PLAIN).unwrap();
    assert_eq!(out, "foo bar! foo");
    assert_eq!(end, 8);
    let re = FindOptions { regex: true, ..PLAIN };
    let hits = find_all("k1 k2", r"k(\d)", &re).unwrap();
    let (out, _) = replace_range("k1 k2", hits[1], r"k(\d)", "v$1", &re).unwrap();
    assert_eq!(out, "k1 v2");
}

#[test]
fn works_with_multibyte_text() {
    let hits = find_all("測試 錯誤 測試", "測試", &PLAIN).unwrap();
    assert_eq!(hits.len(), 2);
    let (out, n) = replace_all("測試 錯誤 測試", "測試", "通過", &PLAIN).unwrap();
    assert_eq!((out.as_str(), n), ("通過 錯誤 通過", 2));
}
