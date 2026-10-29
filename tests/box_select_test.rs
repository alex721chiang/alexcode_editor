use alexcode::editor::{compute_box_select, BoxRange};

#[test]
fn test_normal_rectangle() {
    let len = vec![10, 10, 10];
    let r = compute_box_select(0, 2, 2, 6, &len);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], BoxRange { line: 0, start: 2, end: 6 });
    assert_eq!(r[1], BoxRange { line: 1, start: 2, end: 6 });
    assert_eq!(r[2], BoxRange { line: 2, start: 2, end: 6 });
}

#[test]
fn test_short_line_clamped() {
    let len = vec![10, 3, 10]; // 第 1 行只有 3 字
    let r = compute_box_select(0, 2, 2, 6, &len);
    assert_eq!(r[1], BoxRange { line: 1, start: 2, end: 3 }); // end clamp 到行長 3
}

#[test]
fn test_very_short_line_collapses_to_point() {
    let len = vec![10, 1, 10]; // 第 1 行只有 1 字，欄位 [2,6) 全超出
    let r = compute_box_select(0, 2, 2, 6, &len);
    assert_eq!(r[1], BoxRange { line: 1, start: 1, end: 1 }); // start/end 都 clamp 到 1 → 零寬
}

#[test]
fn test_zero_width_insertion_column() {
    let len = vec![10, 10, 10];
    let r = compute_box_select(0, 4, 2, 4, &len); // 同欄 → 每行零寬插入點
    for x in &r {
        assert_eq!(x.start, x.end);
    }
    assert_eq!(r.len(), 3);
}

#[test]
fn test_reverse_drag_normalised() {
    let len = vec![10, 10, 10];
    let fwd = compute_box_select(0, 2, 2, 6, &len);
    let rev = compute_box_select(2, 6, 0, 2, &len); // 反向拖曳應得相同結果
    assert_eq!(fwd, rev);
}

#[test]
fn test_single_line_degenerate() {
    let len = vec![20];
    let r = compute_box_select(0, 3, 0, 8, &len);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], BoxRange { line: 0, start: 3, end: 8 });
}
