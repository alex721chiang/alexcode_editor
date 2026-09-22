#pragma once
#include <QSyntaxHighlighter>
#include <QTextCharFormat>
#include <QHash>
#include <QVector>
#include <QString>
#include <QStringList>
#include "TsCategory.h"
#include "TsSymbols.h"

struct TSParser;
struct TSTree;
struct TSNode;
class QTimer;

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
    QVector<TsSymbols::Symbol> symbols() const;          // 走訪語法樹擷取類別/函式符號

    struct CallInfo { QString root; QStringList callees; };   // Call Graph：根函式 + 被呼叫者
    CallInfo callInfoAt(int line) const;                 // 游標所在函式內實際呼叫的函式（離線精準）

    static Lang langForExtension(const QString& ext);
    static QString languageName(Lang lang);

protected:
    void highlightBlock(const QString& text) override;

private:
    void buildFormats();
    void reparse();                                      // 全量重解析（語言切換/主題/初次）
    void onContentsChange(int pos, int removed, int added); // 累積編輯、標記樹、排程
    void doReparse();                                    // 實際解析（增量；debounce 後）
    void rehighlightRange(int charLo, int charHi);       // 只重畫涵蓋此字元範圍的 block
    void collect(const TSNode& node, const QByteArray& utf8, const QVector<int>& byteToChar);
    void addSpan(int charStart, int charEnd, const QTextCharFormat& fmt);
    const QTextCharFormat& formatFor(TsCategory::Category c) const;

    TSParser* m_parser = nullptr;
    TSTree* m_tree = nullptr;
    Lang m_lang = Lang::None;
    QMetaObject::Connection m_conn;
    bool m_reparsing = false;       // 防再進入：rehighlight() 的 markContentsDirty 會再發 contentsChange
    QString m_lastText;             // 上一次解析時的文件文字（計算增量編輯用）
    QTimer* m_debounce = nullptr;   // 輸入後延遲合併解析（~120ms）
    int m_pendingLo = 0;            // 自上次解析以來累積的編輯字元範圍
    int m_pendingHi = -1;           // m_pendingHi < 0 表示尚無待解析編輯
    bool m_fullDirty = true;        // 需要全量重解析 + 全量重畫

    struct Span { int start; int len; const QTextCharFormat* fmt; };
    QHash<int, QVector<Span>> m_blockSpans;     // blockNumber → spans

    QTextCharFormat m_fKeyword, m_fType, m_fComment, m_fString, m_fPreproc, m_fNumber;
};
