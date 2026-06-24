// 純索引/搜尋/序列化邏輯（不依賴 tree-sitter / 檔案系統）。掃描/解析/快取 IO 在 ProjectSymbolIndexBuild.cpp。
#include "ProjectSymbolIndex.h"
#include "CommandMatch.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <algorithm>

void ProjectSymbolIndex::clear() { m_byFile.clear(); m_mtime.clear(); }

void ProjectSymbolIndex::setFileSymbols(const QString& file, const QVector<TsSymbols::Symbol>& syms,
                                        qint64 mtime) {
    if (mtime >= 0) m_mtime.insert(file, mtime);
    if (syms.isEmpty()) { m_byFile.remove(file); return; }
    QVector<Entry> entries;
    entries.reserve(syms.size());
    for (const TsSymbols::Symbol& s : syms)
        entries.append({ s.name, s.kind, file, s.line });
    m_byFile.insert(file, entries);
}

void ProjectSymbolIndex::removeFile(const QString& file) { m_byFile.remove(file); m_mtime.remove(file); }

QStringList ProjectSymbolIndex::files() const { return m_byFile.keys(); }

qint64 ProjectSymbolIndex::mtimeOf(const QString& file) const { return m_mtime.value(file, -1); }

int ProjectSymbolIndex::symbolCount() const {
    int n = 0;
    for (auto it = m_byFile.constBegin(); it != m_byFile.constEnd(); ++it) n += it.value().size();
    return n;
}

QVector<ProjectSymbolIndex::Entry> ProjectSymbolIndex::all() const {
    QVector<Entry> out;
    for (auto it = m_byFile.constBegin(); it != m_byFile.constEnd(); ++it) out += it.value();
    return out;
}

QVector<ProjectSymbolIndex::Entry> ProjectSymbolIndex::exact(const QString& name) const {
    QVector<Entry> out;
    for (auto it = m_byFile.constBegin(); it != m_byFile.constEnd(); ++it)
        for (const Entry& e : it.value())
            if (e.name == name) out.append(e);
    return out;
}

QVector<ProjectSymbolIndex::Entry> ProjectSymbolIndex::search(const QString& query, int limit) const {
    struct Hit { int score; Entry e; };
    QVector<Hit> hits;
    for (auto it = m_byFile.constBegin(); it != m_byFile.constEnd(); ++it) {
        for (const Entry& e : it.value()) {
            const int s = CommandMatch::score(e.name, query);
            if (s >= 0) hits.append({ s, e });
        }
    }
    std::stable_sort(hits.begin(), hits.end(),
                     [](const Hit& a, const Hit& b) { return a.score > b.score; });
    QVector<Entry> out;
    const int n = qMin(hits.size(), limit);
    out.reserve(n);
    for (int i = 0; i < n; ++i) out.append(hits[i].e);
    return out;
}

QByteArray ProjectSymbolIndex::serialize() const {
    QJsonObject root;
    root["version"] = 1;
    QJsonObject filesObj;
    for (auto it = m_mtime.constBegin(); it != m_mtime.constEnd(); ++it) {  // m_mtime 含所有已索引檔（含無符號者）
        QJsonObject fo;
        fo["mtime"] = static_cast<double>(it.value());
        QJsonArray arr;
        for (const Entry& e : m_byFile.value(it.key())) {
            QJsonObject so;
            so["n"] = e.name; so["k"] = e.kind; so["l"] = e.line;
            arr.append(so);
        }
        fo["s"] = arr;
        filesObj[it.key()] = fo;
    }
    root["files"] = filesObj;
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

void ProjectSymbolIndex::deserialize(const QByteArray& json) {
    clear();
    const QJsonObject root = QJsonDocument::fromJson(json).object();
    const QJsonObject filesObj = root["files"].toObject();
    for (auto it = filesObj.begin(); it != filesObj.end(); ++it) {
        const QJsonObject fo = it.value().toObject();
        const qint64 mtime = fo["mtime"].toInteger();
        QVector<TsSymbols::Symbol> syms;
        const QJsonArray arr = fo["s"].toArray();
        for (const QJsonValue& v : arr) {
            const QJsonObject so = v.toObject();
            TsSymbols::Symbol s;
            s.name = so["n"].toString();
            s.kind = so["k"].toString();
            s.line = so["l"].toInt();
            syms.append(s);
        }
        setFileSymbols(it.key(), syms, mtime);   // 空 syms → 只記 mtime（無符號檔）
    }
}
