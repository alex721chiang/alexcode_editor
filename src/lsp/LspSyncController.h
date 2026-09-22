#pragma once
#include <QObject>
#include <QList>
#include <QPointer>
#include "LspProtocol.h"

class QTimer;
class LspManager;
class CodeEditor;

// 從 MainWindow 拆出的 LSP 文件同步子系統（LSP 子系統拆分的第 2 段）：
// 編輯後 400ms 防抖送出 didChange，以及把伺服器回傳的 TextEdit（格式化/重新命名結果）
// 以單一 Undo 步驟套用回編輯器。
class LspSyncController : public QObject {
    Q_OBJECT
public:
    explicit LspSyncController(LspManager* lsp, QObject* parent = nullptr);

    void markDirty(CodeEditor* editor);    // 編輯後呼叫：加入待送出清單、（重新）啟動防抖計時器
    void editorClosed(CodeEditor* editor); // 分頁關閉時呼叫：從待送出清單移除，避免懸空指標

    // 純函式：由後往前套用（避免位置位移），不依賴任何 controller 狀態
    static void applyTextEdits(CodeEditor* editor, const QList<LspProtocol::TextEdit>& edits);

private:
    LspManager* m_lsp = nullptr;
    QTimer* m_changeTimer = nullptr;
    QList<QPointer<CodeEditor>> m_dirtyEditors;
};
