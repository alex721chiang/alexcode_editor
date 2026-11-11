use alexcode::editor::{detect_and_decode, FileEncoding};

#[test]
fn utf8_stays_utf8() {
    let (t, e) = detect_and_decode("中文 abc".as_bytes());
    assert_eq!((t.as_str(), e), ("中文 abc", FileEncoding::Utf8));
}

#[test]
fn big5_round_trips_byte_for_byte() {
    // 「測試」的 Big5 位元組
    let bytes = [0xB4, 0xFA, 0xB8, 0xD5, b'\n', b'o', b'k'];
    let (t, e) = detect_and_decode(&bytes);
    assert_eq!(e, FileEncoding::Big5);
    assert_eq!(t, "測試\nok");
    assert_eq!(e.encode(&t).unwrap(), bytes);
}

#[test]
fn boms_are_detected_and_preserved() {
    let utf8bom = [0xEF, 0xBB, 0xBF, b'h', b'i'];
    let (t, e) = detect_and_decode(&utf8bom);
    assert_eq!((t.as_str(), e), ("hi", FileEncoding::Utf8Bom));
    assert_eq!(e.encode(&t).unwrap(), utf8bom);

    let utf16le = [0xFF, 0xFE, 0x2D, 0x4E, b'a', 0x00];
    let (t, e) = detect_and_decode(&utf16le);
    assert_eq!((t.as_str(), e), ("中a", FileEncoding::Utf16Le));
    assert_eq!(e.encode(&t).unwrap(), utf16le);

    let utf16be = [0xFE, 0xFF, 0x4E, 0x2D, 0x00, b'a'];
    let (t, e) = detect_and_decode(&utf16be);
    assert_eq!((t.as_str(), e), ("中a", FileEncoding::Utf16Be));
    assert_eq!(e.encode(&t).unwrap(), utf16be);
}

#[test]
fn arbitrary_bytes_never_lose_data() {
    // 任何編碼都解不開的位元組：退回 Windows-1252，仍可原樣寫回
    let bytes: Vec<u8> = (0u8..=255).collect();
    let (t, e) = detect_and_decode(&bytes);
    assert_eq!(e.encode(&t).unwrap(), bytes);
}

#[test]
fn unrepresentable_char_is_reported_not_replaced() {
    assert_eq!(FileEncoding::Big5.encode("ok 😀"), Err('😀'));
    assert!(FileEncoding::Big5.encode("測試").is_ok());
}
