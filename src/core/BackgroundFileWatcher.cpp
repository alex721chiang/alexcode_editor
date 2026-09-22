#include "BackgroundFileWatcher.h"
#include <QFileSystemWatcher>
#include <QFileInfo>
#include <memory>

// 只在背景執行緒存取：等待重建的檔案、為此暫時監看的目錄
struct BackgroundFileWatcher::Rewatch {
    QSet<QString> pending;
    QSet<QString> tempDirs;
};

BackgroundFileWatcher::BackgroundFileWatcher(QObject* parent) : QObject(parent) {
    m_thread.setObjectName(QStringLiteral("BackgroundFileWatcher"));
    m_watcher = new QFileSystemWatcher;
    m_watcher->moveToThread(&m_thread);
    connect(&m_thread, &QThread::finished, m_watcher, &QObject::deleteLater);

    QFileSystemWatcher* w = m_watcher;
    auto st = std::make_shared<Rewatch>();
    m_rewatch = st;

    // 被刪除/覆寫重建的檔案會從監看清單掉出：存在就立刻加回；若刪除通知早於重建（非原子存檔），
    // 暫時監看其所在目錄，檔案重新出現時加回並補發一次 fileChanged。
    connect(m_watcher, &QFileSystemWatcher::fileChanged, m_watcher, [this, w, st](const QString& p) {
        if (!w->files().contains(p)) {
            if (QFileInfo::exists(p)) {
                w->addPath(p);
            } else {
                st->pending.insert(p);
                const QString dir = QFileInfo(p).absolutePath();
                if (!w->directories().contains(dir) && w->addPath(dir)) st->tempDirs.insert(dir);
            }
        }
        emit fileChanged(p);                       // 跨執行緒 → 以 queued 送達擁有者
    });
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, m_watcher, [this, w, st](const QString& d) {
        bool stillPending = false;
        for (auto it = st->pending.begin(); it != st->pending.end();) {
            if (QFileInfo(*it).absolutePath() != d) { ++it; continue; }
            if (QFileInfo::exists(*it)) {
                w->addPath(*it);
                emit fileChanged(*it);             // 內容已被替換
                it = st->pending.erase(it);
            } else {
                stillPending = true;
                ++it;
            }
        }
        if (st->tempDirs.contains(d)) {            // 暫時監看的目錄：不轉發給擁有者
            if (!stillPending) { w->removePath(d); st->tempDirs.remove(d); }
            return;
        }
        emit directoryChanged(d);
    });
    m_thread.start();
}

BackgroundFileWatcher::~BackgroundFileWatcher() {
    m_thread.quit();
    m_thread.wait();
}

void BackgroundFileWatcher::addPath(const QString& path) {
    addPaths(QStringList{path});
}

void BackgroundFileWatcher::addPaths(const QStringList& paths) {
    QStringList fresh;
    for (const QString& p : paths)
        if (!p.isEmpty() && !m_paths.contains(p)) { m_paths.insert(p); fresh << p; }
    if (fresh.isEmpty()) return;
    QFileSystemWatcher* w = m_watcher;
    QMetaObject::invokeMethod(w, [w, fresh]() { w->addPaths(fresh); }, Qt::QueuedConnection);
}

void BackgroundFileWatcher::clear() {
    m_paths.clear();
    QFileSystemWatcher* w = m_watcher;
    auto st = m_rewatch;
    QMetaObject::invokeMethod(w, [w, st]() {
        st->pending.clear();
        st->tempDirs.clear();
        const QStringList all = w->files() + w->directories();
        if (!all.isEmpty()) w->removePaths(all);
    }, Qt::QueuedConnection);
}

QStringList BackgroundFileWatcher::paths() const {
    return QStringList(m_paths.cbegin(), m_paths.cend());
}

void BackgroundFileWatcher::waitForIdle() {
    QMetaObject::invokeMethod(m_watcher, []() {}, Qt::BlockingQueuedConnection);
}
