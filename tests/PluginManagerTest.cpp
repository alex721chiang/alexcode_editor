// 腳本外掛系統測試：真的用 QJSEngine 載入暫存資料夾裡的 .js、註冊指令、
// 對真實 CodeEditor 執行 API 操作（不是 mock）。
#include <gtest/gtest.h>
#include <QTemporaryDir>
#include <QFile>
#include "PluginManager.h"
#include "CodeEditor.h"

namespace {

void writeScript(const QString& dir, const QString& name, const QByteArray& body) {
    QFile f(dir + "/" + name);
    ASSERT_TRUE(f.open(QIODevice::WriteOnly));
    f.write(body);
}

struct Fixture {
    QTemporaryDir tmp;
    std::unique_ptr<CodeEditor> editor{ new CodeEditor() };
    std::unique_ptr<PluginManager> mgr;
    QStringList openedFiles;

    Fixture() {
        mgr.reset(new PluginManager(
            [this]() { return editor.get(); },
            [this](const QString& p) { openedFiles.append(p); },
            tmp.path()));
    }
};

} // namespace

TEST(PluginManager, RegistersCommandsFromScript) {
    Fixture fx;
    writeScript(fx.tmp.path(), "a.js",
                "alexcode.registerCommand(\"cmd-one\", function() {});\n"
                "alexcode.registerCommand(\"cmd-two\", function() {});\n");
    fx.mgr->reload();
    EXPECT_EQ(fx.mgr->scriptCount(), 1);
    EXPECT_EQ(fx.mgr->commandNames(), (QStringList{"cmd-one", "cmd-two"}));
    EXPECT_TRUE(fx.mgr->errors().isEmpty());
}

TEST(PluginManager, CommandManipulatesEditor) {
    Fixture fx;
    fx.editor->setPlainText("hello");
    writeScript(fx.tmp.path(), "ins.js",
                "alexcode.registerCommand(\"append\", function() {\n"
                "    alexcode.gotoLine(1);\n"
                "    alexcode.setText(alexcode.text() + \" world\");\n"
                "});\n");
    fx.mgr->reload();
    ASSERT_TRUE(fx.mgr->runCommand("append"));
    EXPECT_EQ(fx.editor->toPlainText(), QString("hello world"));
    fx.editor->undo();                                   // setText 走游標 → 可復原
    EXPECT_EQ(fx.editor->toPlainText(), QString("hello"));
}

TEST(PluginManager, SelectedTextAndInsert) {
    Fixture fx;
    fx.editor->setPlainText("abc def");
    QTextCursor c = fx.editor->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);           // 選 "abc"
    fx.editor->setTextCursor(c);
    writeScript(fx.tmp.path(), "wrap.js",
                "alexcode.registerCommand(\"wrap\", function() {\n"
                "    alexcode.insertText(\"[\" + alexcode.selectedText() + \"]\");\n"
                "});\n");
    fx.mgr->reload();
    ASSERT_TRUE(fx.mgr->runCommand("wrap"));
    EXPECT_EQ(fx.editor->toPlainText(), QString("[abc] def"));
}

TEST(PluginManager, SyntaxErrorIsReportedNotFatal) {
    Fixture fx;
    writeScript(fx.tmp.path(), "bad.js", "this is not js;;;\n");
    writeScript(fx.tmp.path(), "good.js",
                "alexcode.registerCommand(\"ok\", function() {});\n");
    fx.mgr->reload();
    EXPECT_EQ(fx.mgr->scriptCount(), 1);                 // 壞的算錯誤、好的照載
    EXPECT_FALSE(fx.mgr->errors().isEmpty());
    EXPECT_TRUE(fx.mgr->commandNames().contains("ok"));
}

TEST(PluginManager, RunUnknownCommandReturnsFalse) {
    Fixture fx;
    fx.mgr->reload();
    EXPECT_FALSE(fx.mgr->runCommand("nope"));
}

TEST(PluginManager, ReloadReplacesCommands) {
    Fixture fx;
    writeScript(fx.tmp.path(), "v1.js",
                "alexcode.registerCommand(\"old\", function() {});\n");
    fx.mgr->reload();
    ASSERT_TRUE(fx.mgr->commandNames().contains("old"));
    QFile::remove(fx.tmp.path() + "/v1.js");
    writeScript(fx.tmp.path(), "v2.js",
                "alexcode.registerCommand(\"new\", function() {});\n");
    fx.mgr->reload();
    EXPECT_FALSE(fx.mgr->commandNames().contains("old"));
    EXPECT_TRUE(fx.mgr->commandNames().contains("new"));
}
