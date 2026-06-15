#pragma once
#include <QString>
#include <QStringList>
#include <algorithm>

// 終端機文字選取的純邏輯：給定目前顯示的各行文字與選取的起訖（行,欄），
// 取出選取範圍的文字（多行以 \n 連接）。不依賴 GUI，方便單元測試。
namespace TermSelection {

// 將 (r,c) 以閱讀順序正規化，使 a 在 b 之前
inline void normalize(int& r0, int& c0, int& r1, int& c1) {
    if (r0 > r1 || (r0 == r1 && c0 > c1)) {
        std::swap(r0, r1);
        std::swap(c0, c1);
    }
}

inline QString extractText(const QStringList& lines, int r0, int c0, int r1, int c1) {
    if (lines.isEmpty()) return QString();
    normalize(r0, c0, r1, c1);
    r0 = std::max(0, r0);
    r1 = std::min(r1, int(lines.size()) - 1);
    if (r0 > r1) return QString();

    if (r0 == r1)
        return lines[r0].mid(c0, std::max(0, c1 - c0));   // mid 會自動 clamp 到行長

    QString out = lines[r0].mid(c0);
    for (int r = r0 + 1; r < r1; ++r)
        out += QLatin1Char('\n') + lines[r];
    out += QLatin1Char('\n') + lines[r1].left(c1);
    return out;
}

} // namespace TermSelection
