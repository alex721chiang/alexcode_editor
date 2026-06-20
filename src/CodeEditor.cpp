#include "CodeEditor.h"
#include <QPainter>
#include <QTextBlock>
#include <QWheelEvent>
#include <QKeyEvent>
#include <QContextMenuEvent>
#include <QMenu>
#include <QRegularExpression>
#include <QDebug>
#include <QCompleter>
#include <QStringListModel>
#include <QAbstractItemView>
#include <QScrollBar>
#include <QTimer>
#include <QToolTip>
#include <QHelpEvent>
#include <algorithm>
#include <QFileInfo>
#include <QTextOption>
#include "BoxSelect.h"
#include "LocalCompletion.h"
#include "AutoPair.h"
#include "StickyScroll.h"
#include "TreeSitterHighlighter.h"
#include "AICompletionProvider.h"
#include "SuggestionWidget.h"
#include "CallGraphWidget.h"
#include "Theme.h"
#include "GitGutter.h"

CodeEditor::CodeEditor(QWidget *parent) : QPlainTextEdit(parent) {
    lineNumberArea = new LineNumberArea(this);
    stickyArea = new StickyHeaderArea(this, viewport());   // 覆蓋於文字 viewport 上方
    stickyArea->hide();
    highlighter = new SyntaxHighlighter(document());
    suggestionWidget = new SuggestionWidget(this);

    connect(this, &CodeEditor::blockCountChanged, this, &CodeEditor::updateLineNumberAreaWidth);
    connect(this, &CodeEditor::updateRequest, this, &CodeEditor::updateLineNumberArea);
    connect(this, &CodeEditor::cursorPositionChanged, this, &CodeEditor::updateExtraHighlights);
    // 游標落入摺疊隱藏區（搜尋/跳行等）→ 自動展開所在區域
    connect(this, &CodeEditor::cursorPositionChanged, this, [this]() {
        while (!m_folds.isEmpty() && !textCursor().block().isVisible())
            unfoldCurrentRegion();
    });
    connect(suggestionWidget, &SuggestionWidget::suggestionSelected, this, &CodeEditor::insertSuggestion);

    setTabStopDistance(fontMetrics().horizontalAdvance(QLatin1Char(' ')) * 4);

    updateLineNumberAreaWidth(0);
    updateExtraHighlights();
    setupCompleter();
}

void CodeEditor::setSyntaxLanguage(SyntaxHighlighter::Language lang, const QString& filePath) {
    const QString ext = filePath.isEmpty() ? QString() : QFileInfo(filePath).suffix();
    const TreeSitterHighlighter::Lang tsl =
        m_largeFile ? TreeSitterHighlighter::Lang::None
                    : TreeSitterHighlighter::langForExtension(ext);

    if (tsl != TreeSitterHighlighter::Lang::None) {
        // 支援語言 → 改用 tree-sitter，卸下 regex 高亮（同一文件只留一個高亮器）
        if (highlighter) highlighter->setDocument(nullptr);
        if (!tsHighlighter) tsHighlighter = new TreeSitterHighlighter(this);
        tsHighlighter->attach(document());
        tsHighlighter->setLanguage(tsl);
    } else {
        if (tsHighlighter) tsHighlighter->detach();
        if (highlighter) {
            if (highlighter->document() != document()) highlighter->setDocument(document());
            highlighter->setLanguage(lang);
        }
    }
}

void CodeEditor::refreshSyntaxTheme() {
    if (tsHighlighter && tsHighlighter->document()) tsHighlighter->refreshTheme();
    if (highlighter && highlighter->document()) highlighter->refreshTheme();
}

QVector<TsSymbols::Symbol> CodeEditor::documentSymbols() const {
    if (!tsHighlighter || !tsHighlighter->document()) return {};
    const int rev = document()->revision();              // 依文件版本快取，捲動/移游標免重走樹
    if (rev != m_symCacheRev) {
        m_symCache = tsHighlighter->symbols();
        m_symCacheRev = rev;
    }
    return m_symCache;
}

void CodeEditor::setStickyScrollEnabled(bool on) {
    m_stickyEnabled = on;
    updateSticky();
}

// 依目前捲動位置重算要固定的標頭，並排版 sticky 區。
void CodeEditor::updateSticky() {
    if (!stickyArea) return;
    if (!m_stickyEnabled || m_largeFile) { stickyArea->hide(); return; }
    const QVector<TsSymbols::Symbol> syms = documentSymbols();
    if (syms.isEmpty()) { stickyArea->hide(); return; }

    const int firstLine = firstVisibleBlock().blockNumber();
    m_stickyHeaders = StickyScroll::headers(syms, firstLine);
    if (m_stickyHeaders.isEmpty()) { stickyArea->hide(); return; }

    const int lineH = fontMetrics().height();
    stickyArea->setGeometry(0, 0, viewport()->width(), m_stickyHeaders.size() * lineH);
    stickyArea->raise();
    stickyArea->show();
    stickyArea->update();
}

void CodeEditor::stickyPaintEvent(QPaintEvent*) {
    QPainter p(stickyArea);
    const int lineH = fontMetrics().height();
    const int x = qRound(document()->documentMargin());
    p.setFont(font());
    for (int i = 0; i < m_stickyHeaders.size(); ++i) {
        const QRect row(0, i * lineH, stickyArea->width(), lineH);
        p.fillRect(row, QColor(Theme::LINE_NUM_BG));
        const QString lineText = document()->findBlockByNumber(m_stickyHeaders[i].line).text();
        p.setPen(QColor(Theme::EDITOR_FG));
        p.drawText(row.adjusted(x, 0, -6, 0), Qt::AlignVCenter, lineText);
    }
    p.setPen(QColor(Theme::ACCENT));                       // 底部分隔線
    p.drawLine(0, stickyArea->height() - 1, stickyArea->width(), stickyArea->height() - 1);
}

void CodeEditor::stickyMousePress(QMouseEvent* e) {
    const int lineH = fontMetrics().height();
    const int idx = e->pos().y() / lineH;
    if (idx >= 0 && idx < m_stickyHeaders.size())
        gotoLine(m_stickyHeaders[idx].line + 1);          // line 為 0-based
}

void CodeEditor::setShowWhitespace(bool on) {
    QTextOption opt = document()->defaultTextOption();
    QTextOption::Flags f = opt.flags();
    f.setFlag(QTextOption::ShowTabsAndSpaces, on);
    f.setFlag(QTextOption::ShowLineAndParagraphSeparators, on);
    opt.setFlags(f);
    document()->setDefaultTextOption(opt);
    viewport()->update();
}

int CodeEditor::lineNumberAreaWidth() {
    int digits = 1;
    int max = qMax(1, blockCount());
    while (max >= 10) {
        max /= 10;
        ++digits;
    }
    // 多留左右邊距，讓行號區更易讀（右側 14px 為摺疊標記區）
    int space = 30 + fontMetrics().horizontalAdvance(QLatin1Char('9')) * digits + 14; // 左側留書籤標記空間
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

    updateSticky();                                       // 捲動/重繪時更新 sticky 標頭
}

void CodeEditor::resizeEvent(QResizeEvent *e) {
    QPlainTextEdit::resizeEvent(e);
    QRect cr = contentsRect();
    lineNumberArea->setGeometry(QRect(cr.left(), cr.top(), lineNumberAreaWidth(), cr.height()));
    updateSticky();
}

