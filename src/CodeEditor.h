#pragma once

#include <QPlainTextEdit>
#include <QWidget>
#include <QRegularExpression>
#include <QSet>
#include "SyntaxHighlighter.h"
#include "LspProtocol.h"

class QCompleter;
class QTimer;

class AICompletionProvider;
class SuggestionWidget;

class CodeEditor : public QPlainTextEdit {
    Q_OBJECT

public:
    explicit CodeEditor(QWidget *parent = nullptr);

    void lineNumberAreaPaintEvent(QPaintEvent *event);
    int lineNumberAreaWidth();
    void setAIProvider(AICompletionProvider* provider);

    // Notepad++ 風格編輯操作
    void duplicateCurrentLine();
    void deleteCurrentLine();
    void moveLineUp();
    void moveLineDown();
    void toggleComment();
    void selectionToUpper();
    void selectionToLower();
    void gotoLine(int line);
    void zoomEditorIn();
    void zoomEditorOut();
    void zoomEditorReset();

    // 搜尋結果全部標示（空 pattern 表示清除）
    void setSearchHighlightPattern(const QRegularExpression& pattern);
    // 多關鍵字多色標示（log 分析；空清單表示清除）
    void setKeywordHighlights(const QList<QPair<QString, QColor>>& kws);

    // 註解前綴（依語言："//" 或 "#"）
    void setCommentPrefix(const QString& prefix) { m_commentPrefix = prefix; }

    // 書籤（Notepad++ 風格：Ctrl+F2 切換、F2 / Shift+F2 跳轉）
    void toggleBookmark();
    void nextBookmark();
    void prevBookmark();
    QList<int> bookmarkedLines() const;          // 0-based 行號（給 Session 保存）
    void setBookmarkedLines(const QList<int>& lines);

    // 巨集錄製 / 重播
    void startMacroRecording();
    void stopMacroRecording();
    void playMacro(int times = 1);
    bool isMacroRecording() const { return m_macroRecording; }

    // 大檔案模式（停用高亮/補全/括號配對）
    void setLargeFileMode(bool on) { m_largeFile = on; }

    // 括號/引號自動配對
    void setAutoPairEnabled(bool on) { m_autoPair = on; }

    // 語法高亮：支援的語言用 tree-sitter，其餘用 regex SyntaxHighlighter
    void setSyntaxLanguage(SyntaxHighlighter::Language lang, const QString& filePath = QString());
    void refreshSyntaxTheme();

    // ---- Snippet 樣板（trigger + Tab 展開）----
    void setSnippets(const QHash<QString, QString>& snippets) { m_snippets = snippets; }

    // ---- 多游標（精簡版：Ctrl+D 加入下一個相同字串，輸入同步套用）----
    void addNextOccurrence();                            // Ctrl+D
    void clearExtraCursors();                            // Esc / 滑鼠點擊
    bool hasExtraCursors() const { return !m_extraCursors.isEmpty(); }

    // ---- 程式碼摺疊（大括號 + 縮排混合判斷）----
    void toggleFoldAt(int line);                 // 0-based；摺疊/展開該行起始的區域
    void foldCurrentRegion();                    // Ctrl+Shift+[
    void unfoldCurrentRegion();                  // Ctrl+Shift+]
    void lineNumberAreaMousePress(QMouseEvent* e);
    QList<int> foldedStartLines() const;         // 已摺疊區域的起始行（給 Session 保存）
    void applyFolds(const QList<int>& lines);    // 還原摺疊（內容載入後呼叫）

    // ---- LSP（語言伺服器）----
    void setLspEnabled(bool on) { m_lspEnabled = on && !m_largeFile; }
    bool lspEnabled() const { return m_lspEnabled; }
    void setDiagnostics(const QList<LspProtocol::Diagnostic>& diags);
    void diagnosticCounts(int* errors, int* warnings) const;
    void showLspCompletions(const QStringList& items);   // 重用 SuggestionWidget，插入時取代前綴
    void showHoverText(const QString& text);             // 於最後 hover 位置顯示 QToolTip
    void setLspCompletionTriggers(const QString& chars) { m_lspTriggers = chars; }

