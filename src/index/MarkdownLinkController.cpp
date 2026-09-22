#include "MarkdownLinkController.h"
#include "IoPool.h"
#include "GraphView.h"
#include <QMainWindow>
#include <QDockWidget>
#include <QListWidget>
#include <QFileInfo>
#include <QFile>
#include <QDir>
#include <QtConcurrent>
#include <utility>

MarkdownLinkController::MarkdownLinkController(QMainWindow* host, QObject* parent)
    : QObject(parent) {
    // ---------- Backlinks（Obsidian 風：誰連到目前這篇）----------
    m_backlinksDock = new QDockWidget(tr("BACKLINKS — 反向連結"), host);
    m_backlinksList = new QListWidget(host);
    m_backlinksDock->setWidget(m_backlinksList);
    host->addDockWidget(Qt::RightDockWidgetArea, m_backlinksDock);
    m_backlinksDock->hide();
    connect(m_backlinksList, &QListWidget::itemActivated, this, [this](QListWidgetItem* it) {
        if (it) emit fileOpenRequested(it->data(Qt::UserRole).toString());
    });

    // ---------- 關係圖（Obsidian 風：節點=檔，邊=連結）----------
    m_graphDock = new QDockWidget(tr("GRAPH — 關係圖"), host);
    m_graphView = new GraphView(host);
    m_graphDock->setWidget(m_graphView);
    host->addDockWidget(Qt::RightDockWidgetArea, m_graphDock);
    m_graphDock->hide();
    connect(m_graphView, &GraphView::nodeClicked, this,
            [this](const QString& f) { emit fileOpenRequested(f); });
}

void MarkdownLinkController::rebuildIndex(const QString& projectFolder, const QString& activeFilePath) {
    if (projectFolder.isEmpty()) return;
    m_folder = projectFolder;
    m_activeFile = activeFilePath;
    m_stale = true;
    if (!m_backlinksDock->isVisible() && !m_graphDock->isVisible()) return;   // 延後：沒人看就不建
    startBuild();
}

void MarkdownLinkController::startBuild() {
    if (m_folder.isEmpty()) return;
    if (!m_buildWatcher) {
        m_buildWatcher = new QFutureWatcher<MarkdownLinkIndex>(this);
        connect(m_buildWatcher, &QFutureWatcher<MarkdownLinkIndex>::finished, this, [this]() {
            m_index = m_buildWatcher->result();          // 在主執行緒換上新索引
            if (m_rerun) { m_rerun = false; startBuild(); return; }   // 期間又有變更：以最新狀態重建
            refreshBacklinks(m_activeFile);
            if (m_graphDock->isVisible()) showGraphView(m_activeFile);   // 關係圖同步更新
            const auto pending = std::exchange(m_afterBuild, {});
            for (const auto& fn : pending) fn();
            emit indexRebuilt();
        });
    }
    if (m_buildWatcher->isRunning()) { m_rerun = true; return; }
    m_stale = false;
    const QString folder = m_folder;
    m_buildWatcher->setFuture(QtConcurrent::run(IoPool::instance(), [folder]() {
        MarkdownLinkIndex idx;
        idx.build(folder);
        return idx;
    }));
}

void MarkdownLinkController::refreshBacklinks(const QString& activeFilePath) {
    m_backlinksList->clear();
    const bool isMd = activeFilePath.endsWith(QLatin1String(".md"), Qt::CaseInsensitive)
                   || activeFilePath.endsWith(QLatin1String(".markdown"), Qt::CaseInsensitive);
    if (!isMd) {
        auto* it = new QListWidgetItem(tr("（非 Markdown 檔）"));
        it->setFlags(Qt::NoItemFlags);
        m_backlinksList->addItem(it);
        return;
    }
    const QStringList back = m_index.backlinks(QFileInfo(activeFilePath).absoluteFilePath());
    if (back.isEmpty()) {
        auto* it = new QListWidgetItem(tr("（沒有其他檔連到這篇）"));
        it->setFlags(Qt::NoItemFlags);
        m_backlinksList->addItem(it);
        return;
    }
    for (const QString& src : back) {
        auto* it = new QListWidgetItem(QFileInfo(src).fileName());
        it->setData(Qt::UserRole, src);
        it->setToolTip(src);
        m_backlinksList->addItem(it);
    }
}

void MarkdownLinkController::showGraphView(const QString& activeFilePath) {
    const QString active = activeFilePath.isEmpty() ? QString()
                                                     : QFileInfo(activeFilePath).absoluteFilePath();
    m_graphView->setGraph(m_index.files(), m_index.edges(), active);
}

void MarkdownLinkController::openOrCreateWikilink(const QString& target, const QString& projectFolder) {
    // 索引過期（延後建立 / 建置中 / 換了資料夾）時先等索引就緒，
    // 否則既有筆記會被誤判為不存在而另建新檔。
    if (!projectFolder.isEmpty() && (m_stale || isIndexing() || m_folder != projectFolder)) {
        m_folder = projectFolder;
        m_afterBuild.append([this, target, projectFolder]() { openOrCreateWikilinkNow(target, projectFolder); });
        startBuild();
        return;
    }
    openOrCreateWikilinkNow(target, projectFolder);
}

void MarkdownLinkController::openOrCreateWikilinkNow(const QString& target, const QString& projectFolder) {
    QString resolved = m_index.resolve(target);
    if (resolved.isEmpty()) {
        // vault 內找不到 → 在 projectFolder 建立新筆記（Obsidian 行為）
        if (projectFolder.isEmpty()) {
            emit statusMessage(tr("找不到 [[%1]]，且尚未開啟資料夾").arg(target), 4000);
            return;
        }
        QString name = target;
        if (!name.endsWith(QLatin1String(".md"), Qt::CaseInsensitive)) name += QStringLiteral(".md");
        resolved = QDir(projectFolder).absoluteFilePath(name);
        QFile f(resolved);
        if (f.open(QIODevice::WriteOnly | QIODevice::Text)) {
            f.write(QStringLiteral("# %1\n\n").arg(target).toUtf8());
            f.close();
        }
        rebuildIndex(projectFolder, resolved);
    }
    emit fileOpenRequested(resolved);
}
