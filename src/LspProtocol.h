#pragma once

#include <QByteArray>
#include <QJsonObject>
#include <QJsonArray>
#include <QJsonValue>
#include <QString>
#include <QStringList>
#include <QList>

// LSP JSON-RPC 2.0 純邏輯層：框架編解碼與回應解析。
// 不依賴 QProcess / GUI，方便單元測試（見 tests/LspProtocolTest.cpp）。
namespace LspProtocol {

// ---- 框架（Content-Length header + JSON body）----
QByteArray frame(const QJsonObject& msg);

// 串流讀取器：feed() 餵入任意切割的位元組，takeMessages() 取出完整訊息
class Reader {
public:
    void feed(const QByteArray& data);
    QList<QJsonObject> takeMessages();
private:
    QByteArray m_buf;
    QList<QJsonObject> m_messages;
    void parseBuffer();
};

// ---- 訊息建構 ----
QJsonObject makeRequest(qint64 id, const QString& method, const QJsonObject& params);
QJsonObject makeNotification(const QString& method, const QJsonObject& params);
QJsonObject makeResponse(const QJsonValue& id, const QJsonValue& result);

// ---- URI / 位置 ----
QString uriFromPath(const QString& path);
QString pathFromUri(const QString& uri);
QJsonObject positionJson(int line, int character);              // 0-based
QJsonObject textDocumentPositionParams(const QString& path, int line, int character);

// ---- 診斷 ----
struct Diagnostic {
    int startLine = 0, startChar = 0, endLine = 0, endChar = 0;  // 0-based
    int severity = 1;                                            // 1=Error 2=Warning 3=Info 4=Hint
    QString message;
    QString source;
};
QList<Diagnostic> parseDiagnostics(const QJsonArray& arr);

// ---- 回應解析（皆容忍伺服器間的格式差異）----
// completion: CompletionItem[] 或 CompletionList{items}；回傳插入文字（snippet 佔位符已剝除），上限 maxItems
QStringList extractCompletions(const QJsonValue& result, int maxItems = 50);
// definition: Location | Location[] | LocationLink[]；成功時填入 path/line/ch（0-based）並回傳 true
bool extractDefinition(const QJsonValue& result, QString* path, int* line, int* character);
// hover: contents 為 string | MarkupContent | MarkedString[]；回傳純文字（截斷至 maxLen）
QString extractHoverText(const QJsonValue& result, int maxLen = 1200);
// snippet 佔位符剝除："foo(${1:bar}, $0)" → "foo(bar, )"
QString stripSnippetPlaceholders(const QString& s);

// ---- v2：references / rename / formatting ----
struct Location {
    QString path;
    int line = 0, character = 0;                                 // 0-based
};
// references: Location[]（容忍單一 Location / LocationLink[]）
QList<Location> extractLocations(const QJsonValue& result);

struct TextEdit {
    int startLine = 0, startChar = 0, endLine = 0, endChar = 0;  // 0-based
    QString newText;
};
// formatting: TextEdit[]
QList<TextEdit> extractTextEdits(const QJsonValue& result);
// rename: WorkspaceEdit；changes{uri:edits} 與 documentChanges[TextDocumentEdit] 皆支援
// （documentChanges 中的 CreateFile/RenameFile/DeleteFile 操作忽略）；鍵為本機路徑
QHash<QString, QList<TextEdit>> extractWorkspaceEdit(const QJsonValue& result);
// 將編輯套用至全文（位置以原文計；LSP 規範保證編輯間不重疊）
QString applyTextEdits(const QString& text, QList<TextEdit> edits);

// ---- v2：增量同步 ----
// 計算 old → new 的單一範圍編輯（共同前綴/後綴修剪，UTF-16 代理對安全）；無變更回傳 false
bool computeIncrementalEdit(const QString& oldText, const QString& newText, TextEdit* edit);

// ---- v2：initialize 結果解析 ----
struct ServerCaps {
    int syncKind = 1;                  // 0=None 1=Full 2=Incremental
    QString completionTriggers;        // 補全觸發字元串接（如 ".>:"）
    bool references = false, rename = false, formatting = false;
};
ServerCaps parseServerCapabilities(const QJsonObject& initResult);

} // namespace LspProtocol
