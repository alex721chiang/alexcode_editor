#include "SyntaxHighlighter.h"
#include "Theme.h"
#include <QFileInfo>

SyntaxHighlighter::SyntaxHighlighter(QTextDocument *parent)
    : QSyntaxHighlighter(parent)
{
    buildFormats();
}

void SyntaxHighlighter::buildFormats()
{
    keywordFormat = QTextCharFormat();
    keywordFormat.setForeground(QColor(Theme::SYN_KEYWORD));
    keywordFormat.setFontWeight(QFont::Bold);

    typeFormat = QTextCharFormat();
    typeFormat.setForeground(QColor(Theme::SYN_TYPE));

    commentFormat = QTextCharFormat();
    commentFormat.setForeground(QColor(Theme::SYN_COMMENT));
    commentFormat.setFontItalic(true);

    stringFormat = QTextCharFormat();
    stringFormat.setForeground(QColor(Theme::SYN_STRING));

    functionFormat = QTextCharFormat();
    functionFormat.setForeground(QColor(Theme::SYN_FUNCTION));

    preprocessorFormat = QTextCharFormat();
    preprocessorFormat.setForeground(QColor(Theme::SYN_PREPROC));

    numberFormat = QTextCharFormat();
    numberFormat.setForeground(QColor(Theme::SYN_NUMBER));
}

SyntaxHighlighter::Language SyntaxHighlighter::detectLanguage(const QString &filePath)
{
    const QString ext = QFileInfo(filePath).suffix().toLower();
    const QString name = QFileInfo(filePath).fileName().toLower();
    if (ext == "cpp" || ext == "cxx" || ext == "cc" || ext == "c" ||
        ext == "h"   || ext == "hpp" || ext == "hxx" || ext == "inl")
        return Language::CPP;
    if (ext == "py" || ext == "pyw")            return Language::Python;
    if (ext == "js" || ext == "jsx" || ext == "ts" || ext == "tsx" ||
        ext == "mjs" || ext == "cjs")           return Language::JavaScript;
    if (ext == "json")                          return Language::Json;
    if (ext == "xml" || ext == "html" || ext == "htm" || ext == "xhtml" ||
        ext == "qrc" || ext == "ui"  || ext == "svg")  return Language::Xml;
    if (ext == "md" || ext == "markdown")       return Language::Markdown;
    if (ext == "cmake" || name == "cmakelists.txt") return Language::CMakeLang;
    if (ext == "sh" || ext == "bash" || ext == "zsh") return Language::Bash;
    return Language::Unknown;
}

QString SyntaxHighlighter::languageName(Language lang)
{
    switch (lang) {
        case Language::CPP:        return "C/C++";
        case Language::Python:     return "Python";
        case Language::JavaScript: return "JavaScript";
        case Language::Json:       return "JSON";
        case Language::Xml:        return "XML/HTML";
        case Language::Markdown:   return "Markdown";
        case Language::CMakeLang:  return "CMake";
        case Language::Bash:       return "Shell";
        default:                   return "Plain Text";
    }
}

void SyntaxHighlighter::setLanguage(Language lang)
{
    currentLanguage = lang;
    applyRules();
    rehighlight();
}

void SyntaxHighlighter::refreshTheme()
{
    buildFormats();
    applyRules();
    rehighlight();
}

void SyntaxHighlighter::applyRules()
{
    clearRules();
    switch (currentLanguage) {
        case Language::CPP:        setupCppRules();        break;
        case Language::Python:     setupPythonRules();     break;
        case Language::JavaScript: setupJavaScriptRules(); break;
        case Language::Json:       setupJsonRules();       break;
        case Language::Xml:        setupXmlRules();        break;
        case Language::Markdown:   setupMarkdownRules();   break;
        case Language::CMakeLang:  setupCMakeRules();      break;
        case Language::Bash:       setupBashRules();       break;
        default: break;
    }
}

void SyntaxHighlighter::clearRules()
{
    highlightingRules.clear();
    commentStartExpression = QRegularExpression();
    commentEndExpression   = QRegularExpression();
}

void SyntaxHighlighter::addKeywords(const QStringList &words)
{
    for (const QString &w : words) {
        HighlightingRule rule;
        rule.pattern = QRegularExpression(QStringLiteral("\\b%1\\b").arg(w));
        rule.format  = keywordFormat;
        highlightingRules.append(rule);
    }
}

