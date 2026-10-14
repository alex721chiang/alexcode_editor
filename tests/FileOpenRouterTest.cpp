// macOS：Finder 雙擊 / 「打開方式」/ 拖到 Dock 圖示，檔案以 QFileOpenEvent 送給 QApplication
// （不是 argv）。冷啟動時事件可能早於主視窗建立，必須暫存。此處以手動送出事件驗證（跨平台邏輯）。
#include <gtest/gtest.h>
#include <QCoreApplication>
#include <QFileOpenEvent>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QTabWidget>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
#include "FileOpenRouter.h"
#include "MainWindow.h"
#include "Portable.h"

namespace {
bool sendOpen(const QString& path) {
    QFileOpenEvent ev(path);
    return QCoreApplication::sendEvent(QCoreApplication::instance(), &ev);
}
}

// 主視窗就緒前收到的開檔事件先暫存，就緒時依序交付
TEST(FileOpenRouterTest, BuffersUntilReadyThenDeliversInOrder) {
    FileOpenRouter router;
    QSignalSpy spy(&router, &FileOpenRouter::filesRequested);
    EXPECT_TRUE(sendOpen("/a.txt"));
    EXPECT_TRUE(sendOpen("/中文 b.txt"));
    EXPECT_EQ(spy.count(), 0);
    router.setReady();
    ASSERT_EQ(spy.count(), 1);
    EXPECT_EQ(spy.at(0).at(0).toStringList(), (QStringList{"/a.txt", "/中文 b.txt"}));
}

// 就緒後的開檔事件立即交付
TEST(FileOpenRouterTest, DeliversImmediatelyWhenReady) {
    FileOpenRouter router;
    router.setReady();
    QSignalSpy spy(&router, &FileOpenRouter::filesRequested);
    EXPECT_TRUE(sendOpen("/c.log"));
    ASSERT_EQ(spy.count(), 1);
    EXPECT_EQ(spy.at(0).at(0).toStringList(), QStringList{"/c.log"});
}

// 路由器結束後不再攔截（不影響其他事件處理）
TEST(FileOpenRouterTest, StopsFilteringWhenDestroyed) {
    {
        FileOpenRouter router;
        router.setReady();
    }
    EXPECT_FALSE(sendOpen("/d.txt"));          // 無人處理：回傳 false，且不會崩潰
}

// 串到主視窗：Finder 開檔 → 開成分頁
TEST(FileOpenRouterTest, OpensFileInMainWindow) {
    QStandardPaths::setTestModeEnabled(true);
    QDir(Portable::dataDir()).removeRecursively();
    { AppSettings s; s.setValue("session/restore", false); s.sync(); }
    QTemporaryDir dir;
    const QString p = dir.filePath("finder.txt");
    { QFile f(p); f.open(QIODevice::WriteOnly); f.write("from finder\n"); }
    {
        FileOpenRouter router;
        MainWindow w;
        QObject::connect(&router, &FileOpenRouter::filesRequested, &w, &MainWindow::openFilesFromOtherInstance);
        sendOpen(p);                           // 冷啟動：事件早於就緒
        router.setReady();
        auto* tabs = w.findChild<QTabWidget*>();
        ASSERT_TRUE(tabs);
        EXPECT_EQ(tabs->currentWidget()->property("filePath").toString(), p);
    }
    QStandardPaths::setTestModeEnabled(false);
}
