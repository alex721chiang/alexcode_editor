#pragma once
#include <QString>

// tree-sitter 節點型別 → 程式符號（類別/函式…）分類，純邏輯可單元測試。
// 供 Go to Symbol 與麵包屑使用；符號擷取的樹走訪在 TreeSitterHighlighter。
namespace TsSymbols {

struct Symbol {
    QString name;       // 符號名稱
    QString kind;       // "class" / "function" / "method" / "namespace" / …
    int line = 0;       // 0-based 起始行
    int endLine = 0;    // 0-based 結束行（用於麵包屑包含判斷）
    int depth = 0;      // 巢狀深度（清單縮排 / 階層）
};

// 回傳該節點型別對應的符號類別；非符號回傳空字串。
inline QString symbolKind(const QString& type) {
    if (type == QLatin1String("function_definition")            // C/C++、Python
     || type == QLatin1String("function_declaration")           // JS
     || type == QLatin1String("generator_function_declaration"))
        return QStringLiteral("function");
    if (type == QLatin1String("method_definition"))             // JS
        return QStringLiteral("method");
    if (type == QLatin1String("class_specifier")                // C++
     || type == QLatin1String("class_definition")               // Python
     || type == QLatin1String("class_declaration"))             // JS
        return QStringLiteral("class");
    if (type == QLatin1String("struct_specifier"))  return QStringLiteral("struct");
    if (type == QLatin1String("union_specifier"))   return QStringLiteral("union");
    if (type == QLatin1String("enum_specifier"))    return QStringLiteral("enum");
    if (type == QLatin1String("namespace_definition")) return QStringLiteral("namespace");
    return QString();
}

inline bool isSymbolNode(const QString& type) { return !symbolKind(type).isEmpty(); }

} // namespace TsSymbols
