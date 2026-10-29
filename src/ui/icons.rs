//! 霓虹線條圖示（移植自原版 C++ `NeonIcons.cpp`）
//!
//! 以 0..1 正規化座標作畫：主線條用前景色（alpha 0.78，比純前景低調一階），
//! 點綴部位用強調色，讓重點跳出來。全部向量繪製，任何 DPI 都清晰。

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    New,
    Open,
    Save,
    Undo,
    Redo,
    Find,
    FindInFiles,
    Terminal,
    Graph,
    Palette,
    Markdown,
}

struct Pen<'a> {
    painter: &'a Painter,
    rect: Rect,
    mirror: bool,
}

impl<'a> Pen<'a> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let x = if self.mirror { 1.0 - x } else { x };
        Pos2::new(
            self.rect.left() + x * self.rect.width(),
            self.rect.top() + y * self.rect.height(),
        )
    }
    fn s(&self) -> f32 {
        self.rect.width()
    }
    fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, st: Stroke) {
        self.painter.line_segment([self.pt(x1, y1), self.pt(x2, y2)], st);
    }
    fn polyline(&self, pts: &[(f32, f32)], st: Stroke) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.painter.add(Shape::line(v, st));
    }
    fn polygon(&self, pts: &[(f32, f32)], st: Stroke) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.painter.add(Shape::closed_line(v, st));
    }
    fn circle(&self, cx: f32, cy: f32, r: f32, st: Stroke) {
        self.painter.circle_stroke(self.pt(cx, cy), r * self.s(), st);
    }
    /// Qt 風格弧線：角度以度為單位、逆時針為正、0° 在 3 點鐘方向
    fn arc(&self, cx: f32, cy: f32, r: f32, start_deg: f32, sweep_deg: f32, st: Stroke) {
        let n = 24;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = (start_deg + sweep_deg * i as f32 / n as f32).to_radians();
                self.pt(cx + r * a.cos(), cy - r * a.sin())
            })
            .collect();
        self.painter.add(Shape::line(pts, st));
    }
}

/// 在 `rect` 內繪製指定圖示
pub fn paint_icon(painter: &Painter, rect: Rect, kind: IconKind, fg: Color32, accent: Color32) {
    let s = rect.width();
    let w = (s * 0.075).max(1.4);
    let line = Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), (255.0 * 0.78) as u8);
    let pen = Stroke::new(w, line);
    let acc = Stroke::new(w, accent);
    let d = Pen {
        painter,
        rect,
        mirror: kind == IconKind::Redo,
    };

    match kind {
        IconKind::New => {
            // 文件（右上摺角，摺線用點綴色）
            d.polygon(&[(0.30, 0.14), (0.58, 0.14), (0.72, 0.28), (0.72, 0.86), (0.30, 0.86)], pen);
            d.polyline(&[(0.58, 0.14), (0.58, 0.28), (0.72, 0.28)], acc);
        }
        IconKind::Open => {
            // 資料夾（開啟的前蓋用點綴色）
            d.polyline(
                &[(0.80, 0.42), (0.80, 0.32), (0.44, 0.32), (0.38, 0.24), (0.16, 0.24), (0.16, 0.76)],
                pen,
            );
            d.polygon(&[(0.16, 0.76), (0.28, 0.44), (0.88, 0.44), (0.76, 0.76)], acc);
        }
        IconKind::Save => {
            // 下載托盤（箭頭用點綴色）
            d.polyline(&[(0.18, 0.60), (0.18, 0.82), (0.82, 0.82), (0.82, 0.60)], pen);
            d.line(0.50, 0.14, 0.50, 0.62, acc);
            d.polyline(&[(0.36, 0.48), (0.50, 0.62), (0.64, 0.48)], acc);
        }
        IconKind::Undo | IconKind::Redo => {
            // 弧形箭頭（箭頭尖用點綴色；Redo 為水平鏡像）
            d.arc(0.50, 0.54, 0.28, -20.0, 230.0, pen);
            d.polyline(&[(0.16, 0.20), (0.30, 0.32), (0.17, 0.46)], acc);
        }
        IconKind::Find => {
            // 放大鏡（握把用點綴色）
            d.circle(0.42, 0.42, 0.22, pen);
            d.line(0.59, 0.59, 0.82, 0.82, acc);
        }
        IconKind::FindInFiles => {
            // 資料夾 + 小放大鏡（放大鏡用點綴色）
            d.polyline(
                &[(0.14, 0.72), (0.14, 0.24), (0.36, 0.24), (0.42, 0.32), (0.78, 0.32), (0.78, 0.48)],
                pen,
            );
            d.circle(0.62, 0.62, 0.15, acc);
            d.line(0.73, 0.73, 0.88, 0.88, acc);
        }
        IconKind::Terminal => {
            // 終端機外框 + 提示符號
            d.polygon(&[(0.12, 0.20), (0.88, 0.20), (0.88, 0.80), (0.12, 0.80)], pen);
            d.polyline(&[(0.26, 0.38), (0.40, 0.50), (0.26, 0.62)], acc);
            d.line(0.48, 0.64, 0.70, 0.64, acc);
        }
        IconKind::Graph => {
            // 節點關聯圖
            d.line(0.28, 0.30, 0.70, 0.36, pen);
            d.line(0.28, 0.30, 0.42, 0.72, pen);
            d.line(0.70, 0.36, 0.42, 0.72, pen);
            d.circle(0.28, 0.30, 0.09, acc);
            d.circle(0.70, 0.36, 0.09, acc);
            d.circle(0.42, 0.72, 0.09, acc);
        }
        IconKind::Palette => {
            // 命令面板：大於符號 + 底線
            d.polygon(&[(0.12, 0.22), (0.88, 0.22), (0.88, 0.78), (0.12, 0.78)], pen);
            d.polyline(&[(0.28, 0.40), (0.40, 0.50), (0.28, 0.60)], acc);
            d.line(0.46, 0.40, 0.72, 0.40, pen);
            d.line(0.46, 0.60, 0.66, 0.60, pen);
        }
        IconKind::Markdown => {
            // Markdown 標誌：外框 + 左側 M + 右側向下箭頭（點綴色）
            d.polygon(&[(0.10, 0.22), (0.90, 0.22), (0.90, 0.78), (0.10, 0.78)], pen);
            d.polyline(
                &[(0.20, 0.65), (0.20, 0.35), (0.34, 0.52), (0.48, 0.35), (0.48, 0.65)],
                acc,
            );
            d.line(0.70, 0.35, 0.70, 0.65, acc);
            d.polyline(&[(0.60, 0.53), (0.70, 0.65), (0.80, 0.53)], acc);
        }
    }
}

/// 霓虹工具列按鈕：透明底，hover 時出現淡強調色底與發光邊框（對應原版 QToolButton QSS）
pub fn tool_button(
    ui: &mut egui::Ui,
    kind: IconKind,
    tooltip: &str,
    fg: Color32,
    accent: Color32,
    faint: Color32,
    glow: Color32,
) -> egui::Response {
    let size = egui::vec2(30.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let (fg_c, acc_c) = if resp.hovered() {
            painter.rect(rect, 4.0, faint, Stroke::new(1.0_f32, glow));
            (accent, accent)
        } else {
            (fg, accent)
        };
        if resp.is_pointer_button_down_on() {
            painter.rect_filled(rect, 4.0, glow.linear_multiply(0.35));
        }
        let icon_rect = Rect::from_center_size(rect.center(), egui::vec2(20.0, 20.0));
        paint_icon(painter, icon_rect, kind, fg_c, acc_c);
    }
    resp.on_hover_text(tooltip)
}