// ----------------------------------------------------------------
// 行高亮 + 括號配對 + 搜尋結果標示（合併為一組 ExtraSelections）
// ----------------------------------------------------------------
void CodeEditor::updateExtraHighlights() {
    QList<QTextEdit::ExtraSelection> extraSelections;

    if (!isReadOnly()) {
        QTextEdit::ExtraSelection selection;
        selection.format.setBackground(QColor(Theme::CURRENT_LINE));
        selection.format.setProperty(QTextFormat::FullWidthSelection, true);
        selection.cursor = textCursor();
        selection.cursor.clearSelection();
        extraSelections.append(selection);
    }

    // 搜尋結果全部標示（僅可視範圍附近，避免大檔案卡頓）
    if (m_searchPattern.isValid() && !m_searchPattern.pattern().isEmpty()) {
        QTextCharFormat fmt;
        fmt.setBackground(QColor(Theme::SEARCH_MATCH_BG));
        fmt.setForeground(QColor(Theme::SEARCH_MATCH_FG));

        QTextBlock block = firstVisibleBlock();
        int painted = 0;
        const int maxBlocks = 400; // 可視區 + 緩衝
        for (int i = 0; block.isValid() && i < maxBlocks && painted < 2000; ++i, block = block.next()) {
            const QString text = block.text();
            auto it = m_searchPattern.globalMatch(text);
            while (it.hasNext()) {
                auto m = it.next();
                if (m.capturedLength() == 0) break;
                QTextEdit::ExtraSelection sel;
                sel.format = fmt;
                sel.cursor = QTextCursor(block);
                sel.cursor.setPosition(block.position() + m.capturedStart());
                sel.cursor.setPosition(block.position() + m.capturedEnd(), QTextCursor::KeepAnchor);
                extraSelections.append(sel);
                ++painted;
            }
        }
    }

    // 游標所在字詞的所有出現處（whole-word、僅可視範圍、無選取時）
    if (!m_largeFile && !textCursor().hasSelection()) {
        const QString w = wordUnderCursor();
        auto isWordChar = [](QChar c) { return c.isLetterOrNumber() || c == QLatin1Char('_'); };
        bool isIdent = w.size() >= 2;
        for (int i = 0; isIdent && i < w.size(); ++i)
            if (!isWordChar(w.at(i))) isIdent = false;
        if (isIdent) {
            QTextCharFormat fmt;
            fmt.setBackground(QColor(Theme::OCCURRENCE_BG));
            QTextBlock block = firstVisibleBlock();
            for (int i = 0; block.isValid() && i < 400; ++i, block = block.next()) {
                const QString text = block.text();
                int from = 0;
                while (true) {
                    const int idx = text.indexOf(w, from);
                    if (idx < 0) break;
                    const bool leftOk  = idx == 0 || !isWordChar(text.at(idx - 1));
                    const bool rightOk = idx + w.size() >= text.size()
                                      || !isWordChar(text.at(idx + w.size()));
                    if (leftOk && rightOk) {
                        QTextEdit::ExtraSelection sel;
                        sel.format = fmt;
                        sel.cursor = QTextCursor(block);
                        sel.cursor.setPosition(block.position() + idx);
                        sel.cursor.setPosition(block.position() + idx + w.size(), QTextCursor::KeepAnchor);
                        extraSelections.append(sel);
                    }
                    from = idx + w.size();
                }
            }
        }
    }

    // 多關鍵字多色標示（僅可視範圍）
    if (!m_kwHighlights.isEmpty()) {
        QTextBlock block = firstVisibleBlock();
        for (int i = 0; block.isValid() && i < 400; ++i, block = block.next()) {
            const QString text = block.text();
            for (const auto& kw : m_kwHighlights) {
                int from = 0;
                while (true) {
                    const int idx = text.indexOf(kw.first, from, Qt::CaseInsensitive);
                    if (idx < 0) break;
                    QTextEdit::ExtraSelection sel;
                    sel.format.setBackground(kw.second);
                    sel.format.setForeground(QColor("#0a0e17"));
                    sel.format.setFontWeight(QFont::Bold);
                    sel.cursor = QTextCursor(block);
                    sel.cursor.setPosition(block.position() + idx);
                    sel.cursor.setPosition(block.position() + idx + kw.first.length(),
                                           QTextCursor::KeepAnchor);
                    extraSelections.append(sel);
                    from = idx + kw.first.length();
                }
            }
        }
    }

    // 多游標：額外游標的選取範圍標示
    for (const QTextCursor& c : m_extraCursors) {
        if (!c.hasSelection()) continue;
        QTextEdit::ExtraSelection sel;
        sel.format.setBackground(QColor(Theme::SEARCH_MATCH_BG));
        sel.format.setForeground(QColor(Theme::SEARCH_MATCH_FG));
        sel.cursor = c;
        extraSelections.append(sel);
    }

    appendDiagnosticSelections(extraSelections);
    appendBracketMatchSelections(extraSelections);
    setExtraSelections(extraSelections);
}

void CodeEditor::setSearchHighlightPattern(const QRegularExpression& pattern) {
    m_searchPattern = pattern;
    updateExtraHighlights();
}

// 括號配對高亮
void CodeEditor::appendBracketMatchSelections(QList<QTextEdit::ExtraSelection>& selections) {
    if (m_largeFile) return;
    static const QString opens  = QStringLiteral("([{");
    static const QString closes = QStringLiteral(")]}");

    QTextDocument* doc = document();
    const int pos = textCursor().position();

    auto charAt = [doc](int p) -> QChar {
        return doc->characterAt(p);
    };

    int bracketPos = -1;
    QChar bracket;
    // 游標前一個或後一個字元是否為括號
    if (pos > 0 && (opens.contains(charAt(pos - 1)) || closes.contains(charAt(pos - 1)))) {
        bracketPos = pos - 1;
        bracket = charAt(pos - 1);
    } else if (opens.contains(charAt(pos)) || closes.contains(charAt(pos))) {
        bracketPos = pos;
        bracket = charAt(pos);
    }
    if (bracketPos < 0) return;

    const bool forward = opens.contains(bracket);
    const QChar matchChar = forward ? closes.at(opens.indexOf(bracket))
                                    : opens.at(closes.indexOf(bracket));
    int depth = 0;
    int matchPos = -1;
    const int docLen = doc->characterCount();
    const int limit = 200000; // 避免超大檔掃描過久

    if (forward) {
        for (int p = bracketPos, steps = 0; p < docLen && steps < limit; ++p, ++steps) {
            const QChar c = charAt(p);
            if (c == bracket) ++depth;
            else if (c == matchChar && --depth == 0) { matchPos = p; break; }
        }
    } else {
        for (int p = bracketPos, steps = 0; p >= 0 && steps < limit; --p, ++steps) {
            const QChar c = charAt(p);
            if (c == bracket) ++depth;
            else if (c == matchChar && --depth == 0) { matchPos = p; break; }
        }
    }

    QTextCharFormat fmt;
    fmt.setBackground(QColor(Theme::BRACKET_MATCH_BG));
    fmt.setForeground(QColor(Theme::BRACKET_MATCH_FG));
    fmt.setFontWeight(QFont::Bold);

    auto makeSel = [&](int p) {
        QTextEdit::ExtraSelection sel;
        sel.format = fmt;
        sel.cursor = textCursor();
        sel.cursor.setPosition(p);
        sel.cursor.setPosition(p + 1, QTextCursor::KeepAnchor);
        selections.append(sel);
    };
    makeSel(bracketPos);
    if (matchPos >= 0) makeSel(matchPos);
}

