#include "FilterEngine.h"

void FilterEngine::setKeywords(const QStringList& keywords) { m_keywords = keywords; }
void FilterEngine::setLogic(FilterLogic logic) { m_logic = logic; }
void FilterEngine::setFuzzy(bool fuzzy) { m_fuzzy = fuzzy; }

QStringList FilterEngine::parseQuery(const QString& query) {
    // 優先使用 "||"，若沒有則退回舊式 "|" 以保持相容
    const QString sep = query.contains(QStringLiteral("||")) ? QStringLiteral("||")
                                                             : QStringLiteral("|");
    QStringList parts = query.split(sep, Qt::SkipEmptyParts);
    QStringList keywords;
    for (QString p : parts) {
        p = p.trimmed();
        if (!p.isEmpty()) keywords << p;
    }
    return keywords;
}

bool FilterEngine::fuzzyContains(const QString& haystack, const QString& needle) {
    if (needle.isEmpty()) return true;
    int hi = 0;
    const int hLen = haystack.length();
    for (const QChar& nc : needle) {
        const QChar target = nc.toLower();
        bool found = false;
        while (hi < hLen) {
            if (haystack.at(hi).toLower() == target) { found = true; ++hi; break; }
            ++hi;
        }
        if (!found) return false;
    }
    return true;
}

bool FilterEngine::matchLine(const QString& line) const {
    if (m_keywords.isEmpty()) return true;

    auto matches = [this, &line](const QString& kw) {
        return m_fuzzy ? fuzzyContains(line, kw)
                       : line.contains(kw, Qt::CaseInsensitive);
    };

    if (m_logic == FilterLogic::OR) {
        for (const auto& kw : m_keywords)
            if (matches(kw)) return true;
        return false;
    } else {
        for (const auto& kw : m_keywords)
            if (!matches(kw)) return false;
        return true;
    }
}

// ----------------------------------------------------------------
// v3.3 進階篩選語法
// ----------------------------------------------------------------
void FilterEngine::compile(const QString& query) {
    m_groups.clear();
    const bool advanced = query.contains(QStringLiteral("&&")) || query.contains('!');
    QStringList orParts;
    if (query.contains(QStringLiteral("||")))
        orParts = query.split(QStringLiteral("||"), Qt::SkipEmptyParts);
    else if (!advanced && query.contains('|'))            // 舊式相容
        orParts = query.split('|', Qt::SkipEmptyParts);
    else
        orParts << query;

    for (const QString& g : orParts) {
        QList<Term> terms;
        const QStringList andParts = g.split(QStringLiteral("&&"), Qt::SkipEmptyParts);
        for (QString t : andParts) {
            t = t.trimmed();
            bool neg = t.startsWith('!');
            if (neg) t = t.mid(1).trimmed();
            if (t.isEmpty()) continue;
            Term term{t, neg, false, QRegularExpression()};
            if (t.startsWith(QStringLiteral("re:"))) {        // regex 詞
                const QString pattern = t.mid(3).trimmed();
                QRegularExpression re(pattern, QRegularExpression::CaseInsensitiveOption
                                             | QRegularExpression::DontCaptureOption);
                if (!pattern.isEmpty() && re.isValid()) {
                    re.optimize();                            // 預先 JIT 編譯，加速超大檔逐行比對
                    term.isRegex = true;
                    term.regex = re;
                }                                             // 無效 pattern：退回字面比對
            }
            terms.append(term);
        }
        if (!terms.isEmpty()) m_groups.append(terms);
    }
}

bool FilterEngine::matchCompiled(const QString& line) const {
    if (m_groups.isEmpty()) return true;
    for (const auto& group : m_groups) {
        bool all = true;
        for (const auto& term : group) {
            const bool hit = term.isRegex ? term.regex.match(line).hasMatch()
                           : m_fuzzy     ? fuzzyContains(line, term.text)
                                         : line.contains(term.text, Qt::CaseInsensitive);
            if (hit == term.negate) { all = false; break; }
        }
        if (all) return true;
    }
    return false;
}

QStringList FilterEngine::positiveKeywords() const {
    QStringList kws;
    for (const auto& group : m_groups)
        for (const auto& term : group)
            if (!term.negate && !term.isRegex && !kws.contains(term.text, Qt::CaseInsensitive))
                kws << term.text;
    return kws;
}
