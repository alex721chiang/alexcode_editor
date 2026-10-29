use alexcode::terminal::VtParser;

#[test]
fn test_plain_text_and_cursor() {
    let mut p = VtParser::new(5, 20);
    p.feed(b"hello");
    assert_eq!(p.line_text(0), "hello");
    assert_eq!(p.cursor_row(), 0);
    assert_eq!(p.cursor_col(), 5);
}

#[test]
fn test_crlf_move_cursor() {
    let mut p = VtParser::new(5, 20);
    p.feed(b"ab\r\ncd");
    assert_eq!(p.line_text(0), "ab");
    assert_eq!(p.line_text(1), "cd");
    assert_eq!(p.cursor_row(), 1);
    assert_eq!(p.cursor_col(), 2);
}

#[test]
fn test_backspace_and_overwrite() {
    let mut p = VtParser::new(3, 20);
    p.feed(b"abc\x08\x08X"); // 退兩格後覆寫 b → "aXc"
    assert_eq!(p.line_text(0), "aXc");
}

#[test]
fn test_tab_advances_to_multiple_of_8() {
    let mut p = VtParser::new(3, 40);
    p.feed(b"a\tb");
    assert_eq!(p.cell_at(0, 0).ch, 'a');
    assert_eq!(p.cell_at(0, 8).ch, 'b');
}

#[test]
fn test_auto_wrap_at_eol() {
    let mut p = VtParser::new(3, 4);
    p.feed(b"abcde"); // 4 欄滿後換行
    assert_eq!(p.line_text(0), "abcd");
    assert_eq!(p.line_text(1), "e");
    assert_eq!(p.cursor_row(), 1);
}

#[test]
fn test_scroll_up_pushes_scrollback() {
    let mut p = VtParser::new(2, 10);
    p.feed(b"L0\r\nL1\r\nL2"); // 第三行使第一行捲離
    assert_eq!(p.scrollback_count(), 1);
    assert_eq!(p.scrollback_line(0), "L0");
    assert_eq!(p.line_text(0), "L1");
    assert_eq!(p.line_text(1), "L2");
}

#[test]
fn test_csi_cursor_position() {
    let mut p = VtParser::new(10, 20);
    p.feed(b"\x1b[3;5HX"); // CUP 第3列第5欄（1-based）
    assert_eq!(p.cell_at(2, 4).ch, 'X');
}

#[test]
fn test_csi_cursor_moves() {
    let mut p = VtParser::new(10, 20);
    p.feed(b"\x1b[5;5H"); // 移到 (4,4)
    p.feed(b"\x1b[2A");   // 上 2 → row 2
    p.feed(b"\x1b[3C");   // 右 3 → col 7
    assert_eq!(p.cursor_row(), 2);
    assert_eq!(p.cursor_col(), 7);
}

#[test]
fn test_erase_line_and_screen() {
    let mut p = VtParser::new(3, 10);
    p.feed(b"hello\r\nworld");
    p.feed(b"\x1b[H");  // 回左上
    p.feed(b"\x1b[2J"); // 清全螢幕
    assert_eq!(p.line_text(0), "");
    assert_eq!(p.line_text(1), "");
}

#[test]
fn test_erase_to_end_of_line() {
    let mut p = VtParser::new(3, 10);
    p.feed(b"abcdef");
    p.feed(b"\x1b[3G"); // 移到第 3 欄（col 2）
    p.feed(b"\x1b[K");  // 清到行尾
    assert_eq!(p.line_text(0), "ab");
}

#[test]
fn test_sgr_color_and_bold() {
    let mut p = VtParser::new(3, 20);
    p.feed(b"\x1b[1;31mR\x1b[0mN"); // 紅色粗體 R，重置後 N
    let r = p.cell_at(0, 0);
    assert_eq!(r.ch, 'R');
    assert_eq!(r.fg, 1);
    assert!(r.bold);
    let n = p.cell_at(0, 1);
    assert_eq!(n.fg, -1);
    assert!(!n.bold);
}

#[test]
fn test_bright_foreground_and_background() {
    let mut p = VtParser::new(3, 20);
    p.feed(b"\x1b[92;44mX"); // 亮綠前景(8+2=10)、藍背景(4)
    let x = p.cell_at(0, 0);
    assert_eq!(x.fg, 10);
    assert_eq!(x.bg, 4);
}

#[test]
fn test_split_escape_across_feeds() {
    let mut p = VtParser::new(3, 20);
    p.feed(b"\x1b["); // 序列被切斷
    p.feed(b"31m");
    p.feed(b"Z");
    assert_eq!(p.cell_at(0, 0).ch, 'Z');
    assert_eq!(p.cell_at(0, 0).fg, 1); // 紅，證明跨 feed 狀態保留
}

#[test]
fn test_ignores_unknown_and_osc_sequences() {
    let mut p = VtParser::new(3, 20);
    p.feed(b"\x1b]0;window title\x07OK"); // OSC 設定標題，應被吞掉
    assert_eq!(p.line_text(0), "OK");
    p.feed(b"\r\n\x1b[?25lY"); // 私有模式 ?25l 應略過
    assert_eq!(p.line_text(1), "Y");
}

#[test]
fn test_utf8_multibyte_cjk() {
    let mut p = VtParser::new(3, 20);
    p.feed("a中文b".as_bytes());
    assert_eq!(p.line_text(0), "a中文b");
}

#[test]
fn test_utf8_split_across_feeds() {
    let mut p = VtParser::new(3, 20);
    let u = "中".as_bytes(); // [0xe4, 0xb8, 0xad]
    p.feed(&u[0..2]); // 前 2 位元組（不完整）
    p.feed(&u[2..]);  // 第 3 位元組補齊
    assert_eq!(p.line_text(0), "中");
}

#[test]
fn test_utf8_four_byte_emoji() {
    let mut p = VtParser::new(3, 20);
    p.feed("x😀".as_bytes());
    assert_eq!(p.line_text(0), "x😀");
}

#[test]
fn test_utf8_newline_does_not_corrupt_next() {
    let mut p = VtParser::new(3, 20);
    p.feed("中\r\n文".as_bytes());
    assert_eq!(p.line_text(0), "中");
    assert_eq!(p.line_text(1), "文");
}
