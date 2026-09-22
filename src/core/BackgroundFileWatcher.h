#pragma once
#include <QObject>
#include <QSet>
#include <QStringList>
#include <QThread>
#include <memory>

class QFileSystemWatcher;

// QFileSystemWatcher 搬到專屬背景執行緒。
// 原因：QFileSystemWatcher::addPaths() 會在「呼叫端執行緒」對每個路徑做 stat 並註冊監看
// （Windows：FindFirstChangeNotification；Linux：inotify_add_watch）。路徑在網路分享上時，
// 每個都是網路來回，在主執行緒呼叫會讓 UI 卡住數秒（"Not Responding"）。
// 介面與 QFileSystemWatcher 相近；所有操作非同步排入背景執行緒，訊號以 queued 方式回到擁有者執行緒。
// 檔案被「刪除後重建」（許多編輯器存檔的方式）時，背景執行緒自動重新加入監看。
class BackgroundFileWatcher : public QObject {
    Q_OBJECT
public:
    explicit BackgroundFileWatcher(QObject* parent = nullptr);
    ~BackgroundFileWatcher() override;

    void addPath(const QString& path);
    void addPaths(const QStringList& paths);
    void clear();                                  // 移除全部監看
    QStringList paths() const;                     // 已要求監看的路徑（本地鏡像，不需等背景完成）
    void waitForIdle();                            // 等背景執行緒處理完已排入的操作（測試用）

signals:
    void fileChanged(const QString& path);
    void directoryChanged(const QString& path);

private:
    QThread m_thread;
    QFileSystemWatcher* m_watcher = nullptr;       // 屬於 m_thread
    QSet<QString> m_paths;
    struct Rewatch;                                // 背景執行緒的重新監看狀態（僅背景存取）
    std::shared_ptr<Rewatch> m_rewatch;
};
