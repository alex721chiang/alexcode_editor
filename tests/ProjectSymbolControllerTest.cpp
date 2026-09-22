#include <gtest/gtest.h>
#include <QTemporaryDir>
#include <QDir>
#include <QSignalSpy>
#include <QListWidget>
#include <QDockWidget>
#include "ProjectSymbolController.h"
#include "ProjectSymbolDialog.h"   // QPointer<ProjectSymbolDialog> 解構需完整型別

TEST(ProjectSymbolControllerTest, CollectWatchDirsPrunesSkipDirsAndCaps) {
    QTemporaryDir tmp;
    QDir d(tmp.path());
    d.mkpath("src/a");
    d.mkpath(".git/objects");
    d.mkpath("node_modules/x");
    const QStringList dirs = ProjectSymbolController::collectWatchDirs(tmp.path(), 512);
    EXPECT_TRUE(dirs.contains(tmp.path()));
    EXPECT_TRUE(dirs.contains(d.absoluteFilePath("src/a")));
    for (const QString& p : dirs) {
        EXPECT_FALSE(p.contains("/.git")) << p.toStdString();
        EXPECT_FALSE(p.contains("node_modules")) << p.toStdString();
    }
    EXPECT_EQ(ProjectSymbolController::collectWatchDirs(tmp.path(), 2).size(), 2);
}

// 監看清單改在背景掃描：呼叫當下不做目錄巡訪，完成後才掛上監看
TEST(ProjectSymbolControllerTest, AutoRefreshScansDirsInBackground) {
    QTemporaryDir tmp;
    QDir(tmp.path()).mkpath("src/a");
    QListWidget refs;
    QDockWidget dock;
    ProjectSymbolController c(nullptr, &refs, &dock);
    QSignalSpy spy(&c, &ProjectSymbolController::watchDirsUpdated);

    c.enableAutoRefresh(tmp.path());
    EXPECT_TRUE(c.watchedDirs().isEmpty());
    ASSERT_TRUE(spy.wait(5000));
    EXPECT_TRUE(c.watchedDirs().contains(QDir(tmp.path()).absoluteFilePath("src/a")));
}

// 網路資料夾預設不掛目錄監看（Windows 每個目錄一個 ReadDirectoryChangesW，SMB 上代價高）
TEST(ProjectSymbolControllerTest, NetworkFolderIsNotWatchedByDefault) {
    EXPECT_FALSE(ProjectSymbolController::shouldWatch("//server/share/proj"));
    EXPECT_TRUE(ProjectSymbolController::shouldWatch(QDir::tempPath()));
}
