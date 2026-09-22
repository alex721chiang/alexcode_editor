#pragma once
#include <QList>
#include <QVector>
#include <algorithm>

// 矩形（欄位/box/column）選取的純邏輯：給定錨點與目前點（行,欄）及各行長度，
// 算出每一行要選取的欄位範圍 [start, end)。不依賴 GUI，方便單元測試。
namespace BoxSelect {

struct Range {
    int line;
    int start;
    int end;          // 半開區間 [start, end)；start==end 表示零寬（純插入點）
    bool operator==(const Range& o) const {
        return line == o.line && start == o.start && end == o.end;
    }
};

// lineLengths[i] = 第 i 行字元數。超出長度的欄位會 clamp 到行尾（短行）。
inline QList<Range> compute(int anchorLine, int anchorCol,
                            int curLine, int curCol,
                            const QVector<int>& lineLengths) {
    const int l0 = std::min(anchorLine, curLine);
    const int l1 = std::max(anchorLine, curLine);
    const int c0 = std::min(anchorCol, curCol);
    const int c1 = std::max(anchorCol, curCol);
    QList<Range> out;
    for (int l = l0; l <= l1; ++l) {
        const int len = (l >= 0 && l < lineLengths.size()) ? lineLengths[l] : 0;
        const int s = std::min(c0, len);
        const int e = std::min(c1, len);
        out.append({ l, s, e });
    }
    return out;
}

} // namespace BoxSelect
