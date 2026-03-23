#pragma once
#include <QStringList>
#include <QString>

enum class FilterLogic { AND, OR };

class FilterEngine {
public:
    void setKeywords(const QStringList& keywords);
    void setLogic(FilterLogic logic);
    bool matchLine(const QString& line) const;
private:
    QStringList m_keywords;
    FilterLogic m_logic = FilterLogic::OR;
};
