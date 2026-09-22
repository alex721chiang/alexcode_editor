#pragma once

#include <QObject>
#include <QProcess>
#include <QHash>
#include "LspProtocol.h"

// 管理單一 LSP 伺服器行程（stdio）：initialize 握手、文件同步（full sync）、
// completion / definition / hover 請求與 publishDiagnostics 通知。
class LspClient : public QObject {
    Q_OBJECT
public:
    LspClient(const QString& command, const QStringList& args,
              const QString& rootPath, QObject* parent = nullptr);

    void start();
    void shutdown();                       // 送 shutdown/exit 並終止行程
    bool isReady() const { return m_ready; }
    bool hasFailed() const { return m_failed; }
    QString command() const { return m_command; }
    const LspProtocol::ServerCaps& caps() const { return m_caps; }   // initialize 後有效

    // 文件同步（path 為本機絕對路徑；text 為完整內容）
    void openDocument(const QString& path, const QString& languageId, const QString& text);
    void changeDocument(const QString& path, const QString& text);
    void saveDocument(const QString& path);
    void closeDocument(const QString& path);
    bool isDocumentOpen(const QString& path) const { return m_docVersions.contains(path); }

    // 請求（line/character 皆 0-based；結果以 signal 回傳）
    void requestCompletion(const QString& path, int line, int character);
    void requestDefinition(const QString& path, int line, int character);
    void requestHover(const QString& path, int line, int character);
    void requestReferences(const QString& path, int line, int character);
    void requestRename(const QString& path, int line, int character, const QString& newName);
    void requestFormatting(const QString& path, int tabSize, bool insertSpaces);

signals:
    void ready();
    void failed(const QString& reason);    // 啟動失敗或行程異常結束
    void diagnosticsReceived(const QString& path, const QList<LspProtocol::Diagnostic>& diags);
    void completionReady(const QString& path, const QStringList& items);
    void definitionReady(const QString& path, int line, int character);
    void hoverReady(const QString& path, const QString& text);
    void referencesReady(const QString& path, const QList<LspProtocol::Location>& locations);
    void renameReady(const QHash<QString, QList<LspProtocol::TextEdit>>& edits);
    void formattingReady(const QString& path, const QList<LspProtocol::TextEdit>& edits);

private slots:
    void onReadyRead();
    void onProcessError(QProcess::ProcessError err);
    void onProcessFinished(int exitCode, QProcess::ExitStatus status);

private:
    void send(const QJsonObject& msg);
    qint64 sendRequest(const QString& method, const QJsonObject& params, const QString& contextPath);
    void handleMessage(const QJsonObject& msg);
    void handleResponse(const QJsonObject& msg);
    void handleServerRequest(const QJsonObject& msg);

    QString m_command;
    QStringList m_args;
    QString m_rootPath;
    QProcess m_proc;
    LspProtocol::Reader m_reader;
    qint64 m_nextId = 1;
    struct Pending { QString method; QString path; };
    QHash<qint64, Pending> m_pending;
    QHash<QString, int> m_docVersions;
    QHash<QString, QString> m_docTexts;    // 上次同步的內容（增量 didChange 用）
    LspProtocol::ServerCaps m_caps;
    bool m_ready = false;
    bool m_failed = false;
    bool m_shuttingDown = false;
};
