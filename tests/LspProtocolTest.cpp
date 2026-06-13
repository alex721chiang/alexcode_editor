#include <gtest/gtest.h>
#include <QJsonDocument>
#include <QJsonArray>
#include "../src/LspProtocol.h"

using namespace LspProtocol;

static QJsonObject obj(const char* json) {
    return QJsonDocument::fromJson(json).object();
}

// ---- 框架編碼 ----
TEST(LspFrame, EncodesContentLengthHeader) {
    const QByteArray out = frame(QJsonObject{{"jsonrpc", "2.0"}, {"method", "x"}});
    ASSERT_TRUE(out.startsWith("Content-Length: "));
    const int headerEnd = out.indexOf("\r\n\r\n");
    ASSERT_GT(headerEnd, 0);
    const int len = out.mid(16, headerEnd - 16).toInt();
    EXPECT_EQ(len, out.size() - headerEnd - 4);
}

// ---- 串流讀取器 ----
TEST(LspReader, ParsesWholeMessage) {
    Reader r;
    r.feed(frame(QJsonObject{{"id", 1}, {"method", "test"}}));
    const auto msgs = r.takeMessages();
    ASSERT_EQ(msgs.size(), 1);
    EXPECT_EQ(msgs[0].value("method").toString(), "test");
}

TEST(LspReader, HandlesChunkSplitMidHeaderAndMidBody) {
    Reader r;
    const QByteArray full = frame(QJsonObject{{"id", 7}, {"method", "split"}});
    r.feed(full.left(5));                       // header 中段切斷
    EXPECT_TRUE(r.takeMessages().isEmpty());
    r.feed(full.mid(5, full.size() - 10));      // body 中段切斷
    EXPECT_TRUE(r.takeMessages().isEmpty());
    r.feed(full.right(5));
    const auto msgs = r.takeMessages();
    ASSERT_EQ(msgs.size(), 1);
    EXPECT_EQ(msgs[0].value("id").toInt(), 7);
}

TEST(LspReader, ParsesTwoMessagesInOneChunk) {
    Reader r;
    r.feed(frame(QJsonObject{{"id", 1}}) + frame(QJsonObject{{"id", 2}}));
    const auto msgs = r.takeMessages();
    ASSERT_EQ(msgs.size(), 2);
    EXPECT_EQ(msgs[1].value("id").toInt(), 2);
}

TEST(LspReader, IgnoresExtraHeadersAndCaseInsensitiveContentLength) {
    Reader r;
    const QByteArray body = R"({"id":3})";
    r.feed("content-type: application/json\r\ncontent-length: " +
           QByteArray::number(body.size()) + "\r\n\r\n" + body);
    const auto msgs = r.takeMessages();
    ASSERT_EQ(msgs.size(), 1);
    EXPECT_EQ(msgs[0].value("id").toInt(), 3);
}

// ---- URI ----
TEST(LspUri, RoundTripWithSpaces) {
    const QString path = "/tmp/my project/main.cpp";
    const QString uri = uriFromPath(path);
    EXPECT_TRUE(uri.startsWith("file://"));
    EXPECT_EQ(pathFromUri(uri), path);
}

TEST(LspUri, NonFileUriReturnsEmpty) {
    EXPECT_TRUE(pathFromUri("https://example.com/x").isEmpty());
}

// ---- 診斷 ----
TEST(LspDiagnostics, ParsesRangeSeverityMessage) {
    const QJsonArray arr = QJsonDocument::fromJson(R"([
      {"range": {"start": {"line": 2, "character": 4}, "end": {"line": 2, "character": 9}},
       "severity": 2, "message": "unused variable", "source": "clangd"}
    ])").array();
    const auto diags = parseDiagnostics(arr);
    ASSERT_EQ(diags.size(), 1);
    EXPECT_EQ(diags[0].startLine, 2);
    EXPECT_EQ(diags[0].startChar, 4);
    EXPECT_EQ(diags[0].endChar, 9);
    EXPECT_EQ(diags[0].severity, 2);
    EXPECT_EQ(diags[0].message, "unused variable");
    EXPECT_EQ(diags[0].source, "clangd");
}

