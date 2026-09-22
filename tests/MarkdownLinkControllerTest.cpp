#include <gtest/gtest.h>
#include <QMainWindow>
#include <QDockWidget>
#include <QListWidget>
#include <QTemporaryDir>
#include <QFile>
#include <QSignalSpy>
#include <QCoreApplication>
#include "MarkdownLinkController.h"

namespace {
void writeFile(const QString& path, const QByteArray& data) {
    QFile f(path);
    ASSERT_TRUE(f.open(QIODevice::WriteOnly));
    f.write(data);
}
QStringList backlinkItems(QDockWidget* dock) {
    QStringList out;
    auto* list = dock->findChild<QListWidget*>();
    for (int i = 0; list && i < list->count(); ++i) out << list->item(i)->text();
    return out;
}
}

// 面板都沒開：不建索引（網路資料夾開啟時不再整棵掃描）
TEST(MarkdownLinkControllerTest, NoIndexingWhileDocksHidden) {
    QTemporaryDir dir;
    writeFile(dir.filePath("a.md"), "[[b]]");
    writeFile(dir.filePath("b.md"), "# b");
    QMainWindow host;
    MarkdownLinkController c(&host);
    c.rebuildIndex(dir.path(), dir.filePath("b.md"));
    EXPECT_FALSE(c.isIndexing());
}

// 面板開啟：背景建索引，呼叫當下立即返回（不在主執行緒掃描），完成後刷新 backlinks
TEST(MarkdownLinkControllerTest, BuildsInBackgroundWhenDockVisible) {
    QTemporaryDir dir;
    writeFile(dir.filePath("a.md"), "[[b]]");
    writeFile(dir.filePath("b.md"), "# b");
    QMainWindow host;
    host.show();
    MarkdownLinkController c(&host);
    c.backlinksDock()->show();
    QSignalSpy spy(&c, &MarkdownLinkController::indexRebuilt);

    c.rebuildIndex(dir.path(), dir.filePath("b.md"));
    EXPECT_TRUE(c.isIndexing());
    EXPECT_FALSE(backlinkItems(c.backlinksDock()).contains("a.md"));   // 尚未換上新索引

    ASSERT_TRUE(spy.wait(5000));
    EXPECT_FALSE(c.isIndexing());
    EXPECT_TRUE(backlinkItems(c.backlinksDock()).contains("a.md"));
}

// 面板未開時點 wikilink：先建索引再解析，已存在的筆記不可被誤判為新筆記而重建
TEST(MarkdownLinkControllerTest, WikilinkWaitsForIndexInsteadOfCreatingDuplicate) {
    QTemporaryDir dir;
    QDir(dir.path()).mkdir("sub");
    writeFile(dir.filePath("sub/Note.md"), "# existing");
    QMainWindow host;
    MarkdownLinkController c(&host);
    QSignalSpy spy(&c, &MarkdownLinkController::fileOpenRequested);

    c.openOrCreateWikilink("Note", dir.path());
    ASSERT_TRUE(spy.wait(5000));
    EXPECT_EQ(QFileInfo(spy.at(0).at(0).toString()).absoluteFilePath(),
              QFileInfo(dir.filePath("sub/Note.md")).absoluteFilePath());
    EXPECT_FALSE(QFile::exists(dir.filePath("Note.md")));
}
