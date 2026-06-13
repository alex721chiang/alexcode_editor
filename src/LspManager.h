#pragma once

#include <QObject>
#include <QHash>
#include <QStringList>
#include "LspClient.h"

// 依副檔名將文件路由至對應的 LSP 伺服器；設定來自 alexcode-lsp.json
// （與外部工具選單相同模式：首次啟動寫入預設檔，使用者可自行編輯）。
class LspManager : public QObject {
    Q_OBJECT
public:
    explicit LspManager(QObject* parent = nullptr);
    ~LspManager() override;

    void setRootPath(const QString& path);          // 專案根目錄（重啟所有伺服器）
    static QString configFilePath();
    QString languageIdForFile(const QString& path) const;   // 空字串 = 無對應伺服器

    // 文件生命週期（無對應伺服器時靜默忽略）
    void documentOpened(const QString& path, const QString& text);
    void documentChanged(const QString& path, const QString& text);
    void documentSaved(const QString& path);
    void documentClosed(const QString& path);

    // 請求（0-based 位置）
    void requestCompletion(const QString& path, int line, int character);
    void requestDefinition(const QString& path, int line, int character);
    void requestHover(const QString& path, int line, int character);
    void requestReferences(const QString& path, int line, int character);
    void requestRename(const QString& path, int line, int character, const QString& newName);
    void requestFormatting(const QString& path, int tabSize, bool insertSpaces);

    QString serverNameForFile(const QString& path) const;   // 狀態列顯示用；"" = 無
    bool isReadyForFile(const QString& path) const;
    QString completionTriggersForFile(const QString& path); // 伺服器未就緒時為空字串

signals:
    void diagnosticsReceived(const QString& path, const QList<LspProtocol::Diagnostic>& diags);
    void completionReady(const QString& path, const QStringList& items);
    void definitionReady(const QString& path, int line, int character);
    void hoverReady(const QString& path, const QString& text);
    void referencesReady(const QString& path, const QList<LspProtocol::Location>& locations);
    void renameReady(const QHash<QString, QList<LspProtocol::TextEdit>>& edits);
    void formattingReady(const QString& path, const QList<LspProtocol::TextEdit>& edits);
    void statusChanged();                            // ready / failed 變化（更新狀態列）
    void serverFailed(const QString& reason);        // 顯示一次性訊息

private:
    struct ServerConfig {
        QString languageId;
        QStringList extensions;
        QString command;
        QStringList args;
    };
    void loadConfig();
    int configIndexForFile(const QString& path) const;       // -1 = 無
    LspClient* clientForFile(const QString& path, bool startIfNeeded);

    QList<ServerConfig> m_configs;
    QHash<int, LspClient*> m_clients;                // config index → client
    QString m_rootPath;
};
