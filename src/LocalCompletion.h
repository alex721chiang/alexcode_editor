#pragma once
#include <QString>
#include <QStringList>
#include <QHash>
#include <QRegularExpression>
#include <QList>
#include <algorithm>

// 本地（離線、不需網路/金鑰）智慧補全：從目前文件擷取識別字，
// 依「前綴 > 模糊子序列」比對種類、出現頻率、與游標就近度排序候選。純邏輯，可單元測試。
namespace LocalCompletion {

// 子序列模糊比對（不分大小寫）：needle 的字元依序出現在 hay 中
inline bool fuzzy(const QString& hay, const QString& needle) {
    if (needle.isEmpty()) return true;
    int j = 0;
    for (int i = 0; i < hay.size() && j < needle.size(); ++i)
        if (hay[i].toLower() == needle[j].toLower()) ++j;
    return j == needle.size();
}

inline QStringList suggest(const QString& text, int cursorOffset,
                           const QString& prefix, int maxItems = 30) {
    static const QRegularExpression idRe(QStringLiteral("[A-Za-z_][A-Za-z0-9_]*"));
    struct Info { int count = 0; int minDist = INT_MAX; };
    QHash<QString, Info> ids;
    auto it = idRe.globalMatch(text);
    while (it.hasNext()) {
        const auto m = it.next();
        const QString id = m.captured();
        Info& info = ids[id];
        info.count++;
        info.minDist = std::min(info.minDist, int(qAbs(m.capturedStart() - cursorOffset)));
    }

    struct Cand { QString text; qint64 score; };
    QList<Cand> cands;
    for (auto i = ids.constBegin(); i != ids.constEnd(); ++i) {
        const QString& id = i.key();
        if (id == prefix) continue;                       // 不建議「補成自己」
        int kind = 0;                                     // 2=前綴, 1=模糊
        if (prefix.isEmpty())                       kind = 2;
        else if (id.startsWith(prefix, Qt::CaseInsensitive)) kind = 2;
        else if (fuzzy(id, prefix))                 kind = 1;
        if (kind == 0) continue;
        const qint64 proximity = 1000 - std::min(i.value().minDist, 1000);
        const qint64 score = qint64(kind) * 1000000000LL
                           + qint64(i.value().count) * 100000LL
                           + proximity;
        cands.append({ id, score });
    }
    std::sort(cands.begin(), cands.end(), [](const Cand& a, const Cand& b) {
        if (a.score != b.score) return a.score > b.score;
        return a.text < b.text;                           // 分數相同時依字典序，結果穩定
    });
    QStringList out;
    for (const Cand& c : cands) {
        if (out.size() >= maxItems) break;
        out.append(c.text);
    }
    return out;
}

} // namespace LocalCompletion