// ----------------------------------------------------------------
// 行號區（深色 + 當前行霓虹高亮）
// ----------------------------------------------------------------
void CodeEditor::lineNumberAreaPaintEvent(QPaintEvent *event) {
    QPainter painter(lineNumberArea);
    painter.fillRect(event->rect(), QColor(Theme::LINE_NUM_BG));

    // 右側細分隔線
    painter.setPen(QColor(Theme::BORDER));
    painter.drawLine(event->rect().topRight(), event->rect().bottomRight());

    QTextBlock block = firstVisibleBlock();
    int blockNumber = block.blockNumber();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    int bottom = top + qRound(blockBoundingRect(block).height());
    const int currentLine = textCursor().blockNumber();

    QSet<int> marked;
    for (const QTextCursor& c : m_bookmarks) marked.insert(c.blockNumber());

    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top()) {
            QString number = QString::number(blockNumber + 1);
            if (blockNumber == currentLine) {
                painter.setPen(QColor(Theme::LINE_NUM_ACTIVE));
                QFont f = painter.font(); f.setBold(true); painter.setFont(f);
            } else {
                painter.setPen(QColor(Theme::LINE_NUM_FG));
                QFont f = painter.font(); f.setBold(false); painter.setFont(f);
            }
            painter.drawText(0, top, lineNumberArea->width() - 20, fontMetrics().height(),
                             Qt::AlignRight | Qt::AlignVCenter, number);
            // 摺疊標記：可摺疊 ▾、已摺疊 ▸（右緣 14px 區）
            if (!m_largeFile) {
                const bool folded = foldIndexAtStart(blockNumber) >= 0;
                if (folded || isFoldCandidate(block)) {
                    painter.setPen(folded ? QColor(Theme::LINE_NUM_ACTIVE) : QColor(Theme::LINE_NUM_FG));
                    painter.drawText(lineNumberArea->width() - 16, top, 14, fontMetrics().height(),
                                     Qt::AlignCenter,
                                     folded ? tr("▸") : tr("▾"));
                }
            }
            if (marked.contains(blockNumber)) {                 // 書籤：次強調色圓點
                painter.setBrush(QColor(Theme::ACCENT2));
                painter.setPen(Qt::NoPen);
                const int r = 3;
                painter.drawEllipse(QPoint(8, top + fontMetrics().height() / 2), r, r);
                painter.setBrush(Qt::NoBrush);
            }
            // Git gutter：左緣 3px 色條（新增青 / 修改黃）、刪除洋紅三角
            const int gitState = m_gitLineStates.value(blockNumber, 0);
            if (gitState) {
                const int lineH = bottom - top;
                if (gitState & GitGutter::Added)
                    painter.fillRect(0, top, 3, lineH, QColor(Theme::GIT_ADDED));
                else if (gitState & GitGutter::Modified)
                    painter.fillRect(0, top, 3, lineH, QColor(Theme::GIT_MODIFIED));
                if (gitState & GitGutter::DeletedAbove) {
                    painter.setBrush(QColor(Theme::GIT_DELETED));
                    painter.setPen(Qt::NoPen);
                    const QPoint tri[3] = { QPoint(0, top - 4), QPoint(0, top + 4), QPoint(6, top) };
                    painter.drawPolygon(tri, 3);
                    painter.setBrush(Qt::NoBrush);
                }
            }
        }

        block = block.next();
        top = bottom;
        bottom = top + qRound(blockBoundingRect(block).height());
        ++blockNumber;
    }
}

// ----------------------------------------------------------------
// 縮放
// ----------------------------------------------------------------
void CodeEditor::wheelEvent(QWheelEvent *event) {
    if (event->modifiers() & Qt::ControlModifier) {
        if (event->angleDelta().y() > 0) zoomEditorIn();
        else zoomEditorOut();
        event->accept();
        return;
    }
    QPlainTextEdit::wheelEvent(event);
}

void CodeEditor::zoomEditorIn() {
    if (m_baseFontSize == 0) m_baseFontSize = font().pointSize();
    QFont f = font();
    f.setPointSize(f.pointSize() + 1);
    setFont(f);
    setTabStopDistance(fontMetrics().horizontalAdvance(QLatin1Char(' ')) * 4);
}

void CodeEditor::zoomEditorOut() {
    if (m_baseFontSize == 0) m_baseFontSize = font().pointSize();
    QFont f = font();
    f.setPointSize(qMax(6, f.pointSize() - 1));
    setFont(f);
    setTabStopDistance(fontMetrics().horizontalAdvance(QLatin1Char(' ')) * 4);
}

void CodeEditor::zoomEditorReset() {
    if (m_baseFontSize <= 0) return;
    QFont f = font();
    f.setPointSize(m_baseFontSize);
    setFont(f);
    setTabStopDistance(fontMetrics().horizontalAdvance(QLatin1Char(' ')) * 4);
}

void CodeEditor::setAIProvider(AICompletionProvider* provider) {
    aiProvider = provider;
    if (aiProvider) {
        connect(aiProvider, &AICompletionProvider::suggestionsReady, this, &CodeEditor::onAICompletionReady);
    }
}

// ----------------------------------------------------------------
// 鍵盤：自動縮排 / Tab 縮排選取 / AI 觸發
// ----------------------------------------------------------------
void CodeEditor::keyPressEvent(QKeyEvent *e) {
    if (m_macroRecording)
        m_macro.append({e->key(), e->modifiers(), e->text()});

    // 補全選單開啟時，讓選單接管導覽鍵
    if (m_completer && m_completer->popup()->isVisible()) {
        switch (e->key()) {
        case Qt::Key_Enter: case Qt::Key_Return: case Qt::Key_Escape:
        case Qt::Key_Tab:   case Qt::Key_Backtab:
            e->ignore();
            return;
        default: break;
        }
    }

    // 多游標：Ctrl+Shift+D 加入下一個相同字串；有額外游標時輸入同步套用
    if (e->key() == Qt::Key_D && e->modifiers() == (Qt::ControlModifier | Qt::ShiftModifier)) {
        addNextOccurrence();
        e->accept();
        return;
    }
    if (handleMultiCursorKey(e)) {
        e->accept();
        return;
    }

    // Tab / Shift+Tab：多行縮排
    if ((e->key() == Qt::Key_Tab || e->key() == Qt::Key_Backtab) && textCursor().hasSelection()) {
        indentSelection(e->key() == Qt::Key_Backtab || (e->modifiers() & Qt::ShiftModifier));
        e->accept();
        return;
    }

    // Snippet：trigger + Tab 展開
    if (e->key() == Qt::Key_Tab && e->modifiers() == Qt::NoModifier &&
        !m_snippets.isEmpty() && expandSnippet()) {
        e->accept();
        return;
    }

    // 括號/引號自動配對（含包圍選取、跳過閉合、退格刪空配對）
    if (m_autoPair && !m_largeFile && m_extraCursors.isEmpty() && handleAutoPair(e)) {
        e->accept();
        return;
    }

    // Enter：自動縮排
    if (e->key() == Qt::Key_Return || e->key() == Qt::Key_Enter) {
        QPlainTextEdit::keyPressEvent(e);
        handleAutoIndent();
        return;
    }

    // LSP：F12 跳至定義 / Shift+F12 全部引用
    if (m_lspEnabled && e->key() == Qt::Key_F12) {
        const QTextCursor c = textCursor();
        if (e->modifiers() & Qt::ShiftModifier)
            emit lspReferencesRequested(c.blockNumber(), c.positionInBlock());
        else
            emit lspDefinitionRequested(c.blockNumber(), c.positionInBlock());
        e->accept();
        return;
    }

    // 程式碼摺疊：Ctrl+Shift+[ 摺疊 / Ctrl+Shift+] 展開
    if ((e->modifiers() & Qt::ControlModifier) && (e->modifiers() & Qt::ShiftModifier)) {
        if (e->key() == Qt::Key_BracketLeft)  { foldCurrentRegion();   e->accept(); return; }
        if (e->key() == Qt::Key_BracketRight) { unfoldCurrentRegion(); e->accept(); return; }
    }

    // LSP：Ctrl+Alt+R 重新命名符號 / Shift+Alt+F 格式化文件
    if (m_lspEnabled && e->key() == Qt::Key_R &&
        e->modifiers() == (Qt::ControlModifier | Qt::AltModifier)) {
        const QTextCursor c = textCursor();
        emit lspRenameRequested(c.blockNumber(), c.positionInBlock());
        e->accept();
        return;
    }
    if (m_lspEnabled && e->key() == Qt::Key_F &&
        e->modifiers() == (Qt::ShiftModifier | Qt::AltModifier)) {
        emit lspFormatRequested();
        e->accept();
        return;
    }

    // LSP：Ctrl+Space 補全（優先於 AI 補全）
    if (m_lspEnabled && (e->modifiers() & Qt::ControlModifier) && e->key() == Qt::Key_Space) {
        const QTextCursor c = textCursor();
        emit lspCompletionRequested(c.blockNumber(), c.positionInBlock());
        e->accept();
        return;
    }
    // 未啟用 LSP 時：Ctrl+Space 觸發離線智慧補全
    if (!m_lspEnabled && (e->modifiers() & Qt::ControlModifier) && e->key() == Qt::Key_Space) {
        triggerLocalCompletion();
        e->accept();
        return;
    }

    QPlainTextEdit::keyPressEvent(e);

    // 字詞補全：輸入 3 個字元以上自動彈出（輔助功能關閉時跳過）
    if (m_assist && m_completer && !e->text().isEmpty()) {
        const QString prefix = wordUnderCursor();
        if (prefix.length() >= 3 && (e->text().at(0).isLetterOrNumber() || e->text() == "_")) {
            if (prefix != m_completer->completionPrefix()) {
                m_completer->setCompletionPrefix(prefix);
                m_completer->popup()->setCurrentIndex(m_completer->completionModel()->index(0, 0));
            }
            if (m_completer->completionCount() > 0 &&
                !(m_completer->completionCount() == 1 &&
                  m_completer->currentCompletion() == prefix)) {
                QRect cr = cursorRect();
                cr.setWidth(m_completer->popup()->sizeHintForColumn(0)
                            + m_completer->popup()->verticalScrollBar()->sizeHint().width() + 20);
                m_completer->complete(cr);
            } else {
                m_completer->popup()->hide();
            }
        } else {
            m_completer->popup()->hide();
        }
    } else if (m_completer && (e->key() == Qt::Key_Backspace || e->key() == Qt::Key_Delete)) {
        m_completer->popup()->hide();
    }

    // LSP 補全自動觸發：輸入伺服器宣告的觸發字元（如 . > :）
    if (m_lspEnabled && e->text().size() == 1 && m_lspTriggers.contains(e->text().at(0))) {
        const QTextCursor c = textCursor();
        emit lspCompletionRequested(c.blockNumber(), c.positionInBlock());
        return;                                  // 觸發字元交給 LSP，不再觸發 AI 補全
    }

    // 補全已由 LSP（啟用時）與本地智慧補全（Ctrl+Space，見上方）負責。
    // 雲端 AICompletionProvider 為「選項 B」未來骨架，預設不自動觸發（避免對未啟動的本機/雲端 API 發無謂請求）。
}

