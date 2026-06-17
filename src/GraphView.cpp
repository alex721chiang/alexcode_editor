#include "GraphView.h"
#include "GraphLayout.h"
#include "Theme.h"
#include <QPainter>
#include <QMouseEvent>
#include <QFileInfo>
#include <QHash>
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
    m_activeIndex = -1;

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
    const double w = std::max(width(), 320);
    const double h = std::max(height(), 240);
    const QVector<QPointF> pos = GraphLayout::compute(m_nodes.size(), m_edges, w, h);
    for (int i = 0; i < m_nodes.size() && i < pos.size(); ++i)
        m_nodes[i].pos = pos[i];
}

void GraphView::resizeEvent(QResizeEvent*) {
    relayout();
}

static double nodeRadius(int degree) {
    return 6.0 + std::min(degree, 8) * 1.6;
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

    // 邊
    p.setPen(QPen(QColor(Theme::SYN_COMMENT), 1.0));
    for (const auto& e : m_edges)
        p.drawLine(m_nodes[e.first].pos, m_nodes[e.second].pos);

    // 節點
    const QColor accent(Theme::SYN_KEYWORD);
    const QColor active(Theme::SYN_PREPROC);
    const QColor dot(Theme::SYN_TYPE);
    QFont f = p.font();
    f.setPointSizeF(f.pointSizeF() - 0.5);
    p.setFont(f);
    for (int i = 0; i < m_nodes.size(); ++i) {
        const Node& n = m_nodes[i];
        const double r = nodeRadius(n.degree);
        const bool isActive = (i == m_activeIndex);
        p.setBrush(isActive ? active : dot);
        p.setPen(QPen(isActive ? active : accent, isActive ? 2.0 : 1.0));
        p.drawEllipse(n.pos, r, r);
        p.setPen(QColor(Theme::EDITOR_FG));
        p.drawText(QPointF(n.pos.x() + r + 3, n.pos.y() + 4), n.label);
    }
}

void GraphView::mousePressEvent(QMouseEvent* e) {
    const QPointF click = e->position();
    for (int i = 0; i < m_nodes.size(); ++i) {
        const double r = nodeRadius(m_nodes[i].degree) + 4;
        const QPointF d = m_nodes[i].pos - click;
        if (std::hypot(d.x(), d.y()) <= r) {
            emit nodeClicked(m_nodes[i].file);
            return;
        }
    }
    QWidget::mousePressEvent(e);
}