TEST(LspDiagnostics, MissingSeverityDefaultsToError) {
    const QJsonArray arr = QJsonDocument::fromJson(
        R"([{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"message":"x"}])").array();
    ASSERT_EQ(parseDiagnostics(arr)[0].severity, 1);
}

// ---- 補全解析 ----
TEST(LspCompletion, AcceptsItemArrayAndCompletionList) {
    const QJsonValue asArray = QJsonDocument::fromJson(
        R"json([{"label": "foo"}, {"label": "bar", "insertText": "bar()"}])json").array();
    EXPECT_EQ(extractCompletions(asArray), (QStringList{"foo", "bar()"}));

    const QJsonValue asList = obj(R"({"isIncomplete": false, "items": [{"label": "baz"}]})");
    EXPECT_EQ(extractCompletions(asList), QStringList{"baz"});
}

TEST(LspCompletion, StripsSnippetPlaceholders) {
    EXPECT_EQ(stripSnippetPlaceholders("foo(${1:int x}, $0)"), "foo(int x, )");
    EXPECT_EQ(stripSnippetPlaceholders("name${2}"), "name");
    const QJsonValue v = QJsonDocument::fromJson(
        R"json([{"label": "fn", "insertText": "fn(${1:a})", "insertTextFormat": 2}])json").array();
    EXPECT_EQ(extractCompletions(v), QStringList{"fn(a)"});
}

TEST(LspCompletion, RespectsMaxItemsAndDeduplicates) {
    QJsonArray arr;
    for (int i = 0; i < 100; ++i)
        arr.append(QJsonObject{{"label", QStringLiteral("item%1").arg(i % 30)}});
    EXPECT_EQ(extractCompletions(arr, 20).size(), 20);
    EXPECT_EQ(extractCompletions(arr, 50).size(), 30);   // 去重後僅 30 個
}

