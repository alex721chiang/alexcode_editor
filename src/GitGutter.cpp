#include "GitGutter.h"
#include <QVector>

namespace GitGutter {

// 將一段「舊版刪了 delCount 行、新版加了 addLines」的變更段落轉為行狀態：
// 配對部分標 Modified，多出的新行標 Added，純刪除在下一行標 DeletedAbove。
static void emitChangeRun(QHash<int, int>& states, int newStart,
                          int delCount, int addCount, int newTotal) {
    const int paired = qMin(delCount, addCount);
    for (int k = 0; k < paired; ++k)
        states[newStart + k] |= Modified;
    for (int k = paired; k < addCount; ++k)
        states[newStart + k] |= Added;
    if (delCount > addCount) {                       // 淨刪除：標示於刪除點下一行
        const int mark = qMin(newStart + addCount, newTotal - 1);
        if (mark >= 0) states[mark] |= DeletedAbove;
    }
}

QHash<int, int> diffLineStates(const QStringList& oldLines,
                               const QStringList& newLines,
                               int maxMiddle) {
    QHash<int, int> states;
    const int n = oldLines.size(), m = newLines.size();

    // 共同前綴 / 後綴修剪
    int prefix = 0;
    while (prefix < n && prefix < m && oldLines[prefix] == newLines[prefix])
        ++prefix;
    int suffix = 0;
    while (suffix < n - prefix && suffix < m - prefix &&
           oldLines[n - 1 - suffix] == newLines[m - 1 - suffix])
        ++suffix;

    const int on = n - prefix - suffix;              // 中段長度
    const int om = m - prefix - suffix;
    if (on == 0 && om == 0) return states;           // 無差異
    if (on == 0) {                                   // 純新增
        emitChangeRun(states, prefix, 0, om, m);
        return states;
    }
    if (om == 0) {                                   // 純刪除
        emitChangeRun(states, prefix, on, 0, m);
        return states;
    }
    if (on > maxMiddle || om > maxMiddle) {          // 過大：粗略整段標 Modified
        emitChangeRun(states, prefix, on, om, m);
        return states;
    }

    // 中段 LCS（與 showDiff 同思路）
    QVector<QVector<int>> dp(on + 1, QVector<int>(om + 1, 0));
    for (int i = on - 1; i >= 0; --i)
        for (int j = om - 1; j >= 0; --j)
            dp[i][j] = (oldLines[prefix + i] == newLines[prefix + j])
                           ? dp[i + 1][j + 1] + 1
                           : qMax(dp[i + 1][j], dp[i][j + 1]);

    int i = 0, j = 0;
    int runDel = 0, runAdd = 0, runStart = 0;        // 累積連續變更段
    auto flush = [&]() {
        if (runDel || runAdd)
            emitChangeRun(states, prefix + runStart, runDel, runAdd, m);
        runDel = runAdd = 0;
    };
    while (i < on && j < om) {
        if (oldLines[prefix + i] == newLines[prefix + j]) {
            flush();
            ++i; ++j;
        } else {
            if (runDel == 0 && runAdd == 0) runStart = j;
            if (dp[i + 1][j] >= dp[i][j + 1]) { ++runDel; ++i; }
            else                               { ++runAdd; ++j; }
        }
    }
    if (i < on || j < om) {
        if (runDel == 0 && runAdd == 0) runStart = j;
        runDel += on - i;
        runAdd += om - j;
    }
    flush();
    return states;
}

} // namespace GitGutter
