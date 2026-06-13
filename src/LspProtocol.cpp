#include "LspProtocol.h"
#include <QJsonDocument>
#include <QUrl>
#include <QRegularExpression>
#include <algorithm>

namespace LspProtocol {

QByteArray frame(const QJsonObject& msg) {
    const QByteArray body = QJsonDocument(msg).toJson(QJsonDocument::Compact);
    return "Content-Length: " + QByteArray::number(body.size()) + "\r\n\r\n" + body;
}

void Reader::feed(const QByteArray& data) {
    m_buf.append(data);
    parseBuffer();
}

void Reader::parseBuffer() {
    while (true) {
        const int headerEnd = m_buf.indexOf("\r\n\r\n");
        if (headerEnd < 0) return;

        // 解析 headers，取 Content-Length（大小寫不敏感，忽略其他 header）
        int contentLength = -1;
        const QList<QByteArray> lines = m_buf.left(headerEnd).split('\n');
        for (QByteArray line : lines) {
            line = line.trimmed();
            const int colon = line.indexOf(':');
            if (colon < 0) continue;
            if (line.left(colon).trimmed().toLower() == "content-length")
                contentLength = line.mid(colon + 1).trimmed().toInt();
        }
        if (contentLength < 0) {            // 無效框架：丟棄此 header 避免卡死
            m_buf.remove(0, headerEnd + 4);
            continue;
        }

        const int bodyStart = headerEnd + 4;
        if (m_buf.size() < bodyStart + contentLength) return;   // body 未到齊

        const QByteArray body = m_buf.mid(bodyStart, contentLength);
        m_buf.remove(0, bodyStart + contentLength);

        QJsonParseError err;
        const QJsonDocument doc = QJsonDocument::fromJson(body, &err);
        if (err.error == QJsonParseError::NoError && doc.isObject())
            m_messages.append(doc.object());
    }
}

QList<QJsonObject> Reader::takeMessages() {
    QList<QJsonObject> out;
    out.swap(m_messages);
    return out;
}

QJsonObject makeRequest(qint64 id, const QString& method, const QJsonObject& params) {
    return { {"jsonrpc", "2.0"}, {"id", id}, {"method", method}, {"params", params} };
}

QJsonObject makeNotification(const QString& method, const QJsonObject& params) {
    return { {"jsonrpc", "2.0"}, {"method", method}, {"params", params} };
}

QJsonObject makeResponse(const QJsonValue& id, const QJsonValue& result) {
    return { {"jsonrpc", "2.0"}, {"id", id}, {"result", result} };
}

QString uriFromPath(const QString& path) {
    return QUrl::fromLocalFile(path).toString();
}

QString pathFromUri(const QString& uri) {
    const QUrl url(uri);
    return url.isLocalFile() ? url.toLocalFile() : QString();
}

QJsonObject positionJson(int line, int character) {
    return { {"line", line}, {"character", character} };
}

QJsonObject textDocumentPositionParams(const QString& path, int line, int character) {
    return { {"textDocument", QJsonObject{{"uri", uriFromPath(path)}}},
             {"position", positionJson(line, character)} };
}

QList<Diagnostic> parseDiagnostics(const QJsonArray& arr) {
    QList<Diagnostic> out;
    for (const QJsonValue& v : arr) {
        const QJsonObject d = v.toObject();
        const QJsonObject range = d.value("range").toObject();
        const QJsonObject s = range.value("start").toObject();
        const QJsonObject e = range.value("end").toObject();
        Diagnostic diag;
        diag.startLine = s.value("line").toInt();
        diag.startChar = s.value("character").toInt();
        diag.endLine   = e.value("line").toInt();
        diag.endChar   = e.value("character").toInt();
        diag.severity  = d.value("severity").toInt(1);
        diag.message   = d.value("message").toString();
        diag.source    = d.value("source").toString();
        out.append(diag);
    }
    return out;
}

QString stripSnippetPlaceholders(const QString& s) {
    QString out = s;
    // ${1:default} → default；${1} / $1 / $0 → 空
    out.replace(QRegularExpression(R"(\$\{\d+:([^}]*)\})"), "\\1");
    out.replace(QRegularExpression(R"(\$\{\d+\})"), "");
    out.replace(QRegularExpression(R"(\$\d+)"), "");
    return out;
}

QStringList extractCompletions(const QJsonValue& result, int maxItems) {
    QJsonArray items;
    if (result.isArray())
        items = result.toArray();
    else if (result.isObject())
        items = result.toObject().value("items").toArray();

    QStringList out;
    for (const QJsonValue& v : items) {
        if (out.size() >= maxItems) break;
        const QJsonObject item = v.toObject();
        QString text = item.value("insertText").toString();
        if (text.isEmpty()) text = item.value("label").toString().trimmed();
        if (item.value("insertTextFormat").toInt(1) == 2)   // 2 = Snippet
            text = stripSnippetPlaceholders(text);
        if (!text.isEmpty() && !out.contains(text)) out.append(text);
    }
    return out;
}

bool extractDefinition(const QJsonValue& result, QString* path, int* line, int* character) {
    QJsonObject loc;
    if (result.isObject())
        loc = result.toObject();
    else if (result.isArray() && !result.toArray().isEmpty())
        loc = result.toArray().first().toObject();
    if (loc.isEmpty()) return false;

    // Location{uri,range} 或 LocationLink{targetUri,targetSelectionRange}
    QString uri = loc.value("uri").toString();
    QJsonObject range = loc.value("range").toObject();
    if (uri.isEmpty()) {
        uri = loc.value("targetUri").toString();
        range = loc.value("targetSelectionRange").toObject();
        if (range.isEmpty()) range = loc.value("targetRange").toObject();
    }
    if (uri.isEmpty()) return false;

    const QString p = pathFromUri(uri);
    if (p.isEmpty()) return false;
    const QJsonObject start = range.value("start").toObject();
    *path = p;
    *line = start.value("line").toInt();
    *character = start.value("character").toInt();
    return true;
}

static Location locationFromJson(const QJsonObject& loc) {
    // Location{uri,range} 或 LocationLink{targetUri,targetSelectionRange}
    QString uri = loc.value("uri").toString();
    QJsonObject range = loc.value("range").toObject();
    if (uri.isEmpty()) {
        uri = loc.value("targetUri").toString();
        range = loc.value("targetSelectionRange").toObject();
        if (range.isEmpty()) range = loc.value("targetRange").toObject();
    }
    Location out;
    out.path = pathFromUri(uri);
    const QJsonObject start = range.value("start").toObject();
    out.line = start.value("line").toInt();
    out.character = start.value("character").toInt();
    return out;
}

QList<Location> extractLocations(const QJsonValue& result) {
    QJsonArray arr;
    if (result.isArray())
        arr = result.toArray();
    else if (result.isObject())
        arr.append(result);
    QList<Location> out;
    for (const QJsonValue& v : arr) {
        const Location loc = locationFromJson(v.toObject());
        if (!loc.path.isEmpty()) out.append(loc);
    }
    return out;
}

static TextEdit textEditFromJson(const QJsonObject& o) {
    const QJsonObject range = o.value("range").toObject();
    const QJsonObject s = range.value("start").toObject();
    const QJsonObject e = range.value("end").toObject();
    TextEdit edit;
    edit.startLine = s.value("line").toInt();
    edit.startChar = s.value("character").toInt();
    edit.endLine   = e.value("line").toInt();
    edit.endChar   = e.value("character").toInt();
    edit.newText   = o.value("newText").toString();
    return edit;
}

QList<TextEdit> extractTextEdits(const QJsonValue& result) {
    QList<TextEdit> out;
    for (const QJsonValue& v : result.toArray())
        out.append(textEditFromJson(v.toObject()));
    return out;
}

QHash<QString, QList<TextEdit>> extractWorkspaceEdit(const QJsonValue& result) {
    QHash<QString, QList<TextEdit>> out;
    const QJsonObject we = result.toObject();

    const QJsonObject changes = we.value("changes").toObject();
    for (auto it = changes.begin(); it != changes.end(); ++it) {
        const QString path = pathFromUri(it.key());
        if (!path.isEmpty()) out[path].append(extractTextEdits(it.value()));
    }
    for (const QJsonValue& v : we.value("documentChanges").toArray()) {
        const QJsonObject dc = v.toObject();
        if (dc.contains("kind")) continue;          // CreateFile / RenameFile / DeleteFile：忽略
        const QString path = pathFromUri(dc.value("textDocument").toObject().value("uri").toString());
        if (!path.isEmpty()) out[path].append(extractTextEdits(dc.value("edits")));
    }
    return out;
}

// line/char（0-based，UTF-16 碼元）→ 全文偏移；超界 clamp 至合法範圍
static int offsetOfPosition(const QString& text, int line, int character) {
    int off = 0;
    for (int i = 0; i < line; ++i) {
        const int nl = text.indexOf(QLatin1Char('\n'), off);
        if (nl < 0) return text.size();
        off = nl + 1;
    }
    int lineEnd = text.indexOf(QLatin1Char('\n'), off);
    if (lineEnd < 0) lineEnd = text.size();
    return qMin(off + qMax(character, 0), lineEnd);
}

QString applyTextEdits(const QString& text, QList<TextEdit> edits) {
    // 先以原文計算偏移，再由後往前套用（避免前面的編輯位移後面的位置）
    struct Span { int start, end; QString newText; };
    QList<Span> spans;
    spans.reserve(edits.size());
    for (const TextEdit& e : edits)
        spans.append({ offsetOfPosition(text, e.startLine, e.startChar),
                       offsetOfPosition(text, e.endLine, e.endChar), e.newText });
    std::sort(spans.begin(), spans.end(),
              [](const Span& a, const Span& b) { return a.start > b.start; });
    QString out = text;
    for (const Span& s : spans)
        out.replace(s.start, qMax(s.end - s.start, 0), s.newText);
    return out;
}

// 全文偏移 → line/char（0-based）
static void positionOfOffset(const QString& text, int offset, int* line, int* character) {
    int ln = 0, lineStart = 0;
    for (int i = 0; i < offset; ++i) {
        if (text.at(i) == QLatin1Char('\n')) { ++ln; lineStart = i + 1; }
    }
    *line = ln;
    *character = offset - lineStart;
}

bool computeIncrementalEdit(const QString& oldText, const QString& newText, TextEdit* edit) {
    if (oldText == newText) return false;
    const int oldLen = oldText.size(), newLen = newText.size();

    int prefix = 0;
    const int maxPrefix = qMin(oldLen, newLen);
    while (prefix < maxPrefix && oldText.at(prefix) == newText.at(prefix)) ++prefix;
    if (prefix > 0 && oldText.at(prefix - 1).isHighSurrogate()) --prefix;   // 不切開代理對

    int suffix = 0;
    const int maxSuffix = qMin(oldLen, newLen) - prefix;
    while (suffix < maxSuffix &&
           oldText.at(oldLen - 1 - suffix) == newText.at(newLen - 1 - suffix)) ++suffix;
    if (suffix > 0 && oldText.at(oldLen - suffix).isLowSurrogate()) --suffix;

    positionOfOffset(oldText, prefix, &edit->startLine, &edit->startChar);
    positionOfOffset(oldText, oldLen - suffix, &edit->endLine, &edit->endChar);
    edit->newText = newText.mid(prefix, newLen - suffix - prefix);
    return true;
}

static bool providerEnabled(const QJsonValue& v) {
    return v.isObject() || v.toBool();      // bool true 或 options 物件皆視為支援
}

ServerCaps parseServerCapabilities(const QJsonObject& initResult) {
    ServerCaps caps;
    const QJsonObject c = initResult.value("capabilities").toObject();

    const QJsonValue sync = c.value("textDocumentSync");
    if (sync.isDouble())
        caps.syncKind = sync.toInt(1);
    else if (sync.isObject())
        caps.syncKind = sync.toObject().value("change").toInt(1);

    for (const QJsonValue& t : c.value("completionProvider").toObject()
                                   .value("triggerCharacters").toArray())
        caps.completionTriggers += t.toString();

    caps.references = providerEnabled(c.value("referencesProvider"));
    caps.rename     = providerEnabled(c.value("renameProvider"));
    caps.formatting = providerEnabled(c.value("documentFormattingProvider"));
    return caps;
}

static QString markedStringToText(const QJsonValue& v) {
    if (v.isString()) return v.toString();
    if (v.isObject()) return v.toObject().value("value").toString();   // MarkupContent / {language,value}
    return QString();
}

QString extractHoverText(const QJsonValue& result, int maxLen) {
    if (!result.isObject()) return QString();
    const QJsonValue contents = result.toObject().value("contents");
    QString text;
    if (contents.isArray()) {
        QStringList parts;
        for (const QJsonValue& v : contents.toArray()) {
            const QString t = markedStringToText(v);
            if (!t.isEmpty()) parts.append(t);
        }
        text = parts.join("\n\n");
    } else {
        text = markedStringToText(contents);
    }
    text = text.trimmed();
    if (text.size() > maxLen) text = text.left(maxLen) + QStringLiteral("…");
    return text;
}

} // namespace LspProtocol
