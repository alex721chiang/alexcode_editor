#include "FindInFilesEngine.h"
#include "FilterEngine.h"
#include <QDirIterator>
#include <QFile>
#include <QRegularExpression>
#include <QTextStream>

namespace FindInFiles {
namespace {
QRegularExpression literalCaseInsensitive(const QString& term) {
    return QRegularExpression(QRegularExpression::escape(term), QRegularExpression::CaseInsensitiveOption);
}
int countMatches(const QRegularExpression& re, const QString& text) {
    int n = 0;
    for (auto it = re.globalMatch(text); it.hasNext(); it.next()) ++n;
    return n;
}
}

Stats search(const Options& opt, const std::atomic<bool>& cancel, const BatchFn& onBatch,
             int maxResults, int batchSize) {
    Stats st;
    if (opt.root.isEmpty() || opt.query.isEmpty()) return st;
    FilterEngine engine;
    engine.setKeywords(FilterEngine::parseQuery(opt.query));
    engine.setLogic(FilterLogic::OR);

    QVector<Match> batch;
    auto flush = [&] { if (!batch.isEmpty()) { onBatch(batch); batch.clear(); } };

    QDirIterator it(opt.root, opt.nameFilters, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        if (cancel) { st.canceled = true; break; }
        const QString path = it.next();
        ++st.filesScanned;
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) continue;
        QTextStream in(&file);
        int lineNum = 1;
        while (!in.atEnd()) {
            const QString line = in.readLine();
            if (engine.matchLine(line)) {
                QString snippet = line.trimmed();
                if (snippet.length() > 100) snippet = snippet.left(97) + QStringLiteral("...");
                batch.append({path, lineNum, snippet});
                if (++st.matches >= maxResults) { st.truncated = true; flush(); return st; }
                if (batch.size() >= batchSize) flush();
            }
            ++lineNum;
        }
        flush();                                   // 每個檔案結束就回報，結果逐步出現
    }
    flush();
    return st;
}

QVector<ReplaceHit> planReplace(const Options& opt, const std::atomic<bool>& cancel) {
    QVector<ReplaceHit> hits;
    if (opt.root.isEmpty() || opt.query.isEmpty()) return hits;
    const QRegularExpression re = literalCaseInsensitive(opt.query);
    QDirIterator it(opt.root, opt.nameFilters, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        if (cancel) return {};
        const QString path = it.next();
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly)) continue;
        const int n = countMatches(re, QString::fromUtf8(file.readAll()));
        if (n > 0) hits.append({path, n});
    }
    return hits;
}

ReplaceResult applyReplace(const QVector<ReplaceHit>& plan, const QString& term,
                           const QString& replacement, const std::atomic<bool>& cancel) {
    ReplaceResult res;
    const QRegularExpression re = literalCaseInsensitive(term);
    for (const ReplaceHit& h : plan) {
        if (cancel) { res.canceled = true; break; }
        QFile in(h.path);
        if (!in.open(QIODevice::ReadOnly)) continue;
        QString text = QString::fromUtf8(in.readAll());     // 以寫入當下的內容為準
        in.close();
        const int n = countMatches(re, text);
        if (n == 0) continue;
        text.replace(re, replacement);
        QFile out(h.path);
        if (!out.open(QIODevice::WriteOnly)) continue;
        out.write(text.toUtf8());
        ++res.filesWritten;
        res.replacements += n;
    }
    return res;
}

} // namespace FindInFiles
