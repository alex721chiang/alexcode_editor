#include "DiffCalc.h"
#include <QVector>

namespace {

// 把累積的刪除/新增區塊配對成 Modified（一刪配一增），剩餘落單
void flushPending(QList<DiffCalc::Row>& rows, QList<int>& dels, QList<int>& adds) {
    const int paired = qMin(dels.size(), adds.size());
    for (int t = 0; t < paired; ++t)
        rows.append({ dels[t], adds[t], DiffCalc::Modified });
    for (int t = paired; t < dels.size(); ++t)
        rows.append({ dels[t], -1, DiffCalc::Removed });
    for (int t = paired; t < adds.size(); ++t)
        rows.append({ -1, adds[t], DiffCalc::Added });
    dels.clear();
    adds.clear();
}

} // namespace

QList<DiffCalc::Row> DiffCalc::align(const QStringList& a, const QStringList& b) {
    const int n = a.size(), m = b.size();
    // LCS 動態規劃（與統一 diff 相同做法；上限保護由呼叫端負責）
    QVector<QVector<int>> dp(n + 1, QVector<int>(m + 1, 0));
    for (int i = n - 1; i >= 0; --i)
        for (int j = m - 1; j >= 0; --j)
            dp[i][j] = (a[i] == b[j]) ? dp[i+1][j+1] + 1 : qMax(dp[i+1][j], dp[i][j+1]);

    QList<Row> rows;
    QList<int> dels, adds;
    int i = 0, j = 0;
    while (i < n && j < m) {
        if (a[i] == b[j]) {
            flushPending(rows, dels, adds);
            rows.append({ i++, j++, Same });
        } else if (dp[i+1][j] >= dp[i][j+1]) {
            dels.append(i++);
        } else {
            adds.append(j++);
        }
    }
    while (i < n) dels.append(i++);
    while (j < m) adds.append(j++);
    flushPending(rows, dels, adds);
    return rows;
}

int DiffCalc::changeCount(const QList<Row>& rows) {
    int c = 0;
    for (const Row& r : rows)
        if (r.type != Same) ++c;
    return c;
}
