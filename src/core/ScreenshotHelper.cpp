#include "ScreenshotHelper.h"
#include "MainWindow.h"
#include "UiHelpers.h"
#include "CommandPalette.h"
#include "TerminalWidget.h"
#include "MarkdownLinkController.h"
#include "ProjectSymbolController.h"
#include "SettingsDialog.h"
#include "LspManager.h"
#include "SnippetsController.h"
#include "ProjectSymbolDialog.h"   // dialog()->grab() 需完整型別
#include <QDockWidget>             // termDock/mdDock/backlinksDock/graphDock->show()
#include <QMenuBar>                // menuBar()->actions()
#include <QMessageBox>
#include <QMenu>
#include <QTimer>

// 原 MainWindow::*ForShot 實作搬移至此；經 friend class 存取 MainWindow 私有成員。
// tr() 凡涉及既有翻譯一律保留原 context（MainWindow::tr / QObject::tr），避免 .ts 失效。

ScreenshotHelper::ScreenshotHelper(MainWindow* window, QObject* parent)
    : QObject(parent), m_window(window) {}

void ScreenshotHelper::openTerminal(const QString& cmd) {
    m_window->termDock->show();
    m_window->terminal->startShell(m_window->projectFolder);
    if (!cmd.isEmpty()) m_window->terminal->sendText((cmd + QStringLiteral("\r")).toUtf8());
}

void ScreenshotHelper::openVault(const QString& folder) {
    m_window->setProjectFolder(folder);
    m_window->mdLinkController->backlinksDock()->show();
    m_window->mdLinkController->refreshBacklinks(m_window->currentFilePath());
}

void ScreenshotHelper::showMarkdownPreview() {
    if (m_window->mdDock) {
        m_window->mdDock->show();
        m_window->mdDock->resize(560, 700);
        m_window->refreshMarkdownPreview();
    }
}

void ScreenshotHelper::openGraph(const QString& folder) {
    m_window->setProjectFolder(folder);
    m_window->mdLinkController->graphDock()->show();
    m_window->mdLinkController->graphDock()->resize(560, 520);
    m_window->mdLinkController->showGraphView(m_window->currentFilePath());
}

void ScreenshotHelper::gotoLine(int line) {
    if (CodeEditor* e = m_window->activeEditor()) { e->gotoLine(line); m_window->updateBreadcrumb(); }
}

void ScreenshotHelper::openMenu(int index, const QString& outPng) {
    const QList<QAction*> acts = m_window->menuBar()->actions();
    if (index < 0 || index >= acts.size() || !acts[index]->menu()) return;
    QMenu* m = acts[index]->menu();
    m->popup(m_window->mapToGlobal(QPoint(40 + index * 70, 30)));
    QTimer::singleShot(700, m_window, [m, outPng]() { if (m) m->grab().save(outPng); });
}

void ScreenshotHelper::openCallGraph(const QString& outPng) {
    CodeEditor* e = m_window->activeEditor();
    if (!e) return;
    QWidget* dlg = e->showCallGraphAt(e->textCursor().blockNumber());
    QTimer::singleShot(700, m_window, [dlg, outPng]() { if (dlg) dlg->grab().save(outPng); });
}

void ScreenshotHelper::findRefs(const QString& name) {
    m_window->symbolController->buildSync(m_window->projectFolder);   // 截圖：同步建索引（避開背景非同步）
    m_window->symbolController->findReferences(name);
}

void ScreenshotHelper::openCommandPalette(const QString& outPng) {
    if (!m_window->commandPalette) m_window->commandPalette = new CommandPalette(m_window);
    m_window->commandPalette->openWith(UiHelpers::gatherCommands(m_window), QStringLiteral("go"));
    QTimer::singleShot(700, m_window, [this, outPng]() {
        if (m_window->commandPalette) m_window->commandPalette->grab().save(outPng);
    });
}

void ScreenshotHelper::openProjectSymbol(const QString& outPng) {
    m_window->symbolController->rebuild(m_window->projectFolder);
    m_window->symbolController->showSearch(m_window->projectFolder);
    QTimer::singleShot(700, m_window, [this, outPng]() {
        if (m_window->symbolController->dialog()) m_window->symbolController->dialog()->grab().save(outPng);
    });
}

void ScreenshotHelper::openSettings(const QString& outPng) {
    auto* dlg = new SettingsDialog(LspManager::configFilePath(), SnippetsController::configPath(),
                                   MainWindow::keymapConfigPath(),
                                   UiHelpers::gatherShortcutActions(m_window), m_window);
    dlg->setAttribute(Qt::WA_DeleteOnClose);
    dlg->show();
    QTimer::singleShot(900, dlg, [dlg, outPng]() { dlg->grab().save(outPng); });
}

void ScreenshotHelper::openAbout(const QString& outPng) {
    auto* box = new QMessageBox(QMessageBox::Information, MainWindow::tr("關於 AlexCode"),
                                UiHelpers::aboutHtml(), QMessageBox::Ok, m_window);
    box->setTextFormat(Qt::RichText);
    box->setAttribute(Qt::WA_DeleteOnClose);
    box->show();
    QTimer::singleShot(700, box, [box, outPng]() { box->grab().save(outPng); });
}

void ScreenshotHelper::openGitDiff(const QString& outPng) {
    m_window->resize(1200, 800);
    m_window->compareActiveWithGitHead();
    QTimer::singleShot(700, m_window, [this, outPng]() { m_window->grab().save(outPng); });
}
