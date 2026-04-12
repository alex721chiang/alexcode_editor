#include "FilterEngine.h"

void FilterEngine::setKeywords(const QStringList& keywords) { m_keywords = keywords; }
void FilterEngine::setLogic(FilterLogic logic) { m_logic = logic; }
bool FilterEngine::matchLine(const QString& line) const {
    if (m_keywords.isEmpty()) return true;
    if (m_logic == FilterLogic::OR) {
        for (const auto& kw : m_keywords) {
            if (line.contains(kw, Qt::CaseInsensitive)) return true;
        }
        return false;
    } else {
        for (const auto& kw : m_keywords) {
            if (!line.contains(kw, Qt::CaseInsensitive)) return false;
        }
        return true;
    }
}
