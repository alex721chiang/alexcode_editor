//! AlexCode Editor 執行檔入口 (main.rs)
#![windows_subsystem = "windows"]

use alexcode::ui::AlexCodeApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
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
        Box::new(|cc| Box::new(AlexCodeApp::with_context(cc))),
    )
}
