#pragma once
#include <QObject>
#include <functional>
#include "LspProtocol.h"

class QMainWindow;
class QListWidget;
class QDockWidget;
class LspManager;
class CodeEditor;

// 從 MainWindow 拆出的 LSP 子系統核心（LSP 子系統拆分的第 3 段）：
// 建立並持有 LspManager；處理「找引用」「重新命名」「格式化」「伺服器失敗」
// 這幾個不強烈依賴「目前作用中分頁是誰」的邏輯。
//
// diagnosticsReceived / completionReady / definitionReady / hoverReady / statusChanged
// 這幾個天生要看 activeEditor() 的邏輯，仍留在 MainWindow::setupLsp()（透過
// lspController->lsp() 存取底層 LspManager）——硬搬過來對降低耦合度沒有實質幫助，
// 只是換個地方寫一樣的東西，徒增這次改動的風險。
class LspController : public QObject {
    Q_OBJECT
public:
    // resolveEditor(path, openIfMissing)：找已開啟的分頁；openIfMissing 為 true 時，
    // 找不到就開新分頁再回傳。用回呼取代 controller 直接認識 tabWidget。
    LspController(std::function<CodeEditor*(const QString&, bool)> resolveEditor,
                 QMainWindow* host, QObject* parent = nullptr);

    LspManager* lsp() const { return m_lsp; }
    QDockWidget* refsDock() const { return m_refsDock; }
    QListWidget* refsList() const { return m_refsList; }

signals:
    void statusMessage(const QString& text, int timeoutMs);
    void referenceActivated(const QString& path, int line);   // 雙擊引用結果：開檔後跳到該行

private:
    std::function<CodeEditor*(const QString&, bool)> m_resolveEditor;
    LspManager* m_lsp = nullptr;
    QListWidget* m_refsList = nullptr;
    QDockWidget* m_refsDock = nullptr;
};
