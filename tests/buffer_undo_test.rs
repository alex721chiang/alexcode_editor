use alexcode::editor::{EditorBuffer, Position};

fn buf(text: &str) -> EditorBuffer {
    EditorBuffer::from_text(text)
}

#[test]
fn undo_redo_restores_text_and_cursor() {
    let mut b = buf("one\ntwo\nthree");
    b.replace_all_text("one\ntwo!\nthree");
    b.replace_all_text("one\ntwo!\nxx\nthree");
    assert!(b.undo());
    assert_eq!(b.edit_text(), "one\ntwo!\nthree");
    assert!(b.undo());
    assert_eq!(b.edit_text(), "one\ntwo\nthree");
    assert_eq!(b.selections[0].head, Position { line: 1, col: 3 });
    assert!(!b.undo());
    assert!(b.redo());
    assert_eq!(b.edit_text(), "one\ntwo!\nthree");
    assert_eq!(b.selections[0].head, Position { line: 1, col: 4 });
    assert!(b.redo());
    assert_eq!(b.edit_text(), "one\ntwo!\nxx\nthree");
    assert!(!b.redo());
}

#[test]
fn undo_stores_only_changed_lines() {
    // 10 萬行的文件改一行：復原紀錄只該保存那一行，而不是整份文件
    let text: String = (0..100_000).map(|i| format!("line {i}\n")).collect();
    let mut b = buf(&text);
    let mut lines: Vec<String> = b.lines.clone();
    for k in 0..50 {
        lines[500] = format!("edited {k}");
        b.replace_all_text(&lines.join("\n"));
    }
    assert!(b.undo_bytes() < 16 * 1024, "undo history uses {} bytes", b.undo_bytes());
    for _ in 0..50 {
        assert!(b.undo());
    }
    assert_eq!(b.lines[500], "line 500");
}

#[test]
fn undo_history_is_bounded() {
    let mut b = buf("x");
    let big = "y".repeat(1024 * 1024);
    for i in 0..400 {
        b.replace_all_text(&format!("{big}{i}"));
    }
    assert!(b.undo_bytes() <= EditorBuffer::MAX_UNDO_BYTES);
    assert!(b.undo());
}

#[test]
fn consecutive_typing_is_undone_as_one_step() {
    let mut b = buf("");
    for t in ["h", "he", "hel", "hell", "hello"] {
        b.apply_typed_text(t);
    }
    assert!(b.undo());
    assert_eq!(b.edit_text(), "");
    assert!(b.redo());
    assert_eq!(b.edit_text(), "hello");
}

#[test]
fn modified_flag_tracks_saved_state() {
    let mut b = buf("a");
    assert!(!b.modified);
    b.replace_all_text("ab");
    assert!(b.modified);
    b.mark_saved();
    assert!(!b.modified);
    b.replace_all_text("abc");
    assert!(b.modified);
    b.undo();
    assert!(!b.modified, "undo back to the saved state is not modified");
    b.undo();
    assert!(b.modified, "undo past the saved state is modified");
    b.redo();
    assert!(!b.modified);
}

#[test]
fn line_commands_are_undoable() {
    let mut b = buf("a\nb\nc");
    b.selections[0].head = Position { line: 1, col: 0 };
    b.delete_current_line();
    assert_eq!(b.edit_text(), "a\nc");
    b.duplicate_current_line();
    assert_eq!(b.edit_text(), "a\nc\nc");
    b.undo();
    b.undo();
    assert_eq!(b.edit_text(), "a\nb\nc");
}

#[test]
fn toggle_comment_round_trips() {
    let mut b = buf("    let x = 1;");
    b.toggle_comment("//");
    assert_eq!(b.edit_text(), "    // let x = 1;");
    b.toggle_comment("//");
    assert_eq!(b.edit_text(), "    let x = 1;");
}

#[test]
fn revision_changes_only_when_text_changes() {
    let mut b = buf("a");
    let r0 = b.revision();
    b.replace_all_text("a");
    assert_eq!(b.revision(), r0);
    b.replace_all_text("b");
    assert_ne!(b.revision(), r0);
}

fn pos(line: usize, col: usize) -> Position {
    Position { line, col }
}

#[test]
fn replace_range_edits_in_place_and_can_be_undone() {
    let mut b = buf("one\ntw\u{4e2d}o\nthree");
    // insert inside a line (after a multi-byte character)
    b.replace_range(pos(1, 3), pos(1, 3), "X", true);
    assert_eq!(b.edit_text(), "one\ntw\u{4e2d}Xo\nthree");
    assert_eq!(b.selections[0].head, pos(1, 4));
    assert!(b.modified);
    // replace a selection that spans lines; the arguments may come in either order
    b.replace_range(pos(2, 2), pos(0, 1), "A\r\nB\nC", false);
    assert_eq!(b.edit_text(), "oA\nB\nCree");
    assert_eq!(b.selections[0].head, pos(2, 1));
    // delete: join two lines
    b.replace_range(pos(0, 2), pos(1, 0), "", false);
    assert_eq!(b.edit_text(), "oAB\nCree");
    assert_eq!(b.selections[0].head, pos(0, 2));
    // out-of-range positions are clamped; replacing text with itself records nothing
    let rev = b.revision();
    b.replace_range(pos(9, 9), pos(9, 9), "", false);
    b.replace_range(pos(0, 0), pos(0, 1), "o", false);
    assert_eq!(b.revision(), rev);
    b.replace_range(pos(9, 9), pos(9, 9), "!", false);
    assert_eq!(b.edit_text(), "oAB\nCree!");

    assert!(b.undo());
    assert!(b.undo());
    assert_eq!(b.edit_text(), "oA\nB\nCree");
    assert!(b.undo());
    assert!(b.undo());
    assert_eq!(b.edit_text(), "one\ntw\u{4e2d}o\nthree");
    assert!(!b.modified);
    assert!(b.redo());
    assert_eq!(b.edit_text(), "one\ntw\u{4e2d}Xo\nthree");
}

#[test]
fn replace_range_stores_only_the_lines_it_touched() {
    let text: String = (0..100_000).map(|i| format!("line {i}\n")).collect();
    let mut b = buf(&text);
    b.replace_range(pos(50_000, 4), pos(50_000, 4), "Z", true);
    b.replace_range(pos(50_000, 5), pos(50_000, 5), "\n", true);
    assert_eq!(b.lines.len(), 100_002);
    assert_eq!(b.lines[50_000], "lineZ");
    assert_eq!(b.lines[50_001], " 50000");
    assert!(b.undo_bytes() < 1024, "undo keeps {} bytes", b.undo_bytes());
    // typing in quick succession is undone as one step
    assert!(b.undo());
    assert_eq!(b.lines[50_000], "line 50000");
    assert_eq!(b.lines.len(), 100_001);
    assert!(!b.undo());
}