//! AlexCode Editor 執行檔入口 (main.rs)
#![windows_subsystem = "windows"]

use alexcode::core::single_instance::{acquire, Instance};
use alexcode::ui::AlexCodeApp;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    // 單一實例：程式已在執行時（例如在檔案總管雙擊檔案），把檔案交給現有視窗後直接結束。
    // 加上 --new-instance 可強制另開視窗。
    let args: Vec<String> = std::env::args().skip(1).collect();
    let force_new = args.iter().any(|a| a == "--new-instance");
    let files: Vec<PathBuf> = args
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .map(|p| std::path::absolute(&p).unwrap_or(p))
        .collect();
    let waker: Arc<Mutex<Option<egui::Context>>> = Arc::default();
    let mut incoming = None;
    let mut _guard = None;
    if !force_new {
        if let Some(dir) = alexcode::core::config_dir() {
            let w = waker.clone();
            let wake = move || {
                if let Some(ctx) = w.lock().ok().and_then(|g| g.clone()) {
                    ctx.request_repaint();
                }
            };
            match acquire(&dir, &files, wake) {
                Ok(Instance::Forwarded) => return Ok(()),
                Ok(Instance::Primary(rx, guard)) => {
                    incoming = Some(rx);
                    _guard = Some(guard);
                }
                Err(_) => {} // 無法建立通道：照常啟動，只是沒有單一實例功能
            }
        }
    }

    // 載入原版龍形圖示作為視窗圖示（與原版 Qt 的 a.setWindowIcon(":/icon.png") 一致）
    let icon_data = eframe::icon_data::from_png_bytes(alexcode::ui::style::ICON_PNG).ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 480.0])
        .with_title("AlexCode — Neon Edition (Rust)");

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "AlexCode",
        native_options,
        Box::new(move |cc| {
            if let Ok(mut g) = waker.lock() {
                *g = Some(cc.egui_ctx.clone());
            }
            let mut app = AlexCodeApp::with_context(cc);
            if let Some(rx) = incoming {
                app.set_incoming(rx);
            }
            Box::new(app)
        }),
    )
}
