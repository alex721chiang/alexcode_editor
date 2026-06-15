#pragma once

#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QStringList>

/**
 * @brief AICompletionProvider — 雲端/本機 LLM 補全（OpenAI 相容 API，如 LM Studio）。
 *
 * 【選項 B 未來骨架，預設未啟用】目前日常補全由 LSP（啟用時）與離線 LocalCompletion
 * （Ctrl+Space）負責；此類別保留作為未來「雲端 AI 補全/對話式輔助」的接點，預設不會自動
 * 發送請求（CodeEditor 不再於輸入時呼叫 requestCompletion）。要啟用雲端 AI 時再接回觸發點。
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
