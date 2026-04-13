#pragma once

#include <QPlainTextEdit>
#include <QWidget>
#include "SyntaxHighlighter.h"

class AICompletionProvider;
class SuggestionWidget;

class CodeEditor : public QPlainTextEdit {
    Q_OBJECT

public:
    explicit CodeEditor(QWidget *parent = nullptr);

    void lineNumberAreaPaintEvent(QPaintEvent *event);
    int lineNumberAreaWidth();
    void setAIProvider(AICompletionProvider* provider);

protected:
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void keyPressEvent(QKeyEvent *e) override;
    void contextMenuEvent(QContextMenuEvent *event) override;

private slots:
    void updateLineNumberAreaWidth(int newBlockCount);
    void highlightCurrentLine();
    void updateLineNumberArea(const QRect &rect, int dy);
    void onAICompletionReady(const QStringList &suggestions);
    void insertSuggestion(const QString &text);

private:
    QWidget *lineNumberArea;
    SyntaxHighlighter *highlighter;
    AICompletionProvider *aiProvider = nullptr;
    SuggestionWidget *suggestionWidget = nullptr;
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

private:
    CodeEditor *codeEditor;
};
