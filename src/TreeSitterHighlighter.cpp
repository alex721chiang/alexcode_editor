#include "TreeSitterHighlighter.h"
#include "Theme.h"
#include <tree_sitter/api.h>
#include <QTextDocument>
#include <QTextBlock>

extern "C" {
const TSLanguage* tree_sitter_cpp(void);
const TSLanguage* tree_sitter_python(void);
const TSLanguage* tree_sitter_javascript(void);
const TSLanguage* tree_sitter_json(void);
}

TreeSitterHighlighter::TreeSitterHighlighter(QObject* parent)
    : QSyntaxHighlighter(parent) {     // 預設不接文件（detached），由 attach() 接上
    m_parser = ts_parser_new();
    buildFormats();
}

void TreeSitterHighlighter::attach(QTextDocument* doc) {
    if (document() == doc) return;
    disconnect(m_conn);
    setDocument(doc);
    if (doc)
        m_conn = connect(doc, &QTextDocument::contentsChanged, this, [this]() { reparse(); });
}

void TreeSitterHighlighter::detach() {
    disconnect(m_conn);
    setDocument(nullptr);
    m_blockSpans.clear();
}

TreeSitterHighlighter::~TreeSitterHighlighter() {
    if (m_tree) ts_tree_delete(m_tree);
    if (m_parser) ts_parser_delete(m_parser);
}

void TreeSitterHighlighter::buildFormats() {
    m_fKeyword = QTextCharFormat(); m_fKeyword.setForeground(QColor(Theme::SYN_KEYWORD)); m_fKeyword.setFontWeight(QFont::Bold);
    m_fType    = QTextCharFormat(); m_fType.setForeground(QColor(Theme::SYN_TYPE));
    m_fComment = QTextCharFormat(); m_fComment.setForeground(QColor(Theme::SYN_COMMENT)); m_fComment.setFontItalic(true);
    m_fString  = QTextCharFormat(); m_fString.setForeground(QColor(Theme::SYN_STRING));
    m_fPreproc = QTextCharFormat(); m_fPreproc.setForeground(QColor(Theme::SYN_PREPROC));
    m_fNumber  = QTextCharFormat(); m_fNumber.setForeground(QColor(Theme::SYN_NUMBER));
}

const QTextCharFormat& TreeSitterHighlighter::formatFor(TsCategory::Category c) const {
    using C = TsCategory::Category;
    switch (c) {
        case C::Keyword: return m_fKeyword;
        case C::Type:    return m_fType;
        case C::Comment: return m_fComment;
        case C::String:  return m_fString;
        case C::Preproc: return m_fPreproc;
        case C::Number:  return m_fNumber;
        default:         return m_fKeyword;
    }
}

TreeSitterHighlighter::Lang TreeSitterHighlighter::langForExtension(const QString& ext) {
    const QString e = ext.toLower();
    if (e == "cpp" || e == "cxx" || e == "cc" || e == "c" || e == "h" || e == "hpp" || e == "hxx" || e == "hh")
        return Lang::Cpp;
    if (e == "py" || e == "pyw") return Lang::Python;
    if (e == "js" || e == "jsx" || e == "mjs" || e == "cjs" || e == "ts") return Lang::JavaScript;
    if (e == "json") return Lang::Json;
    return Lang::None;
}

QString TreeSitterHighlighter::languageName(Lang lang) {
    switch (lang) {
        case Lang::Cpp:        return "C/C++";
        case Lang::Python:     return "Python";
        case Lang::JavaScript: return "JavaScript";
        case Lang::Json:       return "JSON";
        default:               return "Plain Text";
    }
}

void TreeSitterHighlighter::setLanguage(Lang lang) {
    m_lang = lang;
    const TSLanguage* tsl = nullptr;
    switch (lang) {
        case Lang::Cpp:        tsl = tree_sitter_cpp();        break;
        case Lang::Python:     tsl = tree_sitter_python();     break;
        case Lang::JavaScript: tsl = tree_sitter_javascript(); break;
        case Lang::Json:       tsl = tree_sitter_json();       break;
        default: break;
    }
    if (tsl) ts_parser_set_language(m_parser, tsl);
    reparse();
}

