#pragma once
#include <QObject>
#include "TsSymbols.h"

class QMainWindow;
class QTreeWidget;
class QDockWidget;
class CodeEditor;

// Function List 常駐面板（對應 Notepad++ 的 Function List）：側欄樹狀顯示目前檔案的
// 類別/函式（依巢狀深度分層），點擊跳轉到該行。跟 Ctrl+Shift+O 的 SymbolDialog 是同一份
// 資料來源（CodeEditor::documentSymbols()，重用既有 tree-sitter 符號擷取），
// 差別只在「彈出式快速跳轉」vs.「常駐側欄瀏覽」，成本很低。
class FunctionListController : public QObject {
    Q_OBJECT
public:
    explicit FunctionListController(QMainWindow* host, QObject* parent = nullptr);

    QDockWidget* dock() const { return m_dock; }
    void refresh(CodeEditor* activeEditor);   // 重新整理清單；nullptr 表示清空（沒有作用中分頁）

signals:
    void lineActivated(int line);   // 使用者點了某個符號項目 → 該行（0-based）

private:
    QDockWidget* m_dock = nullptr;
    QTreeWidget* m_tree = nullptr;
};
