#include "LspClient.h"
#include <QJsonDocument>
#include <QJsonArray>
#include <QCoreApplication>

using namespace LspProtocol;

LspClient::LspClient(const QString& command, const QStringList& args,
                     const QString& rootPath, QObject* parent)
    : QObject(parent), m_command(command), m_args(args), m_rootPath(rootPath) {
    connect(&m_proc, &QProcess::readyReadStandardOutput, this, &LspClient::onReadyRead);
    connect(&m_proc, &QProcess::errorOccurred, this, &LspClient::onProcessError);
    connect(&m_proc, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, &LspClient::onProcessFinished);
    m_proc.setProcessChannelMode(QProcess::SeparateChannels);   // stderr 忽略（伺服器 log）
}

void LspClient::start() {
    if (m_proc.state() != QProcess::NotRunning) return;
    m_proc.setWorkingDirectory(m_rootPath);

    // initialize 握手須等行程進入 Running（QProcess::started），否則 write 會被丟棄
    connect(&m_proc, &QProcess::started, this, [this]() {
        const QJsonObject caps{
            {"textDocument", QJsonObject{
                {"synchronization", QJsonObject{{"didSave", true}}},
                {"completion", QJsonObject{{"completionItem", QJsonObject{{"snippetSupport", false}}}}},
                {"hover", QJsonObject{{"contentFormat", QJsonArray{"plaintext", "markdown"}}}},
                {"publishDiagnostics", QJsonObject{}},
                {"references", QJsonObject{}},
                {"rename", QJsonObject{}},
                {"formatting", QJsonObject{}}
            }}
        };
        const QJsonObject params{
            {"processId", static_cast<qint64>(QCoreApplication::applicationPid())},
            {"rootUri", uriFromPath(m_rootPath)},
            {"capabilities", caps},
            {"clientInfo", QJsonObject{{"name", "AlexCode"}, {"version", "4.2"}}}
        };
        sendRequest("initialize", params, QString());
    }, Qt::SingleShotConnection);

    m_proc.start(m_command, m_args);
}

void LspClient::shutdown() {
    if (m_proc.state() == QProcess::NotRunning) return;
    m_shuttingDown = true;
    if (m_ready) {
        sendRequest("shutdown", QJsonObject(), QString());
        send(makeNotification("exit", QJsonObject()));
        m_proc.waitForFinished(800);
    }
    if (m_proc.state() != QProcess::NotRunning) {
        m_proc.kill();
        m_proc.waitForFinished(500);
    }
}

void LspClient::send(const QJsonObject& msg) {
    if (m_proc.state() != QProcess::Running) return;
    m_proc.write(frame(msg));
}

qint64 LspClient::sendRequest(const QString& method, const QJsonObject& params, const QString& contextPath) {
    const qint64 id = m_nextId++;
    m_pending.insert(id, {method, contextPath});
    send(makeRequest(id, method, params));
    return id;
}

void LspClient::openDocument(const QString& path, const QString& languageId, const QString& text) {
    if (!m_ready || m_docVersions.contains(path)) return;
    m_docVersions.insert(path, 1);
    m_docTexts.insert(path, text);
    send(makeNotification("textDocument/didOpen", QJsonObject{
        {"textDocument", QJsonObject{
            {"uri", uriFromPath(path)}, {"languageId", languageId},
            {"version", 1}, {"text", text}}}}));
}

void LspClient::changeDocument(const QString& path, const QString& text) {
    auto it = m_docVersions.find(path);
    if (!m_ready || it == m_docVersions.end()) return;

    QJsonObject change;                                 // full sync 預設：送完整內容
    if (m_caps.syncKind == 2) {                         // incremental：只送變更範圍
        TextEdit e;
        if (!computeIncrementalEdit(m_docTexts.value(path), text, &e))
            return;                                     // 內容未變：不送
        change = QJsonObject{
            {"range", QJsonObject{
                {"start", positionJson(e.startLine, e.startChar)},
                {"end",   positionJson(e.endLine, e.endChar)}}},
            {"text", e.newText}};
    } else {
        change = QJsonObject{{"text", text}};
    }
    m_docTexts.insert(path, text);
    const int version = ++it.value();
    send(makeNotification("textDocument/didChange", QJsonObject{
        {"textDocument", QJsonObject{{"uri", uriFromPath(path)}, {"version", version}}},
        {"contentChanges", QJsonArray{ change }}}));
}

void LspClient::saveDocument(const QString& path) {
    if (!m_ready || !m_docVersions.contains(path)) return;
    send(makeNotification("textDocument/didSave", QJsonObject{
        {"textDocument", QJsonObject{{"uri", uriFromPath(path)}}}}));
}

void LspClient::closeDocument(const QString& path) {
    if (!m_ready || !m_docVersions.remove(path)) return;
    m_docTexts.remove(path);
    send(makeNotification("textDocument/didClose", QJsonObject{
        {"textDocument", QJsonObject{{"uri", uriFromPath(path)}}}}));
}

void LspClient::requestCompletion(const QString& path, int line, int character) {
    if (!m_ready) return;
    sendRequest("textDocument/completion",
                textDocumentPositionParams(path, line, character), path);
}