void SyntaxHighlighter::addCommon(bool cStyleStrings)
{
    HighlightingRule rule;
    // 數字（整數 / 浮點 / 十六進位）
    rule.pattern = QRegularExpression(QStringLiteral("\\b(0[xX][0-9a-fA-F]+|\\d+\\.?\\d*([eE][+-]?\\d+)?)\\b"));
    rule.format  = numberFormat;
    highlightingRules.append(rule);
    // 函式呼叫
    rule.pattern = QRegularExpression(QStringLiteral("\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*\\()"));
    rule.format  = functionFormat;
    highlightingRules.append(rule);
    // 字串（雙引號 + 單引號）；cStyleStrings 控制是否支援反斜線跳脫
    if (cStyleStrings) {
        rule.pattern = QRegularExpression(QStringLiteral("\"(?:[^\"\\\\]|\\\\.)*\""));
        rule.format  = stringFormat;
        highlightingRules.append(rule);
        rule.pattern = QRegularExpression(QStringLiteral("'(?:[^'\\\\]|\\\\.)*'"));
        rule.format  = stringFormat;
        highlightingRules.append(rule);
    } else {
        rule.pattern = QRegularExpression(QStringLiteral("\"[^\"]*\""));
        rule.format  = stringFormat;
        highlightingRules.append(rule);
        rule.pattern = QRegularExpression(QStringLiteral("'[^']*'"));
        rule.format  = stringFormat;
        highlightingRules.append(rule);
    }
}

void SyntaxHighlighter::setupCppRules()
{
    addKeywords({
        "auto","bool","break","case","catch","char","class","const","constexpr",
        "continue","default","delete","do","double","else","enum","explicit","extern",
        "false","float","for","friend","goto","if","inline","int","long","mutable",
        "namespace","new","nullptr","operator","override","private","protected","public",
        "return","short","signals","signed","sizeof","slots","static","struct","switch",
        "template","this","throw","true","try","typedef","typename","union","unsigned",
        "using","virtual","void","volatile","while"
    });
    HighlightingRule rule;
    rule.pattern = QRegularExpression("\\bQ[A-Za-z]+\\b");   // Qt 類別
    rule.format  = typeFormat;
    highlightingRules.append(rule);
    addCommon(true);
    rule.pattern = QRegularExpression("^\\s*#[^\\n]*");       // 前置處理
    rule.format  = preprocessorFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("//[^\n]*");
    rule.format  = commentFormat;
    highlightingRules.append(rule);
    commentStartExpression = QRegularExpression("/\\*");
    commentEndExpression   = QRegularExpression("\\*/");
}

void SyntaxHighlighter::setupPythonRules()
{
    addKeywords({
        "and","as","assert","async","await","break","class","continue","def","del",
        "elif","else","except","False","finally","for","from","global","if","import",
        "in","is","lambda","None","nonlocal","not","or","pass","raise","return","True",
        "try","while","with","yield","self","cls"
    });
    HighlightingRule rule;
    for (const QString &t : {"int","float","str","bool","list","dict","set","tuple","bytes","object"}) {
        rule.pattern = QRegularExpression(QStringLiteral("\\b%1\\b").arg(t));
        rule.format  = typeFormat;
        highlightingRules.append(rule);
    }
    rule.pattern = QRegularExpression("@[A-Za-z_][A-Za-z0-9_.]*");   // 裝飾器
    rule.format  = preprocessorFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"\"\"[\\s\\S]*?\"\"\"|'''[\\s\\S]*?'''");
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    addCommon(true);
    rule.pattern = QRegularExpression("#[^\n]*");
    rule.format  = commentFormat;
    highlightingRules.append(rule);
}

void SyntaxHighlighter::setupJavaScriptRules()
{
    addKeywords({
        "abstract","arguments","async","await","break","case","catch","class","const",
        "continue","debugger","default","delete","do","else","export","extends","false",
        "finally","for","function","if","implements","import","in","instanceof","interface",
        "let","new","null","of","return","static","super","switch","this","throw","true",
        "try","typeof","undefined","var","void","while","yield"
    });
    HighlightingRule rule;
    rule.pattern = QRegularExpression("`(?:[^`\\\\]|\\\\.)*`");   // 樣板字串
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    addCommon(true);
    rule.pattern = QRegularExpression("//[^\n]*");
    rule.format  = commentFormat;
    highlightingRules.append(rule);
    commentStartExpression = QRegularExpression("/\\*");
    commentEndExpression   = QRegularExpression("\\*/");
}

