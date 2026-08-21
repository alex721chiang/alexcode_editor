#include "HudOverlay.h"
#include "Theme.h"
#include <QPainter>
#include <QPen>
#include <QColor>

HudOverlay::HudOverlay(QWidget* parent) : QWidget(parent) {
    setAttribute(Qt::WA_TransparentForMouseEvents);   // 完全不吃滑鼠，點擊穿透到底下的編輯器
    setAttribute(Qt::WA_NoSystemBackground);
    setAttribute(Qt::WA_TranslucentBackground);
    setFocusPolicy(Qt::NoFocus);
}

void HudOverlay::paintEvent(QPaintEvent*) {
    if (Theme::currentThemeName != QStringLiteral("Neon HUD")) return;   // 僅此主題顯示

    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);

    const QColor accent(Theme::ACCENT);
    const int m = 9;      // 距邊界內縮
    const int len = 15;   // 角標臂長
    const QRect r = rect().adjusted(m, m, -m - 1, -m - 1);
    if (r.width() < 4 * len || r.height() < 4 * len) return;   // 太小就不畫，避免雜訊

    QColor tick = accent; tick.setAlpha(205);
    QPen tp(tick, 2);
    tp.setCapStyle(Qt::FlatCap);
    p.setPen(tp);

    const int L = r.left(), T = r.top(), R = r.right(), B = r.bottom();
    // 左上
    p.drawLine(L, T, L + len, T);   p.drawLine(L, T, L, T + len);
    // 右上
    p.drawLine(R, T, R - len, T);   p.drawLine(R, T, R, T + len);
    // 左下
    p.drawLine(L, B, L + len, B);   p.drawLine(L, B, L, B - len);
    // 右下
    p.drawLine(R, B, R - len, B);   p.drawLine(R, B, R, B - len);
}
