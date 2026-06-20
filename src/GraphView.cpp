#include "GraphView.h"
#include "GraphLayout.h"
#include "Theme.h"
#include <QPainter>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QFileInfo>
#include <QHash>
#include <QSet>
#include <cmath>

GraphView::GraphView(QWidget* parent) : QWidget(parent) {
    setMinimumSize(320, 240);
    setMouseTracking(true);
}

void GraphView::setGraph(const QStringList& files,
                         const QList<QPair<QString, QString>>& edges,
                         const QString& activeFile) {
    m_nodes.clear();
    m_edges.clear();
    m_activeIndex = m_hoverIndex = m_dragIndex = -1;
    m_scale = 1.0;
    m_offset = QPointF(0, 0);
    m_userAdjusted = false;

    QHash<QString, int> indexOf;
    for (const QString& f : files) {
        indexOf.insert(f, m_nodes.size());
        Node n;
        n.file = f;
        n.label = QFileInfo(f).completeBaseName();
        m_nodes.append(n);
    }
    if (!activeFile.isEmpty()) m_activeIndex = indexOf.value(activeFile, -1);

    for (const auto& e : edges) {
        const int a = indexOf.value(e.first, -1);
        const int b = indexOf.value(e.second, -1);
        if (a < 0 || b < 0 || a == b) continue;
        m_edges.append({a, b});
        m_nodes[a].degree++;
        m_nodes[b].degree++;
    }
    relayout();
    update();
}

void GraphView::relayout() {
    if (m_nodes.isEmpty()) return;
    const double w = std::max(width(), 200);             // 用實際寬高，初始視圖即填滿（之後可縮放/平移）
    const double h = std::max(height(), 200);
    const QVector<QPointF> pos = GraphLayout::compute(m_nodes.size(), m_edges, w, h);
    for (int i = 0; i < m_nodes.size() && i < pos.size(); ++i)
        m_nodes[i].pos = pos[i];
}

void GraphView::resizeEvent(QResizeEvent*) {
    if (!m_userAdjusted) {                                // 尚未手動調整 → 依新尺寸重排以填滿
        relayout();
        m_scale = 1.0;
        m_offset = QPointF(0, 0);
    }
    update();
}

static double nodeRadius(int degree) {
    return 6.0 + std::min(degree, 8) * 1.6;
}

QPointF GraphView::toScene(const QPointF& widgetPos) const {
    return (widgetPos - m_offset) / m_scale;
}

int GraphView::nodeAt(const QPointF& widgetPos) const {
    const QPointF s = toScene(widgetPos);
    for (int i = 0; i < m_nodes.size(); ++i) {
        const QPointF d = m_nodes[i].pos - s;
        if (std::hypot(d.x(), d.y()) <= nodeRadius(m_nodes[i].degree) + 4)
            return i;
    }
    return -1;
}

void GraphView::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.fillRect(rect(), QColor(Theme::EDITOR_BG));

    if (m_nodes.isEmpty()) {
        p.setPen(QColor(Theme::SYN_COMMENT));
        p.drawText(rect(), Qt::AlignCenter, tr("沒有 Markdown 檔（請先開啟含 .md 的資料夾）"));
        return;
    }

    p.translate(m_offset);
    p.scale(m_scale, m_scale);

    // hover 時要強調的節點集合（自己 + 相鄰）
    QSet<int> emphasised;
    if (m_hoverIndex >= 0) {
        emphasised.insert(m_hoverIndex);
        for (const auto& e : m_edges) {
            if (e.first == m_hoverIndex) emphasised.insert(e.second);
            if (e.second == m_hoverIndex) emphasised.insert(e.first);
        }
    }
    const bool hovering = m_hoverIndex >= 0;

    // 邊
    for (const auto& e : m_edges) {
        const bool active = !hovering || e.first == m_hoverIndex || e.second == m_hoverIndex;
        QColor c(Theme::SYN_COMMENT);
        c.setAlpha(active ? 200 : 50);
        p.setPen(QPen(c, active && hovering ? 1.6 : 1.0));
        p.drawLine(m_nodes[e.first].pos, m_nodes[e.second].pos);
    }

    // 節點
    const QColor accent(Theme::SYN_KEYWORD);
    const QColor activeCol(Theme::SYN_PREPROC);
    const QColor dot(Theme::SYN_TYPE);
    const QColor isolated(Theme::SYN_COMMENT);
    QFont f = p.font();
    f.setPointSizeF(std::max(6.0, f.pointSizeF() - 0.5));
    p.setFont(f);
    for (int i = 0; i < m_nodes.size(); ++i) {
        const Node& n = m_nodes[i];
        const double r = nodeRadius(n.degree);
        const bool isActive = (i == m_activeIndex);
        const bool dim = hovering && !emphasised.contains(i);
        QColor fill = isActive ? activeCol : (n.degree == 0 ? isolated : dot);
        QColor pen = isActive ? activeCol : accent;
        if (dim) { fill.setAlpha(50); pen.setAlpha(50); }
        p.setBrush(fill);
        p.setPen(QPen(pen, isActive ? 2.0 : 1.0));
        p.drawEllipse(n.pos, r, r);
        QColor textCol(Theme::EDITOR_FG);
        if (dim) textCol.setAlpha(60);
        p.setPen(textCol);
        p.drawText(QPointF(n.pos.x() + r + 3, n.pos.y() + 4), n.label);
    }
}

void GraphView::mousePressEvent(QMouseEvent* e) {
    m_moved = false;
    if (e->button() == Qt::LeftButton) {
        m_dragIndex = nodeAt(e->position());
        if (m_dragIndex < 0) { m_panning = true; m_lastMouse = e->position(); }
    }
}

void GraphView::mouseMoveEvent(QMouseEvent* e) {
    if (m_dragIndex >= 0) {                               // 拖曳節點
        m_nodes[m_dragIndex].pos = toScene(e->position());
        m_moved = true;
        m_userAdjusted = true;
        update();
        return;
    }
    if (m_panning) {                                     // 平移畫布
        m_offset += e->position() - m_lastMouse;
        m_lastMouse = e->position();
        m_moved = true;
        m_userAdjusted = true;
        update();
        return;
    }
    const int h = nodeAt(e->position());                  // hover 偵測
    if (h != m_hoverIndex) { m_hoverIndex = h; update(); }
}

void GraphView::mouseReleaseEvent(QMouseEvent* e) {
    if (m_dragIndex >= 0 && !m_moved)                     // 純點擊節點 → 開檔
        emit nodeClicked(m_nodes[m_dragIndex].file);
    m_dragIndex = -1;
    m_panning = false;
    Q_UNUSED(e);
}

void GraphView::wheelEvent(QWheelEvent* e) {
    const double factor = e->angleDelta().y() > 0 ? 1.15 : 1.0 / 1.15;
    const double newScale = std::clamp(m_scale * factor, 0.2, 5.0);
    const QPointF mouse = e->position();
    const QPointF scenePt = (mouse - m_offset) / m_scale; // 以游標為錨點縮放
    m_scale = newScale;
    m_offset = mouse - scenePt * m_scale;
    m_userAdjusted = true;
    update();
}

void GraphView::leaveEvent(QEvent*) {
    if (m_hoverIndex != -1) { m_hoverIndex = -1; update(); }
}
