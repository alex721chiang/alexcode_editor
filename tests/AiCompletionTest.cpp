// AI 輔助測試：設定解析與回應解析是純邏輯；端到端用本機 mock HTTP 伺服器
// 走真實 QNetworkAccessManager 往返（不需要真的 LLM）。ghost text 為 widget 行為。
#include <gtest/gtest.h>
#include <QTcpServer>
#include <QTcpSocket>
#include <QSignalSpy>
#include <QTest>
#include "../src/AICompletionProvider.h"
#include "../src/CodeEditor.h"

TEST(AIConfig, ParsesAllFields) {
    const auto c = AIConfig::fromJson(QByteArray(
        R"({"endpoint":"http://x/v1","apiKey":"k","model":"m","maxTokens":99,)"
        R"("temperature":0.7,"autoTrigger":true,"autoDelayMs":500})"));
    EXPECT_EQ(c.endpoint, QString("http://x/v1"));
    EXPECT_EQ(c.apiKey, QString("k"));
    EXPECT_EQ(c.model, QString("m"));
    EXPECT_EQ(c.maxTokens, 99);
    EXPECT_DOUBLE_EQ(c.temperature, 0.7);
    EXPECT_TRUE(c.autoTrigger);
    EXPECT_EQ(c.autoDelayMs, 500);
}

TEST(AIConfig, MissingFieldsUseDefaults) {
    const auto c = AIConfig::fromJson(QByteArray("{}"));
    EXPECT_FALSE(c.endpoint.isEmpty());
    EXPECT_FALSE(c.autoTrigger);          // 預設不自動觸發
    EXPECT_EQ(c.maxTokens, 256);
}

TEST(AIParse, ExtractsFirstChoiceContent) {
    const QByteArray resp =
        R"({"choices":[{"message":{"role":"assistant","content":"return 42;"}}]})";
    EXPECT_EQ(AICompletionProvider::parseContent(resp), QString("return 42;"));
}

TEST(AIParse, StripsMarkdownFences) {
    const QByteArray resp =
        "{\"choices\":[{\"message\":{\"content\":\"```cpp\\nint x = 1;\\n```\"}}]}";
    EXPECT_EQ(AICompletionProvider::parseContent(resp), QString("int x = 1;"));
}

TEST(AIParse, MalformedYieldsEmpty) {
    EXPECT_TRUE(AICompletionProvider::parseContent("not json").isEmpty());
    EXPECT_TRUE(AICompletionProvider::parseContent(R"({"choices":[]})").isEmpty());
}

// ---- ghost text（widget 行為，無網路）----

TEST(GhostText, TabAcceptsFullSuggestion) {
    std::unique_ptr<CodeEditor> e(new CodeEditor());
    e->setPlainText("int main() {");
    QTextCursor c = e->textCursor();
    c.movePosition(QTextCursor::End);
    e->setTextCursor(c);
    e->setGhostText("\n    return 0;\n}");
    ASSERT_TRUE(e->hasGhostText());
    QTest::keyClick(e.get(), Qt::Key_Tab);
    EXPECT_EQ(e->toPlainText(), QString("int main() {\n    return 0;\n}"));
    EXPECT_FALSE(e->hasGhostText());
}

TEST(GhostText, EscapeRejects) {
    std::unique_ptr<CodeEditor> e(new CodeEditor());
    e->setPlainText("abc");
    QTextCursor c = e->textCursor();
    c.movePosition(QTextCursor::End);
    e->setTextCursor(c);
    e->setGhostText("XYZ");
    QTest::keyClick(e.get(), Qt::Key_Escape);
    EXPECT_FALSE(e->hasGhostText());
    EXPECT_EQ(e->toPlainText(), QString("abc"));
}

TEST(GhostText, TypingClearsAndInsertsNormally) {
    std::unique_ptr<CodeEditor> e(new CodeEditor());
    e->setPlainText("abc");
    QTextCursor c = e->textCursor();
    c.movePosition(QTextCursor::End);
    e->setTextCursor(c);
    e->setGhostText("XYZ");
    QTest::keyClicks(e.get(), "d");
    EXPECT_FALSE(e->hasGhostText());
    EXPECT_EQ(e->toPlainText(), QString("abcd"));
}

TEST(GhostText, CursorMoveClears) {
    std::unique_ptr<CodeEditor> e(new CodeEditor());
    e->setPlainText("abc def");
    QTextCursor c = e->textCursor();
    c.movePosition(QTextCursor::End);
    e->setTextCursor(c);
    e->setGhostText("XYZ");
    c.setPosition(0);
    e->setTextCursor(c);
    EXPECT_FALSE(e->hasGhostText());
}

// ---- 端到端：mock OpenAI 相容伺服器 → provider → suggestionsReady ----

namespace {

// 極簡 HTTP 伺服器：收到任何請求就回一份固定的 chat completion JSON
class MockLlmServer : public QObject {
public:
    QTcpServer server;
    QByteArray lastRequest;

    explicit MockLlmServer(const QByteArray& content) {
        EXPECT_TRUE(server.listen(QHostAddress::LocalHost, 0));
        QObject::connect(&server, &QTcpServer::newConnection, this, [this, content]() {
            QTcpSocket* s = server.nextPendingConnection();
            QObject::connect(s, &QTcpSocket::readyRead, s, [this, s, content]() {
                lastRequest += s->readAll();
                if (!lastRequest.contains("\r\n\r\n")) return;   // 等 header 收完
                const QByteArray body =
                    "{\"choices\":[{\"message\":{\"content\":\"" + content + "\"}}]}";
                s->write("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                         "Content-Length: " + QByteArray::number(body.size()) +
                         "\r\nConnection: close\r\n\r\n" + body);
                s->flush();
                s->disconnectFromHost();
            });
        });
    }
    QString url() const {
        return QStringLiteral("http://127.0.0.1:%1/v1/chat/completions").arg(server.serverPort());
    }
};

} // namespace

TEST(AIEndToEnd, CompletionRoundTripThroughHttp) {
    MockLlmServer mock("counter += 1;");
    AICompletionProvider provider;
    provider.setEndpointForTest(mock.url());

    QSignalSpy spy(&provider, &AICompletionProvider::suggestionsReady);
    provider.requestCompletion("int counter = 0;\n", "cpp");
    ASSERT_TRUE(spy.wait(5000));
    const QStringList suggestions = spy.takeFirst().at(0).toStringList();
    ASSERT_EQ(suggestions.size(), 1);
    EXPECT_EQ(suggestions.first(), QString("counter += 1;"));
    EXPECT_TRUE(mock.lastRequest.contains("chat/completions"));   // 打對端點
    EXPECT_TRUE(mock.lastRequest.contains("max_tokens"));
}

TEST(AIEndToEnd, ErrorSignalOnUnreachableEndpoint) {
    AICompletionProvider provider;
    provider.setEndpointForTest("http://127.0.0.1:1/v1/chat/completions");   // 不可達
    QSignalSpy errSpy(&provider, &AICompletionProvider::errorOccurred);
    provider.requestCompletion("x", "cpp");
    EXPECT_TRUE(errSpy.wait(5000));
}
