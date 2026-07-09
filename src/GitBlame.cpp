#include "GitBlame.h"
#include <QDateTime>
#include <QStringList>
#include <QObject>

QHash<int, GitBlame::LineInfo> GitBlame::parsePorcelain(const QByteArray& out) {
    QHash<int, LineInfo> result;
    const QStringList lines = QString::fromUtf8(out).split(QChar('\n'));

    LineInfo cur;
    int finalLine = -1;                          // 1-based（porcelain header 第三欄）
    for (const QString& line : lines) {
        if (line.startsWith(QLatin1Char('\t'))) {         // 內容行 = 這一筆結束
            if (finalLine > 0) result.insert(finalLine - 1, cur);
            cur = LineInfo();
            finalLine = -1;
            continue;
        }
        if (finalLine < 0) {                              // header：<sha> <orig> <final> [count]
            const QStringList parts = line.split(QChar(' '), Qt::SkipEmptyParts);
            if (parts.size() >= 3 && parts[0].size() == 40) {
                cur.shortSha = parts[0].left(8);
                cur.uncommitted = (parts[0] == QString(40, QLatin1Char('0')));
                finalLine = parts[2].toInt();
            }
            continue;
        }
        if (line.startsWith(QLatin1String("author ")))
            cur.author = line.mid(7);
        else if (line.startsWith(QLatin1String("author-time ")))
            cur.date = QDateTime::fromSecsSinceEpoch(line.mid(12).toLongLong())
                           .toString(QStringLiteral("yyyy-MM-dd"));
        else if (line.startsWith(QLatin1String("summary ")))
            cur.summary = line.mid(8);
    }
    return result;
}

QString GitBlame::statusText(const GitBlame::LineInfo& info) {
    if (info.shortSha.isEmpty()) return QString();
    if (info.uncommitted) return QObject::tr("⎇ 未提交變更");
    return QStringLiteral("⎇ %1 %2 %3 · %4")
        .arg(info.shortSha, info.author, info.date, info.summary);
}