// 括號/引號自動配對。回傳 true 表示已處理該按鍵。
bool CodeEditor::handleAutoPair(QKeyEvent* e) {
    QTextCursor cur = textCursor();
    const QString docText = document()->toPlainText();

    // 退格：游標在一對空配對中間（如 () "" 內）→ 一併刪除右側閉合
    if (e->key() == Qt::Key_Backspace && !cur.hasSelection()) {
        const int p = cur.position();
        const QChar before = p > 0 ? docText.at(p - 1) : QChar();
        const QChar after  = p < docText.size() ? docText.at(p) : QChar();
        if (AutoPair::shouldDeletePair(before, after)) {
            cur.beginEditBlock();
            cur.deleteChar();                 // 刪右側閉合
            cur.deletePreviousChar();         // 刪左側開啟
            cur.endEditBlock();
            return true;
        }
        return false;
    }

    const QString t = e->text();
    if (t.size() != 1) return false;
    const QChar typed = t.at(0);
    if (!AutoPair::isRelevant(typed)) return false;

    const int selStart = cur.selectionStart();
    const int selEnd   = cur.selectionEnd();
    const QChar before = selStart > 0 ? docText.at(selStart - 1) : QChar();
    const QChar after  = selEnd < docText.size() ? docText.at(selEnd) : QChar();

    const AutoPair::Decision d = AutoPair::decide(typed, before, after, cur.hasSelection());
    switch (d.action) {
        case AutoPair::Action::Surround: {
            const QString sel = cur.selectedText();
            cur.beginEditBlock();
            cur.insertText(QString(d.open) + sel + QString(d.close));
            cur.endEditBlock();
            // 重新選取被包圍的內容
            QTextCursor sc = textCursor();
            sc.setPosition(selStart + 1);
            sc.setPosition(selStart + 1 + sel.size(), QTextCursor::KeepAnchor);
            setTextCursor(sc);
            return true;
        }
        case AutoPair::Action::AutoClose: {
            cur.beginEditBlock();
            cur.insertText(QString(d.open) + QString(d.close));
            cur.endEditBlock();
            cur.movePosition(QTextCursor::PreviousCharacter);
            setTextCursor(cur);
            return true;
        }
        case AutoPair::Action::SkipOver: {
            cur.movePosition(QTextCursor::NextCharacter);
            setTextCursor(cur);
            return true;
        }
        case AutoPair::Action::Insert:
        default:
            return false;
    }
}

void CodeEditor::handleAutoIndent() {
    QTextCursor cursor = textCursor();
    QTextBlock prevBlock = cursor.block().previous();
    if (!prevBlock.isValid()) return;

    const QString prevText = prevBlock.text();
    QString indent;
    for (const QChar& c : prevText) {
        if (c == ' ' || c == '\t') indent += c;
        else break;
    }
    // 前一行以 { 或 : 結尾 → 額外縮排一層
    const QString trimmed = prevText.trimmed();
    if (trimmed.endsWith('{') || trimmed.endsWith(':'))
        indent += QStringLiteral("    ");

    if (!indent.isEmpty())
        cursor.insertText(indent);
}

void CodeEditor::indentSelection(bool unindent) {
    QTextCursor cursor = textCursor();
    int start = cursor.selectionStart();
    int end = cursor.selectionEnd();

    cursor.beginEditBlock();
    QTextBlock block = document()->findBlock(start);
    const QTextBlock endBlock = document()->findBlock(end);

    while (block.isValid()) {
        QTextCursor lineCursor(block);
        lineCursor.movePosition(QTextCursor::StartOfBlock);
        if (unindent) {
            const QString text = block.text();
            int remove = 0;
            if (text.startsWith('\t')) remove = 1;
            else { while (remove < 4 && remove < text.length() && text.at(remove) == ' ') ++remove; }
            if (remove > 0) {
                lineCursor.movePosition(QTextCursor::Right, QTextCursor::KeepAnchor, remove);
                lineCursor.removeSelectedText();
            }
        } else {
            lineCursor.insertText(QStringLiteral("    "));
        }
        if (block == endBlock) break;
        block = block.next();
    }
    cursor.endEditBlock();
}

// ----------------------------------------------------------------
// Notepad++ 風格行操作
// ----------------------------------------------------------------
void CodeEditor::duplicateCurrentLine() {
    QTextCursor cursor = textCursor();
    cursor.beginEditBlock();
    const int col = cursor.positionInBlock();
    QTextCursor lineCursor = cursor;
    lineCursor.movePosition(QTextCursor::StartOfBlock);
    lineCursor.movePosition(QTextCursor::EndOfBlock, QTextCursor::KeepAnchor);
    const QString lineText = lineCursor.selectedText();
    lineCursor.movePosition(QTextCursor::EndOfBlock);
    lineCursor.insertText(QStringLiteral("\n") + lineText);
    cursor.endEditBlock();

    // 游標移到新行的相同欄位
    QTextCursor newCursor = textCursor();
    newCursor.movePosition(QTextCursor::Down);
    newCursor.movePosition(QTextCursor::StartOfBlock);
    newCursor.movePosition(QTextCursor::Right, QTextCursor::MoveAnchor,
                           qMin(col, int(newCursor.block().text().length())));
    setTextCursor(newCursor);
}

void CodeEditor::deleteCurrentLine() {
    QTextCursor cursor = textCursor();
    cursor.beginEditBlock();
    cursor.movePosition(QTextCursor::StartOfBlock);
    cursor.movePosition(QTextCursor::NextBlock, QTextCursor::KeepAnchor);
    if (!cursor.hasSelection()) // 最後一行
        cursor.movePosition(QTextCursor::EndOfBlock, QTextCursor::KeepAnchor);
    cursor.removeSelectedText();
    cursor.endEditBlock();
}

void CodeEditor::moveLineUp() {
    QTextCursor cursor = textCursor();
    QTextBlock block = cursor.block();
    QTextBlock prev = block.previous();
    if (!prev.isValid()) return;

    const int col = cursor.positionInBlock();
    const QString currentText = block.text();
    const QString prevText = prev.text();

    cursor.beginEditBlock();
    QTextCursor edit(prev);
    edit.movePosition(QTextCursor::StartOfBlock);
    edit.setPosition(block.position() + currentText.length(), QTextCursor::KeepAnchor);
    edit.insertText(currentText + QStringLiteral("\n") + prevText);
    cursor.endEditBlock();

    QTextCursor newCursor(document()->findBlockByNumber(prev.blockNumber()));
    newCursor.movePosition(QTextCursor::Right, QTextCursor::MoveAnchor,
                           qMin(col, int(newCursor.block().text().length())));
    setTextCursor(newCursor);
}

