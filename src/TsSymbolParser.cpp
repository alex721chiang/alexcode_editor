#include "TsSymbolParser.h"
#include "TreeSitterHighlighter.h"      // 重用 langForExtension（單一來源的副檔名對應）
#include <tree_sitter/api.h>

extern "C" {
const TSLanguage* tree_sitter_cpp(void);
const TSLanguage* tree_sitter_python(void);
const TSLanguage* tree_sitter_javascript(void);
const TSLanguage* tree_sitter_json(void);
}

// ---- 由 TreeSitterHighlighter 移來的符號走訪邏輯（共用）----
static QString tsNodeText(const QByteArray& utf8, TSNode n) {
    const int s = int(ts_node_start_byte(n));
    const int e = int(ts_node_end_byte(n));
    if (s < 0 || e > utf8.size() || e < s) return QString();
    return QString::fromUtf8(utf8.mid(s, e - s)).trimmed();
}

// 在 node 子孫中（淺層）找第一個型別以 "identifier" 結尾的節點。
static bool tsFindIdentifier(TSNode node, int depthLeft, TSNode& result) {
    const uint32_t c = ts_node_child_count(node);
    for (uint32_t i = 0; i < c; ++i) {
        TSNode ch = ts_node_child(node, i);
        if (QString::fromUtf8(ts_node_type(ch)).endsWith(QLatin1String("identifier"))) {
            result = ch; return true;
        }
    }
    if (depthLeft <= 0) return false;
    for (uint32_t i = 0; i < c; ++i)
        if (tsFindIdentifier(ts_node_child(node, i), depthLeft - 1, result)) return true;
    return false;
}

static QString tsSymbolName(TSNode node, const QByteArray& utf8) {
    TSNode nameNode = ts_node_child_by_field_name(node, "name", 4);
    if (!ts_node_is_null(nameNode)) return tsNodeText(utf8, nameNode);
    TSNode decl = ts_node_child_by_field_name(node, "declarator", 10);
    TSNode id;
    if (tsFindIdentifier(ts_node_is_null(decl) ? node : decl, 6, id))
        return tsNodeText(utf8, id);
    return QString();
}

static void tsCollectSymbols(TSNode node, const QByteArray& utf8, int depth,
                             QVector<TsSymbols::Symbol>& out) {
    const QString type = QString::fromUtf8(ts_node_type(node));
    const QString kind = TsSymbols::symbolKind(type);
    int childDepth = depth;
    if (!kind.isEmpty()) {
        const QString name = tsSymbolName(node, utf8);
        if (!name.isEmpty()) {
            TsSymbols::Symbol s;
            s.name = name; s.kind = kind;
            s.line = int(ts_node_start_point(node).row);
            s.endLine = int(ts_node_end_point(node).row);
            s.depth = depth;
            out.append(s);
            childDepth = depth + 1;
        }
    }
    const uint32_t c = ts_node_child_count(node);
    for (uint32_t i = 0; i < c; ++i)
        tsCollectSymbols(ts_node_child(node, i), utf8, childDepth, out);
}

static const TSLanguage* languageForExt(const QString& ext) {
    switch (TreeSitterHighlighter::langForExtension(ext)) {
        case TreeSitterHighlighter::Lang::Cpp:        return tree_sitter_cpp();
        case TreeSitterHighlighter::Lang::Python:     return tree_sitter_python();
        case TreeSitterHighlighter::Lang::JavaScript: return tree_sitter_javascript();
        case TreeSitterHighlighter::Lang::Json:       return tree_sitter_json();
        default:                                      return nullptr;
    }
}

namespace TsSymbolParser {

bool supports(const QString& ext) { return languageForExt(ext) != nullptr; }

QVector<TsSymbols::Symbol> fromTree(TSTree* tree, const QByteArray& utf8) {
    QVector<TsSymbols::Symbol> out;
    if (tree) tsCollectSymbols(ts_tree_root_node(tree), utf8, 0, out);
    return out;
}

QVector<TsSymbols::Symbol> parse(const QByteArray& utf8, const QString& ext) {
    const TSLanguage* lang = languageForExt(ext);
    if (!lang) return {};
    TSParser* parser = ts_parser_new();
    ts_parser_set_language(parser, lang);
    TSTree* tree = ts_parser_parse_string(parser, nullptr, utf8.constData(),
                                          static_cast<uint32_t>(utf8.size()));
    QVector<TsSymbols::Symbol> out = fromTree(tree, utf8);
    if (tree) ts_tree_delete(tree);
    ts_parser_delete(parser);
    return out;
}

} // namespace TsSymbolParser
