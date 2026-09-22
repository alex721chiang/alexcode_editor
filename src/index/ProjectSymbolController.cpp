#include "ProjectSymbolController.h"
#include "IoPool.h"
#include "ProjectSymbolDialog.h"
#include "TextRefs.h"
#include <QtConcurrent>
#include <QListWidget>
#include <QDockWidget>
#include <QWidget>
#include <QFile>
#include <QFileInfo>
#include <QCoreApplication>
#include "BackgroundFileWatcher.h"
#include <QTimer>
#include <QDir>
#include "PathKind.h"
#include "Portable.h"
#include "StallMonitor.h"

ProjectSymbolController::ProjectSymbolController(QWidget* dialogParent, QListWidget* refsList,
                                                 QDockWidget* refsDock, QObject* parent)
    : QObject(parent), m_dialogParent(dialogParent), m_refsList(refsList), m_refsDock(refsDock) {}

void ProjectSymbolController::rebuild(const QString& projectFolder) {
    if (projectFolder.isEmpty()) return;
    if (m_watcher && m_watcher->isRunning()) return;   // 已在索引中

    if (!m_watcher) {
        m_watcher = new QFutureWatcher<ProjectSymbolIndex>(this);
        connect(m_watcher, &QFutureWatcher<ProjectSymbolIndex>::finished, this, [this]() {
            m_index = m_watcher->result();      // 在主執行緒換上新索引
            emit statusMessage(
                tr("專案符號索引完成：%1 檔、%2 個符號")
                    .arg(m_index.fileCount()).arg(m_index.symbolCount()), 4000);
            rescanWatchedDirs();                // 子目錄可能有增減，重掃監看清單
        });
    }
    emit statusMessage(tr("正在背景索引專案符號…"), 0);
    const QString folder = projectFolder;
    m_watcher->setFuture(QtConcurrent::run(IoPool::instance(), [folder]() {       // 背景掃描+解析，不卡 UI
        ProjectSymbolIndex idx;
        idx.build(folder);
        return idx;
    }));
}

// Phase 3：監看專案資料夾。QFileSystemWatcher 的 directoryChanged 只在目錄內
// 新增/刪除/更名時觸發（單純改內容不會），正好對應「git pull / 外部工具產生檔案」
// 這種現有 updateFile()（存檔 hook）涵蓋不到的情境。debounce 吸收批次變更風暴，
// rebuild() 走 mtime 增量，未變更的檔不重解析，代價很低。
void ProjectSymbolController::enableAutoRefresh(const QString& projectFolder) {
    m_folder = projectFolder;
    m_watchEnabled = shouldWatch(projectFolder);
    if (!m_fsWatcher) {
        m_fsWatcher = new BackgroundFileWatcher(this);
        m_fsDebounce = new QTimer(this);
        m_fsDebounce->setSingleShot(true);
        m_fsDebounce->setInterval(1500);
        connect(m_fsWatcher, &BackgroundFileWatcher::directoryChanged, this,
                [this](const QString&) { m_fsDebounce->start(); });
        connect(m_fsDebounce, &QTimer::timeout, this, [this]() {
            if (m_watcher && m_watcher->isRunning()) { m_fsDebounce->start(); return; }   // 建置中，稍後再試
            rebuild(m_folder);
        });
    }
    if (!m_watchEnabled) {                               // 網路資料夾：清掉舊監看，不再掛新的
        ++m_scanGeneration;
        m_fsWatcher->clear();
        return;
    }
    rescanWatchedDirs();
}

bool ProjectSymbolController::shouldWatch(const QString& folder) {
    if (!PathKind::isNetworkPath(folder)) return true;
    return AppSettings().value("index/watchNetworkFolders", false).toBool();
}

QStringList ProjectSymbolController::watchedDirs() const {
    return m_fsWatcher ? m_fsWatcher->paths() : QStringList();
}

QStringList ProjectSymbolController::collectWatchDirs(const QString& root, int maxDirs) {
    QStringList dirs{root};
    QStringList queue{root};
    while (!queue.isEmpty() && dirs.size() < maxDirs) {
        const QString cur = queue.takeFirst();
        const QFileInfoList subs = QDir(cur).entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
        for (const QFileInfo& fi : subs) {
            if (ProjectSymbolIndex::skipDirs().contains(fi.fileName())) continue;   // 剪枝
            if (dirs.size() >= maxDirs) break;
            dirs.append(fi.absoluteFilePath());
            queue.append(fi.absoluteFilePath());
        }
    }
    return dirs;
}

