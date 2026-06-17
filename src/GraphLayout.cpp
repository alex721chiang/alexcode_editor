#include "GraphLayout.h"
#include <QtMath>
#include <algorithm>

namespace GraphLayout {

QVector<QPointF> compute(int n, const QVector<QPair<int, int>>& edges,
                         double width, double height, int iterations) {
    QVector<QPointF> pos(n);
    if (n <= 0) return pos;
    if (n == 1) { pos[0] = QPointF(width / 2.0, height / 2.0); return pos; }

    // 決定性初始位置：固定圓周
    const double cx = width / 2.0, cy = height / 2.0;
    const double r0 = std::min(width, height) / 3.0;
    for (int i = 0; i < n; ++i) {
        const double a = 2.0 * M_PI * i / n;
        pos[i] = QPointF(cx + r0 * std::cos(a), cy + r0 * std::sin(a));
    }

    const double area = width * height;
    const double k = std::sqrt(area / n);                 // 理想間距
    double t = std::min(width, height) / 10.0;            // 溫度（最大位移）
    const double cool = t / (iterations + 1);

    QVector<QPointF> disp(n);
    for (int iter = 0; iter < iterations; ++iter) {
        for (int i = 0; i < n; ++i) disp[i] = QPointF(0, 0);

        // 斥力：所有節點兩兩相斥
        for (int i = 0; i < n; ++i) {
            for (int j = i + 1; j < n; ++j) {
                QPointF d = pos[i] - pos[j];
                double dist = std::hypot(d.x(), d.y());
                if (dist < 0.01) { d = QPointF(0.01 * (i - j), 0.01); dist = std::hypot(d.x(), d.y()); }
                const double force = (k * k) / dist;       // fr = k^2/d
                const QPointF u = d / dist;
                disp[i] += u * force;
                disp[j] -= u * force;
            }
        }

        // 引力：有連結的節點互相吸引
        for (const auto& e : edges) {
            if (e.first < 0 || e.first >= n || e.second < 0 || e.second >= n) continue;
            if (e.first == e.second) continue;
            QPointF d = pos[e.first] - pos[e.second];
            double dist = std::hypot(d.x(), d.y());
            if (dist < 0.01) dist = 0.01;
            const double force = (dist * dist) / k;        // fa = d^2/k
            const QPointF u = d / dist;
            disp[e.first]  -= u * force;
            disp[e.second] += u * force;
        }

        // 套用位移（限制在溫度內），並夾在畫布範圍
        for (int i = 0; i < n; ++i) {
            double dl = std::hypot(disp[i].x(), disp[i].y());
            if (dl < 0.01) continue;
            const QPointF step = (disp[i] / dl) * std::min(dl, t);
            pos[i] += step;
            pos[i].setX(std::clamp(pos[i].x(), 0.0, width));
            pos[i].setY(std::clamp(pos[i].y(), 0.0, height));
        }
        t = std::max(t - cool, 1.0);
    }

    // 正規化：縮放至帶邊距的畫布
    double minX = pos[0].x(), maxX = pos[0].x(), minY = pos[0].y(), maxY = pos[0].y();
    for (const QPointF& p : pos) {
        minX = std::min(minX, p.x()); maxX = std::max(maxX, p.x());
        minY = std::min(minY, p.y()); maxY = std::max(maxY, p.y());
    }
    const double margin = std::min(width, height) * 0.1 + 20.0;
    const double spanX = std::max(maxX - minX, 1.0);
    const double spanY = std::max(maxY - minY, 1.0);
    const double sx = (width  - 2 * margin) / spanX;
    const double sy = (height - 2 * margin) / spanY;
    const double s = std::min(sx, sy);
    for (QPointF& p : pos) {
        p.setX(margin + (p.x() - minX) * s);
        p.setY(margin + (p.y() - minY) * s);
    }
    return pos;
}

} // namespace GraphLayout
