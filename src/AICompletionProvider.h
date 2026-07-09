#pragma once

#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPointer>
#include <QStringList>

// AI 設定（dataDir()/alexcode-ai.json；首次啟動寫入範本，與 LSP/Snippet 同模式）。
// 端點為 OpenAI 相容 /v1/chat/completions：本機 LM Studio/Ollama 或雲端皆可。
struct AIConfig {
    QString endpoint = QStringLiteral("http://127.0.0.1:1234/v1/chat/completions");
    QString apiKey;                       // 空 = 不帶 Authorization（本機服務）
    QString model = QStringLiteral("google/gemma-4-e4b");
    int maxTokens = 256;
    double temperature = 0.2;
    bool autoTrigger = false;             // 編輯停頓自動觸發 ghost 補全（預設關，避免意外請求）
    int autoDelayMs = 1200;

    static AIConfig fromJson(const QByteArray& json);   // 純解析，可測；缺欄位用預設值
    static QString configFilePath();                    // 不存在時寫入範本
};

/**
 * @brief AICompletionProvider — 雲端/本機 LLM 輔助（OpenAI 相容 API）。
 *
 * 兩種用途：
 * - requestCompletion()：程式碼接續補全（ghost text 用；回 suggestionsReady）
 * - requestChat()：一般指令（解釋/重構選取；回 chatReady）
 * 新請求會取消在途請求（補全跟著游標跑，過期結果沒有意義）。
 */
class AICompletionProvider : public QObject {
    Q_OBJECT

public:
    explicit AICompletionProvider(QObject *parent = nullptr);

    void reloadConfig();                                  // 設定檔存檔後重載
    const AIConfig& config() const { return m_config; }
    void setEndpointForTest(const QString& url) { m_config.endpoint = url; }

    void requestCompletion(const QString &context, const QString &language = "cpp");
    void requestChat(const QString& systemPrompt, const QString& userContent);
    void cancel();                                        // 取消在途請求（不發訊號）

    // 解析 OpenAI 相容回應的第一個 choice 內容（純邏輯，可測）；解析失敗回空字串
    static QString parseContent(const QByteArray& response);

signals:
    void suggestionsReady(const QStringList &suggestions);   // requestCompletion 結果
    void chatReady(const QString& content);                  // requestChat 結果
    void errorOccurred(const QString &errorMsg);

private:
    void post(const class QJsonArray& messages, bool isChat);

    QNetworkAccessManager *networkManager;
    QPointer<QNetworkReply> m_inflight;
    AIConfig m_config;
};
