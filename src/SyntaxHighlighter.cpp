#include "SyntaxHighlighter.h"
#include <QFileInfo>

SyntaxHighlighter::SyntaxHighlighter(QTextDocument *parent)
    : QSyntaxHighlighter(parent)
{
    keywordFormat.setForeground(Qt::darkBlue);
    keywordFormat.setFontWeight(QFont::Bold);
    classFormat.setFontWeight(QFont::Bold);
    classFormat.setForeground(Qt::darkMagenta);
    singleLineCommentFormat.setForeground(Qt::darkGreen);
    multiLineCommentFormat.setForeground(Qt::darkGreen);
    quotationFormat.setForeground(Qt::darkRed);
    functionFormat.setFontItalic(true);
    functionFormat.setForeground(Qt::blue);
    preprocessorFormat.setForeground(QColor(128, 0, 128));
}

SyntaxHighlighter::Language SyntaxHighlighter::detectLanguage(const QString &filePath)
{
    QFileInfo fi(filePath);
    QString ext = fi.suffix().toLower();
    if (ext == "cpp" || ext == "cxx" || ext == "cc" ||
        ext == "c"   || ext == "h"   || ext == "hpp" || ext == "hxx")
        return Language::CPP;
    if (ext == "py" || ext == "pyw")
        return Language::Python;
    return Language::Unknown;
}

void SyntaxHighlighter::setLanguage(Language lang)
{
    if (currentLanguage == lang) return;
    currentLanguage = lang;
    clearRules();
    switch (lang) {
        case Language::CPP:    setupCppRules();    break;
        case Language::Python: setupPythonRules(); break;
        default: break;
    }
    rehighlight();
}

void SyntaxHighlighter::clearRules()
{
    highlightingRules.clear();
    commentStartExpression = QRegularExpression();
    commentEndExpression   = QRegularExpression();
}

void SyntaxHighlighter::setupCppRules()
{
    HighlightingRule rule;
    QStringList cppKeywords = {
        "\\bauto\\b", "\\bbool\\b", "\\bbreak\\b", "\\bcase\\b",
        "\\bcatch\\b", "\\bchar\\b", "\\bclass\\b", "\\bconst\\b",
        "\\bconstexpr\\b", "\\bcontinue\\b", "\\bdefault\\b", "\\bdelete\\b",
        "\\bdo\\b", "\\bdouble\\b", "\\belse\\b", "\\benum\\b",
        "\\bexplicit\\b", "\\bextern\\b", "\\bfalse\\b", "\\bfloat\\b",
        "\\bfor\\b", "\\bfriend\\b", "\\bgoto\\b", "\\bif\\b",
        "\\binline\\b", "\\bint\\b", "\\blong\\b", "\\bmutable\\b",
        "\\bnamespace\\b", "\\bnew\\b", "\\bnullptr\\b", "\\boperator\\b",
        "\\boverride\\b", "\\bprivate\\b", "\\bprotected\\b", "\\bpublic\\b",
        "\\breturn\\b", "\\bshort\\b", "\\bsignals\\b", "\\bsigned\\b",
        "\\bsizeof\\b", "\\bslots\\b", "\\bstatic\\b", "\\bstruct\\b",
        "\\bswitch\\b", "\\btemplate\\b", "\\bthis\\b", "\\bthrow\\b",
        "\\btrue\\b", "\\btry\\b", "\\btypedef\\b", "\\btypename\\b",
        "\\bunion\\b", "\\bunsigned\\b", "\\busing\\b", "\\bvirtual\\b",
        "\\bvoid\\b", "\\bvolatile\\b", "\\bwhile\\b"
    };
    for (const QString &kw : cppKeywords) {
        rule.pattern = QRegularExpression(kw);
        rule.format  = keywordFormat;
        highlightingRules.append(rule);
    }
    rule.pattern = QRegularExpression("\\bQ[A-Za-z]+\\b");
    rule.format  = classFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("^\\s*#[^\\n]*");
    rule.format  = preprocessorFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"(?:[^\"\\\\]|\\\\.)*\"");
    rule.format  = quotationFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("'(?:[^'\\\\]|\\\\.)*'");
    rule.format  = quotationFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*\\()");
    rule.format  = functionFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("//[^\n]*");
    rule.format  = singleLineCommentFormat;
    highlightingRules.append(rule);
    multiLineCommentFormat.setForeground(Qt::darkGreen);
    commentStartExpression = QRegularExpression("/\\*");
    commentEndExpression   = QRegularExpression("\\*/");
}

