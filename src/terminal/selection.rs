//! 終端機選取文字擷取邏輯 (TermSelection)
//!
//! 給定目前畫面各行字串與起訖點 (r0, c0) 到 (r1, c1)，
//! 按照閱讀順序正規化後擷取範圍內的文字。

pub fn normalize(
    r0: &mut usize,
    c0: &mut usize,
    r1: &mut usize,
    c1: &mut usize,
) {
    if *r0 > *r1 || (*r0 == *r1 && *c0 > *c1) {
        std::mem::swap(r0, r1);
        std::mem::swap(c0, c1);
    }
}

pub fn extract_text(
    lines: &[String],
    mut r0: usize,
    mut c0: usize,
    mut r1: usize,
    mut c1: usize,
) -> String {
    if lines.is_empty() {
        return String::new();
    }

    normalize(&mut r0, &mut c0, &mut r1, &mut c1);

    r0 = r0.min(lines.len() - 1);
    r1 = r1.min(lines.len() - 1);
    if r0 > r1 {
        return String::new();
    }

    if r0 == r1 {
        let line = &lines[r0];
        let start = c0.min(line.len());
        let end = c1.min(line.len());
        return if start <= end {
            line[start..end].to_string()
        } else {
            String::new()
        };
    }

    let mut out = String::new();
    let l0 = &lines[r0];
    let start0 = c0.min(l0.len());
    out.push_str(&l0[start0..]);

    for r in (r0 + 1)..r1 {
        out.push('\n');
        out.push_str(&lines[r]);
    }

    out.push('\n');
    let l1 = &lines[r1];
    let end1 = c1.min(l1.len());
    out.push_str(&l1[..end1]);

    out
}
