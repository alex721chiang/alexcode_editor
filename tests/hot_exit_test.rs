use alexcode::core::{disk_signature, stamp_of, HotExit, UnsavedTab};

#[test]
fn hot_exit_round_trips_and_tolerates_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("hot_exit.json");
    assert!(HotExit::load(&path).is_none());

    let saved = HotExit {
        tabs: vec![
            UnsavedTab {
                path: Some(r"C:\work\a.rs".into()),
                text: "fn a() {}\n// 未存檔的中文\n\"quoted\" \\ back\tslash".into(),
                crlf: true,
                encoding: "Big5".into(),
                cursor_line: 1,
                cursor_col: 4,
                active: false,
                disk: Some((9, 1_700_000_000_123)),
            },
            UnsavedTab { text: "scratch".into(), encoding: "UTF-8".into(), active: true, ..Default::default() },
        ],
    };
    saved.save(&path).unwrap();
    assert_eq!(HotExit::load(&path).unwrap(), saved);
    assert!(!path.with_extension("json.tmp").exists(), "the temporary file is renamed into place");

    std::fs::write(&path, b"{ not json").unwrap();
    assert!(HotExit::load(&path).is_none());
    // 缺欄位（之後的版本增減欄位）：其餘欄位用預設值
    std::fs::write(&path, br#"{"tabs":[{"text":"x"}]}"#).unwrap();
    let old = HotExit::load(&path).unwrap();
    assert_eq!(old.tabs[0], UnsavedTab { text: "x".into(), ..Default::default() });
}

#[test]
fn saving_no_tabs_removes_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hot_exit.json");
    // 本來就沒有檔案也不算錯誤
    HotExit::default().save(&path).unwrap();
    assert!(!path.exists());

    HotExit { tabs: vec![UnsavedTab { text: "x".into(), ..Default::default() }] }.save(&path).unwrap();
    assert!(path.exists());
    HotExit::default().save(&path).unwrap();
    assert!(!path.exists(), "nothing unsaved is left, so nothing is kept on disk");
}

#[test]
fn disk_signature_changes_with_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.txt");
    assert_eq!(disk_signature(stamp_of(&path)), None, "a missing file has no signature");
    std::fs::write(&path, "one").unwrap();
    let first = disk_signature(stamp_of(&path)).unwrap();
    assert_eq!(first.0, 3);
    assert_eq!(disk_signature(stamp_of(&path)), Some(first), "unchanged file, same signature");
    std::fs::write(&path, "longer").unwrap();
    assert_ne!(disk_signature(stamp_of(&path)), Some(first));
}
