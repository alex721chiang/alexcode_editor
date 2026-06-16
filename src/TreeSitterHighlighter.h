#pragma once
#include <QSyntaxHighlighter>
#include <QTextCharFormat>
#include <QHash>
#include <QVector>
#include "TsCategory.h"

struct TSParser;
struct TSTree;
struct TSNode;

// 以 tree-sitter 語法樹驅動的高亮器：解析整份文件、依節點型別套主題色票。
// 比 regex 準確（理解巢狀/結構）。支援的語言才用此器，其餘 fallback 回 SyntaxHighlighter。
class TreeSitterHighlighter : public QSyntaxHighlighter {
    Q_OBJECT
public:
    enum class Lang { None, Cpp, Python, JavaScript, Json };

    explicit TreeSitterHighlighter(QObject* parent = nullptr);
    ~TreeSitterHighlighter() override;

    void attach(QTextDocument* doc);     // 接上文件並開始監看內容變更
    void detach();                       // 卸下（改用 regex 高亮時）
    void setLanguage(Lang lang);
    void refreshTheme();
    static Lang langForExtension(const QString& ext);
    static QString languageName(Lang lang);

protected:
    void highlightBlock(const QString& text) override;

private:
    void buildFormats();
    void reparse();
    void collect(const TSNode& node, const QByteArray& utf8, const QVector<int>& byteToChar);
    void addSpan(int charStart, int charEnd, const QTextCharFormat& fmt);
    const QTextCharFormat& formatFor(TsCategory::Category c) const;

    TSParser* m_parser = nullptr;
    TSTree* m_tree = nullptr;
    Lang m_lang = Lang::None;
    QMetaObject::Connection m_conn;
    bool m_reparsing = false;       // 防再進入：rehighlight() 的 endEditBlock 會再觸發 contentsChanged

    struct Span { int start; int len; const QTextCharFormat* fmt; };
    QHash<int, QVector<Span>> m_blockSpans;     // blockNumber → spans

    QTextCharFormat m_fKeyword, m_fType, m_fComment, m_fString, m_fPreproc, m_fNumber;
};
