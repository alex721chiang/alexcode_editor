#include "AICompletionProvider.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QNetworkRequest>
#include <QDebug>

AICompletionProvider::AICompletionProvider(QObject *parent)
    : QObject(parent), networkManager(new QNetworkAccessManager(this)) 
{
    // 預設指向本地 LM Studio 的 Port 1234
    apiUrl = "http://127.0.0.1:1234/v1/chat/completions";
}

void AICompletionProvider::requestCompletion(const QString &context, const QString &language) {
    QNetworkRequest request((QUrl(apiUrl)));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");

    // 建立 OpenAI 相容格式的 Payload
    QJsonObject root;
    root["model"] = "google/gemma-4-e4b"; // 使用你剛更新的本地模型
    
    QJsonArray messages;
    QJsonObject systemMsg;
    systemMsg["role"] = "system";
    systemMsg["content"] = QString("You are a helpful programming assistant for %1. "
                                   "Complete the following code block. Provide ONLY the next few lines of code.").arg(language);
    
    QJsonObject userMsg;
    userMsg["role"] = "user";
    userMsg["content"] = context;

    messages.append(systemMsg);
    messages.append(userMsg);
    root["messages"] = messages;
    root["temperature"] = 0.2;
    root["max_tokens"] = 128;

    QNetworkReply *reply = networkManager->post(request, QJsonDocument(root).toJson());
    connect(reply, &QNetworkReply::finished, this, &AICompletionProvider::onReplyFinished);
}

void AICompletionProvider::onReplyFinished() {
    QNetworkReply *reply = qobject_cast<QNetworkReply*>(sender());
    if (!reply) return;

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray responseData = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(responseData);
        QJsonObject root = doc.object();
        
        QStringList suggestions;
        QJsonArray choices = root["choices"].toArray();
        for (const QJsonValue &value : choices) {
            QString content = value.toObject()["message"].toObject()["content"].toString();
            if (!content.isEmpty()) {
                suggestions << content.trimmed();
            }
        }
        
        emit suggestionsReady(suggestions);
    } else {
        emit errorOccurred(reply->errorString());
        qWarning() << "AI Provider Error:" << reply->errorString();
    }

    reply->deleteLater();
}