void LspClient::requestDefinition(const QString& path, int line, int character) {
    if (!m_ready) return;
    sendRequest("textDocument/definition",
                textDocumentPositionParams(path, line, character), path);
}

void LspClient::requestHover(const QString& path, int line, int character) {
    if (!m_ready) return;
    sendRequest("textDocument/hover",
                textDocumentPositionParams(path, line, character), path);
}

void LspClient::requestReferences(const QString& path, int line, int character) {
    if (!m_ready) return;
    QJsonObject params = textDocumentPositionParams(path, line, character);
    params.insert("context", QJsonObject{{"includeDeclaration", true}});
    sendRequest("textDocument/references", params, path);
}

void LspClient::requestRename(const QString& path, int line, int character, const QString& newName) {
    if (!m_ready) return;
    QJsonObject params = textDocumentPositionParams(path, line, character);
    params.insert("newName", newName);
    sendRequest("textDocument/rename", params, path);
}

void LspClient::requestFormatting(const QString& path, int tabSize, bool insertSpaces) {
    if (!m_ready) return;
    sendRequest("textDocument/formatting", QJsonObject{
        {"textDocument", QJsonObject{{"uri", uriFromPath(path)}}},
        {"options", QJsonObject{{"tabSize", tabSize}, {"insertSpaces", insertSpaces}}}}, path);
}

void LspClient::onReadyRead() {
    m_reader.feed(m_proc.readAllStandardOutput());
    const QList<QJsonObject> msgs = m_reader.takeMessages();
    for (const QJsonObject& msg : msgs)
        handleMessage(msg);
}

void LspClient::handleMessage(const QJsonObject& msg) {
    if (msg.contains("id") && !msg.contains("method")) {       // 回應
        handleResponse(msg);
    } else if (msg.contains("id")) {                            // 伺服器→客戶端請求
        handleServerRequest(msg);
    } else {                                                    // 通知
        const QString method = msg.value("method").toString();
        if (method == QLatin1String("textDocument/publishDiagnostics")) {
            const QJsonObject params = msg.value("params").toObject();
            const QString path = pathFromUri(params.value("uri").toString());
            if (!path.isEmpty())
                emit diagnosticsReceived(path, parseDiagnostics(params.value("diagnostics").toArray()));
        }
        // 其他通知（window/logMessage 等）忽略
    }
}

void LspClient::handleResponse(const QJsonObject& msg) {
    const qint64 id = static_cast<qint64>(msg.value("id").toDouble());
    const Pending pending = m_pending.take(id);
    if (pending.method.isEmpty()) return;
    const QJsonValue result = msg.value("result");

    if (pending.method == QLatin1String("initialize")) {
        m_caps = parseServerCapabilities(result.toObject());
        send(makeNotification("initialized", QJsonObject()));
        m_ready = true;
        emit ready();
    } else if (pending.method == QLatin1String("textDocument/completion")) {
        emit completionReady(pending.path, extractCompletions(result));
    } else if (pending.method == QLatin1String("textDocument/definition")) {
        QString defPath; int line = 0, ch = 0;
        if (extractDefinition(result, &defPath, &line, &ch))
            emit definitionReady(defPath, line, ch);
    } else if (pending.method == QLatin1String("textDocument/hover")) {
        const QString text = extractHoverText(result);
        if (!text.isEmpty())
            emit hoverReady(pending.path, text);
    } else if (pending.method == QLatin1String("textDocument/references")) {
        emit referencesReady(pending.path, extractLocations(result));
    } else if (pending.method == QLatin1String("textDocument/rename")) {
        emit renameReady(extractWorkspaceEdit(result));
    } else if (pending.method == QLatin1String("textDocument/formatting")) {
        emit formattingReady(pending.path, extractTextEdits(result));
    }
}

void LspClient::handleServerRequest(const QJsonObject& msg) {
    // 最小回覆，避免伺服器等待逾時（如 client/registerCapability、workspace/configuration）
    const QString method = msg.value("method").toString();
    QJsonValue result = QJsonValue::Null;
    if (method == QLatin1String("workspace/configuration")) {
        const int n = msg.value("params").toObject().value("items").toArray().size();
        QJsonArray arr; for (int i = 0; i < n; ++i) arr.append(QJsonValue::Null);
        result = arr;
    }
    send(makeResponse(msg.value("id"), result));
}

void LspClient::onProcessError(QProcess::ProcessError err) {
    if (err == QProcess::FailedToStart && !m_failed) {
        m_failed = true;
        emit failed(QStringLiteral("無法啟動 %1（請確認已安裝並在 PATH 中）").arg(m_command));
    }
}

void LspClient::onProcessFinished(int exitCode, QProcess::ExitStatus status) {
    const bool wasReady = m_ready;
    m_ready = false;
    m_docVersions.clear();
    m_docTexts.clear();
    m_pending.clear();
    if (!m_shuttingDown && !m_failed &&
        (status == QProcess::CrashExit || (wasReady && exitCode != 0))) {
        m_failed = true;
        emit failed(QStringLiteral("%1 異常結束（exit %2）").arg(m_command).arg(exitCode));
    }
}