void CodeEditor::moveLineDown() {
    QTextCursor cursor = textCursor();
    QTextBlock block = cursor.block();
    QTextBlock next = block.next();
    if (!next.isValid()) return;

    const int col = cursor.positionInBlock();
    const int targetLine = next.blockNumber();
    const QString currentText = block.text();
    const QString nextText = next.text();

    cursor.beginEditBlock();
    QTextCursor edit(block);
    edit.movePosition(QTextCursor::StartOfBlock);
    edit.setPosition(next.position() + nextText.length(), QTextCursor::KeepAnchor);
    edit.insertText(nextText + QStringLiteral("\n") + currentText);
    cursor.endEditBlock();

    QTextCursor newCursor(document()->findBlockByNumber(targetLine));
    newCursor.movePosition(QTextCursor::Right, QTextCursor::MoveAnchor,
                           qMin(col, int(newCursor.block().text().length())));
    setTextCursor(newCursor);
}

void CodeEditor::toggleComment() {
    const QString prefix = m_commentPrefix;
    QTextCursor cursor = textCursor();
    int start = cursor.hasSelection() ? cursor.selectionStart() : cursor.position();
    int end   = cursor.hasSelection() ? cursor.selectionEnd()   : cursor.position();

    QTextBlock block = document()->findBlock(start);
    const QTextBlock endBlock = document()->findBlock(end);

    // 判斷是要加註解還是移除（全部都已註解 → 移除）
    bool allCommented = true;
    for (QTextBlock b = block; b.isValid(); b = b.next()) {
        const QString t = b.text().trimmed();
        if (!t.isEmpty() && !t.startsWith(prefix)) { allCommented = false; }
        if (b == endBlock) break;
    }

    cursor.beginEditBlock();
    for (QTextBlock b = block; b.isValid(); b = b.next()) {
        QTextCursor lc(b);
        const QString text = b.text();
        if (allCommented) {
            int idx = text.indexOf(prefix);
            if (idx >= 0) {
                lc.setPosition(b.position() + idx);
                int len = prefix.length();
                if (idx + len < text.length() && text.at(idx + len) == ' ') ++len;
                lc.movePosition(QTextCursor::Right, QTextCursor::KeepAnchor, len);
                lc.removeSelectedText();
            }
        } else {
            if (!text.trimmed().isEmpty()) {
                lc.movePosition(QTextCursor::StartOfBlock);
                lc.insertText(prefix + QStringLiteral(" "));
            }
        }
        if (b == endBlock) break;
    }
    cursor.endEditBlock();
}

void CodeEditor::selectionToUpper() {
    QTextCursor cursor = textCursor();
    if (!cursor.hasSelection()) return;
    const int a = cursor.selectionStart(), b = cursor.selectionEnd();
    cursor.insertText(cursor.selectedText().toUpper());
    cursor.setPosition(a);
    cursor.setPosition(b, QTextCursor::KeepAnchor);
    setTextCursor(cursor);
}

void CodeEditor::selectionToLower() {
    QTextCursor cursor = textCursor();
    if (!cursor.hasSelection()) return;
    const int a = cursor.selectionStart(), b = cursor.selectionEnd();
    cursor.insertText(cursor.selectedText().toLower());
    cursor.setPosition(a);
    cursor.setPosition(b, QTextCursor::KeepAnchor);
    setTextCursor(cursor);
}

void CodeEditor::gotoLine(int line) {
    QTextBlock block = document()->findBlockByNumber(qBound(0, line - 1, blockCount() - 1));
    if (!block.isValid()) return;
    QTextCursor cursor(block);
    setTextCursor(cursor);
    centerCursor();
    setFocus();
}

void CodeEditor::onAICompletionReady(const QStringList &suggestions) {
    if (suggestions.isEmpty()) return;
    m_suggestionsAreLsp = false;
    QRect cr = cursorRect();
    QPoint pos = mapToGlobal(cr.bottomLeft());
    suggestionWidget->showSuggestions(suggestions, pos);
}

void CodeEditor::insertSuggestion(const QString &text) {
    if (m_suggestionsAreLsp) {
        // LSP 補全：取代游標下的字詞前綴
        QTextCursor cursor = textCursor();
        const QString prefix = wordUnderCursor();
        for (int i = 0; i < prefix.length(); ++i)
            cursor.movePosition(QTextCursor::PreviousCharacter, QTextCursor::KeepAnchor);
        cursor.insertText(text);
        setTextCursor(cursor);
        m_suggestionsAreLsp = false;
        return;
    }
    insertPlainText(text);
}

// ----------------------------------------------------------------
// LSP：診斷波浪底線、滑鼠提示、補全顯示
// ----------------------------------------------------------------
void CodeEditor::setGitLineStates(const QHash<int, int>& states) {
    if (states == m_gitLineStates) return;
    m_gitLineStates = states;
    lineNumberArea->update();
}

// ----------------------------------------------------------------
// 多游標（精簡版）：Ctrl+D 逐一選取下一個相同字串，輸入/刪除同步套用全部位置
// Esc 或滑鼠點擊結束多游標狀態
// ----------------------------------------------------------------
void CodeEditor::addNextOccurrence() {
    QTextCursor c = textCursor();
    if (!c.hasSelection()) {                     // 無選取：先選游標下字詞
        c.select(QTextCursor::WordUnderCursor);
        if (!c.hasSelection()) return;
        setTextCursor(c);
        return;
    }
    const QString target = c.selectedText();
    // 從最後一個游標處往後找（必要時回頭從文件開頭找）
    int from = c.selectionEnd();
    for (const QTextCursor& ec : m_extraCursors)
        from = qMax(from, ec.selectionEnd());
    QTextDocument* doc = document();
    QTextCursor found = doc->find(target, from);
    if (found.isNull())
        found = doc->find(target, 0);
    if (found.isNull()) return;
    // 避免重複加入同一位置
    if (found.selectionStart() == c.selectionStart()) return;
    for (const QTextCursor& ec : m_extraCursors)
        if (ec.selectionStart() == found.selectionStart()) return;
    m_extraCursors.append(found);
    updateExtraHighlights();
    viewport()->update();
}

void CodeEditor::clearExtraCursors() {
    if (m_extraCursors.isEmpty()) return;
    m_extraCursors.clear();
    updateExtraHighlights();
    viewport()->update();
}

bool CodeEditor::handleMultiCursorKey(QKeyEvent* e) {
    if (m_extraCursors.isEmpty()) return false;
    if (e->key() == Qt::Key_Escape) {
        clearExtraCursors();
        return true;
    }
    const bool isText = !e->text().isEmpty() && e->text().at(0).isPrint();
    const bool isNewline = e->key() == Qt::Key_Return || e->key() == Qt::Key_Enter;
    const bool isBackspace = e->key() == Qt::Key_Backspace;
    const bool isDelete = e->key() == Qt::Key_Delete;
    if (!isText && !isNewline && !isBackspace && !isDelete) return false;   // 導覽鍵等交回預設

    QTextCursor main = textCursor();
    main.beginEditBlock();
    auto applyOn = [&](QTextCursor& c) {
        if (isText)            c.insertText(e->text());
        else if (isNewline)    c.insertText(QStringLiteral("\n"));
        else if (isBackspace) { if (!c.hasSelection()) c.movePosition(QTextCursor::PreviousCharacter, QTextCursor::KeepAnchor); c.removeSelectedText(); }
        else if (isDelete)    { if (!c.hasSelection()) c.movePosition(QTextCursor::NextCharacter, QTextCursor::KeepAnchor); c.removeSelectedText(); }
    };
    applyOn(main);
    for (QTextCursor& c : m_extraCursors)
        applyOn(c);
    main.endEditBlock();
    setTextCursor(main);
    updateExtraHighlights();
    viewport()->update();
    return true;
}

