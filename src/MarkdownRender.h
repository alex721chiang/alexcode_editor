#pragma once
#include <QString>
#include <QStringList>
#include <QUrl>
#include <QRegularExpression>

// 高擬真 Markdown 渲染輔助（純邏輯，可單元測試）。
// 1) 把 [[wikilink]] 預處理成內部連結 [label](alexcode:Target)，讓預覽可點擊導覽。
// 2) 提供 GitHub 風深色 CSS，套在 QTextBrowser 上做高擬真排版。
namespace MarkdownRender {

inline QString wikilinkUrl(const QString& target) {
    return QStringLiteral("alexcode:") + QString::fromUtf8(QUrl::toPercentEncoding(target.trimmed()));
}

// 把一段「非程式碼」文字中的 [[Target]] / [[Target|alias]] / [[Target#h]] 轉成 markdown 連結。
inline QString replaceWikilinksInText(const QString& seg) {
    static const QRegularExpression re(QStringLiteral("\\[\\[([^\\]\\[]+)\\]\\]"));
    QString out;
    int last = 0;
    auto it = re.globalMatch(seg);
    while (it.hasNext()) {
        const auto m = it.next();
        out += seg.mid(last, m.capturedStart() - last);
        QString inner = m.captured(1);
        QString target = inner, label = inner;
        const int bar = inner.indexOf(QLatin1Char('|'));
        if (bar >= 0) { target = inner.left(bar); label = inner.mid(bar + 1); }
        const int hash = target.indexOf(QLatin1Char('#'));
        if (hash >= 0) target = target.left(hash);
        out += QStringLiteral("[%1](%2)").arg(label.trimmed(), wikilinkUrl(target));
        last = m.capturedEnd();
    }
    out += seg.mid(last);
    return out;
}

// 預處理整份文件：略過圍欄程式碼區塊與行內 `code`，只轉換一般文字中的 wikilink。
inline QString preprocessWikilinks(const QString& md) {
    const QStringList lines = md.split(QLatin1Char('\n'));
    QString out;
    bool inFence = false;
    for (int li = 0; li < lines.size(); ++li) {
        const QString& line = lines[li];
        const QString t = line.trimmed();
        if (t.startsWith(QLatin1String("```")) || t.startsWith(QLatin1String("~~~"))) {
            inFence = !inFence;
            out += line;
            if (li + 1 < lines.size()) out += QLatin1Char('\n');
            continue;
        }
        if (inFence) {
            out += line;
            if (li + 1 < lines.size()) out += QLatin1Char('\n');
            continue;
        }
        // 以反引號切出行內程式碼片段，只轉換非程式碼段
        QString rebuilt;
        bool inCode = false;
        QString seg;
        for (QChar c : line) {
            if (c == QLatin1Char('`')) {
                if (inCode) { rebuilt += QLatin1Char('`') + seg + QLatin1Char('`'); }
                else        { rebuilt += replaceWikilinksInText(seg); }
                seg.clear();
                inCode = !inCode;
            } else {
                seg += c;
            }
        }
        rebuilt += inCode ? (QLatin1Char('`') + seg) : replaceWikilinksInText(seg);
        out += rebuilt;
        if (li + 1 < lines.size()) out += QLatin1Char('\n');
    }
    return out;
}

// GitHub 風深色樣式表（QTextBrowser 支援的 CSS 子集）。
inline QString styleSheet() {
    return QStringLiteral(
        "body { font-family: 'Segoe UI','Microsoft JhengHei',sans-serif; color:#d6e4ff; line-height:1.6; }"
        "h1 { font-size:22pt; color:#9ad7ff; border-bottom:1px solid #2a3a5a; }"
        "h2 { font-size:18pt; color:#9ad7ff; border-bottom:1px solid #2a3a5a; }"
        "h3 { font-size:15pt; color:#8fb8ff; }"
        "h4,h5,h6 { color:#8fb8ff; }"
        "a { color:#00e5ff; text-decoration:none; }"
        "code { background-color:#11203a; color:#ffd49a; font-family:'Consolas',monospace; }"
        "pre { background-color:#0d1830; color:#cfe3ff; padding:8px; }"
        "blockquote { color:#9fb0cc; border-left:3px solid #3d5a80; padding-left:10px; }"
        "table { border-collapse:collapse; }"
        "th { background-color:#13233f; border:1px solid #2a3a5a; padding:4px 8px; }"
        "td { border:1px solid #2a3a5a; padding:4px 8px; }"
        "hr { border:1px solid #2a3a5a; }");
}

} // namespace MarkdownRender
