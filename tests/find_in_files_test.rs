//! 全專案取代：預檢 (plan_replace) 與寫入 (apply_replace)

use alexcode::search::{apply_replace_in_files, plan_replace_in_files, search_in_files};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, bytes).unwrap();
    p
}

fn no_cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn name(path: &str) -> String {
    Path::new(path).file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn plan_counts_matches_per_file_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.rs", b"foo foo\nbar Foo\n");
    write(dir.path(), "sub/b.txt", b"x foo");
    write(dir.path(), "c.rs", b"nothing here");
    // 與搜尋相同：略過 .git / target / node_modules / build
    write(dir.path(), "target/gen.rs", b"foo");
    write(dir.path(), ".git/config", b"foo");

    let (hits, stats) = plan_replace_in_files(dir.path(), &[], "foo", &no_cancel());
    let got: Vec<(String, usize)> = hits.iter().map(|h| (name(&h.path), h.count)).collect();
    assert_eq!(got, [("a.rs".to_string(), 3), ("b.txt".to_string(), 1)], "sorted by path, case-insensitive");
    assert_eq!(stats.matches, 4);
    assert_eq!(stats.files_scanned, 3);
    assert!(!stats.canceled);
    assert_eq!(fs::read(&a).unwrap(), b"foo foo\nbar Foo\n", "the preview never writes");

    // 預覽涵蓋的檔案與搜尋結果相同（搜尋以「行」列出命中：a.rs 兩行、b.txt 一行）
    let (matches, _) = search_in_files(dir.path(), &[], "foo", &no_cancel(), 500);
    let mut files: Vec<String> = matches.iter().map(|m| name(&m.file_path)).collect();
    files.sort();
    assert_eq!(files, ["a.rs", "a.rs", "b.txt"]);

    // 檔名過濾
    let (hits, _) = plan_replace_in_files(dir.path(), &["*.txt".to_string()], "foo", &no_cancel());
    assert_eq!(hits.len(), 1);
    assert_eq!(name(&hits[0].path), "b.txt");

    // 空的搜尋字串不比對任何東西
    let (hits, stats) = plan_replace_in_files(dir.path(), &[], "", &no_cancel());
    assert!(hits.is_empty());
    assert_eq!(stats.matches, 0);
}

#[test]
fn apply_rewrites_only_the_matches_and_keeps_every_other_byte() {
    let dir = tempfile::tempdir().unwrap();
    // UTF-8 BOM + CRLF + 中文 + 結尾沒有換行
    let a = write(dir.path(), "a.txt", "\u{feff}foo 測試\r\nFOO\tend\r\nlast foo".as_bytes());
    let untouched = write(dir.path(), "other.txt", b"bar\r\n");
    let before = fs::metadata(&untouched).unwrap().modified().unwrap();

    let (hits, _) = plan_replace_in_files(dir.path(), &[], "foo", &no_cancel());
    let res = apply_replace_in_files(&hits, "foo", "qux", &no_cancel());
    assert_eq!((res.files_written, res.replacements), (1, 3));
    assert!(res.skipped.is_empty() && !res.canceled);
    assert_eq!(fs::read(&a).unwrap(), "\u{feff}qux 測試\r\nqux\tend\r\nlast qux".as_bytes());
    assert_eq!(fs::metadata(&untouched).unwrap().modified().unwrap(), before, "files without matches are not rewritten");
}

#[test]
fn query_and_replacement_are_literal_text() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.txt", b"f(x) = a.b; f(x)\nfxx = aXb");
    let (hits, _) = plan_replace_in_files(dir.path(), &[], "f(x)", &no_cancel());
    assert_eq!(hits[0].count, 2, "parentheses are not a regex group");
    let res = apply_replace_in_files(&hits, "f(x)", "$1\\n", &no_cancel());
    assert_eq!(res.replacements, 2);
    assert_eq!(fs::read_to_string(&a).unwrap(), "$1\\n = a.b; $1\\n\nfxx = aXb", "`$1` and `\\n` are inserted as written");

    let (hits, _) = plan_replace_in_files(dir.path(), &[], "a.b", &no_cancel());
    assert_eq!(hits[0].count, 1, "`.` only matches a dot");
    // 取代成空字串 = 刪除
    apply_replace_in_files(&hits, "a.b", "", &no_cancel());
    assert_eq!(fs::read_to_string(&a).unwrap(), "$1\\n = ; $1\\n\nfxx = aXb");
}

#[test]
fn files_that_are_not_utf8_text_are_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    // Big5「測試」後面接 foo：不是合法的 UTF-8
    let big5 = write(dir.path(), "big5.txt", &[0xB4, 0xFA, 0xB8, 0xD5, b' ', b'f', b'o', b'o']);
    // 含 NUL 的二進位檔
    let bin = write(dir.path(), "data.bin", b"foo\0foo");
    let ok = write(dir.path(), "ok.txt", b"foo");

    let (hits, _) = plan_replace_in_files(dir.path(), &[], "foo", &no_cancel());
    assert_eq!(hits.iter().map(|h| name(&h.path)).collect::<Vec<_>>(), ["ok.txt"]);
    apply_replace_in_files(&hits, "foo", "qux", &no_cancel());
    assert_eq!(fs::read(&big5).unwrap(), [0xB4, 0xFA, 0xB8, 0xD5, b' ', b'f', b'o', b'o']);
    assert_eq!(fs::read(&bin).unwrap(), b"foo\0foo");
    assert_eq!(fs::read(&ok).unwrap(), b"qux");
}

#[test]
fn apply_skips_files_that_changed_after_the_preview() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.txt", b"foo foo");
    let b = write(dir.path(), "b.txt", b"foo");
    let gone = write(dir.path(), "gone.txt", b"foo");
    let (hits, _) = plan_replace_in_files(dir.path(), &[], "foo", &no_cancel());
    assert_eq!(hits.len(), 3);

    // 預覽之後、執行之前：a.txt 被別的程式改了，gone.txt 被刪除
    fs::write(&a, b"foo foo and one more foo").unwrap();
    fs::remove_file(&gone).unwrap();
    let res = apply_replace_in_files(&hits, "foo", "qux", &no_cancel());
    assert_eq!((res.files_written, res.replacements), (1, 1));
    let skipped: Vec<String> = res.skipped.iter().map(|(p, _)| name(p)).collect();
    assert_eq!(skipped, ["a.txt", "gone.txt"]);
    assert_eq!(fs::read(&a).unwrap(), b"foo foo and one more foo", "a changed file is never written with a stale count");
    assert_eq!(fs::read(&b).unwrap(), b"qux");
    assert!(!gone.exists(), "a deleted file is not recreated");
}

#[test]
fn cancel_stops_before_touching_anything_else() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.txt", b"foo");
    let (hits, _) = plan_replace_in_files(dir.path(), &[], "foo", &no_cancel());
    let cancel = Arc::new(AtomicBool::new(true));
    let res = apply_replace_in_files(&hits, "foo", "qux", &cancel);
    assert!(res.canceled);
    assert_eq!(res.files_written, 0);
    assert_eq!(fs::read(&a).unwrap(), b"foo");

    let (hits, stats) = plan_replace_in_files(dir.path(), &[], "foo", &cancel);
    assert!(stats.canceled && hits.is_empty());
}
