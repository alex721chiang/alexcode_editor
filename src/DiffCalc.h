#pragma once
#include <QString>
#include <QStringList>
#include <QList>

// 並排 diff 的行對齊（純邏輯，可單元測試）：以 LCS 對齊兩份文字的行，
// 產生左右成對的「列」。連續的刪除+新增會盡量配對成 Modified（左右各佔一列），
// 落單者以 -1 表示該側為填充列。DiffViewer 據此渲染雙欄。
namespace DiffCalc {

enum RowType { Same, Added, Removed, Modified };

struct Row {
    int left = -1;     // a 的 0-based 行號；-1 = 左側填充
    int right = -1;    // b 的 0-based 行號；-1 = 右側填充
    RowType type = Same;
};

QList<Row> align(const QStringList& a, const QStringList& b);
int changeCount(const QList<Row>& rows);   // 非 Same 的列數

} // namespace DiffCalc