void SyntaxHighlighter::setupJsonRules()
{
    HighlightingRule rule;
    rule.pattern = QRegularExpression("\"(?:[^\"\\\\]|\\\\.)*\"\\s*:");   // 鍵
    rule.format  = keywordFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"(?:[^\"\\\\]|\\\\.)*\"");        // 值字串
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    for (const QString &w : {"true","false","null"}) {
        rule.pattern = QRegularExpression(QStringLiteral("\\b%1\\b").arg(w));
        rule.format  = keywordFormat;
        highlightingRules.append(rule);
    }
    rule.pattern = QRegularExpression("\\b-?\\d+\\.?\\d*([eE][+-]?\\d+)?\\b");
    rule.format  = numberFormat;
    highlightingRules.append(rule);
}

void SyntaxHighlighter::setupXmlRules()
{
    HighlightingRule rule;
    rule.pattern = QRegularExpression("</?[A-Za-z_][\\w:.-]*");   // 標籤名
    rule.format  = keywordFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("/?>");
    rule.format  = keywordFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\\b[A-Za-z_][\\w:.-]*(?=\\s*=)");   // 屬性名
    rule.format  = typeFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"[^\"]*\"|'[^']*'");
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("&[A-Za-z#0-9]+;");   // 實體
    rule.format  = numberFormat;
    highlightingRules.append(rule);
    commentStartExpression = QRegularExpression("<!--");
    commentEndExpression   = QRegularExpression("-->");
}

void SyntaxHighlighter::setupMarkdownRules()
{
    HighlightingRule rule;
    rule.pattern = QRegularExpression("^#{1,6}\\s.*$");          // 標題
    rule.format  = keywordFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\\*\\*[^*]+\\*\\*|__[^_]+__");   // 粗體
    rule.format  = typeFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("`[^`]+`");               // 行內程式碼
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\\[[^\\]]+\\]\\([^\\)]+\\)");   // 連結
    rule.format  = functionFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("^\\s*([-*+]|\\d+\\.)\\s");   // 清單符號
    rule.format  = preprocessorFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("^>\\s.*$");              // 引言
    rule.format  = commentFormat;
    highlightingRules.append(rule);
    commentStartExpression = QRegularExpression("```");        // 程式碼區塊（粗略）
    commentEndExpression   = QRegularExpression("```");
}

void SyntaxHighlighter::setupCMakeRules()
{
    addKeywords({
        "if","elseif","else","endif","foreach","endforeach","while","endwhile",
        "function","endfunction","macro","endmacro","return","break","continue",
        "set","unset","list","string","file","find_package","add_executable",
        "add_library","add_subdirectory","target_link_libraries","include",
        "option","project","cmake_minimum_required","install","message","find_program"
    });
    HighlightingRule rule;
    rule.pattern = QRegularExpression("\\$\\{[^}]+\\}");        // 變數
    rule.format  = typeFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"[^\"]*\"");
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("#[^\n]*");
    rule.format  = commentFormat;
    highlightingRules.append(rule);
}

void SyntaxHighlighter::setupBashRules()
{
    addKeywords({
        "if","then","elif","else","fi","for","in","do","done","while","until",
        "case","esac","function","return","local","export","readonly","declare",
        "echo","cd","exit","source","alias","set","unset"
    });
    HighlightingRule rule;
    rule.pattern = QRegularExpression("\\$\\{?[A-Za-z_][A-Za-z0-9_]*\\}?|\\$[0-9@*#?]");   // 變數
    rule.format  = typeFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("\"(?:[^\"\\\\]|\\\\.)*\"|'[^']*'");
    rule.format  = stringFormat;
    highlightingRules.append(rule);
    rule.pattern = QRegularExpression("#[^\n]*");
    rule.format  = commentFormat;
    highlightingRules.append(rule);
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
            QRegularExpressionMatch match = commentEndExpression.match(text, startIndex + 1);
            int endIndex      = match.capturedStart();
            int commentLength = 0;
            if (endIndex == -1) {
                setCurrentBlockState(1);
                commentLength = text.length() - startIndex;
            } else {
                commentLength = endIndex - startIndex + match.capturedLength();
            }
            setFormat(startIndex, commentLength, commentFormat);
            startIndex = text.indexOf(commentStartExpression, startIndex + commentLength);
        }
    }
}
