#include "LspStatusController.h"
#include "LspManager.h"
#include "CodeEditor.h"
#include <QLabel>

LspStatusController::LspStatusController(LspManager* lsp, QWidget* labelParent, QObject* parent)
    : QObject(parent), m_lsp(lsp) {
    m_label = new QLabel(QString(), labelParent);
    m_label->setToolTip(tr("LSP 狀態（E=錯誤 W=警告）；設定檔：") + LspManager::configFilePath());
}

void LspStatusController::update(CodeEditor* e) {
    const QString path = e ? e->property("filePath").toString() : QString();
    const QString server = path.isEmpty() ? QString() : m_lsp->serverNameForFile(path);
    if (server.isEmpty() || !e->lspEnabled()) {
        m_label->setText(QString());
        return;
    }
    if (!m_lsp->isReadyForFile(path)) {
        m_label->setText(QStringLiteral("LSP: %1 …").arg(server));
        return;
    }
    int errors = 0, warnings = 0;
    e->diagnosticCounts(&errors, &warnings);
    m_label->setText(QStringLiteral("LSP: %1 ✓ E%2 W%3").arg(server).arg(errors).arg(warnings));
    e->setLspCompletionTriggers(m_lsp->completionTriggersForFile(path));   // v2：自動補全觸發字元
}