// 計算一行前導空白佔的「欄數」（Tab 展開到 4 的倍數）；非全空白回傳其縮排欄數。
static int leadingIndentCols(const QString& text, int tabCols) {
    int cols = 0;
    for (const QChar& c : text) {
        if (c == QLatin1Char(' ')) ++cols;
        else if (c == QLatin1Char('\t')) cols += tabCols - (cols % tabCols);
        else break;
    }
    return cols;
}

void CodeEditor::paintIndentGuides(QPaintEvent* event) {
    const int tabCols = 4;
    const qreal charW = fontMetrics().horizontalAdvance(QLatin1Char(' '));
    if (charW <= 0) return;
    const qreal baseX = contentOffset().x() + document()->documentMargin();

    QPainter p(viewport());
    p.setPen(QPen(QColor(Theme::BORDER), 1));

    QTextBlock block = firstVisibleBlock();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    int carriedCols = 0;                          // 空白行沿用上一行縮排，讓導引線連續
    while (block.isValid() && top <= event->rect().bottom()) {
        const int h = qRound(blockBoundingRect(block).height());
        if (block.isVisible() && top + h >= event->rect().top()) {
            const QString text = block.text();
            const bool blank = text.trimmed().isEmpty();
            int cols = blank ? carriedCols : leadingIndentCols(text, tabCols);
            if (!blank) carriedCols = cols;
            for (int c = tabCols; c < cols; c += tabCols) {   // 內層每一階一條（跳過最左 col 0）
                const int x = qRound(baseX + c * charW);
                p.drawLine(x, top, x, top + h);
            }
        }
        block = block.next();
        top += h;
    }
}

void CodeEditor::paintEvent(QPaintEvent* event) {
    QPlainTextEdit::paintEvent(event);
    if (!m_largeFile) paintIndentGuides(event);   // 縮排輔助線（畫在前導空白區，不蓋文字）

    if (m_extraCursors.isEmpty()) return;
    QPainter p(viewport());                       // 額外游標 caret（強調色細線）
    p.setPen(QPen(QColor(Theme::ACCENT), 2));
    for (const QTextCursor& c : m_extraCursors) {
        const QRect r = cursorRect(c);
        p.drawLine(r.topLeft(), r.bottomLeft());
    }
}

// 位置落在某個 [[target]] 內 → 回傳去掉 alias/heading 的 target；否則空字串。
QString CodeEditor::wikilinkAt(const QPoint& pos) const {
    const QTextCursor c = cursorForPosition(pos);
    const QString line = c.block().text();
    const int col = c.positionInBlock();
    int from = 0;
    while (true) {
        const int open = line.indexOf(QStringLiteral("[["), from);
        if (open < 0) break;
        const int close = line.indexOf(QStringLiteral("]]"), open + 2);
        if (close < 0) break;
        if (col >= open && col <= close + 2) {
            QString inner = line.mid(open + 2, close - (open + 2));
            const int bar = inner.indexOf(QLatin1Char('|'));
            if (bar >= 0) inner = inner.left(bar);
            const int hash = inner.indexOf(QLatin1Char('#'));
            if (hash >= 0) inner = inner.left(hash);
            return inner.trimmed();
        }
        from = close + 2;
    }
    return QString();
}

void CodeEditor::mousePressEvent(QMouseEvent* event) {
    // Ctrl + 左鍵：若落在 [[wikilink]] 上 → 發出導覽訊號（Markdown）
    if ((event->modifiers() & Qt::ControlModifier) && event->button() == Qt::LeftButton) {
        const QString target = wikilinkAt(event->pos());
        if (!target.isEmpty()) {
            emit wikilinkActivated(target);
            event->accept();
            return;
        }
    }
    // Alt + 左鍵：開始矩形（欄位）選取
    if (!m_largeFile && (event->modifiers() & Qt::AltModifier)
        && event->button() == Qt::LeftButton) {
        clearExtraCursors();
        const QTextCursor c = cursorForPosition(event->pos());
        m_boxSelecting = true;
        m_boxAnchorLine = c.blockNumber();
        m_boxAnchorCol  = c.positionInBlock();
        setTextCursor(c);
        event->accept();
        return;
    }
    clearExtraCursors();
    QPlainTextEdit::mousePressEvent(event);
}

void CodeEditor::mouseMoveEvent(QMouseEvent* event) {
    if (m_boxSelecting) {
        const QTextCursor c = cursorForPosition(event->pos());
        applyBoxSelection(c.blockNumber(), c.positionInBlock());
        event->accept();
        return;
    }
    QPlainTextEdit::mouseMoveEvent(event);
}

void CodeEditor::mouseReleaseEvent(QMouseEvent* event) {
    if (m_boxSelecting) {
        m_boxSelecting = false;
        event->accept();
        return;
    }
    QPlainTextEdit::mouseReleaseEvent(event);
}

// 依錨點→目前點建立每行一個 QTextCursor（重用多游標基礎建設）
void CodeEditor::applyBoxSelection(int curLine, int curCol) {
    QTextDocument* doc = document();
    QVector<int> lineLengths;
    lineLengths.reserve(doc->blockCount());
    for (QTextBlock b = doc->begin(); b.isValid(); b = b.next())
        lineLengths.append(b.length() - 1);          // 不含區塊結尾換行

    const QList<BoxSelect::Range> ranges =
        BoxSelect::compute(m_boxAnchorLine, m_boxAnchorCol, curLine, curCol, lineLengths);
    if (ranges.isEmpty()) return;

    m_extraCursors.clear();
    bool first = true;
    for (const BoxSelect::Range& r : ranges) {
        const QTextBlock blk = doc->findBlockByNumber(r.line);
        if (!blk.isValid()) continue;
        QTextCursor cur(doc);
        cur.setPosition(blk.position() + r.start);
        cur.setPosition(blk.position() + r.end, QTextCursor::KeepAnchor);
        if (first) { setTextCursor(cur); first = false; }   // 主游標
        else       { m_extraCursors.append(cur); }          // 其餘為額外游標
    }
    updateExtraHighlights();
    viewport()->update();
}

// ----------------------------------------------------------------
// Snippet 樣板：游標前字詞為 trigger 時以 Tab 展開
// 佔位符：${N:預設} 保留預設值、$0 為展開後游標位置、其餘 $N 移除
// ----------------------------------------------------------------
bool CodeEditor::expandSnippet() {
    const QString trigger = wordUnderCursor();
    const auto it = m_snippets.constFind(trigger);
    if (trigger.isEmpty() || it == m_snippets.constEnd()) return false;

    // 多行 body 跟隨目前行縮排
    QTextCursor c = textCursor();
    const QString lineText = c.block().text();
    QString indent;
    for (const QChar& ch : lineText) {
        if (ch == ' ' || ch == '\t') indent += ch;
        else break;
    }
    QString body = it.value();
    body.replace(QStringLiteral("\n"), QStringLiteral("\n") + indent);

    // $0 = 游標落點（先記位置，再剝除所有佔位符）
    const int cursorMark = body.indexOf(QStringLiteral("$0"));
    const QString before = cursorMark < 0
        ? body : LspProtocol::stripSnippetPlaceholders(body.left(cursorMark));
    body = LspProtocol::stripSnippetPlaceholders(body);

    for (int i = 0; i < trigger.length(); ++i)
        c.movePosition(QTextCursor::PreviousCharacter, QTextCursor::KeepAnchor);
    const int insertPos = c.selectionStart();
    c.insertText(body);
    if (cursorMark >= 0) {
        c.setPosition(insertPos + before.length());
        setTextCursor(c);
    }
    return true;
}

// ----------------------------------------------------------------
// 程式碼摺疊：大括號優先，無大括號時退回縮排判斷（Python 等）
// ----------------------------------------------------------------
static int lineIndent(const QString& text) {
    int n = 0;
    for (const QChar& c : text) {
        if (c == ' ') ++n;
        else if (c == '\t') n += 4;
        else return n;
    }
    return -1;                                   // 空行 / 全空白
}

