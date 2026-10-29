//! 霓虹主題樣式（對應原版 C++ `Theme::stylesheet()` 的 QSS）與字型設定

use crate::theme::Palette;
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Mesh, Pos2, Rect, Rounding, Shape, Stroke};

/// 內嵌原版 Orbitron 科技字型（OFL 授權，用於 Logo / 面板標題）
const ORBITRON_TTF: &[u8] = include_bytes!("../../assets/fonts/Orbitron.ttf");
/// 內嵌原版龍形 Logo
pub const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");

pub const ORBITRON: &str = "orbitron";

/// 已解析好的 egui 顏色（每幀依目前主題產生）
#[derive(Clone, Copy)]
pub struct Colors {
    pub bg: Color32,
    pub panel: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub accent2: Color32,
    pub fg: Color32,
    pub fg_dim: Color32,
    pub fg_dim2: Color32,
    pub fg_list: Color32,
    pub faint: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub current_line: Color32,
    pub sel_bg: Color32,
    pub search_bg: Color32,
    pub search_fg: Color32,
    pub bracket_bg: Color32,
    pub warn: Color32,
    /// 半透明強調色光暈（邊框 / hover 底線）
    pub glow: Color32,
    /// 極淡強調色（選取底色 / 漸層尾）
    pub accent_faint: Color32,
    pub hud: bool,
    pub light: bool,
}

pub fn hex(s: &str) -> Color32 {
    let (r, g, b) = Palette::hex_to_rgb(s);
    Color32::from_rgb(r, g, b)
}

pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

impl Colors {
    pub fn from_palette(p: &Palette, hud: bool) -> Self {
        let accent = hex(p.accent);
        Self {
            bg: hex(p.bg),
            panel: hex(p.panel),
            border: hex(p.border),
            accent,
            accent_dim: hex(p.accent_dim),
            accent2: hex(p.accent2),
            fg: hex(p.fg),
            fg_dim: hex(p.fg_dim),
            fg_dim2: hex(p.fg_dim2),
            fg_list: hex(p.fg_list),
            faint: hex(p.faint),
            hover: hex(p.hover),
            pressed: hex(p.pressed),
            current_line: hex(p.current_line),
            sel_bg: hex(p.sel_bg),
            search_bg: hex(p.search_bg),
            search_fg: hex(p.search_fg),
            bracket_bg: hex(p.bracket_bg),
            warn: hex(p.warn),
            glow: with_alpha(accent, if hud { 150 } else { 110 }),
            accent_faint: with_alpha(accent, if hud { 42 } else { 26 }),
            hud,
            light: p.name == "Paper Light",
        }
    }
}

