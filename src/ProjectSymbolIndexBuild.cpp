// ProjectSymbolIndex 中需要 tree-sitter / 檔案系統的部分（與純索引邏輯分檔，
// 讓單元測試的 ProjectSymbolIndex.cpp 不必連結 tree-sitter）。
#include "ProjectSymbolIndex.h"
#include "TsSymbolParser.h"
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>

bool ProjectSymbolIndex::updateFileFromDisk(const QString& file) {
    const QString ext = QFileInfo(file).suffix();
    if (!TsSymbolParser::supports(ext)) { removeFile(file); return false; }
    QFile f(file);
    if (!f.open(QIODevice::ReadOnly)) { removeFile(file); return false; }
    const QByteArray bytes = f.readAll();
    f.close();
    setFileSymbols(file, TsSymbolParser::parse(bytes, ext));
    return true;
}

void ProjectSymbolIndex::build(const QString& folder) {
    clear();
    static const QStringList skipDirs = {".git", "build", "node_modules", ".vs",
                                         "__pycache__", ".idea", "dist", "out"};
    QDirIterator it(folder, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString path = it.next();
        const QString ext = QFileInfo(path).suffix();
        if (!TsSymbolParser::supports(ext)) continue;
        bool skip = false;
        for (const QString& d : skipDirs)
            if (path.contains("/" + d + "/") || path.contains("\\" + d + "\\")) { skip = true; break; }
        if (skip) continue;
        QFile f(path);
        if (!f.open(QIODevice::ReadOnly)) continue;
        const QByteArray bytes = f.readAll();
        f.close();
        setFileSymbols(QFileInfo(path).absoluteFilePath(), TsSymbolParser::parse(bytes, ext));
    }
}
