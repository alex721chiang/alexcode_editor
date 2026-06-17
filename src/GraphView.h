#pragma once
#include <QWidget>
#include <QVector>
#include <QPointF>
#include <QStringList>
#include <QList>
#include <QPair>

// Obsidian 風關係圖：節點 = .md 檔，邊 = 連結。力導向佈局，點節點開檔。
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

private:
    void relayout();

    struct Node { QString file; QString label; QPointF pos; int degree = 0; };
    QVector<Node> m_nodes;
    QVector<QPair<int, int>> m_edges;     // 節點索引對
    int m_activeIndex = -1;
};
