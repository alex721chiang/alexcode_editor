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

/// 建立一個有一筆提交的暫存 Git 儲存庫；測試環境沒有 git 時回傳 None
fn repo_with_commit(file: &str, content: &str) -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git").current_dir(dir.path()).args(args).output().is_ok_and(|o| o.status.success())
    };
    if !git(&["init", "-q"]) {
        return None;
    }
    let path = dir.path().join(file);
    std::fs::write(&path, content).unwrap();
    assert!(git(&["add", "."]));
    assert!(git(&["-c", "user.name=Alice", "-c", "user.email=a@example.com", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "first commit"]));
    Some((dir, path))
}

#[test]
fn blame_contents_blames_the_text_it_is_given() {
    let Some((dir, path)) = repo_with_commit("f.txt", "one\ntwo\n") else { return };

    // 與提交的內容相同：每一行都屬於那筆提交
    let map = alexcode::git::blame_contents(&path, b"one\ntwo\n").expect("tracked file");
    assert_eq!(map.len(), 2);
    assert_eq!(map[&0].author, "Alice");
    assert_eq!(map[&1].summary, "first commit");
    assert!(!map[&0].uncommitted);

    // 尚未存檔的內容（磁碟上還是舊的）：新增的行標為未提交，原有的行往下移仍對得上
    let map = alexcode::git::blame_contents(&path, b"inserted\none\ntwo\n").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"one\ntwo\n", "blame never touches the file");
    assert!(map[&0].uncommitted);
    assert_eq!((map[&1].author.as_str(), map[&1].uncommitted), ("Alice", false));
    assert_eq!(map[&2].summary, "first commit");

    // 未追蹤的檔案、不在儲存庫內的檔案：沒有 blame
    let untracked = dir.path().join("new.txt");
    std::fs::write(&untracked, "x").unwrap();
    assert!(alexcode::git::blame_contents(&untracked, b"x").is_none());
    let outside = tempfile::tempdir().unwrap();
    let lone = outside.path().join("lone.txt");
    std::fs::write(&lone, "x").unwrap();
    assert!(alexcode::git::blame_contents(&lone, b"x").is_none());
}