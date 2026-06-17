#pragma once
#include <QString>
#include <QStringList>
#include <QList>
#include <QRegularExpression>

// Markdown 連結解析（純邏輯，可單元測試）。
// 支援 Obsidian wikilink [[Note]]、[[Note|alias]]、[[Note#heading]]
// 以及一般 Markdown 連結 [text](path.md)（略過 http(s)/mailto/純錨點）。
namespace MdLink {

struct Ref {
    QString target;   // 連結目標（已去掉 alias/heading）
    bool wiki;        // true = wikilink；false = [text](path)
};

// 去掉圍欄程式碼區塊（``` … ```）與行內程式碼（`…`），避免誤抓其中的中括號。
inline QString stripCode(const QString& text) {
    QString out;
    out.reserve(text.size());
    const QStringList lines = text.split(QLatin1Char('\n'));
    bool inFence = false;
    for (const QString& line : lines) {
        const QString t = line.trimmed();
        if (t.startsWith(QLatin1String("```")) || t.startsWith(QLatin1String("~~~"))) {
            inFence = !inFence;
            out += QLatin1Char('\n');
            continue;
        }
        if (inFence) { out += QLatin1Char('\n'); continue; }
        // 移除行內程式碼 `…`
        QString cleaned;
        bool inCode = false;
        for (QChar c : line) {
            if (c == QLatin1Char('`')) { inCode = !inCode; continue; }
            cleaned += inCode ? QLatin1Char(' ') : c;
        }
        out += cleaned;
        out += QLatin1Char('\n');
    }
    return out;
}

// 目標標準化為比對鍵：取檔名（去路徑）、去 .md 副檔名、小寫、去頭尾空白。
inline QString normalizeKey(const QString& target) {
    QString t = target.trimmed();
    const int slash = qMax(t.lastIndexOf(QLatin1Char('/')), t.lastIndexOf(QLatin1Char('\\')));
    if (slash >= 0) t = t.mid(slash + 1);
    if (t.endsWith(QLatin1String(".md"), Qt::CaseInsensitive)) t.chop(3);
    return t.trimmed().toLower();
}

// 從 markdown 內文擷取連結參照。
inline QList<Ref> extractRefs(const QString& text) {
    const QString src = stripCode(text);
    QList<Ref> refs;

    // wikilink：[[ … ]]，內部去掉 |alias 與 #heading
    static const QRegularExpression wiki(QStringLiteral("\\[\\[([^\\]\\[]+)\\]\\]"));
    auto it = wiki.globalMatch(src);
    while (it.hasNext()) {
        QString inner = it.next().captured(1);
        const int bar = inner.indexOf(QLatin1Char('|'));
        if (bar >= 0) inner = inner.left(bar);
        const int hash = inner.indexOf(QLatin1Char('#'));
        if (hash >= 0) inner = inner.left(hash);
        inner = inner.trimmed();
        if (!inner.isEmpty()) refs.append({inner, true});
    }

    // [text](url)：略過 http(s)://、mailto:、純錨點 #...
    static const QRegularExpression md(QStringLiteral("\\[[^\\]]*\\]\\(([^)]+)\\)"));
    auto it2 = md.globalMatch(src);
    while (it2.hasNext()) {
        QString url = it2.next().captured(1).trimmed();
        const int hash = url.indexOf(QLatin1Char('#'));
        if (hash >= 0) url = url.left(hash);          // 去掉錨點
        if (url.isEmpty()) continue;
        if (url.contains(QLatin1String("://"))) continue;
        if (url.startsWith(QLatin1String("mailto:"), Qt::CaseInsensitive)) continue;
        refs.append({url, false});
    }
    return refs;
}

} // namespace MdLink
