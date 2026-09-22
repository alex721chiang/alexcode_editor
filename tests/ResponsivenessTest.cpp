// UI 回應性相關元件測試：卡頓偵測、背景檔案監看、git 不依賴工作目錄。
#include <gtest/gtest.h>
#include <QCoreApplication>
#include <QTemporaryDir>
#include <QSignalSpy>
#include <QElapsedTimer>
#include <QProcess>
#include <QThread>
#include <QFile>
#include <QDir>
#include <thread>
#include "StallMonitor.h"
#include "BackgroundFileWatcher.h"
#include "GitGutterController.h"

namespace {
void writeFile(const QString& p, const QByteArray& d) {
    QFile f(p);
    ASSERT_TRUE(f.open(QIODevice::WriteOnly));
    f.write(d);
}
// QSignalSpy 以 direct connection 計數：背景執行緒發出的訊號可能在 wait() 開始前就已計入，
// 因此改以「計數達到 n」輪詢
bool waitCount(QSignalSpy& spy, int n, int timeoutMs = 3000) {
    QElapsedTimer t;
    t.start();
    while (spy.count() < n && t.elapsed() < timeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        QThread::msleep(5);
    }
    return spy.count() >= n;
}
void pump(int ms) {
    QElapsedTimer t;
    t.start();
    while (t.elapsed() < ms) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        QThread::msleep(5);
    }
}
}

// 主執行緒卡住時，記錄卡在哪個區段；其他執行緒的區段不影響主執行緒歸因
TEST(StallMonitorTest, AttributesStallToMainThreadScope) {
    QTemporaryDir dir;
    StallMonitor::start(dir.filePath("perf.log"), 100);
    pump(100);
    std::thread other([] { StallMonitor::Scope s("worker.scope"); QThread::msleep(400); });
    {
        STALL_SCOPE("test.block");
        QThread::msleep(400);                  // 主執行緒卡 400ms
    }
    pump(200);                                  // 恢復心跳，看門狗寫出紀錄
    other.join();
    StallMonitor::stop();
    const QString last = StallMonitor::lastStall();
    EXPECT_TRUE(last.contains("test.block")) << last.toStdString();
    EXPECT_FALSE(last.contains("worker.scope")) << last.toStdString();
    QFile log(dir.filePath("perf.log"));
    ASSERT_TRUE(log.open(QIODevice::ReadOnly));
    EXPECT_TRUE(log.readAll().contains("test.block"));
}

// 背景執行緒監看：變更通知送回主執行緒；檔案被刪除重建（存檔方式之一）後仍持續監看
TEST(BackgroundFileWatcherTest, NotifiesAndSurvivesReplace) {
    QTemporaryDir dir;
    const QString p = dir.filePath("a.log");
    writeFile(p, "1\n");
    BackgroundFileWatcher w;
    QSignalSpy spy(&w, &BackgroundFileWatcher::fileChanged);
    w.addPath(p);
    w.waitForIdle();
    EXPECT_TRUE(w.paths().contains(p));

    writeFile(p, "2\n");
    ASSERT_TRUE(waitCount(spy, 1));
    EXPECT_EQ(spy.last().at(0).toString(), p);

    QFile::remove(p);                            // 刪除重建
    writeFile(p, "3\n");
    ASSERT_TRUE(waitCount(spy, 2));
    w.waitForIdle();                             // 背景已重新加回監看
    pump(100);
    const int before = spy.count();
    writeFile(p, "4\n");                          // 重建後的修改仍收得到
    ASSERT_TRUE(waitCount(spy, before + 1));

    w.clear();
    EXPECT_TRUE(w.paths().isEmpty());
}

// git 以 -C 指定目錄（不設子行程工作目錄）：HEAD 內容照樣取得
TEST(GitGutterControllerTest, FetchHeadWorksWithoutWorkingDirectory) {
    if (QProcess::execute("git", {"--version"}) != 0) GTEST_SKIP() << "git not installed";
    QTemporaryDir dir;
    QDir(dir.path()).mkpath("sub");
    const QString f = dir.filePath("sub/x.txt");
    writeFile(f, "committed\n");
    auto git = [&](const QStringList& args) {
        return QProcess::execute("git", QStringList{"-C", dir.path(), "-c", "user.email=t@t", "-c", "user.name=t"} + args);
    };
    ASSERT_EQ(git({"init", "-q"}), 0);
    ASSERT_EQ(git({"add", "-A"}), 0);
    ASSERT_EQ(git({"commit", "-qm", "init"}), 0);
    writeFile(f, "changed\n");

    GitGutterController c;
    QSignalSpy head(&c, &GitGutterController::headReady);
    QSignalSpy blame(&c, &GitGutterController::blameReady);
    c.fetchHead(f);
    c.fetchBlame(f);
    ASSERT_TRUE(head.wait(10000));
    EXPECT_EQ(c.headText(f), "committed\n");
    ASSERT_TRUE(blame.count() > 0 || blame.wait(10000));
}
