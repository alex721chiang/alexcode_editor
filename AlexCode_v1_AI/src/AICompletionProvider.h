#pragma once

#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QStringList>

/**
 * @brief AICompletionProvider 負責串接本地 LM Studio (Gemma 4) 或雲端 OpenAI 格式的 API
 */
class AICompletionProvider : public QObject {
    Q_OBJECT

public:
    explicit AICompletionProvider(QObject *parent = nullptr);

    // 發送補全請求
    void requestCompletion(const QString &context, const QString &language = "cpp");

signals:
    // 當 AI 吐出建議時發射
    void suggestionsReady(const QStringList &suggestions);
    // 當發生錯誤時發射
    void errorOccurred(const QString &errorMsg);

private slots:
    void onReplyFinished();

private:
    QNetworkAccessManager *networkManager;
    QString apiUrl;
};
