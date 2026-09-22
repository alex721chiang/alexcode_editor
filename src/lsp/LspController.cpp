#include "LspController.h"
#include "LspManager.h"
#include "LspSyncController.h"
#include "CodeEditor.h"
#include <QMainWindow>
#include <QDockWidget>
#include <QListWidget>
#include <QFile>
#include <QFileInfo>

LspController::LspController(std::function<CodeEditor*(const QString&, bool)> resolveEditor,
                             QMainWindow* host, QObject* parent)
    : QObject(parent), m_resolveEditor(std::move(resolveEditor)) {
    m_lsp = new LspManager(this);

    // ---- 全部引用結果面板（雙擊跳轉，模式同 Find in Files）----
    m_refsDock = new QDockWidget(tr("REFERENCES — 全部引用"), host);
    m_refsList = new QListWidget(host);
    m_refsDock->setWidget(m_refsList);
    host->addDockWidget(Qt::BottomDockWidgetArea, m_refsDock);
    m_refsDock->hide();
    connect(m_refsList, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem* item) {
        const QVariantMap data = item->data(Qt::UserRole).toMap();
        if (data.isEmpty()) return;
        emit referenceActivated(data.value("filePath").toString(), data.value("lineNum").toInt());
    });

    connect(m_lsp, &LspManager::referencesReady, this,
            [this](const QString&, const QList<LspProtocol::Location>& locations) {
        m_refsList->clear();
        QHash<QString, QStringList> fileLines;               // 每檔讀一次，顯示該行內容
        for (const LspProtocol::Location& loc : locations) {
            QString lineText;
            if (CodeEditor* e = m_resolveEditor(loc.path, false)) {
                lineText = e->document()->findBlockByNumber(loc.line).text();
            } else {
                if (!fileLines.contains(loc.path)) {
                    QFile f(loc.path);
                    if (f.size() < 4 * 1024 * 1024 && f.open(QIODevice::ReadOnly))
                        fileLines.insert(loc.path, QString::fromUtf8(f.readAll()).split('\n'));
                    else
                        fileLines.insert(loc.path, QStringList());
                }
                lineText = fileLines[loc.path].value(loc.line);
            }
            auto* item = new QListWidgetItem(QStringLiteral("%1:%2:  %3")
                .arg(QFileInfo(loc.path).fileName()).arg(loc.line + 1).arg(lineText.trimmed()));
            item->setToolTip(loc.path);
            item->setData(Qt::UserRole, QVariantMap{
                {"filePath", loc.path}, {"lineNum", loc.line + 1}});
            m_refsList->addItem(item);
        }
        m_refsDock->setWindowTitle(tr("REFERENCES — 全部引用（%1 處）").arg(locations.size()));
        if (locations.isEmpty())
            emit statusMessage(tr("LSP：找不到引用"), 4000);
        else
            m_refsDock->show();
    });

    connect(m_lsp, &LspManager::renameReady, this,
            [this](const QHash<QString, QList<LspProtocol::TextEdit>>& edits) {
        if (edits.isEmpty()) {
            emit statusMessage(tr("LSP：無法重新命名（伺服器未回傳編輯）"), 4000);
            return;
        }
        int editCount = 0;
        for (auto it = edits.begin(); it != edits.end(); ++it) {
            CodeEditor* e = m_resolveEditor(it.key(), true);   // 未開啟則開進分頁再套用（保留 Undo）
            if (!e) continue;
            LspSyncController::applyTextEdits(e, it.value());
            editCount += it.value().size();
        }
        emit statusMessage(tr("重新命名完成：%1 個檔案、%2 處變更")
                                .arg(edits.size()).arg(editCount), 5000);
    });

    connect(m_lsp, &LspManager::formattingReady, this,
            [this](const QString& path, const QList<LspProtocol::TextEdit>& edits) {
        if (edits.isEmpty()) {
            emit statusMessage(tr("LSP：文件已符合格式（無變更）"), 4000);
            return;
        }
        if (CodeEditor* e = m_resolveEditor(path, false)) {
            LspSyncController::applyTextEdits(e, edits);
            emit statusMessage(tr("格式化完成：%1 處變更").arg(edits.size()), 4000);
        }
    });

    connect(m_lsp, &LspManager::serverFailed, this, [this](const QString& reason) {
        emit statusMessage(tr("LSP：") + reason, 6000);
    });
}
