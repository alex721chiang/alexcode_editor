#include "MarkdownLinkIndex.h"
#include "MdLink.h"
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QSet>
#include <algorithm>

void MarkdownLinkIndex::buildFromContents(const QHash<QString, QString>& fileToContent) {
    m_files.clear();
    m_keyToFile.clear();
    m_rawRefs.clear();
    m_out.clear();
    m_back.clear();

    for (auto it = fileToContent.constBegin(); it != fileToContent.constEnd(); ++it) {
        const QString file = it.key();
        m_files.append(file);
        m_keyToFile.insert(MdLink::normalizeKey(QFileInfo(file).fileName()), file);
        QStringList raw;
        for (const MdLink::Ref& r : MdLink::extractRefs(it.value()))
            raw.append(r.target);
        m_rawRefs.insert(file, raw);
    }
    std::sort(m_files.begin(), m_files.end());
    finalize();
}

void MarkdownLinkIndex::build(const QString& folder) {
    QHash<QString, QString> contents;
    QDirIterator it(folder, QStringList{QStringLiteral("*.md"), QStringLiteral("*.markdown")},
                    QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString path = it.next();
        QFile f(path);
        if (f.open(QIODevice::ReadOnly | QIODevice::Text))
            contents.insert(QFileInfo(path).absoluteFilePath(), QString::fromUtf8(f.readAll()));
    }
    buildFromContents(contents);
}

QString MarkdownLinkIndex::resolve(const QString& target) const {
    return m_keyToFile.value(MdLink::normalizeKey(target));
}

void MarkdownLinkIndex::finalize() {
    for (const QString& file : m_files) {
        QSet<QString> seen;
        for (const QString& raw : m_rawRefs.value(file)) {
            const QString tgt = resolve(raw);
            if (tgt.isEmpty() || tgt == file || seen.contains(tgt)) continue;
            seen.insert(tgt);
            m_out[file].append(tgt);
            m_back[tgt].append(file);
        }
    }
}

QList<QPair<QString, QString>> MarkdownLinkIndex::edges() const {
    QList<QPair<QString, QString>> result;
    for (const QString& from : m_files)
        for (const QString& to : m_out.value(from))
            result.append({from, to});
    return result;
}
