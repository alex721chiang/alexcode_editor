#include "AICompletionProvider.h"
#include "Portable.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QNetworkRequest>
#include <QFile>
#include <QFileInfo>
#include <QDebug>

// ============================================================
// AIConfig
// ============================================================
AIConfig AIConfig::fromJson(const QByteArray& json) {
    AIConfig c;
    const QJsonObject o = QJsonDocument::fromJson(json).object();
    if (o.contains("endpoint"))    c.endpoint    = o.value("endpoint").toString(c.endpoint);
    if (o.contains("apiKey"))      c.apiKey      = o.value("apiKey").toString();
    if (o.contains("model"))       c.model       = o.value("model").toString(c.model);
    if (o.contains("maxTokens"))   c.maxTokens   = o.value("maxTokens").toInt(c.maxTokens);
    if (o.contains("temperature")) c.temperature = o.value("temperature").toDouble(c.temperature);
    if (o.contains("autoTrigger")) c.autoTrigger = o.value("autoTrigger").toBool(c.autoTrigger);
    if (o.contains("autoDelayMs")) c.autoDelayMs = o.value("autoDelayMs").toInt(c.autoDelayMs);
    return c;
}

QString AIConfig::configFilePath() {
    const QString path = Portable::dataDir() + QStringLiteral("/alexcode-ai.json");
    if (!QFileInfo::exists(path)) {
        QFile f(path);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(QByteArray(
"{\n"
"  \"endpoint\": \"http://127.0.0.1:1234/v1/chat/completions\",\n"
"  \"apiKey\": \"\",\n"
"  \"model\": \"google/gemma-4-e4b\",\n"
"  \"maxTokens\": 256,\n"
"  \"temperature\": 0.2,\n"
"  \"autoTrigger\": false,\n"
"  \"autoDelayMs\": 1200\n"
"}\n"));
        }
    }
    return path;
}

// ============================================================
// AICompletionProvider
// ============================================================
AICompletionProvider::AICompletionProvider(QObject *parent)
    : QObject(parent), networkManager(new QNetworkAccessManager(this))
{
    reloadConfig();
}

void AICompletionProvider::reloadConfig() {
    QFile f(AIConfig::configFilePath());
    m_config = f.open(QIODevice::ReadOnly) ? AIConfig::fromJson(f.readAll()) : AIConfig();
}

void AICompletionProvider::cancel() {
    if (m_inflight) {
        m_inflight->disconnect(this);     // 靜默取消：不發 error/結果
        m_inflight->abort();
        m_inflight->deleteLater();
        m_inflight = nullptr;
    }
}

void AICompletionProvider::post(const QJsonArray& messages, bool isChat) {
    cancel();                             // 新請求取代在途請求（過期補全沒有意義）

    QNetworkRequest request{ QUrl(m_config.endpoint) };
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    if (!m_config.apiKey.isEmpty())
        request.setRawHeader("Authorization", "Bearer " + m_config.apiKey.toUtf8());

    QJsonObject root;
    root["model"] = m_config.model;
    root["messages"] = messages;
    root["temperature"] = m_config.temperature;
    root["max_tokens"] = m_config.maxTokens;

    QNetworkReply* reply = networkManager->post(request, QJsonDocument(root).toJson());
    m_inflight = reply;
    connect(reply, &QNetworkReply::finished, this, [this, reply, isChat]() {
        if (m_inflight == reply) m_inflight = nullptr;
        if (reply->error() == QNetworkReply::NoError) {
            const QString content = parseContent(reply->readAll());
            if (isChat) emit chatReady(content);
            else emit suggestionsReady(content.isEmpty() ? QStringList() : QStringList{ content });
        } else {
            emit errorOccurred(reply->errorString());
        }
        reply->deleteLater();
    });
}

void AICompletionProvider::requestCompletion(const QString &context, const QString &language) {
    QJsonArray messages;
    messages.append(QJsonObject{
        {"role", "system"},
        {"content", QStringLiteral(
             "You are a code completion engine for %1. Continue the code exactly from where it "
             "stops. Output ONLY the continuation (no markdown fences, no explanation, no "
             "repetition of the given code). Keep it short: at most a few lines.").arg(language)}});
    messages.append(QJsonObject{{"role", "user"}, {"content", context}});
    post(messages, /*isChat=*/false);
}

void AICompletionProvider::requestChat(const QString& systemPrompt, const QString& userContent) {
    QJsonArray messages;
    messages.append(QJsonObject{{"role", "system"}, {"content", systemPrompt}});
    messages.append(QJsonObject{{"role", "user"}, {"content", userContent}});
    post(messages, /*isChat=*/true);
}

QString AICompletionProvider::parseContent(const QByteArray& response) {
    const QJsonArray choices = QJsonDocument::fromJson(response).object()["choices"].toArray();
    if (choices.isEmpty()) return QString();
    QString content = choices.first().toObject()["message"].toObject()["content"].toString();
    // 模型常無視指示包 markdown 圍欄；剝掉最外層 ```lang … ```
    QString t = content.trimmed();
    if (t.startsWith(QStringLiteral("```"))) {
        const int firstNl = t.indexOf(QChar('\n'));
        const int lastFence = t.lastIndexOf(QStringLiteral("```"));
        if (firstNl >= 0 && lastFence > firstNl)
            t = t.mid(firstNl + 1, lastFence - firstNl - 1).trimmed();
    }
    return t;
}
