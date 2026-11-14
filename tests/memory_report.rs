//! 大檔案記憶體用量的量測報告。
//!
//! 這個檔案自成一個測試執行檔，裝上會計數的全域配置器，所以量到的是「程式向配置器要了多少記憶體」
//! （不含配置器本身的零頭與作業系統的分頁），在三個平台上都能重現。
//!
//! 執行方式（預設 29 MB / 約 40 萬行，與待辦清單記錄的實測檔案相同；需要數 GB 記憶體）：
//!
//! ```text
//! cargo test --release --test memory_report -- --ignored --nocapture
//! ```
//!
//! 以環境變數 `ALEXCODE_MEM_REPORT_MB` 指定檔案大小（MB），例如先用 4 快速看比例。
//! 設定 `ALEXCODE_MEM_BUDGET`（倍數，例如 8）時，開檔後的用量超過「檔案大小 × 倍數」就算測試失敗。

use alexcode::editor::EditorBuffer;
use alexcode::ui::AlexCodeApp;
use eframe::egui::{self, text::LayoutJob, Color32, FontId, Pos2, RawInput, Rect};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grew(by: usize) {
    let now = CURRENT.fetch_add(by, Relaxed) + by;
    PEAK.fetch_max(now, Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        CURRENT.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            if new_size >= layout.size() {
                grew(new_size - layout.size());
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> usize {
    CURRENT.load(Relaxed)
}

/// 從現在起重新記錄尖峰
fn reset_peak() {
    PEAK.store(live(), Relaxed);
}

fn mb(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn frame(app: &mut AlexCodeApp, ctx: &egui::Context) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        focused: true,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| app.ui_frame(ctx));
}

/// 類似 log 的純文字：每行約 72 位元組
fn sample_text(target_bytes: usize) -> String {
    let mut s = String::with_capacity(target_bytes + 128);
    let mut i = 0usize;
    while s.len() < target_bytes {
        s.push_str(&format!(
            "{i:07} 2026-11-12 10:00:00.123 INFO  worker-{:02} request {:08x} status=ok\n",
            i % 16,
            i.wrapping_mul(2_654_435_761) as u32
        ));
        i += 1;
    }
    s
}

/// 計數器是整個測試執行檔共用的：兩個測試同時跑（`--include-ignored`）會互相干擾，所以一次只跑一個
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 開啟 `path`（共 `file_lines` 行）並等背景載入完成；回傳 (穩定後多用的位元組, 開檔期間的尖峰, 開好檔的 App)
fn open_and_measure(ctx: &egui::Context, path: &std::path::Path, file_lines: usize) -> (usize, usize, AlexCodeApp) {
    let mut app = AlexCodeApp::new();
    for _ in 0..3 {
        frame(&mut app, ctx);
    }
    let base = live();
    reset_peak();
    app.open_path(path);
    let t = std::time::Instant::now();
    while app.tabs[app.active_tab_idx].lines.len() < file_lines {
        assert!(t.elapsed().as_secs() < 300, "background load did not finish");
        frame(&mut app, ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for _ in 0..5 {
        frame(&mut app, ctx);
    }
    (live().saturating_sub(base), PEAK.load(Relaxed).saturating_sub(base), app)
}

/// 每次 `cargo test` 都會跑的回歸測試：5 MB 的檔案（超過大檔案檢視的門檻）開啟後，
/// 程式多用的記憶體不得超過檔案大小的 4 倍。改用大檔案檢視之前約是 200 倍。
#[test]
fn opening_a_large_file_stays_within_a_few_times_its_size() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.log");
    let text = sample_text(5 * 1024 * 1024);
    std::fs::write(&path, &text).unwrap();
    let (file_bytes, file_lines) = (text.len(), text.lines().count());
    drop(text);

    let ctx = egui::Context::default();
    alexcode::ui::style::setup_fonts(&ctx);
    let (settled, peak, _app) = open_and_measure(&ctx, &path, file_lines);
    let ratio = settled as f64 / file_bytes as f64;
    println!("MEM 5 MB file: settled {:.1} MB ({ratio:.1}x), peak {:.1} MB", mb(settled), mb(peak));
    assert!(ratio <= 4.0, "memory after opening is {ratio:.1}x the file size ({:.1} MB)", mb(settled));
    assert!(peak as f64 <= 8.0 * file_bytes as f64, "peak while opening is {:.1} MB", mb(peak));
}

#[test]
#[ignore]
fn large_file_memory_report() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let size_mb: usize = std::env::var("ALEXCODE_MEM_REPORT_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(29);
    let budget: Option<f64> = std::env::var("ALEXCODE_MEM_BUDGET").ok().and_then(|v| v.parse().ok());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.log");
    let file_bytes = {
        let text = sample_text(size_mb * 1024 * 1024);
        std::fs::write(&path, &text).unwrap();
        text.len()
    };
    let file_lines = std::fs::read_to_string(&path).unwrap().lines().count();
    println!("MEM file: {:.1} MB, {} lines", mb(file_bytes), file_lines);

    let ctx = egui::Context::default();
    alexcode::ui::style::setup_fonts(&ctx);

    // ---- 各個部分單獨量 ----
    {
        let text = std::fs::read_to_string(&path).unwrap();
        let base = live();
        let buf = EditorBuffer::from_text(&text);
        let lines = live() - base;
        println!("MEM part buffer lines (Vec<String>)      = {:8.1} MB  ({:.1}x file)", mb(lines), lines as f64 / file_bytes as f64);

        let base = live();
        let joined = buf.edit_text();
        let cache = live() - base;
        println!("MEM part full-text cache (one String)    = {:8.1} MB  ({:.1}x file)", mb(cache), cache as f64 / file_bytes as f64);
        drop(buf);

        // 編輯區把整份文字交給 egui 排版成一個 Galley（每個字元一個字形 + 4 個頂點 + 6 個索引）
        let _ = ctx.run(RawInput::default(), |ctx| {
            let base = live();
            let job = LayoutJob::simple(joined.clone(), FontId::monospace(14.0), Color32::WHITE, f32::INFINITY);
            let galley = ctx.fonts(|f| f.layout_job(job));
            let total = live() - base;
            let glyphs: usize = galley.rows.iter().map(|r| r.glyphs.len()).sum();
            let verts: usize = galley.rows.iter().map(|r| r.visuals.mesh.vertices.len()).sum();
            let idx: usize = galley.rows.iter().map(|r| r.visuals.mesh.indices.len()).sum();
            println!("MEM part egui galley of the whole text   = {:8.1} MB  ({:.1}x file)", mb(total), total as f64 / file_bytes as f64);
            println!(
                "MEM      rows={} glyphs={} ({} B each) vertices={} ({} B each) indices={} (4 B each)",
                galley.rows.len(),
                glyphs,
                std::mem::size_of::<egui::epaint::text::Glyph>(),
                verts,
                std::mem::size_of::<egui::epaint::Vertex>(),
                idx
            );
        });
    }

    // ---- 整個程式：開檔前後的差 ----
    let (settled, peak, mut app) = open_and_measure(&ctx, &path, file_lines);
    let ratio = settled as f64 / file_bytes as f64;
    println!("MEM app after opening the file (settled)  = {:8.1} MB  ({:.1}x file)", mb(settled), ratio);
    println!("MEM app peak while opening                = {:8.1} MB  ({:.1}x file)", mb(peak), peak as f64 / file_bytes as f64);

    // 閒置一幀的成本
    let t = std::time::Instant::now();
    for _ in 0..10 {
        frame(&mut app, &ctx);
    }
    println!("MEM idle frame                            = {:8.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 10.0);

    if let Some(limit) = budget {
        assert!(ratio <= limit, "memory after opening is {ratio:.1}x the file size, budget is {limit}x");
    }
}
