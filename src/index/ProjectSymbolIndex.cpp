// 純索引/搜尋/序列化邏輯（不依賴 tree-sitter / 檔案系統）。掃描/解析/快取 IO 在 ProjectSymbolIndexBuild.cpp。
#include "ProjectSymbolIndex.h"
#include "CommandMatch.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <algorithm>

void ProjectSymbolIndex::clear() {
    m_byFile.clear();
    m_mtime.clear();
    m_nameIndex.clear();
    m_sortedNameIndex.clear();
}

// 移除某檔案在記憶體索引（m_nameIndex/m_sortedNameIndex）中殘留的項目。
// 用 file 欄位比對而非整筆 Entry 相等，避免同名/同行號符號誤刪其他檔案的項目。
void ProjectSymbolIndex::removeFromIndices(const QString& file) {
    const auto it = m_byFile.constFind(file);
    if (it == m_byFile.constEnd()) return;
    for (const Entry& e : it.value()) {
        auto nameIt = m_nameIndex.find(e.name);
        if (nameIt != m_nameIndex.end()) {
            auto& vec = nameIt.value();
            vec.erase(std::remove_if(vec.begin(), vec.end(),
                                     [&file](const Entry& x) { return x.file == file; }),
                     vec.end());
            if (vec.isEmpty()) m_nameIndex.erase(nameIt);
        }
        const QString lower = e.name.toLower();
        auto sortedIt = m_sortedNameIndex.find(lower);
        if (sortedIt != m_sortedNameIndex.end()) {
            auto& vec = sortedIt.value();
            vec.erase(std::remove_if(vec.begin(), vec.end(),
                                     [&file](const Entry& x) { return x.file == file; }),
                     vec.end());
            if (vec.isEmpty()) m_sortedNameIndex.erase(sortedIt);
        }
    }
}

void ProjectSymbolIndex::setFileSymbols(const QString& file, const QVector<TsSymbols::Symbol>& syms,
                                        qint64 mtime) {
    if (mtime >= 0) m_mtime.insert(file, mtime);
    removeFromIndices(file);      // 先清掉該檔舊符號的索引項，避免取代時殘留幽靈結果
    if (syms.isEmpty()) { m_byFile.remove(file); return; }
    QVector<Entry> entries;
    entries.reserve(syms.size());
    for (const TsSymbols::Symbol& s : syms) {
        const Entry e{ s.name, s.kind, file, s.line };
        entries.append(e);
        m_nameIndex[e.name].append(e);
        m_sortedNameIndex[e.name.toLower()].append(e);
    }
    m_byFile.insert(file, entries);
}

void ProjectSymbolIndex::removeFile(const QString& file) {
    removeFromIndices(file);
    m_byFile.remove(file);
    m_mtime.remove(file);
}

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
    return m_nameIndex.value(name);   // O(1) 雜湊查找，取代逐檔線性掃描
}

QVector<ProjectSymbolIndex::Entry> ProjectSymbolIndex::search(const QString& query, int limit) const {
    struct Hit { int score; Entry e; };
    const QString q = query.trimmed();

    if (!q.isEmpty()) {
        // 快速路徑：用排序索引做前綴比對，O(log N + 命中數)，取代全表掃描。
        // 典型 QuickOpen 情境（輸入已知函式名稱的前幾碼）大多在此就能滿足 limit。
        QVector<Hit> fastHits;
        const QString qLower = q.toLower();
        for (auto it = m_sortedNameIndex.lowerBound(qLower);
             it != m_sortedNameIndex.end() && it.key().startsWith(qLower); ++it) {
            for (const Entry& e : it.value())
                fastHits.append({ CommandMatch::score(e.name, q), e });
        }
        // 前綴命中的分數（400-長度）恆高於 contains(300-長度)/fuzzy(200-長度)
        // ——只要符號名稱長度 < 100（一般命名遠短於此）就必然成立。
        // 所以命中數已達 limit 時，其餘符號不可能擠進前 limit 名，可直接回傳、免去全表掃描。
        if (fastHits.size() >= limit) {
            std::stable_sort(fastHits.begin(), fastHits.end(),
                             [](const Hit& a, const Hit& b) { return a.score > b.score; });
            QVector<Entry> out;
            out.reserve(limit);
            for (int i = 0; i < limit; ++i) out.append(fastHits[i].e);
            return out;
        }
    }

    // 回退路徑：query 為空，或前綴命中不足 limit → 全表掃描找 contains/fuzzy 命中，
    // 邏輯與優化前完全相同，確保結果正確性不因索引優化而改變。
    QVector<Hit> hits;
    for (auto it = m_byFile.constBegin(); it != m_byFile.constEnd(); ++it) {
        for (const Entry& e : it.value()) {
            const int s = CommandMatch::score(e.name, q);
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
