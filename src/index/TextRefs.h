#pragma once
#include <QString>
#include <QVector>

// 文字版「找引用」：在內容中找出某符號名的整字出現處（純邏輯，可單元測試）。
// 近似（不做語意分析）：同名字詞的所有出現，含定義與呼叫處。
namespace TextRefs {

struct Hit {
    int line;        // 0-based
    QString text;    // 該行文字（trim 後）
};

inline bool isWordChar(QChar c) { return c.isLetterOrNumber() || c == QLatin1Char('_'); }

inline QVector<Hit> findWholeWord(const QString& content, const QString& name) {
    QVector<Hit> hits;
    if (name.isEmpty()) return hits;
    const QStringList lines = content.split(QLatin1Char('\n'));
    for (int li = 0; li < lines.size(); ++li) {
        const QString& line = lines[li];
        int from = 0;
        bool added = false;
        while (true) {
            const int idx = line.indexOf(name, from);
            if (idx < 0) break;
            const bool leftOk  = idx == 0 || !isWordChar(line.at(idx - 1));
            const bool rightOk = idx + name.size() >= line.size()
                              || !isWordChar(line.at(idx + name.size()));
            if (leftOk && rightOk && !added) {
                hits.append({ li, line.trimmed() });     // 一行只記一次
                added = true;
            }
            from = idx + name.size();
        }
    }
    return hits;
}

} // namespace TextRefs
