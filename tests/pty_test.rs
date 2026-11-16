//! 互動式終端機：shell 在虛擬終端（ConPTY / PTY）裡執行，輸出一讀到就送出來（不等指令結束）

use std::time::{Duration, Instant};

use alexcode::terminal::{available_shells, default_shell, PtySession, VtParser};

/// 終端機畫面上的所有文字（捲出畫面的歷史 + 目前畫面）
fn screen(vt: &VtParser) -> String {
    let mut s: Vec<String> = (0..vt.scrollback_count()).map(|i| vt.scrollback_line(i)).collect();
    s.extend((0..vt.rows()).map(|r| vt.line_text(r)));
    s.join("\n")
}

/// 一直讀 shell 的輸出並餵給 `vt`（終端機要回覆的查詢也一併回覆），直到 `done` 成立或逾時
fn pump(s: &mut PtySession, vt: &mut VtParser, timeout: Duration, done: impl Fn(&str) -> bool) -> bool {
    let end = Instant::now() + timeout;
    while Instant::now() < end {
        let data = s.read_available();
        if !data.is_empty() {
            vt.feed(&data);
            let reply = vt.take_responses();
            if !reply.is_empty() {
                s.write(&reply).unwrap();
            }
        }
        if done(&screen(vt)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn shell_output_streams_while_a_command_is_still_running() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = PtySession::spawn(&default_shell(), &[], dir.path(), 40, 120, || {}).expect("the shell starts in a pseudo terminal");
    let mut vt = VtParser::new(40, 120);

    // 兩個算出來的數字中間隔 2 秒：指令本身（會被 shell 回顯）裡沒有這兩個數字，
    // 所以畫面上出現 1234 時，表示指令還在跑、輸出已經送出來了
    let cmd: &[u8] = if cfg!(windows) {
        b"set /a 1000+234 & ping -n 3 127.0.0.1 >nul & set /a 5000+678\r"
    } else {
        b"echo $((1000+234)); sleep 2; echo $((5000+678))\r"
    };
    // 等 shell 準備好（出現提示字元之類的任何輸出）再送指令
    assert!(pump(&mut s, &mut vt, Duration::from_secs(20), |t| !t.trim().is_empty()), "the shell printed nothing");
    s.write(cmd).unwrap();
    let t0 = Instant::now();
    assert!(pump(&mut s, &mut vt, Duration::from_secs(20), |t| t.contains("1234")), "first result never arrived:\n{}", screen(&vt));
    let first = t0.elapsed();
    assert!(!screen(&vt).contains("5678"), "the first result arrived before the command finished");
    assert!(pump(&mut s, &mut vt, Duration::from_secs(20), |t| t.contains("5678")), "second result never arrived:\n{}", screen(&vt));
    let second = t0.elapsed();
    assert!(second - first >= Duration::from_millis(900), "the two results came {:?} apart", second - first);
    assert!(s.exit_code().is_none(), "the shell is still running");

    // 離開 shell：收到結束代碼
    s.write(b"exit\r").unwrap();
    let end = Instant::now() + Duration::from_secs(15);
    while s.exit_code().is_none() && Instant::now() < end {
        s.read_available();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(s.exit_code(), Some(0), "the shell exits with code 0");
}

#[test]
fn every_offered_shell_runs_commands() {
    let shells = available_shells();
    println!("shells: {:?}", shells.iter().map(|s| (&s.name, &s.program)).collect::<Vec<_>>());
    assert!(!shells.is_empty());
    assert_eq!(shells[0].program, default_shell(), "the default shell comes first");
    if cfg!(windows) {
        assert_eq!(shells[0].name, "cmd");
        assert!(shells.iter().any(|s| s.name == "PowerShell"), "Windows always has PowerShell");
    }
    for sh in &shells {
        let dir = tempfile::tempdir().unwrap();
        let mut s = PtySession::spawn(&sh.program, &sh.args, dir.path(), 40, 120, || {})
            .unwrap_or_else(|e| panic!("{} does not start: {e}", sh.name));
        let mut vt = VtParser::new(40, 120);
        // 算出來的數字不在指令本身裡（指令會被回顯）
        let cmd: &[u8] = match sh.name.as_str() {
            "cmd" => b"set /a 4000+321\r",
            "PowerShell" => b"4000+321\r",
            _ => b"echo $((4000+321))\r",
        };
        assert!(pump(&mut s, &mut vt, Duration::from_secs(30), |t| !t.trim().is_empty()), "{} printed nothing", sh.name);
        s.write(cmd).unwrap();
        assert!(
            pump(&mut s, &mut vt, Duration::from_secs(30), |t| t.contains("4321")),
            "{} did not run the command:\n{}",
            sh.name,
            screen(&vt)
        );
    }
}