bool CodeEditor::isFoldCandidate(const QTextBlock& block) const {
    const QString text = block.text();
    int depth = 0;
    bool hasBrace = false;
    for (const QChar& c : text) {
        if (c == '{') { ++depth; hasBrace = true; }
        else if (c == '}') --depth;
    }
    if (hasBrace) return depth > 0;
    const int indent = lineIndent(text);
    if (indent < 0) return false;
    QTextBlock next = block.next();              // 下一個非空行縮排更深 → 縮排區域起點
    while (next.isValid() && lineIndent(next.text()) < 0) next = next.next();
    return next.isValid() && lineIndent(next.text()) > indent;
}

int CodeEditor::foldEndLine(const QTextBlock& startBlock) const {
    const QString text = startBlock.text();
    int depth = 0;
    bool hasBrace = false;
    for (const QChar& c : text) {
        if (c == '{') { ++depth; hasBrace = true; }
        else if (c == '}') --depth;
    }
    if (hasBrace && depth > 0) {                 // 大括號式：找回到深度 0 的行
        QTextBlock b = startBlock.next();
        while (b.isValid()) {
            for (const QChar& c : b.text()) {
                if (c == '{') ++depth;
                else if (c == '}' && --depth == 0) return b.blockNumber();
            }
            b = b.next();
        }
        return -1;
    }
    const int indent = lineIndent(text);         // 縮排式：連續縮排更深的行（含中間空行）
    if (indent < 0) return -1;
    int lastDeep = -1;
    for (QTextBlock b = startBlock.next(); b.isValid(); b = b.next()) {
        const int i = lineIndent(b.text());
        if (i < 0) continue;                     // 空行先跳過，是否納入取決於後續行
        if (i <= indent) break;
        lastDeep = b.blockNumber();
    }
    return lastDeep > startBlock.blockNumber() ? lastDeep : -1;
}

int CodeEditor::foldIndexAtStart(int line) const {
    for (int i = 0; i < m_folds.size(); ++i)
        if (m_folds[i].start.blockNumber() == line) return i;
    return -1;
}

void CodeEditor::applyFoldVisibility() {
    for (QTextBlock b = document()->begin(); b.isValid(); b = b.next())
        b.setVisible(true);
    for (const Fold& f : m_folds) {
        const int end = f.end.blockNumber();
        QTextBlock b = document()->findBlockByNumber(f.start.blockNumber() + 1);
        for (; b.isValid() && b.blockNumber() <= end; b = b.next())
            b.setVisible(false);
    }
    // 重排版（QPlainTextEdit 需主動要求，否則捲動範圍與繪製不更新）
    document()->markContentsDirty(0, document()->characterCount());
    viewport()->update();
    lineNumberArea->update();
}

QList<int> CodeEditor::foldedStartLines() const {
    QList<int> out;
    for (const Fold& f : m_folds) out << f.start.blockNumber();
    return out;
}

void CodeEditor::applyFolds(const QList<int>& lines) {
    for (int line : lines)
        if (foldIndexAtStart(line) < 0)     // 尚未摺疊才摺
            toggleFoldAt(line);
}

void CodeEditor::toggleFoldAt(int line) {
    if (m_largeFile) return;
    const int existing = foldIndexAtStart(line);
    if (existing >= 0) {
        m_folds.removeAt(existing);              // 展開（巢狀摺疊由 applyFoldVisibility 重套）
        applyFoldVisibility();
        return;
    }
    const QTextBlock startBlock = document()->findBlockByNumber(line);
    if (!startBlock.isValid()) return;
    const int end = foldEndLine(startBlock);
    if (end <= line) return;

    // 游標在摺疊區內 → 移至起始行尾，避免落在隱藏區
    const int curLine = textCursor().blockNumber();
    if (curLine > line && curLine <= end) {
        QTextCursor c(startBlock);
        c.movePosition(QTextCursor::EndOfBlock);
        setTextCursor(c);
    }

    Fold f;
    f.start = QTextCursor(startBlock);
    f.end = QTextCursor(document()->findBlockByNumber(end));
    m_folds.append(f);
    applyFoldVisibility();
}

void CodeEditor::foldCurrentRegion() {
    const int cur = textCursor().blockNumber();
    // 目前行不可摺疊時向上找最近的可摺疊行（上限 200 行）
    for (int line = cur; line >= 0 && line > cur - 200; --line) {
        const QTextBlock b = document()->findBlockByNumber(line);
        if (foldIndexAtStart(line) >= 0) continue;       // 已摺疊的跳過
        const int end = foldEndLine(b);
        if (end >= cur && end > line) {
            toggleFoldAt(line);
            return;
        }
    }
}

void CodeEditor::unfoldCurrentRegion() {
    const int cur = textCursor().blockNumber();
    for (int i = m_folds.size() - 1; i >= 0; --i) {
        if (m_folds[i].start.blockNumber() <= cur && m_folds[i].end.blockNumber() >= cur) {
            m_folds.removeAt(i);
            applyFoldVisibility();
            return;
        }
    }
}

void CodeEditor::lineNumberAreaMousePress(QMouseEvent* e) {
    if (m_largeFile || e->pos().x() < lineNumberArea->width() - 16) return;
    // 找出點擊位置對應的行（同 paint 迴圈的幾何計算）
    QTextBlock block = firstVisibleBlock();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    while (block.isValid() && top <= e->pos().y()) {
        const int bottom = top + qRound(blockBoundingRect(block).height());
        if (block.isVisible() && e->pos().y() < bottom) {
            toggleFoldAt(block.blockNumber());
            return;
        }
        block = block.next();
        top = bottom;
    }
}

void CodeEditor::setDiagnostics(const QList<LspProtocol::Diagnostic>& diags) {
    m_diagnostics = diags;
    updateExtraHighlights();
}

void CodeEditor::diagnosticCounts(int* errors, int* warnings) const {
    int e = 0, w = 0;
    for (const auto& d : m_diagnostics)
        (d.severity <= 1 ? e : w)++;
    if (errors) *errors = e;
    if (warnings) *warnings = w;
}

void CodeEditor::appendDiagnosticSelections(QList<QTextEdit::ExtraSelection>& selections) {
    if (m_diagnostics.isEmpty() || m_largeFile) return;
    QTextDocument* doc = document();
    for (const auto& d : m_diagnostics) {
        const QTextBlock startBlock = doc->findBlockByNumber(d.startLine);
        const QTextBlock endBlock   = doc->findBlockByNumber(d.endLine);
        if (!startBlock.isValid() || !endBlock.isValid()) continue;
        int from = startBlock.position() + qMin(d.startChar, int(startBlock.length()) - 1);
        int to   = endBlock.position()   + qMin(d.endChar,   int(endBlock.length()) - 1);
        if (to <= from) to = qMin(from + 1, doc->characterCount() - 1);   // 零寬範圍至少劃一字元
        if (to <= from) continue;

        QTextEdit::ExtraSelection sel;
        sel.format.setUnderlineStyle(QTextCharFormat::WaveUnderline);
        sel.format.setUnderlineColor(QColor(d.severity <= 1 ? Theme::LSP_ERROR : Theme::LSP_WARNING));
        sel.cursor = QTextCursor(doc);
        sel.cursor.setPosition(from);
        sel.cursor.setPosition(to, QTextCursor::KeepAnchor);
        selections.append(sel);
    }
}

int CodeEditor::diagnosticIndexAt(const QTextCursor& cursor) const {
    const int line = cursor.blockNumber();
    const int ch = cursor.positionInBlock();
    for (int i = 0; i < m_diagnostics.size(); ++i) {
        const auto& d = m_diagnostics[i];
        if (line < d.startLine || line > d.endLine) continue;
        if (line == d.startLine && ch < d.startChar) continue;
        if (line == d.endLine && ch > d.endChar) continue;
        return i;
    }
    return -1;
}

