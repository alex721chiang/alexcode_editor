#include "CallGraphWidget.h"
#include <QVBoxLayout>
#include <QGraphicsEllipseItem>
#include <QGraphicsTextItem>
#include <QGraphicsLineItem>
#include <cmath>

CallGraphWidget::CallGraphWidget(QWidget *parent) : QDialog(parent) {
    setWindowTitle("Call Graph (Navigation 2.0)");
    resize(700, 500);
    
    QVBoxLayout* layout = new QVBoxLayout(this);
    scene = new QGraphicsScene(this);
    view = new QGraphicsView(scene, this);
    view->setRenderHint(QPainter::Antialiasing); // 開啟反鋸齒，圖形更平滑
    layout->addWidget(view);
}

void CallGraphWidget::buildGraph(const QString& rootNode, const QSet<QString>& dependencies) {
    scene->clear();
    
    int centerX = 0;
    int centerY = 0;
    int radius = 150;

    // 繪製中心節點 (Root)
    QGraphicsEllipseItem* rootItem = scene->addEllipse(centerX - 50, centerY - 25, 100, 50, QPen(Qt::black), QBrush(Qt::yellow));
    rootItem->setZValue(1);
    QGraphicsTextItem* rootText = scene->addText(rootNode);
    rootText->setDefaultTextColor(Qt::black);
    rootText->setPos(centerX - rootText->boundingRect().width() / 2, centerY - rootText->boundingRect().height() / 2);
    rootText->setZValue(2);

    if (dependencies.isEmpty()) return;

    // 計算周圍節點的角度分佈
    double angleStep = 2 * M_PI / dependencies.size();
    double currentAngle = 0;

    for (const QString& dep : dependencies) {
        int nodeX = centerX + radius * std::cos(currentAngle);
        int nodeY = centerY + radius * std::sin(currentAngle);

        // 繪製連線
        QGraphicsLineItem* line = scene->addLine(centerX, centerY, nodeX, nodeY, QPen(Qt::gray));
        line->setZValue(-1); // 讓線條在節點下方

        // 繪製依賴節點 (Callees)
        QGraphicsEllipseItem* nodeItem = scene->addEllipse(nodeX - 45, nodeY - 20, 90, 40, QPen(Qt::darkBlue), QBrush(QColor(173, 216, 230))); // 淺藍色
        nodeItem->setZValue(1);
        QGraphicsTextItem* nodeText = scene->addText(dep);
        nodeText->setDefaultTextColor(Qt::black);
        nodeText->setPos(nodeX - nodeText->boundingRect().width() / 2, nodeY - nodeText->boundingRect().height() / 2);
        nodeText->setZValue(2);

        currentAngle += angleStep;
    }
}