// BFS 巡訪與掛監看都在背景執行緒（每層 entryInfoList、每個 addPath 在網路分享上都是網路來回）。
void ProjectSymbolController::rescanWatchedDirs() {
    if (!m_fsWatcher || m_folder.isEmpty() || !m_watchEnabled) return;
    // Windows 每個目錄佔一個 ReadDirectoryChangesW 資源，超大型專案全掛監看反而拖累系統
    constexpr int kMaxWatchedDirs = 512;
    const int gen = ++m_scanGeneration;
    const QString folder = m_folder;
    auto* w = new QFutureWatcher<QStringList>(this);
    connect(w, &QFutureWatcher<QStringList>::finished, this, [this, w, gen]() {
        const QStringList dirs = w->result();
        w->deleteLater();
        if (gen != m_scanGeneration) return;            // 期間已換資料夾或再次重掃：丟棄
        STALL_SCOPE("symbol.watcher.addPaths");
        m_fsWatcher->clear();
        m_fsWatcher->addPaths(dirs);
        emit watchDirsUpdated();
    });
    w->setFuture(QtConcurrent::run(IoPool::instance(), &ProjectSymbolController::collectWatchDirs, folder, kMaxWatchedDirs));
}

void ProjectSymbolController::buildSync(const QString& projectFolder) {
    if (projectFolder.isEmpty()) return;
    m_index.build(projectFolder);        // 截圖：同步建索引（避開背景非同步）
}

void ProjectSymbolController::updateFile(const QString& projectFolder, const QString& file) {
    if (projectFolder.isEmpty() || file.isEmpty()) return;
    const QString abs = QFileInfo(file).absoluteFilePath();
    const QString root = QFileInfo(projectFolder).absoluteFilePath();
    if (!abs.startsWith(root)) return;                 // 只管專案資料夾內的檔
    m_index.updateFileFromDisk(abs);                   // 不支援/已刪則自動移除
}

void ProjectSymbolController::ensureDialog() {
    if (m_dialog) return;
    m_dialog = new ProjectSymbolDialog(m_dialogParent);
    connect(m_dialog, &ProjectSymbolDialog::symbolChosen, this,
            [this](const QString& file, int line) { emit symbolChosen(file, line); });
}

void ProjectSymbolController::showSearch(const QString& projectFolder) {
    if (projectFolder.isEmpty()) {
        emit statusMessage(tr("請先開啟資料夾（Ctrl+Alt+O）以建立專案符號索引"), 3500);
        return;
    }
    if (m_index.symbolCount() == 0) rebuild(projectFolder);
    ensureDialog();
    m_dialog->openWith(&m_index);
}

// 文字版「找引用」：掃描已索引的原始碼檔，列出符號名整字出現處到 REFERENCES 面板。
void ProjectSymbolController::findReferences(const QString& name) {
    if (name.isEmpty() || !m_refsList) return;
    m_refsList->clear();
    int count = 0;
    for (const QString& file : m_index.files()) {
        QFile f(file);
        if (f.size() > 4 * 1024 * 1024 || !f.open(QIODevice::ReadOnly)) continue;
        const QString content = QString::fromUtf8(f.readAll());
        f.close();
        for (const TextRefs::Hit& h : TextRefs::findWholeWord(content, name)) {
            auto* item = new QListWidgetItem(QStringLiteral("%1:%2:  %3")
                .arg(QFileInfo(file).fileName()).arg(h.line + 1).arg(h.text));
            item->setToolTip(file);
            item->setData(Qt::UserRole, QVariantMap{{"filePath", file}, {"lineNum", h.line + 1}});
            m_refsList->addItem(item);
            if (++count >= 5000) break;
        }
        if (count >= 5000) break;
    }
    if (m_refsDock) m_refsDock->setWindowTitle(tr("REFERENCES — 專案引用「%1」（%2 處）").arg(name).arg(count));
    if (count == 0) {
        emit statusMessage(tr("專案中找不到「%1」的引用").arg(name), 4000);
    } else if (m_refsDock) {
        m_refsDock->show();
        m_refsDock->raise();                             // 帶到前面（底部 dock 可能被 tab 疊住）
    }
}