void SyntaxHighlighter::setupPythonRules()
{
    HighlightingRule rule;
    QStringList pyKeywords = {
        "\\band\\b", "\\bas\\b", "\\bassert\\b", "\\bbreak\\b",
        "\\bclass\\b", "\\bcontinue\\b", "\\bdef\\b", "\\bdel\\b",
        "\\belif\\b", "\\belse\\b", "\\bexcept\\b", "\\bFalse\\b",
        "\\bfinally\\b", "\\bfor\\b", "\\bfrom\\b", "\\bglobal\\b",
        "\\bif\\b", "\\bimport\\b", "\\bin\\b", "\\bis\\b",
        "\\blambda\\b", "\\bNone\\b", "\\bnonlocal\\b", "\\bnot\\b",
        "\\bor\\b", "\\bpass\\b", "\\braise\\b", "\\breturn\\b",
        "\\bTrue\\b", "\\btry\\b", "\\bwhile\\b", "\\bwith\\b",
        "\\byield\\b", "\\bself\\b", "\\bcls\\b",
        "\\bprint\\b", "\\blen\\b", "\\brange\\b", "\\btype\\b"
    };
    for (const QString &kw : pyKeywords) {
        rule.pattern = QRegularExpression(kw);
        rule.format  = keywordFormat;
        highlightingRules.append(rule);
    }
    QStringList pyTypes = {
        "\\bint\\b", "\\bfloat\\b", "\\bstr\\b", "\\bbool\\b",
        "\\blist\\b", "\\bdict\\b", "\\bset\\b", "\\btuple\\b",
        "\\bbytes\\b", "\\bobject\\b"
    };
    for (const QString &t : pyTypes) {
        rule.pattern = QRegularExpression(t);
        rule.format  = classFormat;
        highlightingRules.append(rule);
    }
    rule.pattern = QRegularExpression("@[A-Za-z_][A-Za-z0-9_.]*");
    rule.format  = preprocessorFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"\"\"[\\s\\S]*?\"\"\"|'''[\\s\\S]*?'''");
    rule.format  = quotationFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"(?:[^\"\\\\]|\\\\.)*\"");
    rule.format  = quotationFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("'(?:[^'\\\\]|\\\\.)*'");
    rule.format  = quotationFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*\\()");
    rule.format  = functionFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("#[^\n]*");
    rule.format  = singleLineCommentFormat;
    highlightingRules.append(rule);
    commentStartExpression = QRegularExpression();
    commentEndExpression   = QRegularExpression();
}

void SyntaxHighlighter::highlightBlock(const QString &text)
{
    for (const HighlightingRule &rule : highlightingRules) {
        QRegularExpressionMatchIterator it = rule.pattern.globalMatch(text);
        while (it.hasNext()) {
            QRegularExpressionMatch match = it.next();
            setFormat(match.capturedStart(), match.capturedLength(), rule.format);
        }
    }
    if (!commentStartExpression.pattern().isEmpty()) {
        setCurrentBlockState(0);
        int startIndex = 0;
        if (previousBlockState() != 1)
            startIndex = text.indexOf(commentStartExpression);
        while (startIndex >= 0) {
            QRegularExpressionMatch match = commentEndExpression.match(text, startIndex);
            int endIndex      = match.capturedStart();
            int commentLength = 0;
            if (endIndex == -1) {
                setCurrentBlockState(1);
                commentLength = text.length() - startIndex;
            } else {
                commentLength = endIndex - startIndex + match.capturedLength();
            }
            setFormat(startIndex, commentLength, multiLineCommentFormat);
            startIndex = text.indexOf(commentStartExpression, startIndex + commentLength);
        }
    }
}
