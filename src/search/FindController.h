#pragma once
#include <QObject>
#include <QRegularExpression>
#include <QTextDocument>
#include <functional>

class QWidget;
class QDialog;
class QLineEdit;
class QCheckBox;
class QLabel;
class QTimer;
class QListWidget;
class QDockWidget;
class CodeEditor;

// 從 MainWindow 拆出的 Find & Replace 子系統：對話框（大小寫/全字/Regex/即時計數防抖）、
// Find Next/Prev（繞回）、Replace / Replace All（可套用到全部分頁）、
// Count / Mark All / Find All（結果沿用 FILTER RESULTS 面板）。
// 與其他 controller 相同模式：不認識 tabWidget/statusBar —— 編輯器用 std::function 取得、
// 狀態列訊息用 signal 送出；FILTER RESULTS 面板元件在建構時傳入。
class FindController : public QObject {
    Q_OBJECT
public:
    using EditorFn  = std::function<CodeEditor*()>;         // 取目前作用中編輯器（可為 nullptr）
    using EditorsFn = std::function<QList<CodeEditor*>()>;  // 取全部開啟分頁的編輯器
    FindController(QWidget* dialogParent, EditorFn activeEditor, EditorsFn allEditors,
                   QListWidget* resultsList, QLabel* filterCountLabel,
                   QDockWidget* filterResultsDock, QObject* parent = nullptr);

    void showDialog();   // 開啟（或帶到前面）對話框，預填目前選取文字
    void findNext();     // F3；對話框未開時先開（沿用原行為）
    void findPrev();     // Shift+F3

signals:
    void statusMessage(const QString& text, int timeoutMs);

private:
    void ensureDialog();
    void performReplace();
    void performReplaceAll();
    void performFindCount();   // Count 按鈕：精確計數（無上限）
    void performMarkAll();
    void performFindAll();
    void updateFindCount();    // 即時計數（防抖後執行，有上限）
    QRegularExpression buildRegex() const;
    QTextDocument::FindFlags buildFlags(bool backward = false) const;

    QWidget* m_dialogParent = nullptr;
    EditorFn m_activeEditor;
    EditorsFn m_allEditors;
    QListWidget* m_resultsList = nullptr;        // FILTER RESULTS 清單（Find All 填入）
    QLabel* m_filterCountLabel = nullptr;
    QDockWidget* m_filterResultsDock = nullptr;

    QDialog* m_dialog = nullptr;
    QLineEdit* m_findInput = nullptr;
    QLineEdit* m_replaceInput = nullptr;
    QCheckBox* m_caseCheck = nullptr;
    QCheckBox* m_wholeWordCheck = nullptr;
    QCheckBox* m_regexCheck = nullptr;
    QCheckBox* m_replaceAllTabsCheck = nullptr;  // Replace All 是否套用到全部開啟分頁
    QLabel* m_countLabel = nullptr;              // 即時符合筆數
    QTimer* m_countTimer = nullptr;              // 即時計數防抖（大檔全文掃描不能跟著按鍵跑）
};
