#pragma once
#include <QWidget>
#include <QPainter>
#include <QMouseEvent>
#include <QList>

// 5.6 時間軸密度條：以橫向長條呈現篩選命中於整份文件中的分布密度。
// 點擊任一位置跳至文件對應區段（發出 jumpToLine）。
class TimelineBar : public QWidget {
    Q_OBJECT
public:
    explicit TimelineBar(QWidget* parent = nullptr) : QWidget(parent) {
        setFixedHeight(18);
        setToolTip(tr("命中分布（點擊跳至該區段）"));
        setCursor(Qt::PointingHandCursor);
    }

    void setData(const QList<int>& matchedLines, int totalLines) {
        m_lines = matchedLines;
        m_total = qMax(totalLines, 1);
        update();
    }
    void clearData() { m_lines.clear(); m_total = 1; update(); }

signals:
    void jumpToLine(int line);              // 0-based

protected:
    void paintEvent(QPaintEvent*) override {
        QPainter p(this);
        p.fillRect(rect(), QColor("#10141f"));
        if (m_lines.isEmpty()) return;

        // 分桶統計：每個像素欄一桶，亮度 ∝ 命中數
        const int w = qMax(width(), 1);
        QVector<int> buckets(w, 0);
        int maxCount = 1;
        for (int line : m_lines) {
            const int b = qMin(int(qint64(line) * w / m_total), w - 1);
            maxCount = qMax(maxCount, ++buckets[b]);
        }
        for (int x = 0; x < w; ++x) {
            if (!buckets[x]) continue;
            QColor c("#00e5ff");
            c.setAlphaF(0.35 + 0.65 * buckets[x] / maxCount);
            p.fillRect(x, 2, 1, height() - 4, c);
        }
    }

    void mousePressEvent(QMouseEvent* e) override {
        if (m_total > 0 && width() > 0)
            emit jumpToLine(int(qint64(e->pos().x()) * m_total / width()));
    }

private:
    QList<int> m_lines;
    int m_total = 1;
};
