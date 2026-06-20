#pragma once
#include <QVector>
#include <algorithm>
#include "TsSymbols.h"

// Sticky scroll：決定捲動時要固定在編輯器頂端的符號標頭（純邏輯，可單元測試）。
namespace StickyScroll {

// 回傳「包含 firstVisibleLine、且自身標頭行已捲到可視區上方」的符號鏈（外→內，依起始行升冪）。
// maxDepth 限制最多固定幾層（太深只保留最內層數層）。
inline QVector<TsSymbols::Symbol> headers(const QVector<TsSymbols::Symbol>& syms,
                                          int firstVisibleLine, int maxDepth = 5) {
    QVector<TsSymbols::Symbol> out;
    for (const TsSymbols::Symbol& s : syms)
        if (s.line < firstVisibleLine && firstVisibleLine <= s.endLine)
            out.append(s);
    std::sort(out.begin(), out.end(),
              [](const TsSymbols::Symbol& a, const TsSymbols::Symbol& b) { return a.line < b.line; });
    if (maxDepth > 0 && out.size() > maxDepth)
        out = out.mid(out.size() - maxDepth);
    return out;
}

} // namespace StickyScroll