// ---- 定義解析 ----
TEST(LspDefinition, AcceptsLocationLocationArrayAndLocationLink) {
    QString path; int line = -1, ch = -1;
    ASSERT_TRUE(extractDefinition(obj(
        R"({"uri": "file:///tmp/a.h", "range": {"start": {"line": 5, "character": 2}, "end": {"line": 5, "character": 8}}})"),
        &path, &line, &ch));
    EXPECT_EQ(path, "/tmp/a.h"); EXPECT_EQ(line, 5); EXPECT_EQ(ch, 2);

    ASSERT_TRUE(extractDefinition(QJsonDocument::fromJson(
        R"([{"uri": "file:///tmp/b.h", "range": {"start": {"line": 1, "character": 0}}}])").array(),
        &path, &line, &ch));
    EXPECT_EQ(path, "/tmp/b.h"); EXPECT_EQ(line, 1);

    ASSERT_TRUE(extractDefinition(QJsonDocument::fromJson(
        R"([{"targetUri": "file:///tmp/c.h", "targetSelectionRange": {"start": {"line": 9, "character": 3}}}])").array(),
        &path, &line, &ch));
    EXPECT_EQ(path, "/tmp/c.h"); EXPECT_EQ(line, 9); EXPECT_EQ(ch, 3);
}

TEST(LspDefinition, NullResultReturnsFalse) {
    QString p; int l, c;
    EXPECT_FALSE(extractDefinition(QJsonValue::Null, &p, &l, &c));
    EXPECT_FALSE(extractDefinition(QJsonArray(), &p, &l, &c));
}

// ---- Hover 解析 ----
TEST(LspHover, AcceptsStringMarkupContentAndMarkedStringArray) {
    EXPECT_EQ(extractHoverText(obj(R"({"contents": "plain text"})")), "plain text");
    EXPECT_EQ(extractHoverText(obj(R"({"contents": {"kind": "markdown", "value": "**doc**"}})")), "**doc**");
    EXPECT_EQ(extractHoverText(obj(
        R"({"contents": ["part1", {"language": "cpp", "value": "int x"}]})")), "part1\n\nint x");
}

TEST(LspHover, TruncatesLongText) {
    const QString result = extractHoverText(
        QJsonObject{{"contents", QString(2000, 'a')}}, 100);
    EXPECT_LE(result.size(), 101);   // 100 + 省略號
}

// ---- v2：references 解析 ----
TEST(LspLocations, ParsesLocationArrayAndLocationLink) {
    const QJsonValue arr = QJsonDocument::fromJson(R"json([
      {"uri": "file:///tmp/a.cpp", "range": {"start": {"line": 3, "character": 7}, "end": {"line": 3, "character": 12}}},
      {"targetUri": "file:///tmp/b.h", "targetSelectionRange": {"start": {"line": 0, "character": 4}}}
    ])json").array();
    const auto locs = extractLocations(arr);
    ASSERT_EQ(locs.size(), 2);
    EXPECT_EQ(locs[0].path, "/tmp/a.cpp"); EXPECT_EQ(locs[0].line, 3); EXPECT_EQ(locs[0].character, 7);
    EXPECT_EQ(locs[1].path, "/tmp/b.h");   EXPECT_EQ(locs[1].line, 0); EXPECT_EQ(locs[1].character, 4);
}

TEST(LspLocations, ToleratesSingleObjectAndNull) {
    EXPECT_EQ(extractLocations(obj(
        R"({"uri": "file:///tmp/x.h", "range": {"start": {"line": 1, "character": 2}}})")).size(), 1);
    EXPECT_TRUE(extractLocations(QJsonValue::Null).isEmpty());
    EXPECT_TRUE(extractLocations(QJsonArray()).isEmpty());
}

// ---- v2：TextEdit / WorkspaceEdit 解析 ----
TEST(LspTextEdits, ParsesRangeAndNewText) {
    const QJsonValue arr = QJsonDocument::fromJson(R"json([
      {"range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 3}}, "newText": "int"}
    ])json").array();
    const auto edits = extractTextEdits(arr);
    ASSERT_EQ(edits.size(), 1);
    EXPECT_EQ(edits[0].startLine, 1); EXPECT_EQ(edits[0].endChar, 3);
    EXPECT_EQ(edits[0].newText, "int");
}

TEST(LspWorkspaceEdit, ParsesChangesAndDocumentChanges) {
    const auto byChanges = extractWorkspaceEdit(obj(R"json({
      "changes": {"file:///tmp/a.cpp": [
        {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 3}}, "newText": "neo"}]}
    })json"));
    ASSERT_TRUE(byChanges.contains("/tmp/a.cpp"));
    EXPECT_EQ(byChanges["/tmp/a.cpp"].size(), 1);

    const auto byDocChanges = extractWorkspaceEdit(obj(R"json({
      "documentChanges": [
        {"textDocument": {"uri": "file:///tmp/b.cpp", "version": 4}, "edits": [
          {"range": {"start": {"line": 2, "character": 1}, "end": {"line": 2, "character": 4}}, "newText": "x"}]},
        {"kind": "rename", "oldUri": "file:///tmp/c", "newUri": "file:///tmp/d"}
      ]
    })json"));
    ASSERT_EQ(byDocChanges.size(), 1);                       // 檔案操作（kind）被忽略
    EXPECT_EQ(byDocChanges["/tmp/b.cpp"][0].newText, "x");
}

// ---- v2：套用編輯 ----
TEST(LspApplyEdits, AppliesMultipleEditsAgainstOriginalPositions) {
    // 兩個編輯皆以「原文」位置表示；套用順序不得互相影響
    const QString text = "foo bar\nfoo baz\n";
    const QList<TextEdit> edits{
        {0, 0, 0, 3, "renamed"},        // 第 0 行 foo → renamed
        {1, 0, 1, 3, "renamed"},        // 第 1 行 foo → renamed
    };
    EXPECT_EQ(applyTextEdits(text, edits), "renamed bar\nrenamed baz\n");
}

TEST(LspApplyEdits, HandlesInsertionDeletionAndMultiLineRange) {
    EXPECT_EQ(applyTextEdits("ab\ncd\n", {{0, 1, 0, 1, "X"}}), "aXb\ncd\n");       // 插入
    EXPECT_EQ(applyTextEdits("ab\ncd\n", {{0, 0, 1, 1, ""}}), "d\n");              // 跨行刪除
    EXPECT_EQ(applyTextEdits("ab", {{0, 0, 5, 99, "z"}}), "z");                    // 超界 clamp
}

// ---- v2：增量同步 diff ----
TEST(LspIncremental, NoChangeReturnsFalse) {
    TextEdit e;
    EXPECT_FALSE(computeIncrementalEdit("same", "same", &e));
}

TEST(LspIncremental, ComputesMidTextReplacement) {
    TextEdit e;
    ASSERT_TRUE(computeIncrementalEdit("int foo = 1;\nint bar;\n",
                                       "int foo = 42;\nint bar;\n", &e));
    EXPECT_EQ(e.startLine, 0);
    // 套用回去必須重現新文
    const QString applied = applyTextEdits("int foo = 1;\nint bar;\n", {e});
    EXPECT_EQ(applied, "int foo = 42;\nint bar;\n");
}

TEST(LspIncremental, ComputesInsertionAndDeletionAcrossLines) {
    TextEdit e;
    ASSERT_TRUE(computeIncrementalEdit("a\nb\nc\n", "a\nb\nNEW\nc\n", &e));
    EXPECT_EQ(applyTextEdits("a\nb\nc\n", {e}), "a\nb\nNEW\nc\n");

    ASSERT_TRUE(computeIncrementalEdit("a\nb\nc\n", "a\nc\n", &e));
    EXPECT_EQ(applyTextEdits("a\nb\nc\n", {e}), "a\nc\n");

    ASSERT_TRUE(computeIncrementalEdit("", "hello", &e));
    EXPECT_EQ(e.startLine, 0); EXPECT_EQ(e.startChar, 0);
    EXPECT_EQ(e.newText, "hello");
}

TEST(LspIncremental, DoesNotSplitSurrogatePairs) {
    // 𝄞 (U+1D11E) 為代理對；置換其中一個字元不得從代理對中間切開
    const QString a = QStringLiteral("x𝄞y");
    const QString b = QStringLiteral("x𝄢y");                 // 同高位代理、不同低位代理
    TextEdit e;
    ASSERT_TRUE(computeIncrementalEdit(a, b, &e));
    EXPECT_EQ(applyTextEdits(a, {e}), b);
    EXPECT_EQ(e.startChar, 1);                               // 從代理對開頭算起
}

// ---- v2：initialize 能力解析 ----
TEST(LspServerCaps, ParsesSyncKindTriggersAndProviders) {
    const auto caps = parseServerCapabilities(obj(R"json({
      "capabilities": {
        "textDocumentSync": {"openClose": true, "change": 2},
        "completionProvider": {"triggerCharacters": [".", ">", ":"]},
        "referencesProvider": true,
        "renameProvider": {"prepareProvider": true},
        "documentFormattingProvider": true
      }
    })json"));
    EXPECT_EQ(caps.syncKind, 2);
    EXPECT_EQ(caps.completionTriggers, ".>:");
    EXPECT_TRUE(caps.references);
    EXPECT_TRUE(caps.rename);
    EXPECT_TRUE(caps.formatting);
}

TEST(LspServerCaps, NumericSyncAndMissingProvidersDefaultSafely) {
    const auto caps = parseServerCapabilities(obj(
        R"({"capabilities": {"textDocumentSync": 1}})"));
    EXPECT_EQ(caps.syncKind, 1);
    EXPECT_TRUE(caps.completionTriggers.isEmpty());
    EXPECT_FALSE(caps.references);
    EXPECT_FALSE(caps.rename);
    EXPECT_FALSE(caps.formatting);

    EXPECT_EQ(parseServerCapabilities(QJsonObject()).syncKind, 1);   // 預設 full sync
}