    // ---- Git gutter 行標示（0-based 行號 → GitGutter::State 旗標）----
    void setGitLineStates(const QHash<int, int>& states);

signals:
    void lspCompletionRequested(int line, int character);   // 0-based
    void lspDefinitionRequested(int line, int character);
    void lspHoverRequested(int line, int character);
    void lspReferencesRequested(int line, int character);
    void lspRenameRequested(int line, int character);
    void lspFormatRequested();
    void wikilinkActivated(const QString& target);       // Ctrl+點擊 [[…]]（Markdown 導覽）

protected:
    bool event(QEvent* e) override;                      // QEvent::ToolTip → 診斷 / hover

protected:
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void keyPressEvent(QKeyEvent *e) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void paintEvent(QPaintEvent *event) override;        // 多游標 caret 繪製
    void mousePressEvent(QMouseEvent *event) override;   // 點擊清除多游標 / Alt 起始矩形選取
    void mouseMoveEvent(QMouseEvent *event) override;    // Alt 拖曳更新矩形選取
    void mouseReleaseEvent(QMouseEvent *event) override;

private slots:
    void updateLineNumberAreaWidth(int newBlockCount);
    void updateExtraHighlights();
    void updateLineNumberArea(const QRect &rect, int dy);
    void onAICompletionReady(const QStringList &suggestions);
    void insertSuggestion(const QString &text);

private:
    void handleAutoIndent();
    void indentSelection(bool unindent);
    void appendBracketMatchSelections(QList<QTextEdit::ExtraSelection>& selections);
    void setupCompleter();
    void rebuildCompleterModel();
    QString wordUnderCursor() const;
    QString wikilinkAt(const QPoint& pos) const;         // 位置落在 [[target]] 內則回傳 target
    bool handleAutoPair(QKeyEvent* e);                   // 括號/引號自動配對；true = 已處理
    void triggerLocalCompletion();          // 離線智慧補全（LSP 未啟用時的 Ctrl+Space）

    QWidget *lineNumberArea;
    SyntaxHighlighter *highlighter;
    class TreeSitterHighlighter* tsHighlighter = nullptr;   // 支援語言時改用
    AICompletionProvider *aiProvider = nullptr;
    SuggestionWidget *suggestionWidget = nullptr;
    QRegularExpression m_searchPattern;
    QString m_commentPrefix = QStringLiteral("//");
    int m_baseFontSize = 0;
    QList<QPair<QString, QColor>> m_kwHighlights;
    QList<QTextCursor> m_bookmarks;             // 持久游標，編輯時自動跟隨
    QCompleter* m_completer = nullptr;
    bool m_macroRecording = false;
    bool m_largeFile = false;
    bool m_autoPair = true;                              // 括號/引號自動配對
    struct MacroKey { int key; Qt::KeyboardModifiers mods; QString text; };
    QList<MacroKey> m_macro;
    QTimer* m_completerRebuildTimer = nullptr;

    // LSP
    bool m_lspEnabled = false;
    bool m_suggestionsAreLsp = false;                    // 插入時是否取代字詞前綴
    QString m_lspTriggers;                               // 伺服器補全觸發字元（如 ".>:"）
    QList<LspProtocol::Diagnostic> m_diagnostics;
    QPoint m_lastHoverGlobalPos;
    void appendDiagnosticSelections(QList<QTextEdit::ExtraSelection>& selections);
    int diagnosticIndexAt(const QTextCursor& cursor) const;   // -1 = 無

    // Git gutter
    QHash<int, int> m_gitLineStates;                     // 0-based 行號 → GitGutter::State

    // Snippet
    QHash<QString, QString> m_snippets;                  // trigger → body
    bool expandSnippet();                                // 游標前字詞為 trigger 時展開

    // 多游標（持久游標，編輯後位置自動跟隨）
    QList<QTextCursor> m_extraCursors;
    bool handleMultiCursorKey(QKeyEvent* e);             // true = 已處理
    // 矩形（欄位）選取
    bool m_boxSelecting = false;
    int m_boxAnchorLine = 0, m_boxAnchorCol = 0;
    void applyBoxSelection(int curLine, int curCol);     // 依錨點→目前點建立每行選取

    // 程式碼摺疊（持久游標，編輯後位置自動跟隨）
    struct Fold { QTextCursor start, end; };
    QList<Fold> m_folds;
    int foldEndLine(const QTextBlock& startBlock) const; // -1 = 不可摺疊
    bool isFoldCandidate(const QTextBlock& block) const; // gutter 標記用的廉價判斷
    int foldIndexAtStart(int line) const;                // -1 = 該行非摺疊起點
    void applyFoldVisibility();                          // 依 m_folds 重設可視性並重排版
};

class LineNumberArea : public QWidget {
public:
    explicit LineNumberArea(CodeEditor *editor) : QWidget(editor), codeEditor(editor) {}

    QSize sizeHint() const override {
        return QSize(codeEditor->lineNumberAreaWidth(), 0);
    }

protected:
    void paintEvent(QPaintEvent *event) override {
        codeEditor->lineNumberAreaPaintEvent(event);
    }
    void mousePressEvent(QMouseEvent *event) override {
        codeEditor->lineNumberAreaMousePress(event);
    }

private:
    CodeEditor *codeEditor;
};
