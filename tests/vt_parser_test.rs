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

#[test]
fn test_cursor_position_report_is_answered() {
    // ConPTY 啟動時以 ESC[6n 詢問游標位置，要等到回覆才繼續輸出
    let mut p = VtParser::new(5, 20);
    assert!(p.take_responses().is_empty());
    p.feed(b"ab\r\ncde\x1b[6n");
    assert_eq!(p.take_responses(), b"\x1b[2;4R".to_vec());
    assert!(p.take_responses().is_empty(), "each reply is handed out once");
    // ESC[5n（狀態查詢）回覆「正常」；其他私有模式的查詢不回覆
    p.feed(b"\x1b[5n\x1b[?6n");
    assert_eq!(p.take_responses(), b"\x1b[0n".to_vec());
    assert_eq!(p.line_text(1), "cde", "queries print nothing");
}

#[test]
fn test_erase_characters_and_partial_screen_erase() {
    // ConPTY 重畫畫面時常用的 ECH（ESC[nX：從游標起清掉 n 個字，游標不動）與 ED 0 / 1
    let mut p = VtParser::new(3, 10);
    p.feed(b"abcdefgh\x1b[1;3H\x1b[2X");
    assert_eq!(p.line_text(0), "ab  efgh");
    assert_eq!((p.cursor_row(), p.cursor_col()), (0, 2));
    p.feed(b"\x1b[2;1Hline2\x1b[3;1Hline3\x1b[2;3H\x1b[J");
    assert_eq!((p.line_text(0).as_str(), p.line_text(1).as_str(), p.line_text(2).as_str()), ("ab  efgh", "li", ""));
    p.feed(b"\x1b[1;4H\x1b[1J");
    assert_eq!(p.line_text(0), "    efgh");
}

#[test]
fn test_resize_keeps_the_cursor_on_screen() {
    // 列變少：游標以上多出來的列推進歷史，游標所在的那一列留在畫面上
    let mut p = VtParser::new(5, 10);
    p.feed(b"a\r\nb\r\nc\r\nd\r\ne");
    p.resize(3, 10);
    assert_eq!((p.rows(), p.cols()), (3, 10));
    assert_eq!((p.scrollback_count(), p.scrollback_line(0).as_str(), p.scrollback_line(1).as_str()), (2, "a", "b"));
    assert_eq!((p.line_text(0).as_str(), p.line_text(1).as_str(), p.line_text(2).as_str()), ("c", "d", "e"));
    assert_eq!((p.cursor_row(), p.cursor_col()), (2, 1));
    // 列變多：內容與游標不動，下面多出空白列
    p.resize(6, 10);
    assert_eq!((p.line_text(0).as_str(), p.line_text(3).as_str()), ("c", ""));
    assert_eq!((p.cursor_row(), p.cursor_col()), (2, 1));
    p.feed(b"\r\nf");
    assert_eq!(p.line_text(3), "f");
    // 欄變少：每一列截短，游標不超過新的寬度
    let mut q = VtParser::new(2, 10);
    q.feed(b"abcdefgh");
    q.resize(2, 4);
    assert_eq!(q.line_text(0), "abcd");
    assert!(q.cursor_col() <= 4);
    q.resize(2, 8);
    assert_eq!(q.line_text(0), "abcd", "cut columns do not come back");
}

#[test]
fn test_scrollback_keeps_colors() {
    let mut p = VtParser::new(1, 20);
    p.feed(b"\x1b[31mred\x1b[0m plain\r\nnext");
    assert_eq!(p.scrollback_line(0), "red plain");
    let cells = p.scrollback_cells(0);
    assert_eq!((cells[0].ch, cells[0].fg), ('r', 1));
    assert_eq!((cells[4].ch, cells[4].fg), ('p', -1));
    assert_eq!(p.row_cells(0)[0].ch, 'n');
}

#[test]
fn test_extended_colors_do_not_leak_into_basic_ones() {
    // 256 色 / 真彩色的參數（例如 38;2;31;32;33）不能被當成 31（紅）之類的基本色
    let mut p = VtParser::new(1, 20);
    p.feed(b"\x1b[38;2;31;32;33mA\x1b[48;5;41mB\x1b[0;32mC");
    assert_eq!((p.cell_at(0, 0).fg, p.cell_at(0, 0).bg), (-1, -1));
    assert_eq!(p.cell_at(0, 1).bg, -1);
    assert_eq!(p.cell_at(0, 2).fg, 2);
}
