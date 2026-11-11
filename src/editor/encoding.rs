//! 檔案編碼偵測與轉換 (FileEncoding)
//!
//! 開檔時偵測編碼並記在緩衝區上，存檔時用同一種編碼寫回，
//! 避免非 UTF-8 檔案（例如 Big5）被 `from_utf8_lossy` 換成「�」後存回而損毀。
//!
//! 偵測順序：BOM → 合法 UTF-8 → Big5 → GBK → Shift_JIS → Windows-1252。
//! Windows-1252 對所有位元組都能解碼且可原樣編碼回去，作為不遺失資料的最後手段。

use encoding_rs::{Encoding, BIG5, GBK, SHIFT_JIS, WINDOWS_1252};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Big5,
    Gbk,
    ShiftJis,
    Windows1252,
}

impl FileEncoding {
    pub const ALL: [FileEncoding; 8] = [
        FileEncoding::Utf8,
        FileEncoding::Utf8Bom,
        FileEncoding::Utf16Le,
        FileEncoding::Utf16Be,
        FileEncoding::Big5,
        FileEncoding::Gbk,
        FileEncoding::ShiftJis,
        FileEncoding::Windows1252,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FileEncoding::Utf8 => "UTF-8",
            FileEncoding::Utf8Bom => "UTF-8 BOM",
            FileEncoding::Utf16Le => "UTF-16 LE",
            FileEncoding::Utf16Be => "UTF-16 BE",
            FileEncoding::Big5 => "Big5",
            FileEncoding::Gbk => "GBK",
            FileEncoding::ShiftJis => "Shift_JIS",
            FileEncoding::Windows1252 => "Windows-1252",
        }
    }

    fn legacy(self) -> Option<&'static Encoding> {
        match self {
            FileEncoding::Big5 => Some(BIG5),
            FileEncoding::Gbk => Some(GBK),
            FileEncoding::ShiftJis => Some(SHIFT_JIS),
            FileEncoding::Windows1252 => Some(WINDOWS_1252),
            _ => None,
        }
    }

    /// 以此編碼輸出位元組。文字含有此編碼無法表示的字元時回傳 `Err(第一個無法表示的字元)`，
    /// 呼叫端應拒絕存檔（而不是寫入替代字元）。
    pub fn encode(self, text: &str) -> Result<Vec<u8>, char> {
        match self {
            FileEncoding::Utf8 => Ok(text.as_bytes().to_vec()),
            FileEncoding::Utf8Bom => {
                let mut out = Vec::with_capacity(text.len() + 3);
                out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
                out.extend_from_slice(text.as_bytes());
                Ok(out)
            }
            FileEncoding::Utf16Le | FileEncoding::Utf16Be => {
                let le = self == FileEncoding::Utf16Le;
                let mut out = Vec::with_capacity(text.len() * 2 + 2);
                out.extend_from_slice(if le { &[0xFF, 0xFE] } else { &[0xFE, 0xFF] });
                for u in text.encode_utf16() {
                    out.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
                }
                Ok(out)
            }
            _ => {
                let enc = self.legacy().expect("legacy encoding");
                let (bytes, _, had_errors) = enc.encode(text);
                if !had_errors {
                    return Ok(bytes.into_owned());
                }
                let mut tmp = [0u8; 4];
                let bad = text
                    .chars()
                    .find(|ch| enc.encode(ch.encode_utf8(&mut tmp)).2)
                    .unwrap_or('\u{FFFD}');
                Err(bad)
            }
        }
    }

    /// 以指定編碼解碼；第二個回傳值表示是否有無法解碼的位元組（已換成「�」）
    pub fn decode(self, bytes: &[u8]) -> (String, bool) {
        match self {
            FileEncoding::Utf8 | FileEncoding::Utf8Bom => {
                let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
                match std::str::from_utf8(body) {
                    Ok(s) => (s.to_string(), false),
                    Err(_) => (String::from_utf8_lossy(body).into_owned(), true),
                }
            }
            FileEncoding::Utf16Le | FileEncoding::Utf16Be => {
                let le = self == FileEncoding::Utf16Le;
                let bom: &[u8] = if le { &[0xFF, 0xFE] } else { &[0xFE, 0xFF] };
                let body = bytes.strip_prefix(bom).unwrap_or(bytes);
                let units: Vec<u16> = body
                    .chunks_exact(2)
                    .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
                    .collect();
                match String::from_utf16(&units) {
                    Ok(s) => (s, body.len() % 2 != 0),
                    Err(_) => (String::from_utf16_lossy(&units), true),
                }
            }
            _ => {
                let enc = self.legacy().expect("legacy encoding");
                let (s, had_errors) = enc.decode_without_bom_handling(bytes);
                (s.into_owned(), had_errors)
            }
        }
    }
}

/// 偵測編碼並解碼。回傳的文字保證能用回傳的編碼原樣寫回。
pub fn detect_and_decode(bytes: &[u8]) -> (String, FileEncoding) {
    let by_bom = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some(FileEncoding::Utf8Bom)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some(FileEncoding::Utf16Le)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some(FileEncoding::Utf16Be)
    } else {
        None
    };
    if let Some(enc) = by_bom {
        let (text, bad) = enc.decode(bytes);
        if !bad {
            return (text, enc);
        }
    } else if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_string(), FileEncoding::Utf8);
    }
    for enc in [FileEncoding::Big5, FileEncoding::Gbk, FileEncoding::ShiftJis] {
        let (text, bad) = enc.decode(bytes);
        // 解碼無誤且能原樣編碼回去才採用
        if !bad && enc.encode(&text).map_or(false, |b| b == bytes) {
            return (text, enc);
        }
    }
    (FileEncoding::Windows1252.decode(bytes).0, FileEncoding::Windows1252)
}
