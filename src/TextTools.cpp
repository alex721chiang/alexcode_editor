#include "TextTools.h"
#include <QStringList>
#include <QSet>
#include <QRegularExpression>
#include <algorithm>

namespace TextTools {

QString sortLines(const QString& text, bool descending) {
    QStringList lines = text.split('\n');
    if (descending) std::sort(lines.begin(), lines.end(), std::greater<QString>());
    else            std::sort(lines.begin(), lines.end());
    return lines.join('\n');
}

QString removeDuplicateLines(const QString& text) {
    QStringList out;
    QSet<QString> seen;
    for (const QString& l : text.split('\n'))
        if (!seen.contains(l)) { seen.insert(l); out << l; }
    return out.join('\n');
}

QString removeBlankLines(const QString& text) {
    QStringList out;
    for (const QString& l : text.split('\n'))
        if (!l.trimmed().isEmpty()) out << l;
    return out.join('\n');
}

QString reverseLines(const QString& text) {
    QStringList lines = text.split('\n');
    std::reverse(lines.begin(), lines.end());
    return lines.join('\n');
}

QString trimTrailingWhitespace(const QString& text) {
    static const QRegularExpression re(QStringLiteral("[ \\t]+(?=\\n)|[ \\t]+$"));
    QString r = text;
    r.remove(re);
    return r;
}

QString toHalfWidth(const QString& text) {
    QString out;
    out.reserve(text.size());
    for (QChar c : text) {
        const ushort u = c.unicode();
        if (u == 0x3000) out += QChar(' ');
        else if (u >= 0xFF01 && u <= 0xFF5E) out += QChar(ushort(u - 0xFEE0));
        else out += c;
    }
    return out;
}

QString toFullWidth(const QString& text) {
    QString out;
    out.reserve(text.size());
    for (QChar c : text) {
        const ushort u = c.unicode();
        if (u == ' ') out += QChar(0x3000);
        else if (u >= 0x21 && u <= 0x7E) out += QChar(ushort(u + 0xFEE0));
        else out += c;
    }
    return out;
}

QString unicodeEscape(const QString& text) {
    QString out;
    for (QChar c : text) {
        if (c.unicode() < 0x80) out += c;
        else out += QStringLiteral("\\u%1").arg(c.unicode(), 4, 16, QLatin1Char('0'));
    }
    return out;
}

QString unicodeUnescape(const QString& text) {
    static const QRegularExpression re(QStringLiteral("\\\\u([0-9a-fA-F]{4})"));
    QString out = text;
    int from = 0;
    QRegularExpressionMatch m;
    while ((m = re.match(out, from)).hasMatch()) {
        out.replace(m.capturedStart(), 6, QChar(ushort(m.captured(1).toUInt(nullptr, 16))));
        from = m.capturedStart() + 1;
    }
    return out;
}

bool parseInteger(const QString& s0, qlonglong* out) {
    const QString s = s0.trimmed();
    bool ok = false;
    qlonglong v = 0;
    if (s.startsWith("0x", Qt::CaseInsensitive))       v = s.mid(2).toLongLong(&ok, 16);
    else if (s.startsWith("0b", Qt::CaseInsensitive))  v = s.mid(2).toLongLong(&ok, 2);
    else if (s.startsWith('0') && s.size() > 1 && !s.contains('.'))
                                                       v = s.mid(1).toLongLong(&ok, 8);
    if (!ok) v = s.toLongLong(&ok, 10);
    if (ok && out) *out = v;
    return ok;
}

} // namespace TextTools
