// 純索引/搜尋邏輯（不依賴 tree-sitter / 檔案系統）。掃描/解析在 ProjectSymbolIndexBuild.cpp。
#include "ProjectSymbolIndex.h"
#include "CommandMatch.h"
#include <algorithm>

void ProjectSymbolIndex::clear() { m_byFile.clear(); }

void ProjectSymbolIndex::setFileSymbols(const QString& file, const QVector<TsSymbols::Symbol>& syms) {
    if (syms.isEmpty()) { m_byFile.remove(file); return; }
    QVector<Entry> entries;
    entries.reserve(syms.size());
    for (const TsSymbols::Symbol& s : syms)
        entries.append({ s.name, s.kind, file, s.line });
    m_byFile.insert(file, entries);
}

void ProjectSymbolIndex::removeFile(const QString& file) { m_byFile.remove(file); }

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
