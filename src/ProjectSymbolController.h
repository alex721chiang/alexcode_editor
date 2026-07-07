#pragma once
#include <QObject>
#include <QPointer>
#include <QFutureWatcher>
#include "ProjectSymbolIndex.h"

class QListWidget;
class QDockWidget;
class QWidget;
class ProjectSymbolDialog;

// 從 MainWindow 拆出的「專案符號索引」子系統：背景/同步建索引、Ctrl+T 符號搜尋、
// 文字版「找引用」（填 REFERENCES dock）。原本是 MainWindow 的 5 個私有方法 + 3 個成員，
// 直接耦合 statusBar()/openFileByPath()；拆出後改用 signal 通知，MainWindow 只需轉接。
//
// 索引本身用值成員（非 Phase 0 前的 `new ProjectSymbolIndex()`），生命週期跟著
// controller 走，不必手動 delete，也不會有原本「MainWindow 沒有解構子、索引永遠不被
// 釋放」的問題（雖然只在行程結束時才顯現，OS 會回收，但不是乾淨的寫法）。
class ProjectSymbolController : public QObject {
    Q_OBJECT
public:
    // dialogParent：ProjectSymbolDialog 的父視窗（通常是 MainWindow）
    // refsList / refsDock：REFERENCES 結果面板，findReferences() 會填入/顯示
    ProjectSymbolController(QWidget* dialogParent, QListWidget* refsList, QDockWidget* refsDock,
                            QObject* parent = nullptr);

    void rebuild(const QString& projectFolder);                    // 背景重建整個索引（不卡 UI）
    void buildSync(const QString& projectFolder);                  // 同步建索引（截圖/CLI 用）
    void updateFile(const QString& projectFolder, const QString& file);  // 增量：重解析單一檔
    void showSearch(const QString& projectFolder);                 // 開 Ctrl+T 專案符號搜尋對話框
    void findReferences(const QString& name);                      // 文字版找引用 → 填 REFERENCES dock

    ProjectSymbolIndex& index() { return m_index; }
    ProjectSymbolDialog* dialog() const { return m_dialog; }       // 給截圖流程直接操作/截圖用

signals:
    void statusMessage(const QString& text, int timeoutMs);        // 取代原本直接呼叫 statusBar()
    void symbolChosen(const QString& file, int line);               // 使用者在對話框選了符號

private:
    void ensureDialog();

    ProjectSymbolIndex m_index;
    QPointer<ProjectSymbolDialog> m_dialog;
    QFutureWatcher<ProjectSymbolIndex>* m_watcher = nullptr;
    QWidget* m_dialogParent = nullptr;
    QListWidget* m_refsList = nullptr;
    QDockWidget* m_refsDock = nullptr;
};
