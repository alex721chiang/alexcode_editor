#pragma once
#include <QStringList>
#include <QString>
#include <QRegularExpression>

enum class FilterLogic { AND, OR };

// 篩選引擎：
//  - 關鍵字以 "||" 分隔（向下相容單一 "|"）
//  - 支援 AND / OR 邏輯
//  - 支援模糊比對（子序列比對，例如 "mwin" 可比對 "MainWindow"）
class FilterEngine {
public:
    void setKeywords(const QStringList& keywords);
    void setLogic(FilterLogic logic);
    void setFuzzy(bool fuzzy);
    bool matchLine(const QString& line) const;

    // 將使用者輸入解析為關鍵字清單："a || b || c"（也接受舊式 "a|b"）
    static QStringList parseQuery(const QString& query);
    // 子序列模糊比對（不分大小寫）
    static bool fuzzyContains(const QString& haystack, const QString& needle);

    // v3.3 進階語法："error && !heartbeat || timeout"
    //  - "||" 分隔 OR 群組、"&&" 群組內 AND、"!" 前綴排除
    //  - v4.3："re:" 前綴表示正規表示式（不分大小寫），如 "re:err(or)?s? && !heartbeat"
    void compile(const QString& query);
    bool matchCompiled(const QString& line) const;
    QStringList positiveKeywords() const;   // 給多色標示用（不含 regex 詞）

    struct Term { QString text; bool negate; bool isRegex = false; QRegularExpression regex; };

private:
    QList<QList<Term>> m_groups;
    QStringList m_keywords;
    FilterLogic m_logic = FilterLogic::OR;
    bool m_fuzzy = false;
};
