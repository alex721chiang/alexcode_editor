#pragma once
#include <QObject>

class QLabel;
class QWidget;
class LspManager;
class CodeEditor;

// 從 MainWindow 拆出的 LSP 狀態列子系統（LSP 子系統拆分的第 1 段）：
// 一個 QLabel，顯示目前檔案對應的 LSP 伺服器名稱、就緒狀態、錯誤/警告數，
// 就緒時順便把補全觸發字元設回編輯器。
//
// lsp 由呼叫端持有（目前仍在 MainWindow::setupLsp() 建立，尚未搬進 controller——
// 那是第 3 段的工作）；label() 拿到 QLabel 後由呼叫端自己 addPermanentWidget，
// 維持狀態列原本的視覺順序（其他狀態列項目在 setupUI() 別處依序建立）。
class LspStatusController : public QObject {
    Q_OBJECT
public:
    LspStatusController(LspManager* lsp, QWidget* labelParent, QObject* parent = nullptr);

    QLabel* label() const { return m_label; }
    void update(CodeEditor* activeEditor);   // 對應原本 MainWindow::updateLspStatus()

private:
    LspManager* m_lsp = nullptr;
    QLabel* m_label = nullptr;
};
