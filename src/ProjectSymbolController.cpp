#include "ProjectSymbolController.h"
#include "ProjectSymbolDialog.h"
#include "TextRefs.h"
#include <QtConcurrent>
#include <QListWidget>
#include <QDockWidget>
#include <QWidget>
#include <QFile>
#include <QFileInfo>
#include <QCoreApplication>

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
        });
    }
    emit statusMessage(tr("正在背景索引專案符號…"), 0);
    const QString folder = projectFolder;
    m_watcher->setFuture(QtConcurrent::run([folder]() {       // 背景掃描+解析，不卡 UI
        ProjectSymbolIndex idx;
        idx.build(folder);
        return idx;
    }));
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
