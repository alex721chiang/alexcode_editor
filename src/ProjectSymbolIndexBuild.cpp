// ProjectSymbolIndex 中需要 tree-sitter / 檔案系統的部分（與純索引邏輯分檔，
// 讓單元測試的 ProjectSymbolIndex.cpp 不必連結 tree-sitter）。
#include "ProjectSymbolIndex.h"
#include "TsSymbolParser.h"
#include <QDirIterator>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QDateTime>
#include <QSet>

bool ProjectSymbolIndex::updateFileFromDisk(const QString& file) {
    const QString ext = QFileInfo(file).suffix();
    if (!TsSymbolParser::supports(ext)) { removeFile(file); return false; }
    QFile f(file);
    if (!f.open(QIODevice::ReadOnly)) { removeFile(file); return false; }
    const QByteArray bytes = f.readAll();
    f.close();
    const qint64 mtime = QFileInfo(file).lastModified().toSecsSinceEpoch();
    setFileSymbols(file, TsSymbolParser::parse(bytes, ext), mtime);
    return true;
}

void ProjectSymbolIndex::build(const QString& folder) {
    // 先載入磁碟快取（若有）→ 只重新解析 mtime 有變的檔，達到「秒載」。
    const QString cacheDir = folder + "/.alexcode";
    const QString cachePath = cacheDir + "/symbols.json";
    {
        QFile cf(cachePath);
        if (cf.open(QIODevice::ReadOnly)) { deserialize(cf.readAll()); cf.close(); }
    }

    static const QStringList skipDirs = {".git", "build", "node_modules", ".vs",
                                         "__pycache__", ".idea", "dist", "out", ".alexcode"};
    QSet<QString> seen;
    QDirIterator it(folder, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString path = it.next();
        const QString ext = QFileInfo(path).suffix();
        if (!TsSymbolParser::supports(ext)) continue;
        bool skip = false;
        for (const QString& d : skipDirs)
            if (path.contains("/" + d + "/") || path.contains("\\" + d + "\\")) { skip = true; break; }
        if (skip) continue;

        const QFileInfo fi(path);
        const QString abs = fi.absoluteFilePath();
        const qint64 mtime = fi.lastModified().toSecsSinceEpoch();
        seen.insert(abs);
        if (mtimeOf(abs) == mtime) continue;             // 快取命中、未變更 → 不重解析

        QFile f(path);
        if (!f.open(QIODevice::ReadOnly)) continue;
        const QByteArray bytes = f.readAll();
        f.close();
        setFileSymbols(abs, TsSymbolParser::parse(bytes, ext), mtime);
    }
    // 移除已不存在於資料夾的檔（快取中殘留；用 m_mtime 涵蓋無符號檔）
    for (const QString& cached : m_mtime.keys())
        if (!seen.contains(cached)) removeFile(cached);

    // 寫回快取
    QDir().mkpath(cacheDir);
    QFile out(cachePath);
    if (out.open(QIODevice::WriteOnly)) { out.write(serialize()); out.close(); }
}