bool CodeEditor::event(QEvent* e) {
    if (e->type() == QEvent::ToolTip) {
        auto* he = static_cast<QHelpEvent*>(e);
        const QTextCursor cursor = cursorForPosition(viewport()->mapFrom(this, he->pos()));
        const int idx = diagnosticIndexAt(cursor);
        if (idx >= 0) {
            const auto& d = m_diagnostics[idx];
            QString msg = d.message;
            if (!d.source.isEmpty()) msg += QStringLiteral("  [%1]").arg(d.source);
            QToolTip::showText(he->globalPos(), msg, this);
            return true;
        }
        if (m_lspEnabled) {
            m_lastHoverGlobalPos = he->globalPos();
            emit lspHoverRequested(cursor.blockNumber(), cursor.positionInBlock());
            return true;        // 回應到達後由 showHoverText 顯示
        }
    }
    return QPlainTextEdit::event(e);
}

void CodeEditor::showHoverText(const QString& text) {
    if (!text.isEmpty())
        QToolTip::showText(m_lastHoverGlobalPos, text, this);
}

void CodeEditor::triggerLocalCompletion() {
    if (m_largeFile || !m_assist) return;
    const QString prefix = wordUnderCursor();
    const QStringList items =
        LocalCompletion::suggest(toPlainText(), textCursor().position(), prefix, 30);
    if (!items.isEmpty())
        showLspCompletions(items);     // 重用 SuggestionWidget：插入時取代游標下字詞前綴
}

void CodeEditor::showLspCompletions(const QStringList& items) {
    if (items.isEmpty()) return;
    if (m_completer) m_completer->popup()->hide();      // 避免與本機字詞補全同時顯示
    m_suggestionsAreLsp = true;
    suggestionWidget->showSuggestions(items, mapToGlobal(cursorRect().bottomLeft()));
}

void CodeEditor::contextMenuEvent(QContextMenuEvent *event) {
    QMenu *menu = createStandardContextMenu();
    menu->addSeparator();

    if (m_lspEnabled) {
        const QTextCursor c = cursorForPosition(event->pos());
        const int line = c.blockNumber(), ch = c.positionInBlock();
        menu->addAction(tr("跳至定義\tF12"), this, [this, line, ch]() {
            emit lspDefinitionRequested(line, ch);
        });
        menu->addAction(tr("全部引用\tShift+F12"), this, [this, line, ch]() {
            emit lspReferencesRequested(line, ch);
        });
        menu->addAction(tr("重新命名符號\tCtrl+Alt+R"), this, [this, line, ch]() {
            emit lspRenameRequested(line, ch);
        });
        menu->addAction(tr("格式化文件\tShift+Alt+F"), this, [this]() {
            emit lspFormatRequested();
        });
        menu->addSeparator();
    }

    QAction *callGraphAction = menu->addAction(tr("Show Call Graph"));
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
        while (i.hasNext() && deps.size() < 10) {
            QRegularExpressionMatch match = i.next();
            QString func = match.captured(1);
            if (func != word && func != "if" && func != "while" && func != "for" && func != "switch" && func != "catch" && func != "return" && func != "sizeof") {
                deps.insert(func);
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

// ----------------------------------------------------------------
// 書籤（持久 QTextCursor 實作，行插刪後位置自動跟隨）
// ----------------------------------------------------------------
void CodeEditor::toggleBookmark() {
    const int line = textCursor().blockNumber();
    for (int i = 0; i < m_bookmarks.size(); ++i) {
        if (m_bookmarks[i].blockNumber() == line) {
            m_bookmarks.removeAt(i);
            lineNumberArea->update();
            return;
        }
    }
    QTextCursor c(textCursor().block());
    m_bookmarks.append(c);
    lineNumberArea->update();
}

void CodeEditor::nextBookmark() {
    if (m_bookmarks.isEmpty()) return;
    const int current = textCursor().blockNumber();
    int best = -1, wrap = INT_MAX;
    for (const QTextCursor& c : m_bookmarks) {
        const int ln = c.blockNumber();
        if (ln > current && (best == -1 || ln < best)) best = ln;
        wrap = qMin(wrap, ln);
    }
    gotoLine((best != -1 ? best : wrap) + 1);
}

void CodeEditor::prevBookmark() {
    if (m_bookmarks.isEmpty()) return;
    const int current = textCursor().blockNumber();
    int best = -1, wrap = -1;
    for (const QTextCursor& c : m_bookmarks) {
        const int ln = c.blockNumber();
        if (ln < current && ln > best) best = ln;
        wrap = qMax(wrap, ln);
    }
    gotoLine((best != -1 ? best : wrap) + 1);
}

QList<int> CodeEditor::bookmarkedLines() const {
    QList<int> lines;
    for (const QTextCursor& c : m_bookmarks) lines << c.blockNumber();
    std::sort(lines.begin(), lines.end());
    lines.erase(std::unique(lines.begin(), lines.end()), lines.end());
    return lines;
}

void CodeEditor::setBookmarkedLines(const QList<int>& lines) {
    m_bookmarks.clear();
    for (int ln : lines) {
        QTextBlock b = document()->findBlockByNumber(ln);
        if (b.isValid()) m_bookmarks.append(QTextCursor(b));
    }
    lineNumberArea->update();
}

// ----------------------------------------------------------------
// 字詞自動完成（QCompleter，掃描目前文件識別字）
// ----------------------------------------------------------------
void CodeEditor::setupCompleter() {
    m_completer = new QCompleter(this);
    m_completer->setModel(new QStringListModel(m_completer));
    m_completer->setWidget(this);
    m_completer->setCompletionMode(QCompleter::PopupCompletion);
    m_completer->setCaseSensitivity(Qt::CaseInsensitive);
    connect(m_completer, QOverload<const QString&>::of(&QCompleter::activated),
            this, &CodeEditor::insertSuggestion); // 重用既有插入邏輯之前先處理前綴
    disconnect(m_completer, nullptr, this, nullptr);
    connect(m_completer, QOverload<const QString&>::of(&QCompleter::activated),
            this, [this](const QString& completion) {
        QTextCursor tc = textCursor();
        const int extra = completion.length() - m_completer->completionPrefix().length();
        if (extra > 0) tc.insertText(completion.right(extra));
        setTextCursor(tc);
    });

    // 文件變動後 800ms 重建字詞模型（debounce）
    m_completerRebuildTimer = new QTimer(this);
    m_completerRebuildTimer->setSingleShot(true);
    m_completerRebuildTimer->setInterval(800);
    connect(m_completerRebuildTimer, &QTimer::timeout, this, &CodeEditor::rebuildCompleterModel);
    connect(document(), &QTextDocument::contentsChanged, this, [this]() {
        m_completerRebuildTimer->start();
    });
    rebuildCompleterModel();
}

void CodeEditor::rebuildCompleterModel() {
    if (m_largeFile || !m_assist || document()->characterCount() > 2000000) return; // 大檔/停用輔助跳過
    static const QRegularExpression wordRe(QStringLiteral("[A-Za-z_][A-Za-z0-9_]{3,}"));
    QSet<QString> words;
    auto it = wordRe.globalMatch(toPlainText());
    while (it.hasNext() && words.size() < 20000) words.insert(it.next().captured());
    static_cast<QStringListModel*>(m_completer->model())->setStringList(QStringList(words.begin(), words.end()));
}

QString CodeEditor::wordUnderCursor() const {
    QTextCursor tc = textCursor();
    tc.select(QTextCursor::WordUnderCursor);
    return tc.selectedText();
}

void CodeEditor::setKeywordHighlights(const QList<QPair<QString, QColor>>& kws) {
    m_kwHighlights = kws;
    updateExtraHighlights();
    viewport()->update();
}

// ----------------------------------------------------------------
// 巨集
// ----------------------------------------------------------------
void CodeEditor::startMacroRecording() { m_macro.clear(); m_macroRecording = true; }
void CodeEditor::stopMacroRecording()  { m_macroRecording = false; }

void CodeEditor::playMacro(int times) {
    if (m_macroRecording || m_macro.isEmpty()) return;
    QTextCursor c = textCursor();
    c.beginEditBlock();
    for (int t = 0; t < times; ++t) {
        for (const MacroKey& k : m_macro) {
            QKeyEvent ev(QEvent::KeyPress, k.key, k.mods, k.text);
            keyPressEvent(&ev);
        }
    }
    c.endEditBlock();
}
