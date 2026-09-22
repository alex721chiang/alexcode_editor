#pragma once
#include <QObject>
#include "TsSymbols.h"

class QMainWindow;
class QTreeWidget;
class QTreeWidgetItem;
class QDockWidget;
class QLineEdit;
class CodeEditor;

// Function List 常駐面板（對應 Notepad++ 的 Function List）：側欄樹狀顯示目前檔案的
// 類別/函式（依巢狀深度分層），點擊跳轉到該行。跟 Ctrl+Shift+O 的 SymbolDialog 是同一份
// 資料來源（CodeEditor::documentSymbols()，重用既有 tree-sitter 符號擷取），
// 差別只在「彈出式快速跳轉」vs.「常駐側欄瀏覽」，成本很低。
// 對齊 Notepad++ 的兩個互動：頂部過濾框（子字串比對，保留父階層）、
// 游標移動時自動高亮目前所在的符號（不搶焦點）。
class FunctionListController : public QObject {
    Q_OBJECT
public:
    explicit FunctionListController(QMainWindow* host, QObject* parent = nullptr);

    QDockWidget* dock() const { return m_dock; }
    void refresh(CodeEditor* activeEditor);   // 重新整理清單；nullptr 表示清空（沒有作用中分頁）
    void highlightLine(int line);             // 游標移到某行（0-based）→ 選取包含該行的最深符號

signals:
    void lineActivated(int line);   // 使用者點了某個符號項目 → 該行（0-based）

private:
    void applyFilter(const QString& text);    // 依過濾字串顯示/隱藏項目（命中者的祖先保持可見）

    QDockWidget* m_dock = nullptr;
    QTreeWidget* m_tree = nullptr;
    QLineEdit* m_filter = nullptr;
};
