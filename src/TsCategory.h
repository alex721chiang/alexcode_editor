#pragma once
#include <QString>

// tree-sitter 節點型別 → 高亮類別（純邏輯，可單元測試）。
// 用啟發式對映常見節點型別；關鍵字以「匿名節點且型別為純字母」判定。
namespace TsCategory {

enum class Category { None, Keyword, Type, Comment, String, Number, Preproc };

// 是否為「整段上色、不再下探子節點」的類別（註解/字串/數字/前置處理）
inline bool isWhole(Category c) {
    return c == Category::Comment || c == Category::String
        || c == Category::Number  || c == Category::Preproc;
}

inline bool isAllAlpha(const QString& s) {
    if (s.isEmpty()) return false;
    for (QChar c : s)
        if (!(c.isLetter() || c == QLatin1Char('_'))) return false;
    return true;
}

inline Category forNode(const QString& type, bool named) {
    // 關鍵字：匿名節點（標點/關鍵字），型別為純字母且長度>=2
    if (!named) return (type.size() >= 2 && isAllAlpha(type)) ? Category::Keyword : Category::None;

    if (type == QLatin1String("comment")) return Category::Comment;
    if (type.contains(QLatin1String("string")) || type == QLatin1String("char_literal")
        || type == QLatin1String("system_lib_string") || type == QLatin1String("escape_sequence"))
        return Category::String;
    if (type == QLatin1String("number_literal") || type == QLatin1String("integer")
        || type == QLatin1String("float") || type == QLatin1String("number"))
        return Category::Number;
    if (type == QLatin1String("primitive_type") || type == QLatin1String("type_identifier")
        || type == QLatin1String("sized_type_specifier"))
        return Category::Type;
    if (type.startsWith(QLatin1String("preproc")) || type == QLatin1String("preproc_arg"))
        return Category::Preproc;
    if (type == QLatin1String("true") || type == QLatin1String("false")
        || type == QLatin1String("null") || type == QLatin1String("none"))
        return Category::Keyword;
    return Category::None;
}

} // namespace TsCategory
