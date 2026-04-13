#include "CodeEditor.h"
#include <QPainter>
#include <QTextBlock>
#include <QWheelEvent>
#include <QKeyEvent>
#include <QContextMenuEvent>
#include <QMenu>
#include <QRegularExpression>
#include <QDebug>
#include "AICompletionProvider.h"
#include "SuggestionWidget.h"
#include "CallGraphWidget.h"

CodeEditor::CodeEditor(QWidget *parent) : QPlainTextEdit(parent) {
    lineNumberArea = new LineNumberArea(this);
    highlighter = new SyntaxHighlighter(document());
    suggestionWidget = new SuggestionWidget(this);

    connect(this, &CodeEditor::blockCountChanged, this, &CodeEditor::updateLineNumberAreaWidth);
    connect(this, &CodeEditor::updateRequest, this, &CodeEditor::updateLineNumberArea);
    connect(this, &CodeEditor::cursorPositionChanged, this, &CodeEditor::highlightCurrentLine);
    connect(suggestionWidget, &SuggestionWidget::suggestionSelected, this, &CodeEditor::insertSuggestion);

    updateLineNumberAreaWidth(0);
    highlightCurrentLine();
}

int CodeEditor::lineNumberAreaWidth() {
    int digits = 1;
    int max = qMax(1, blockCount());
    while (max >= 10) {
        max /= 10;
        ++digits;
    }
    int space = 3 + fontMetrics().horizontalAdvance(QLatin1Char('9')) * digits;
    return space;
}

void CodeEditor::updateLineNumberAreaWidth(int /* newBlockCount */) {
    setViewportMargins(lineNumberAreaWidth(), 0, 0, 0);
}

void CodeEditor::updateLineNumberArea(const QRect &rect, int dy) {
    if (dy)
        lineNumberArea->scroll(0, dy);
    else
        lineNumberArea->update(0, rect.y(), lineNumberArea->width(), rect.height());

    if (rect.contains(viewport()->rect()))
        updateLineNumberAreaWidth(0);
}

void CodeEditor::resizeEvent(QResizeEvent *e) {
    QPlainTextEdit::resizeEvent(e);
    QRect cr = contentsRect();
    lineNumberArea->setGeometry(QRect(cr.left(), cr.top(), lineNumberAreaWidth(), cr.height()));
}

void CodeEditor::highlightCurrentLine() {
    QList<QTextEdit::ExtraSelection> extraSelections;

    if (!isReadOnly()) {
        QTextEdit::ExtraSelection selection;
        QColor lineColor = QColor(Qt::yellow).lighter(160);

        selection.format.setBackground(lineColor);
        selection.format.setProperty(QTextFormat::FullWidthSelection, true);
        selection.cursor = textCursor();
        selection.cursor.clearSelection();
        extraSelections.append(selection);
    }

    setExtraSelections(extraSelections);
}

void CodeEditor::lineNumberAreaPaintEvent(QPaintEvent *event) {
    QPainter painter(lineNumberArea);
    painter.fillRect(event->rect(), Qt::lightGray);

    QTextBlock block = firstVisibleBlock();
    int blockNumber = block.blockNumber();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    int bottom = top + qRound(blockBoundingRect(block).height());

    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top()) {
            QString number = QString::number(blockNumber + 1);
            painter.setPen(Qt::black);
            painter.drawText(0, top, lineNumberArea->width(), fontMetrics().height(),
                             Qt::AlignRight | Qt::AlignVCenter, number);
        }

        block = block.next();
        top = bottom;
        bottom = top + qRound(blockBoundingRect(block).height());
        ++blockNumber;
    }
}

void CodeEditor::wheelEvent(QWheelEvent *event) {
    if (event->modifiers() & Qt::ControlModifier) {
        QFont f = font();
        int angle = event->angleDelta().y();
        if (angle > 0) {
            f.setPointSize(f.pointSize() + 1);
        } else if (angle < 0) {
            f.setPointSize(qMax(6, f.pointSize() - 1));
        }
        setFont(f);
        event->accept();
        return;
    }
    QPlainTextEdit::wheelEvent(event);
}

void CodeEditor::setAIProvider(AICompletionProvider* provider) {
    aiProvider = provider;
    if (aiProvider) {
        connect(aiProvider, &AICompletionProvider::suggestionsReady, this, &CodeEditor::onAICompletionReady);
    }
}

void CodeEditor::keyPressEvent(QKeyEvent *e) {
    // 呼叫父類處理正常的打字邏輯
    QPlainTextEdit::keyPressEvent(e);

    // 觸發條件：輸入 '.' 或按下 Ctrl + Space
    if (aiProvider && (e->text() == "." || (e->modifiers() & Qt::ControlModifier && e->key() == Qt::Key_Space))) {
        QTextCursor cursor = textCursor();
        cursor.movePosition(QTextCursor::Start, QTextCursor::KeepAnchor);
        QString context = cursor.selectedText().right(2000);
        
        qDebug() << "Triggering AI completion...";
        aiProvider->requestCompletion(context);
    }
}

void CodeEditor::onAICompletionReady(const QStringList &suggestions) {
    if (suggestions.isEmpty()) return;
    
    // 計算彈出位置：在游標正下方
    QRect cr = cursorRect();
    QPoint pos = mapToGlobal(cr.bottomLeft());
    
    suggestionWidget->showSuggestions(suggestions, pos);
}

void CodeEditor::insertSuggestion(const QString &text) {
    insertPlainText(text);
}

void CodeEditor::contextMenuEvent(QContextMenuEvent *event) {
    QMenu *menu = createStandardContextMenu();
    menu->addSeparator();

    QAction *callGraphAction = menu->addAction("Show Call Graph");
    connect(callGraphAction, &QAction::triggered, this, [this, event]() {
        QTextCursor cursor = cursorForPosition(event->pos());
        cursor.select(QTextCursor::WordUnderCursor);
        QString word = cursor.selectedText();

        if (word.isEmpty()) {
            word = "CurrentDocument";
        }

        // MVP 階段：簡單地掃描文件內容，抓取可能是函式呼叫的字眼 (字首 + 括號)
        QString text = toPlainText();
        QRegularExpression re("\\b([a-zA-Z_]\\w*)\\s*\\(");
        QRegularExpressionMatchIterator i = re.globalMatch(text);
        
        QSet<QString> deps;
        int count = 0;
        while (i.hasNext() && deps.size() < 10) { // 限制最多抓 10 個獨立依賴以確保視覺整潔
            QRegularExpressionMatch match = i.next();
            QString func = match.captured(1);
            // 排除基本控制流的關鍵字
            if (func != word && func != "if" && func != "while" && func != "for" && func != "switch" && func != "catch" && func != "return" && func != "sizeof") {
                deps.insert(func);
                count++;
            }
        }

        CallGraphWidget *graphWidget = new CallGraphWidget(this);
        connect(graphWidget, &CallGraphWidget::nodeDoubleClicked, this, [this](const QString& funcName) {
            QTextCursor cursor = document()->find(QRegularExpression("\\b" + funcName + "\\b"));
            if (!cursor.isNull()) {
                setTextCursor(cursor);
                centerCursor();
                setFocus();
            }
        });
        graphWidget->buildGraph(word, deps);
        graphWidget->setAttribute(Qt::WA_DeleteOnClose);
        graphWidget->show();
    });

    menu->exec(event->globalPos());
    delete menu;
}

