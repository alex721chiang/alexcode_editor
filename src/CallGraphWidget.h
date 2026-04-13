#pragma once

#include <QDialog>
#include <QGraphicsView>
#include <QGraphicsScene>
#include <QSet>
#include <QString>

class CallGraphWidget : public QDialog {
    Q_OBJECT
public:
    explicit CallGraphWidget(QWidget *parent = nullptr);
    
    // 建立並繪製圖譜
    void buildGraph(const QString& rootNode, const QSet<QString>& dependencies);

private:
    QGraphicsView* view;
    QGraphicsScene* scene;
};
