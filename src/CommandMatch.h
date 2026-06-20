#pragma once
#include <QString>
#include "FilterEngine.h"

// 命令面板的比對評分（純邏輯，可單元測試）。-1 = 不符合；分數越高越靠前。
namespace CommandMatch {

inline int score(const QString& name, const QString& query) {
    const QString q = query.trimmed();
    if (q.isEmpty()) return 0;                                   // 空查詢：全部保留（依原順序）
    if (name.startsWith(q, Qt::CaseInsensitive)) return 400 - name.length();
    if (name.contains(q, Qt::CaseInsensitive))   return 300 - name.length();
    if (FilterEngine::fuzzyContains(name, q))     return 200 - name.length();
    return -1;
}

} // namespace CommandMatch
