// 大檔/超長行的編輯器效能防護。
#include <gtest/gtest.h>
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QScrollBar>
#include <QThread>
#include "CodeEditor.h"

namespace {
void pump(int ms = 50) {
    QElapsedTimer t;
    t.start();
    while (t.elapsed() < ms) { QCoreApplication::processEvents(QEventLoop::AllEvents, 10); QThread::msleep(1); }
}
QString manyFunctions(int n) {
    QString s;
    for (int i = 0; i < n; ++i) s += QStringLiteral("int f%1(int a) {\n  return a + %1;\n}\n").arg(i);
    return s;
}
}

// tree-sitter 重新高亮會逐行觸發 updateRequest；sticky 標頭（掃描整份符號表）不可每次都重算
//（原本為 行數×符號數，40,000 行的檔案開啟要 13 秒）
TEST(LargeDocumentTest, StickyRecomputeIsCoalesced) {
    CodeEditor e;
    e.resize(800, 600);
    e.show();
    e.setSyntaxLanguage(SyntaxHighlighter::Language::CPP, "big.cpp");
    pump();
    const int before = e.stickyUpdateCount();
    e.setPlainText(manyFunctions(3000));
    pump(200);
    EXPECT_LE(e.stickyUpdateCount() - before, 5);
}

// 合併後 sticky 仍正確：捲到函式內部時，頂端顯示該函式標頭
TEST(LargeDocumentTest, StickyHeadersStillTrackScrolling) {
    CodeEditor e;
    e.resize(800, 200);
    e.show();
    e.setSyntaxLanguage(SyntaxHighlighter::Language::CPP, "s.cpp");
    QString body = "void outer() {\n";
    for (int i = 0; i < 200; ++i) body += QStringLiteral("  int v%1 = %1;\n").arg(i);
    body += "}\n";
    e.setPlainText(body);
    pump(200);
    e.verticalScrollBar()->setValue(50);                 // 捲到函式中段
    pump(100);
    ASSERT_FALSE(e.stickyHeaders().isEmpty());
    EXPECT_EQ(e.stickyHeaders().first().name, "outer");
}

// 壓縮檔（整份只有一行）：「游標所在字詞的出現處」原本以「可見的 400 行」為範圍，
// 單一超長行等於整份文件，一次加入上萬個 ExtraSelection，每次重繪都要重新排版（0.5 秒以上）。
// 改為只標示可見字元範圍內的出現處。
TEST(LargeDocumentTest, OccurrenceHighlightsLimitedToVisibleRangeOnHugeLine) {
    CodeEditor e;
    e.resize(800, 400);
    e.show();
    QString line;
    for (int i = 0; i < 20000; ++i) line += QStringLiteral("foo(bar);");   // 180k 字元、單一行
    e.setPlainText(line);
    QTextCursor c = e.textCursor();
    c.setPosition(1);                                    // 游標在第一個 "foo" 上
    e.setTextCursor(c);
    pump(100);
    const int n = e.extraSelections().size();
    EXPECT_GT(n, 1);                                     // 可見範圍內仍有標示
    EXPECT_LT(n, 2000);                                  // 不是整行 20,000 個
}

// 搜尋結果標示同樣只取可見範圍；捲動後重新計算，新的可見處也有標示
TEST(LargeDocumentTest, SearchHighlightsFollowScrollingOnHugeLine) {
    CodeEditor e;
    e.resize(800, 400);
    e.show();
    QString line;
    for (int i = 0; i < 20000; ++i) line += QStringLiteral("abc xyz; ");
    e.setPlainText(line);
    e.setSearchHighlightPattern(QRegularExpression("xyz"));
    pump(100);
    const int before = e.extraSelections().size();
    EXPECT_GT(before, 1);
    EXPECT_LT(before, 2000);
    e.verticalScrollBar()->setValue(e.verticalScrollBar()->maximum() / 2);
    pump(200);
    const int mid = e.cursorForPosition(QPoint(5, 5)).position();
    bool nearViewport = false;
    for (const auto& s : e.extraSelections())
        if (s.cursor.hasSelection() && qAbs(s.cursor.selectionStart() - mid) < 20000) nearViewport = true;
    EXPECT_TRUE(nearViewport);
}

// ---- 不換行（NoWrap）對超長行是災難：Qt 會把整行當成單一行排版（1MB 單行實測 >10 分鐘）----
#include <QStandardPaths>
#include <QTabWidget>
#include <QAction>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
#include "MainWindow.h"
#include "Portable.h"

namespace {
struct TestEnv {
    TestEnv(int highlightMaxMB) {
        QStandardPaths::setTestModeEnabled(true);
        QDir(Portable::dataDir()).removeRecursively();
        AppSettings s;
        s.setValue("session/restore", false);
        s.setValue("editor/highlightMaxMB", highlightMaxMB);   // 0 → 任何檔案都進「大檔模式」
        s.sync();
    }
    ~TestEnv() { QStandardPaths::setTestModeEnabled(false); }
};
QString writeTmp(const QTemporaryDir& d, const QString& name, const QByteArray& data) {
    QFile f(d.filePath(name));
    f.open(QIODevice::WriteOnly);
    f.write(data);
    return d.filePath(name);
}
QPlainTextEdit::LineWrapMode wrapOf(MainWindow& w, const QString& path) {
    auto* tabs = w.findChild<QTabWidget*>();
    for (int i = 0; i < tabs->count(); ++i)
        if (tabs->widget(i)->property("filePath").toString() == path)
            return qobject_cast<QPlainTextEdit*>(tabs->widget(i))->lineWrapMode();
    ADD_FAILURE() << "tab not found";
    return QPlainTextEdit::NoWrap;
}
}

// 大檔模式原本一律改為不換行：含超長行的檔案保留換行；一般大檔維持不換行
TEST(LargeDocumentTest, LargeFileModeKeepsWrapForOverlongLines) {
    TestEnv env(0);
    QTemporaryDir d;
    const QString longFile = writeTmp(d, "min.js", QByteArray(30000, 'a') + "\n");
    const QString logFile = writeTmp(d, "app.log", QByteArray("short line\n").repeated(100));
    MainWindow w;
    w.openFilesFromOtherInstance({longFile, logFile});
    EXPECT_EQ(wrapOf(w, longFile), QPlainTextEdit::WidgetWidth);
    EXPECT_EQ(wrapOf(w, logFile), QPlainTextEdit::NoWrap);
}

// 關閉「自動換行」時，含超長行的分頁維持換行（其餘分頁照常關閉）
TEST(LargeDocumentTest, WordWrapToggleSkipsOverlongLineTabs) {
    TestEnv env(100);
    QTemporaryDir d;
    const QString longFile = writeTmp(d, "min.js", QByteArray(30000, 'a') + "\n");
    const QString normal = writeTmp(d, "n.cpp", "int x;\n");
    MainWindow w;
    w.openFilesFromOtherInstance({longFile, normal});
    QAction* wrap = nullptr;
    for (QAction* a : w.findChildren<QAction*>())
        if (a->isCheckable() && a->text() == QLatin1String("Word Wrap")) wrap = a;
    ASSERT_TRUE(wrap);
    wrap->trigger();                                         // 勾選 → 取消
    ASSERT_FALSE(wrap->isChecked());
    EXPECT_EQ(wrapOf(w, longFile), QPlainTextEdit::WidgetWidth);
    EXPECT_EQ(wrapOf(w, normal), QPlainTextEdit::NoWrap);
}