/// 嘗試讀取系統字型（第一個存在者）
fn load_first(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

/// 設定字型：
/// - Orbitron（內嵌）作為科技標題字
/// - 系統等寬字（Consolas / Cascadia / DejaVu Sans Mono）作為程式碼字
/// - 系統中文字型（微軟正黑體 / 雅黑 / Noto CJK / PingFang）作為 fallback，修正中文顯示成「□□」
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts
        .font_data
        .insert(ORBITRON.to_owned(), FontData::from_static(ORBITRON_TTF));
    fonts.families.insert(
        FontFamily::Name(ORBITRON.into()),
        vec![ORBITRON.to_owned(), "Ubuntu-Light".to_owned()],
    );

    // UI 介面字：Segoe UI（與原版 QSS 的 font-family 相同）
    if let Some(bytes) = load_first(&[
        r"C:\Windows\Fonts\segoeui.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]) {
        fonts.font_data.insert("ui".to_owned(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "ui".to_owned());
    }

    // 程式碼等寬字
    if let Some(bytes) = load_first(&[
        r"C:\Windows\Fonts\CascadiaMono.ttf",
        r"C:\Windows\Fonts\consola.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]) {
        fonts.font_data.insert("code".to_owned(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "code".to_owned());
    }

    // 中文 fallback（原版 QSS："Microsoft JhengHei UI"）
    if let Some(bytes) = load_first(&[
        r"C:\Windows\Fonts\msjh.ttc",
        r"C:\Windows\Fonts\msjh.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\mingliu.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    ]) {
        fonts.font_data.insert("cjk".to_owned(), FontData::from_owned(bytes));
        for fam in [FontFamily::Proportional, FontFamily::Monospace, FontFamily::Name(ORBITRON.into())] {
            fonts.families.entry(fam).or_default().push("cjk".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

/// 依目前調色盤套用 egui Visuals（對應原版 QSS 各元件規則）
pub fn apply_style(ctx: &egui::Context, c: &Colors) {
    let mut style = (*ctx.style()).clone();
    let mut v = if c.light { egui::Visuals::light() } else { egui::Visuals::dark() };

    v.override_text_color = None;
    v.panel_fill = c.bg;
    v.window_fill = c.panel;
    v.window_stroke = Stroke::new(1.0_f32, c.glow);
    v.window_rounding = Rounding::same(8.0);
    v.menu_rounding = Rounding::same(6.0);
    v.extreme_bg_color = c.panel; // TextEdit 底色
    v.faint_bg_color = c.current_line;
    v.code_bg_color = c.panel;
    v.hyperlink_color = c.accent;
    v.warn_fg_color = c.warn;
    v.error_fg_color = c.accent2;
    v.selection.bg_fill = c.sel_bg;
    v.selection.stroke = Stroke::new(1.0_f32, c.fg);
    v.text_cursor = Stroke::new(2.0_f32, c.accent);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: with_alpha(if c.light { Color32::BLACK } else { c.accent }, if c.light { 40 } else { 28 }),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: with_alpha(Color32::BLACK, 90),
    };

    let r = Rounding::same(5.0);
    // 不可互動（label、分隔線）
    v.widgets.noninteractive.bg_fill = c.panel;
    v.widgets.noninteractive.weak_bg_fill = c.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, c.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, c.fg);
    v.widgets.noninteractive.rounding = r;
    // 一般（按鈕 / 輸入框）
    v.widgets.inactive.bg_fill = c.panel;
    v.widgets.inactive.weak_bg_fill = c.hover;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, c.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, c.fg_dim);
    v.widgets.inactive.rounding = r;
    // hover：強調色光暈
    v.widgets.hovered.bg_fill = c.hover;
    v.widgets.hovered.weak_bg_fill = c.accent_faint;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, c.glow);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, c.accent);
    v.widgets.hovered.rounding = r;
    v.widgets.hovered.expansion = 0.0;
    // 按下
    v.widgets.active.bg_fill = c.pressed;
    v.widgets.active.weak_bg_fill = c.pressed;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, c.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.5_f32, c.accent);
    v.widgets.active.rounding = r;
    // 開啟中的選單 / 下拉
    v.widgets.open.bg_fill = c.hover;
    v.widgets.open.weak_bg_fill = c.accent_faint;
    v.widgets.open.bg_stroke = Stroke::new(1.0_f32, c.accent);
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, c.accent);
    v.widgets.open.rounding = r;

    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.interact_size.y = 22.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.5));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.5));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(14.0));
    ctx.set_style(style);
}

/// 垂直漸層矩形（QSS qlineargradient(x1:0,y1:0,x2:0,y2:1) 的等價物）
pub fn vgradient(rect: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    Shape::mesh(mesh)
}

/// 水平漸層矩形（Dock 標題 accentFaint → panel）
pub fn hgradient(rect: Rect, left: Color32, right: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    Shape::mesh(mesh)
}

/// Neon HUD 角標覆蓋（移植自原版 `HudOverlay.cpp`）
pub fn paint_hud_corners(painter: &egui::Painter, rect: Rect, accent: Color32) {
    let m = 9.0;
    let len = 15.0;
    let r = rect.shrink(m);
    if r.width() < 4.0 * len || r.height() < 4.0 * len {
        return;
    }
    let st = Stroke::new(2.0_f32, with_alpha(accent, 205));
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let seg = |a: (f32, f32), z: (f32, f32)| {
        painter.line_segment([Pos2::new(a.0, a.1), Pos2::new(z.0, z.1)], st);
    };
    seg((l, t), (l + len, t));
    seg((l, t), (l, t + len));
    seg((rr, t), (rr - len, t));
    seg((rr, t), (rr, t + len));
    seg((l, b), (l + len, b));
    seg((l, b), (l, b - len));
    seg((rr, b), (rr - len, b));
    seg((rr, b), (rr, b - len));
}

/// 將 PNG 位元組解碼為 egui ColorImage
pub fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}
