#pragma once

#include <QSyntaxHighlighter>
#include <QRegularExpression>
#include <QTextCharFormat>

class SyntaxHighlighter : public QSyntaxHighlighter {
    Q_OBJECT

public:
    enum class Language {
        Unknown,
        CPP,
        Python,
        JavaScript,
        Json,
        Xml,
        Markdown,
        CMakeLang,
        Bash
    };

    explicit SyntaxHighlighter(QTextDocument *parent = nullptr);

    void setLanguage(Language lang);
    Language language() const { return currentLanguage; }
    void refreshTheme();                         // 重新從 Theme 載入色票並重繪
    static Language detectLanguage(const QString &filePath);
    static QString languageName(Language lang);  // 狀態列顯示用

protected:
    void highlightBlock(const QString &text) override;

private:
    void buildFormats();                         // 由 Theme 建立各 QTextCharFormat
    void applyRules();                            // 依 currentLanguage 重建規則
    void clearRules();
    void setupCppRules();
    void setupPythonRules();
    void setupJavaScriptRules();
    void setupJsonRules();
    void setupXmlRules();
    void setupMarkdownRules();
    void setupCMakeRules();
    void setupBashRules();
    void addKeywords(const QStringList &words);   // 以 \b 包裝並套用 keywordFormat
    void addCommon(bool cStyleStrings = true);    // 字串/數字/函式呼叫等通用規則

    struct HighlightingRule {
        QRegularExpression pattern;
        QTextCharFormat format;
    };
    QVector<HighlightingRule> highlightingRules;

    QTextCharFormat keywordFormat;
    QTextCharFormat typeFormat;
    QTextCharFormat commentFormat;
    QTextCharFormat stringFormat;
    QTextCharFormat functionFormat;
    QTextCharFormat preprocessorFormat;
    QTextCharFormat numberFormat;

    QRegularExpression commentStartExpression;
    QRegularExpression commentEndExpression;

    Language currentLanguage = Language::Unknown;
};
