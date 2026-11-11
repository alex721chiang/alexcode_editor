use alexcode::core::single_instance::{acquire, Instance};
use alexcode::core::{list_dir, stamp_of, AppState, FileWatcher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn app_state_round_trips_and_tolerates_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("state.json");
    assert!(AppState::load(&path).is_none());

    let mut st = AppState { theme: "Matrix".into(), font_size: 16.0, ..Default::default() };
    st.open_files = vec!["a.rs".into(), "b.md".into()];
    st.save(&path).unwrap();
    assert_eq!(AppState::load(&path).unwrap(), st);

    std::fs::write(&path, b"{ not json").unwrap();
    assert!(AppState::load(&path).is_none());
    // 舊版設定檔缺欄位：其餘欄位用預設值
    std::fs::write(&path, br#"{"theme":"Neon HUD"}"#).unwrap();
    assert_eq!(AppState::load(&path).unwrap().theme, "Neon HUD");
}

#[test]
fn recent_files_are_deduplicated_and_capped() {
    let mut st = AppState::default();
    for i in 0..20 {
        st.push_recent(&format!("f{i}"));
    }
    st.push_recent("f10");
    assert_eq!(st.recent_files.len(), 15);
    assert_eq!(st.recent_files[0], "f10");
    assert_eq!(st.recent_files.iter().filter(|p| *p == "f10").count(), 1);
}

#[test]
fn watcher_reports_external_modification_and_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("log.txt");
    std::fs::write(&path, "one").unwrap();
    let w = FileWatcher::new(Duration::from_millis(30), || {});
    w.set_paths(vec![path.clone()]);
    std::thread::sleep(Duration::from_millis(150));
    assert!(w.poll().is_empty(), "an unchanged file is not reported");

    std::fs::write(&path, "one two three").unwrap();
    let wait = |w: &FileWatcher| {
        let t = Instant::now();
        loop {
            let ev = w.poll();
            if !ev.is_empty() || t.elapsed() > Duration::from_secs(5) {
                return ev;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let ev = wait(&w);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].0, path);
    assert_eq!(ev[0].1, stamp_of(&path));

    std::fs::remove_file(&path).unwrap();
    let ev = wait(&w);
    assert_eq!(ev, vec![(path, None)]);
}

#[test]
fn second_instance_forwards_files_to_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let Instance::Primary(rx, guard) = acquire(dir.path(), &[], || {}).unwrap() else {
        panic!("first launch must become the primary instance");
    };
    let files = vec![PathBuf::from("/tmp/a b.txt"), PathBuf::from("/tmp/中文.md")];
    assert!(matches!(acquire(dir.path(), &files, || {}).unwrap(), Instance::Forwarded));
    assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), files);

    // 沒有檔案的第二次啟動也會轉交（讓現有視窗跳到前景）
    assert!(matches!(acquire(dir.path(), &[], || {}).unwrap(), Instance::Forwarded));
    assert!(rx.recv_timeout(Duration::from_secs(3)).unwrap().is_empty());

    // 第一個實例結束後，下一次啟動成為新的第一個實例
    drop(guard);
    assert!(!dir.path().join("instance.json").exists());
    assert!(matches!(acquire(dir.path(), &[], || {}).unwrap(), Instance::Primary(..)));
}

#[test]
fn stale_or_forged_endpoint_is_not_trusted() {
    let dir = tempfile::tempdir().unwrap();
    // 上次異常結束留下的檔案：埠上沒有人在聽
    std::fs::write(dir.path().join("instance.json"), br#"{"port":1,"key":"x"}"#).unwrap();
    let Instance::Primary(rx, _guard) = acquire(dir.path(), &[], || {}).unwrap() else {
        panic!("stale endpoint must not block startup");
    };
    // 金鑰不符的連線不會被當成開檔要求
    let ep: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.path().join("instance.json")).unwrap()).unwrap();
    let port = ep["port"].as_u64().unwrap() as u16;
    use std::io::Write;
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(b"wrong-key\n/etc/passwd\n\n").unwrap();
    assert!(rx.recv_timeout(Duration::from_millis(400)).is_err());
}

#[test]
fn directories_are_listed_first_and_sorted() {
    let dir = tempfile::tempdir().unwrap();
    for f in ["b.txt", "A.txt", "c.rs"] {
        std::fs::write(dir.path().join(f), "").unwrap();
    }
    std::fs::create_dir(dir.path().join("zdir")).unwrap();
    std::fs::create_dir(dir.path().join("Adir")).unwrap();
    let names: Vec<String> = list_dir(dir.path()).unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["Adir", "zdir", "A.txt", "b.txt", "c.rs"]);
}
