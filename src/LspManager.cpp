#include "LspManager.h"
#include "Portable.h"
#include <QStandardPaths>
#include <QJsonDocument>
#include <QJsonArray>
#include <QFile>
#include <QFileInfo>
#include <QDir>

LspManager::LspManager(QObject* parent) : QObject(parent) {
    loadConfig();
}

LspManager::~LspManager() {
    for (LspClient* c : m_clients)
        c->shutdown();
}

QString LspManager::configFilePath() {
    return Portable::dataDir() + QStringLiteral("/alexcode-lsp.json");
}

void LspManager::loadConfig() {
    const QString cfgPath = configFilePath();
    if (!QFileInfo::exists(cfgPath)) {                       // 首次啟動寫入預設
        QFile f(cfgPath);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(QByteArray(
"{\n"
"  \"servers\": [\n"
"    {\"languageId\": \"cpp\",    \"extensions\": [\"c\", \"cc\", \"cpp\", \"cxx\", \"h\", \"hh\", \"hpp\", \"hxx\"],\n"
"     \"command\": \"clangd\", \"args\": [\"--background-index\"]},\n"
"    {\"languageId\": \"python\", \"extensions\": [\"py\", \"pyw\"],\n"
"     \"command\": \"pylsp\", \"args\": []}\n"
"  ]\n"
"}\n"));
        }
    }
    QFile f(cfgPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    const QJsonArray servers = QJsonDocument::fromJson(f.readAll())
                                   .object().value("servers").toArray();
    for (const QJsonValue& v : servers) {
        const QJsonObject o = v.toObject();
        ServerConfig cfg;
        cfg.languageId = o.value("languageId").toString();
        for (const QJsonValue& e : o.value("extensions").toArray())
            cfg.extensions << e.toString().toLower();
        cfg.command = o.value("command").toString();
        for (const QJsonValue& a : o.value("args").toArray())
            cfg.args << a.toString();
        if (!cfg.languageId.isEmpty() && !cfg.command.isEmpty() && !cfg.extensions.isEmpty())
            m_configs.append(cfg);
    }
}

void LspManager::setRootPath(const QString& path) {
    if (path == m_rootPath) return;
    m_rootPath = path;
    for (LspClient* c : m_clients) {                          // 根目錄改變 → 重啟
        c->shutdown();
        c->deleteLater();
    }
    m_clients.clear();
    emit statusChanged();
}

int LspManager::configIndexForFile(const QString& path) const {
    const QString ext = QFileInfo(path).suffix().toLower();
    if (ext.isEmpty()) return -1;
    for (int i = 0; i < m_configs.size(); ++i)
        if (m_configs[i].extensions.contains(ext)) return i;
    return -1;
}

QString LspManager::languageIdForFile(const QString& path) const {
    const int idx = configIndexForFile(path);
    return idx < 0 ? QString() : m_configs[idx].languageId;
}

QString LspManager::serverNameForFile(const QString& path) const {
    const int idx = configIndexForFile(path);
    return idx < 0 ? QString() : m_configs[idx].command;
}

bool LspManager::isReadyForFile(const QString& path) const {
    const int idx = configIndexForFile(path);
    if (idx < 0) return false;
    LspClient* c = m_clients.value(idx, nullptr);
    return c && c->isReady();
}

LspClient* LspManager::clientForFile(const QString& path, bool startIfNeeded) {
    const int idx = configIndexForFile(path);
    if (idx < 0) return nullptr;
    LspClient* c = m_clients.value(idx, nullptr);
    if (!c && startIfNeeded) {
        const ServerConfig& cfg = m_configs[idx];
        const QString root = m_rootPath.isEmpty() ? QFileInfo(path).absolutePath() : m_rootPath;
        c = new LspClient(cfg.command, cfg.args, root, this);
        m_clients.insert(idx, c);
        connect(c, &LspClient::diagnosticsReceived, this, &LspManager::diagnosticsReceived);
        connect(c, &LspClient::completionReady,     this, &LspManager::completionReady);
        connect(c, &LspClient::definitionReady,     this, &LspManager::definitionReady);
        connect(c, &LspClient::hoverReady,          this, &LspManager::hoverReady);
        connect(c, &LspClient::referencesReady,     this, &LspManager::referencesReady);
        connect(c, &LspClient::renameReady,         this, &LspManager::renameReady);
        connect(c, &LspClient::formattingReady,     this, &LspManager::formattingReady);
        connect(c, &LspClient::ready,  this, &LspManager::statusChanged);
        connect(c, &LspClient::failed, this, [this](const QString& reason) {
            emit serverFailed(reason);
            emit statusChanged();
        });
        c->start();
    }
    if (c && c->hasFailed()) return nullptr;                  // 失敗後不重試，避免反覆彈錯
    return c;
}

void LspManager::documentOpened(const QString& path, const QString& text) {
    if (LspClient* c = clientForFile(path, true)) {
        if (c->isReady()) {
            c->openDocument(path, languageIdForFile(path), text);
        } else {
            // 伺服器仍在握手中：ready 後補送 didOpen（一次性連接）
            const QString lang = languageIdForFile(path);
            connect(c, &LspClient::ready, this, [c, path, lang, text]() {
                c->openDocument(path, lang, text);
            }, Qt::SingleShotConnection);
        }
    }
}

void LspManager::documentChanged(const QString& path, const QString& text) {
    if (LspClient* c = clientForFile(path, false))
        c->changeDocument(path, text);
}

void LspManager::documentSaved(const QString& path) {
    if (LspClient* c = clientForFile(path, false))
        c->saveDocument(path);
}

void LspManager::documentClosed(const QString& path) {
    if (LspClient* c = clientForFile(path, false))
        c->closeDocument(path);
}

void LspManager::requestCompletion(const QString& path, int line, int character) {
    if (LspClient* c = clientForFile(path, false))
        c->requestCompletion(path, line, character);
}

void LspManager::requestDefinition(const QString& path, int line, int character) {
    if (LspClient* c = clientForFile(path, false))
        c->requestDefinition(path, line, character);
}

void LspManager::requestHover(const QString& path, int line, int character) {
    if (LspClient* c = clientForFile(path, false))
        c->requestHover(path, line, character);
}

void LspManager::requestReferences(const QString& path, int line, int character) {
    if (LspClient* c = clientForFile(path, false))
        c->requestReferences(path, line, character);
}

void LspManager::requestRename(const QString& path, int line, int character, const QString& newName) {
    if (LspClient* c = clientForFile(path, false))
        c->requestRename(path, line, character, newName);
}

void LspManager::requestFormatting(const QString& path, int tabSize, bool insertSpaces) {
    if (LspClient* c = clientForFile(path, false))
        c->requestFormatting(path, tabSize, insertSpaces);
}

QString LspManager::completionTriggersForFile(const QString& path) {
    LspClient* c = clientForFile(path, false);
    return (c && c->isReady()) ? c->caps().completionTriggers : QString();
}
