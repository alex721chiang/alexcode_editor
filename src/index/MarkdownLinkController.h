#pragma once
#include <QObject>
#include <QFutureWatcher>
#include <functional>
#include "MarkdownLinkIndex.h"

class QMainWindow;
class QDockWidget;
class QListWidget;
class QListWidgetItem;
class GraphView;

// 從 MainWindow 拆出的 Markdown wikilink / backlinks / 關係圖子系統（Obsidian 風）。
// 建構子裡就把 BACKLINKS、GRAPH 兩個 dock 建好並加進 host 視窗；索引本身用值成員
// （取代拆分前 `new MarkdownLinkIndex()` 從不 delete 的寫法）。
//
// 附帶修掉一個既有小 bug：原本 MainWindow 裡「先幫 dock 接 visibilityChanged signal、
// dock 物件晚點才 new」，導致 connect() 在 dock 還是 nullptr 時就執行——Qt 會印
// "invalid nullptr parameter" 警告，而且完全沒接上（點 dock 右上角關閉鈕不會同步
// 選單裡的勾選狀態）。拆出來後 dock 在 controller 建構子裡就緒，順序自然正確。
class MarkdownLinkController : public QObject {
    Q_OBJECT
public:
    explicit MarkdownLinkController(QMainWindow* host, QObject* parent = nullptr);

    QDockWidget* backlinksDock() const { return m_backlinksDock; }
    QDockWidget* graphDock() const { return m_graphDock; }

    // activeFilePath：目前作用中分頁的檔案路徑，由 MainWindow 傳入
    // （controller 不需要認識 CodeEditor/tabWidget，維持解耦）。
    // 索引於背景執行緒建立（整棵資料夾掃描 + 讀所有 .md；網路分享上不再卡住主執行緒），
    // 且 Backlinks / Graph 面板都未開啟時只標記過期、不建（延後到真正需要時）。
    void rebuildIndex(const QString& projectFolder, const QString& activeFilePath);
    bool isIndexing() const { return m_buildWatcher && m_buildWatcher->isRunning(); }
    void refreshBacklinks(const QString& activeFilePath);
    void showGraphView(const QString& activeFilePath);
    void openOrCreateWikilink(const QString& target, const QString& projectFolder);

signals:
    void statusMessage(const QString& text, int timeoutMs);   // 取代原本直接呼叫 statusBar()
    void fileOpenRequested(const QString& path);               // 使用者點了 backlink/圖節點，要求開檔
    void indexRebuilt();                                        // 背景索引完成並已換上

private:
    void startBuild();
    void openOrCreateWikilinkNow(const QString& target, const QString& projectFolder);

    MarkdownLinkIndex m_index;
    QFutureWatcher<MarkdownLinkIndex>* m_buildWatcher = nullptr;
    QString m_folder;                 // 目前（或下一次）要索引的資料夾
    QString m_activeFile;             // 完成時刷新 backlinks / graph 用
    bool m_stale = true;              // 索引與磁碟內容可能不一致（從未建立或已有變更）
    bool m_rerun = false;             // 建置中又收到重建要求：完成後再跑一次
    QList<std::function<void()>> m_afterBuild;   // 等索引就緒才執行的動作（wikilink 解析）
    QDockWidget* m_backlinksDock = nullptr;
    QListWidget* m_backlinksList = nullptr;
    QDockWidget* m_graphDock = nullptr;
    GraphView* m_graphView = nullptr;
};
