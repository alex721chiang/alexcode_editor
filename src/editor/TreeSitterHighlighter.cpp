#include "TreeSitterHighlighter.h"
#include "TsEdit.h"
#include "TsSymbolParser.h"
#include "Theme.h"
#include <tree_sitter/api.h>
#include <QTextDocument>
#include <QTextBlock>
#include <QTimer>
#include <algorithm>
#include <climits>
#include <cstdlib>

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
    m_debounce = new QTimer(this);     // 輸入後延遲合併解析，避免每個按鍵都全量處理
    m_debounce->setSingleShot(true);
    m_debounce->setInterval(120);
    connect(m_debounce, &QTimer::timeout, this, [this]() { doReparse(); });
}

void TreeSitterHighlighter::attach(QTextDocument* doc) {
    if (document() == doc) return;
    disconnect(m_conn);
    setDocument(doc);
    m_lastText = doc ? doc->toPlainText() : QString();
    m_fullDirty = true;
    if (doc)
        m_conn = connect(doc, &QTextDocument::contentsChange, this,
                         [this](int pos, int removed, int added) { onContentsChange(pos, removed, added); });
}

void TreeSitterHighlighter::detach() {
    disconnect(m_conn);
    if (m_debounce) m_debounce->stop();
    setDocument(nullptr);
    m_blockSpans.clear();
    m_lastText.clear();
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

// 全量重解析（語言切換/主題刷新/初次）：丟掉舊樹，從頭解析並重畫全文件。
void TreeSitterHighlighter::reparse() {
    if (m_tree) { ts_tree_delete(m_tree); m_tree = nullptr; }
    m_lastText = document() ? document()->toPlainText() : QString();
    m_fullDirty = true;
    m_pendingHi = -1;
    if (m_debounce) m_debounce->stop();
    doReparse();
}

// 內容變更：把編輯標記到舊樹上（保留可增量解析），累積受影響範圍，排程 debounce 解析。
void TreeSitterHighlighter::onContentsChange(int pos, int removed, int added) {
    if (m_reparsing || !document()) return;      // 重畫造成的 contentsChange 不可再觸發
    const QString newText = document()->toPlainText();

    if (m_lang != Lang::None && m_tree) {
        const TsEdit::ByteEdit e = TsEdit::compute(m_lastText, newText, pos, removed, added);
        TSInputEdit ie;
        ie.start_byte   = e.startByte;
        ie.old_end_byte = e.oldEndByte;
        ie.new_end_byte = e.newEndByte;
        ie.start_point    = TSPoint{ e.startRow,  e.startCol };
        ie.old_end_point  = TSPoint{ e.oldEndRow, e.oldEndCol };
        ie.new_end_point  = TSPoint{ e.newEndRow, e.newEndCol };
        ts_tree_edit(m_tree, &ie);
    } else {
        m_fullDirty = true;                      // 沒有可重用的舊樹 → 下次全量
    }

    const int editHi = pos + added;
    if (m_pendingHi < 0) { m_pendingLo = pos; m_pendingHi = editHi; }
    else { m_pendingLo = std::min(m_pendingLo, pos); m_pendingHi = std::max(m_pendingHi, editHi); }

    m_lastText = newText;
    if (m_debounce) m_debounce->start();
}

void TreeSitterHighlighter::doReparse() {
    if (m_reparsing) return;
    m_reparsing = true;

    if (!document() || m_lang == Lang::None) {
        m_blockSpans.clear();
        if (document()) rehighlight();
        m_pendingHi = -1; m_fullDirty = false; m_reparsing = false;
        return;
    }

    const QByteArray utf8 = m_lastText.toUtf8();
    const int byteN = utf8.size();
    TSTree* oldTree = m_tree;                                  // 已被 ts_tree_edit 標記過（或 null）
    TSTree* neu = ts_parser_parse_string(m_parser, m_fullDirty ? nullptr : oldTree,
                                         utf8.constData(), static_cast<uint32_t>(byteN));
    const QVector<int> byteToChar = buildByteToChar(utf8);
    auto b2c = [&](uint32_t byte) { return byteToChar[std::min<int>(int(byte), byteN)]; };

    int lo = m_pendingLo, hi = m_pendingHi;
    bool full = m_fullDirty || !oldTree || !neu;
    if (!full) {                                              // 用 tree-sitter 的「變更範圍」擴大重畫區
        uint32_t n = 0;
        TSRange* ranges = ts_tree_get_changed_ranges(oldTree, neu, &n);
        for (uint32_t i = 0; i < n; ++i) {
            lo = std::min(lo, b2c(ranges[i].start_byte));
            hi = std::max(hi, b2c(ranges[i].end_byte));
        }
        free(ranges);
    }
    if (oldTree && oldTree != neu) ts_tree_delete(oldTree);
    m_tree = neu;

    m_blockSpans.clear();
    if (m_tree) collect(ts_tree_root_node(m_tree), utf8, byteToChar);

    if (full || hi < lo) rehighlight();                       // 全量重畫
    else rehighlightRange(lo, hi);                            // 只重畫受影響 block（增量）

    m_pendingHi = -1; m_fullDirty = false; m_reparsing = false;
}

// 只對涵蓋 [charLo, charHi] 的 block 重新套用 highlightBlock（其餘 block 既有格式仍正確）。
void TreeSitterHighlighter::rehighlightRange(int charLo, int charHi) {
    QTextDocument* doc = document();
    if (!doc) return;
    charLo = std::max(0, charLo);
    QTextBlock b = doc->findBlock(charLo);
    QTextBlock last = doc->findBlock(std::min(charHi, doc->characterCount() - 1));
    while (b.isValid()) {
        rehighlightBlock(b);
        if (b == last || !last.isValid()) break;
        b = b.next();
    }
}

// 符號擷取（Go to Symbol / 麵包屑）：走訪邏輯已抽到 TsSymbolParser 共用。
QVector<TsSymbols::Symbol> TreeSitterHighlighter::symbols() const {
    if (!m_tree) return {};
    return TsSymbolParser::fromTree(m_tree, m_lastText.toUtf8());
}

// Call Graph（離線精準版）：找包含 line 的最內層函式/方法，回傳其範圍內實際呼叫的函式名。
TreeSitterHighlighter::CallInfo TreeSitterHighlighter::callInfoAt(int line) const {
    CallInfo info;
    if (!m_tree) return info;
    const QByteArray utf8 = m_lastText.toUtf8();

    int bestSpan = INT_MAX, sRow = -1, eRow = -1;        // 找最內層（範圍最小）的函式/方法
    for (const TsSymbols::Symbol& s : TsSymbolParser::fromTree(m_tree, utf8)) {
        if (s.kind != QLatin1String("function") && s.kind != QLatin1String("method")) continue;
        if (s.line <= line && line <= s.endLine && (s.endLine - s.line) < bestSpan) {
            bestSpan = s.endLine - s.line;
            sRow = s.line; eRow = s.endLine; info.root = s.name;
        }
    }
    info.callees = TsSymbolParser::calleesFromTree(m_tree, utf8, sRow, eRow);  // sRow<0 → 全檔
    info.callees.removeAll(info.root);                   // 不把自己列為被呼叫者
    return info;
}

void TreeSitterHighlighter::highlightBlock(const QString&) {
    const auto it = m_blockSpans.constFind(currentBlock().blockNumber());
    if (it == m_blockSpans.constEnd()) return;
    for (const Span& s : it.value())
        setFormat(s.start, s.len, *s.fmt);
}