void TreeSitterHighlighter::refreshTheme() {
    buildFormats();
    reparse();
}

// byte 偏移 → QChar(UTF-16) 偏移對映表（大小 = utf8.size()+1）
static QVector<int> buildByteToChar(const QByteArray& utf8) {
    QVector<int> map(utf8.size() + 1, 0);
    int ch = 0, i = 0;
    const int n = utf8.size();
    while (i < n) {
        const unsigned char b = static_cast<unsigned char>(utf8[i]);
        int blen = 1, clen = 1;
        if (b >= 0xF0)      { blen = 4; clen = 2; }   // 星狀面 → 代理對（2 個 QChar）
        else if (b >= 0xE0) { blen = 3; clen = 1; }
        else if (b >= 0xC0) { blen = 2; clen = 1; }
        for (int k = 0; k < blen && i + k <= n; ++k)
            map[i + k] = ch;
        i += blen;
        ch += clen;
    }
    map[n] = ch;
    return map;
}

void TreeSitterHighlighter::addSpan(int charStart, int charEnd, const QTextCharFormat& fmt) {
    QTextDocument* doc = document();
    QTextBlock b = doc->findBlock(charStart);
    while (b.isValid() && b.position() < charEnd) {
        const int bstart = b.position();
        const int s = qMax(charStart, bstart) - bstart;
        const int e = qMin(charEnd, bstart + b.length()) - bstart;
        if (e > s) m_blockSpans[b.blockNumber()].append({s, e - s, &fmt});
        b = b.next();
    }
}

void TreeSitterHighlighter::collect(const TSNode& node, const QByteArray& utf8,
                                    const QVector<int>& byteToChar) {
    const QString type = QString::fromUtf8(ts_node_type(node));
    const bool named = ts_node_is_named(node);
    const TsCategory::Category cat = TsCategory::forNode(type, named);

    if (cat != TsCategory::Category::None) {
        const uint32_t sb = ts_node_start_byte(node);
        const uint32_t eb = ts_node_end_byte(node);
        if (int(eb) <= byteToChar.size() - 1 || int(sb) < byteToChar.size()) {
            const int cs = byteToChar[qMin<int>(sb, byteToChar.size() - 1)];
            const int ce = byteToChar[qMin<int>(eb, byteToChar.size() - 1)];
            addSpan(cs, ce, formatFor(cat));
        }
        if (TsCategory::isWhole(cat)) return;   // 整段上色，不下探
    }
    const uint32_t childCount = ts_node_child_count(node);
    for (uint32_t i = 0; i < childCount; ++i)
        collect(ts_node_child(node, i), utf8, byteToChar);
}

void TreeSitterHighlighter::reparse() {
    if (m_reparsing) return;        // rehighlight() 的 endEditBlock 會再發 contentsChanged → 避免無限遞迴
    m_reparsing = true;

    m_blockSpans.clear();
    if (!document() || m_lang == Lang::None) {
        if (document()) rehighlight();
        m_reparsing = false;
        return;
    }

    const QByteArray utf8 = document()->toPlainText().toUtf8();
    if (m_tree) { ts_tree_delete(m_tree); m_tree = nullptr; }
    m_tree = ts_parser_parse_string(m_parser, nullptr, utf8.constData(),
                                    static_cast<uint32_t>(utf8.size()));
    if (m_tree) {
        const QVector<int> byteToChar = buildByteToChar(utf8);
        collect(ts_tree_root_node(m_tree), utf8, byteToChar);
    }
    rehighlight();
    m_reparsing = false;
}

void TreeSitterHighlighter::highlightBlock(const QString&) {
    const auto it = m_blockSpans.constFind(currentBlock().blockNumber());
    if (it == m_blockSpans.constEnd()) return;
    for (const Span& s : it.value())
        setFormat(s.start, s.len, *s.fmt);
}
