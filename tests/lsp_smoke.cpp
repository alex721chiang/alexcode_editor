// 無頭煙霧測試：以真實 clangd 驗證 LspClient
// v1：initialize → didOpen（含蓄意錯誤）→ 診斷 → definition → hover
// v2：references → rename → 增量 didChange（修正錯誤）→ 診斷清空 → formatting
// 通過條件：8 步全過（增量同步以「修改後診斷正確清空」間接驗證伺服器端內容一致）。
#include <QCoreApplication>
#include <QTimer>
#include <QFile>
#include <QDir>
#include <QDebug>
#include "LspClient.h"

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);

    // 測試專案：一個有錯誤的 cpp 檔
    const QString dir = QDir::temp().filePath("alexcode_lsp_smoke");
    QDir().mkpath(dir);
    const QString file = dir + "/main.cpp";
    const QString code =
        "int answer() { return 42; }\n"
        "int main() {\n"
        "    int x = answer();\n"
        "    undeclared_fn();\n"        // 蓄意錯誤（第 4 行）
        "    return x;\n"
        "}\n";
    const QString code2 =               // 修正錯誤＋蓄意壞排版（給 formatting）
        "int answer() { return 42; }\n"
        "int main() {\n"
        "    int x = answer();\n"
        "    int y=x+ 1;\n"
        "    return y;\n"
        "}\n";
    { QFile f(file); f.open(QIODevice::WriteOnly); f.write(code.toUtf8()); }

    LspClient client("clangd", {}, dir);
    int passed = 0;
    const int total = 8;
    bool gotErrorDiag = false, changedDoc = false;

    QObject::connect(&client, &LspClient::ready, [&]() {
        qInfo() << "[1] initialize 握手完成, syncKind =" << client.caps().syncKind
                << "triggers =" << client.caps().completionTriggers;
        ++passed;
        client.openDocument(file, "cpp", code);
    });
    QObject::connect(&client, &LspClient::diagnosticsReceived,
                     [&](const QString& path, const QList<LspProtocol::Diagnostic>& diags) {
        bool hasError = false;
        for (const auto& d : diags) hasError |= (d.severity == 1);
        if (!gotErrorDiag) {
            for (const auto& d : diags) {
                if (d.severity == 1 && d.startLine == 3) {       // 0-based 第 4 行
                    qInfo() << "[2] 收到錯誤診斷 @ line" << d.startLine << ":" << d.message;
                    gotErrorDiag = true;
                    ++passed;
                    client.requestDefinition(path, 2, 13);       // 第 3 行的 answer() 呼叫
                    break;
                }
            }
        } else if (changedDoc && !hasError) {
            qInfo() << "[7] 增量 didChange 後診斷清空（伺服器端內容一致）";
            changedDoc = false;                                  // 只記一次
            ++passed;
            client.requestFormatting(file, 4, true);
        }
    });
    QObject::connect(&client, &LspClient::definitionReady,
                     [&](const QString& path, int line, int ch) {
        qInfo() << "[3] 定義跳轉 →" << path << "line" << line << "ch" << ch;
        if (line == 0) ++passed;                                 // answer 定義於第 1 行
        client.requestHover(file, 2, 13);
    });
    QObject::connect(&client, &LspClient::hoverReady,
                     [&](const QString&, const QString& text) {
        qInfo() << "[4] hover:" << text.left(80);
        if (!text.isEmpty()) ++passed;
        client.requestReferences(file, 2, 13);                   // answer 的全部引用
    });
    QObject::connect(&client, &LspClient::referencesReady,
                     [&](const QString&, const QList<LspProtocol::Location>& locs) {
        qInfo() << "[5] references:" << locs.size() << "處";
        if (locs.size() >= 2) ++passed;                          // 定義 + 呼叫
        client.requestRename(file, 0, 4, "answer42");            // 重新命名 answer
    });
    QObject::connect(&client, &LspClient::renameReady,
                     [&](const QHash<QString, QList<LspProtocol::TextEdit>>& edits) {
        int n = 0;
        for (const auto& list : edits) n += list.size();
        qInfo() << "[6] rename:" << edits.size() << "檔案" << n << "處編輯";
        if (n >= 2) ++passed;
        changedDoc = true;                                       // 增量 didChange：修正錯誤
        client.changeDocument(file, code2);
    });
    QObject::connect(&client, &LspClient::formattingReady,
                     [&](const QString&, const QList<LspProtocol::TextEdit>& edits) {
        const QString formatted = LspProtocol::applyTextEdits(code2, edits);
        qInfo() << "[8] formatting:" << edits.size() << "處編輯";
        if (!edits.isEmpty() && formatted.contains("int y = x + 1;")) ++passed;
        client.shutdown();
        qInfo() << (passed == total
                        ? "SMOKE TEST PASSED (8/8)"
                        : QString("SMOKE TEST FAILED (%1/8)").arg(passed).toUtf8().constData());
        QCoreApplication::exit(passed == total ? 0 : 1);
    });
    QObject::connect(&client, &LspClient::failed, [&](const QString& r) {
        qCritical() << "FAILED:" << r;
        QCoreApplication::exit(2);
    });

    QTimer::singleShot(30000, [&]() {                            // 逾時保險
        qCritical() << "TIMEOUT, passed =" << passed;
        QCoreApplication::exit(3);
    });

    client.start();
    return app.exec();
}
