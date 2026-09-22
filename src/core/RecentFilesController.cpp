#include "RecentFilesController.h"
#include "Portable.h"
#include <QMenu>
#include <QAction>
#include <QFileInfo>

RecentFilesController::RecentFilesController(QMenu* fileMenu, QObject* parent) : QObject(parent) {
    m_menu = fileMenu->addMenu(tr("Open Recent"));
    for (int i = 0; i < kMaxRecentFiles; ++i) {
        m_actions[i] = new QAction(this);
        m_actions[i]->setVisible(false);
        connect(m_actions[i], &QAction::triggered, this, [this, i]() {
            emit fileOpenRequested(m_actions[i]->data().toString());
        });
        m_menu->addAction(m_actions[i]);
    }
    load();
    updateActions();
}

void RecentFilesController::addFile(const QString& filePath) {
    m_files.removeAll(filePath);
    m_files.prepend(filePath);
    while (m_files.size() > kMaxRecentFiles) m_files.removeLast();
    save();
    updateActions();
}

void RecentFilesController::updateActions() {
    const int numRecentFiles = qMin(m_files.size(), kMaxRecentFiles);
    for (int i = 0; i < numRecentFiles; ++i) {
        const QString text = tr("&%1 %2").arg(i + 1).arg(QFileInfo(m_files[i]).fileName());
        m_actions[i]->setText(text);
        m_actions[i]->setData(m_files[i]);
        m_actions[i]->setVisible(true);
    }
    for (int j = numRecentFiles; j < kMaxRecentFiles; ++j)
        m_actions[j]->setVisible(false);
}

void RecentFilesController::save() const {
    AppSettings settings;
    settings.setValue("recentFileList", m_files);
}

void RecentFilesController::load() {
    AppSettings settings;
    m_files = settings.value("recentFileList").toStringList();
}
