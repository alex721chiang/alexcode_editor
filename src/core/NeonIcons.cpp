#include "NeonIcons.h"
#include "Theme.h"
#include <QPainter>
#include <QPainterPath>
#include <QPixmap>

namespace {

// 以 0..1 正規化座標作畫，s = 目標像素尺寸
struct Pen2 {
    QPainter& p;
    qreal s;
    QPointF pt(qreal x, qreal y) const { return { x * s, y * s }; }
    void line(qreal x1, qreal y1, qreal x2, qreal y2) const {
        p.drawLine(pt(x1, y1), pt(x2, y2));
    }
};

void drawKind(QPainter& p, const QString& kind, qreal s,
              const QColor& line, const QColor& accent) {
    const qreal w = qMax<qreal>(1.4, s * 0.075);
    QPen pen(line, w, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    QPen accentPen(accent, w, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    p.setRenderHint(QPainter::Antialiasing);
    p.setBrush(Qt::NoBrush);
    Pen2 d{ p, s };

    if (kind == "new") {                       // 文件（右上摺角，摺線用點綴色）
        QPainterPath doc;
        doc.moveTo(d.pt(0.30, 0.14));
        doc.lineTo(d.pt(0.58, 0.14));
        doc.lineTo(d.pt(0.72, 0.28));
        doc.lineTo(d.pt(0.72, 0.86));
        doc.lineTo(d.pt(0.30, 0.86));
        doc.closeSubpath();
        p.setPen(pen);
        p.drawPath(doc);
        p.setPen(accentPen);
        p.drawPolyline(QPolygonF{ d.pt(0.58, 0.14), d.pt(0.58, 0.28), d.pt(0.72, 0.28) });
    } else if (kind == "open") {               // 資料夾（開啟的前蓋用點綴色）
        p.setPen(pen);
        p.drawPolyline(QPolygonF{ d.pt(0.80, 0.42), d.pt(0.80, 0.32), d.pt(0.44, 0.32),
                                  d.pt(0.38, 0.24), d.pt(0.16, 0.24), d.pt(0.16, 0.76) });
        p.setPen(accentPen);
        p.drawPolygon(QPolygonF{ d.pt(0.16, 0.76), d.pt(0.28, 0.44),
                                 d.pt(0.88, 0.44), d.pt(0.76, 0.76) });
    } else if (kind == "save") {               // 下載托盤（箭頭用點綴色）
        p.setPen(pen);
        p.drawPolyline(QPolygonF{ d.pt(0.18, 0.60), d.pt(0.18, 0.82),
                                  d.pt(0.82, 0.82), d.pt(0.82, 0.60) });
        p.setPen(accentPen);
        d.line(0.50, 0.14, 0.50, 0.62);
        p.drawPolyline(QPolygonF{ d.pt(0.36, 0.48), d.pt(0.50, 0.62), d.pt(0.64, 0.48) });
    } else if (kind == "undo" || kind == "redo") {   // 弧形箭頭（箭頭尖用點綴色）
        const bool mirror = (kind == "redo");
        p.save();
        if (mirror) { p.translate(s, 0); p.scale(-1, 1); }
        QPainterPath arc;
        const QRectF r(0.22 * s, 0.26 * s, 0.56 * s, 0.56 * s);
        arc.arcMoveTo(r, -20);
        arc.arcTo(r, -20, 230);
        p.setPen(pen);
        p.drawPath(arc);
        p.setPen(accentPen);
        p.drawPolyline(QPolygonF{ d.pt(0.16, 0.20), d.pt(0.30, 0.32), d.pt(0.17, 0.46) });
        p.restore();
    } else if (kind == "find") {               // 放大鏡（握把用點綴色）
        p.setPen(pen);
        p.drawEllipse(d.pt(0.42, 0.42), 0.22 * s, 0.22 * s);
        p.setPen(accentPen);
        d.line(0.59, 0.59, 0.82, 0.82);
    } else if (kind == "find-in-files") {      // 資料夾 + 小放大鏡（放大鏡用點綴色）
        p.setPen(pen);
        p.drawPolyline(QPolygonF{ d.pt(0.14, 0.72), d.pt(0.14, 0.24), d.pt(0.36, 0.24),
                                  d.pt(0.42, 0.32), d.pt(0.78, 0.32), d.pt(0.78, 0.48) });
        p.setPen(accentPen);
        p.drawEllipse(d.pt(0.62, 0.62), 0.15 * s, 0.15 * s);
        d.line(0.73, 0.73, 0.88, 0.88);
    }
}

QPixmap render(const QString& kind, int size, const QColor& line, const QColor& accent) {
    QPixmap pm(size, size);
    pm.fill(Qt::transparent);
    QPainter p(&pm);
    drawKind(p, kind, size, line, accent);
    return pm;
}

} // namespace

QIcon NeonIcons::icon(const QString& kind) {
    QColor line(Theme::EDITOR_FG);
    line.setAlphaF(0.78);                      // 比純前景低調一階，讓點綴色跳出來
    const QColor accent(Theme::ACCENT);
    QIcon ic;
    for (int s : { 16, 20, 24, 32, 48 })
        ic.addPixmap(render(kind, s, line, accent));
    return ic;
}
