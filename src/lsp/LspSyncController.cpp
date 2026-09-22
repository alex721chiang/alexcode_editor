#include "LspSyncController.h"
#include "LspManager.h"
#include "CodeEditor.h"
#include <QTimer>
#include <QTextDocument>
#include <QTextBlock>
#include <QTextCursor>
#include <algorithm>

LspSyncController::LspSyncController(LspManager* lsp, QObject* parent)
    : QObject(parent), m_lsp(lsp) {
    // didChange 防抖：編輯停頓 400ms 後送出完整內容
    m_changeTimer = new QTimer(this);
    m_changeTimer->setSingleShot(true);
    m_changeTimer->setInterval(400);
    connect(m_changeTimer, &QTimer::timeout, this, [this]() {
        for (const QPointer<CodeEditor>& e : m_dirtyEditors) {
            if (!e) continue;
            const QString path = e->property("filePath").toString();
            if (!path.isEmpty()) m_lsp->documentChanged(path, e->toPlainText());
        }
        m_dirtyEditors.clear();
    });
}

void LspSyncController::markDirty(CodeEditor* editor) {
    if (!m_dirtyEditors.contains(editor)) m_dirtyEditors.append(editor);
    m_changeTimer->start();
}

void LspSyncController::editorClosed(CodeEditor* editor) {
    m_dirtyEditors.removeAll(QPointer<CodeEditor>(editor));
}

// 以單一 Undo 步驟套用 LSP TextEdit（由後往前，避免位置位移）
void LspSyncController::applyTextEdits(CodeEditor* editor, const QList<LspProtocol::TextEdit>& edits) {
    QTextDocument* doc = editor->document();
    const auto offsetOf = [doc](int line, int ch) {
        const QTextBlock b = doc->findBlockByNumber(qMin(line, doc->blockCount() - 1));
        return b.position() + qMin(ch, qMax(int(b.length()) - 1, 0));
    };
    struct Span { int start, end; QString newText; };
    QList<Span> spans;
    spans.reserve(edits.size());
    for (const LspProtocol::TextEdit& e : edits)
        spans.append({ offsetOf(e.startLine, e.startChar), offsetOf(e.endLine, e.endChar), e.newText });
    std::sort(spans.begin(), spans.end(),
              [](const Span& a, const Span& b) { return a.start > b.start; });

    QTextCursor c(doc);
    c.beginEditBlock();
    for (const Span& s : spans) {
        c.setPosition(s.start);
        c.setPosition(qMax(s.end, s.start), QTextCursor::KeepAnchor);
        c.insertText(s.newText);
    }
    c.endEditBlock();
}
