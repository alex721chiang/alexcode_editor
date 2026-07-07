#include "MarkdownLinkController.h"
#include "GraphView.h"
#include <QMainWindow>
#include <QDockWidget>
#include <QListWidget>
#include <QFileInfo>
#include <QFile>
#include <QDir>

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
    m_index.build(projectFolder);
    refreshBacklinks(activeFilePath);
    if (m_graphDock->isVisible()) showGraphView(activeFilePath);   // 關係圖同步更新
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
