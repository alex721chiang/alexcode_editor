#pragma once
#include <QObject>
#include <QPoint>
#include <QPointer>
#include <QList>

class QTabWidget;
class QWidget;
class QTextDocument;
class CodeEditor;

// 分頁「拖出」成獨立視窗（可放到另一個螢幕）。
// 獨立視窗是該分頁文件的另一個檢視（共用同一份 QTextDocument：編輯即時同步、高亮共用），
// 原分頁保留在主視窗——存檔、工作階段還原、LSP、Find 等都以分頁為單位運作，
// 真正把分頁搬走需大幅重構這些假設。
// 文件由分頁的編輯器擁有：分頁關閉 / 主視窗結束前必須先關閉對應視窗（editorClosing / closeAll），
// 否則獨立視窗會存取已刪除的文件。
class EditorWindowsController : public QObject {
    Q_OBJECT
public:
    EditorWindowsController(QTabWidget* tabs, QWidget* host);
    ~EditorWindowsController() override;

    QWidget* detach(CodeEditor* source);           // 已有該分頁的視窗時只帶到前景
    QList<QWidget*> detachedWindows() const;
    CodeEditor* viewOf(QWidget* window) const;
    void editorClosing(CodeEditor* source);        // 分頁即將銷毀（文件隨之銷毀）
    void closeAll();

signals:
    void activateTabRequested(CodeEditor* editor); // 「回到主視窗」
    void saveRequested(CodeEditor* editor);        // 在獨立視窗按 Ctrl+S
    void pinSplitRequested(CodeEditor* editor);    // 分頁右鍵「在分割檢視中並排顯示」

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;   // 分頁列拖出手勢

private:
    struct Entry {
        QPointer<CodeEditor> source;
        QWidget* window = nullptr;
        CodeEditor* view = nullptr;
        QTextDocument* ownDoc = nullptr;           // view 自有的空白文件：關閉前換回，脫離共用文件
    };
    int indexOfWindow(const QWidget* w) const;
    void closeEntry(int i);
    void showTabMenu(const QPoint& pos);

    QTabWidget* m_tabs;
    QWidget* m_host;
    QList<Entry> m_entries;
    QPointer<QWidget> m_pressWidget;               // 按下時的分頁（拖曳中分頁可能重新排序）
};
