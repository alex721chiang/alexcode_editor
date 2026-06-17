#pragma once
#include <QVector>
#include <QPointF>
#include <QPair>

// 力導向圖佈局（Fruchterman-Reingold）。純邏輯、可單元測試。
// 初始位置以固定圓周排列 → 結果具決定性（同輸入同輸出）。
namespace GraphLayout {

// n 個節點、edges 為節點索引對；回傳縮放至 [margin, w-margin]×[margin, h-margin] 的座標。
QVector<QPointF> compute(int n, const QVector<QPair<int, int>>& edges,
                         double width, double height, int iterations = 300);

} // namespace GraphLayout
