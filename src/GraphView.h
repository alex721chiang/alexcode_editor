#pragma once
#include <QWidget>
#include <QVector>
#include <QPointF>
#include <QStringList>
#include <QList>
#include <QPair>

// Obsidian 風關係圖：節點 = .md 檔，邊 = 連結。力導向佈局；
// 支援 hover 高亮相鄰、拖曳節點、滾輪縮放、空白處拖曳平移，孤立節點淡化。
class GraphView : public QWidget {
    Q_OBJECT
public:
    explicit GraphView(QWidget* parent = nullptr);

    void setGraph(const QStringList& files,
                  const QList<QPair<QString, QString>>& edges,
                  const QString& activeFile = QString());

signals:
    void nodeClicked(const QString& filePath);

protected:
    void paintEvent(QPaintEvent*) override;
    void resizeEvent(QResizeEvent*) override;
    void mousePressEvent(QMouseEvent*) override;
    void mouseMoveEvent(QMouseEvent*) override;
    void mouseReleaseEvent(QMouseEvent*) override;
    void wheelEvent(QWheelEvent*) override;
    void leaveEvent(QEvent*) override;

private:
    void relayout();
    int nodeAt(const QPointF& widgetPos) const;          // 命中測試（-1 = 無）
    QPointF toScene(const QPointF& widgetPos) const;     // widget → scene 座標

    struct Node { QString file; QString label; QPointF pos; int degree = 0; };
    QVector<Node> m_nodes;
    QVector<QPair<int, int>> m_edges;     // 節點索引對
    int m_activeIndex = -1;
    int m_hoverIndex = -1;
    int m_dragIndex = -1;                 // 拖曳中的節點
    bool m_panning = false;
    bool m_moved = false;                 // 區分點擊與拖曳
    bool m_userAdjusted = false;          // 使用者已手動拖曳/縮放 → 不再自動重排
    QPointF m_lastMouse;
    double m_scale = 1.0;
    QPointF m_offset;                     // 平移量
};
