use alexcode::editor::{for_size_mb, FileTierLevel};

#[test]
fn test_small_file_is_full() {
    assert_eq!(for_size_mb(0.5, 2, 10), FileTierLevel::Full);
    assert_eq!(for_size_mb(2.0, 2, 10), FileTierLevel::Full); // 剛好在門檻 = 仍為 Full
}

#[test]
fn test_medium_file_disables_assist() {
    assert_eq!(for_size_mb(2.1, 2, 10), FileTierLevel::AssistOff);
    assert_eq!(for_size_mb(5.0, 2, 10), FileTierLevel::AssistOff);
    assert_eq!(for_size_mb(10.0, 2, 10), FileTierLevel::AssistOff);
}

#[test]
fn test_large_file_disables_highlight() {
    assert_eq!(for_size_mb(10.1, 2, 10), FileTierLevel::HighlightOff);
    assert_eq!(for_size_mb(60.0, 2, 10), FileTierLevel::HighlightOff);
}

#[test]
fn test_respects_custom_thresholds() {
    assert_eq!(for_size_mb(3.0, 5, 20), FileTierLevel::Full);
    assert_eq!(for_size_mb(8.0, 5, 20), FileTierLevel::AssistOff);
    assert_eq!(for_size_mb(25.0, 5, 20), FileTierLevel::HighlightOff);
}
