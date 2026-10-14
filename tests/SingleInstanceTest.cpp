// 單一實例：雙擊檔案時交給已開啟的視窗開成新分頁，而不是再開一個視窗。
// 次實例的 sendToPrimary() 是阻塞式（真實情境中它在自己的行程、事件迴圈尚未啟動），
// 測試中以背景執行緒呼叫，主執行緒負責處理主實例的事件。
#include <gtest/gtest.h>
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QLockFile>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QThread>
#include <QUuid>
#include <atomic>
#include <thread>
#include <vector>
#include "SingleInstance.h"
#include "MainWindow.h"
#include "Portable.h"
#include <QStandardPaths>
#include <QTabWidget>
#include <QFile>
#include <QDir>

namespace {
QString uniqueKey() { return "AlexCodeTest-" + QUuid::createUuid().toString(QUuid::Id128).left(12); }

template <class F> bool pumpUntil(F cond, int timeoutMs = 5000) {
    QElapsedTimer t;
    t.start();
    while (!cond()) {
        if (t.elapsed() > timeoutMs) return false;
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        QThread::msleep(2);
    }
    return true;
}

// 在背景執行緒以「次實例」身分傳送，主執行緒持續處理事件直到完成
bool sendFromAnotherThread(const QString& key, const QString& lockDir, const QStringList& files, int timeoutMs = 3000) {
    std::atomic<int> result{-1};
    std::thread t([&] {
        SingleInstance secondary(key, lockDir);
        result = secondary.sendToPrimary(files, timeoutMs) ? 1 : 0;
    });
    pumpUntil([&] { return result.load() != -1; }, timeoutMs + 3000);
    t.join();
    return result.load() == 1;
}
}

TEST(SingleInstanceTest, OnlyOnePrimaryPerKey) {
    QTemporaryDir dir;
    const QString key = uniqueKey();
    SingleInstance a(key, dir.path());
    SingleInstance b(key, dir.path());
    EXPECT_TRUE(a.tryBecomePrimary());
    EXPECT_FALSE(b.tryBecomePrimary());
}

TEST(SingleInstanceTest, SecondaryDeliversFilesToPrimary) {
    QTemporaryDir dir;
    const QString key = uniqueKey();
    SingleInstance primary(key, dir.path());
    ASSERT_TRUE(primary.tryBecomePrimary());
    QSignalSpy spy(&primary, &SingleInstance::filesReceived);

    const QStringList files{"/work/a.cpp", "C:/專案/中文 檔名.txt"};   // 非 ASCII 與空白須原樣送達
    EXPECT_TRUE(sendFromAnotherThread(key, dir.path(), files));
    ASSERT_TRUE(pumpUntil([&] { return spy.count() == 1; }));
    EXPECT_EQ(spy.at(0).at(0).toStringList(), files);
}

// 檔案總管多選後按 Enter：多個行程幾乎同時啟動，每個都要送達同一個主實例
TEST(SingleInstanceTest, ConcurrentSecondariesAllDelivered) {
    QTemporaryDir dir;
    const QString key = uniqueKey();
    SingleInstance primary(key, dir.path());
    ASSERT_TRUE(primary.tryBecomePrimary());
    QStringList got;
    QObject::connect(&primary, &SingleInstance::filesReceived, [&](const QStringList& f) { got += f; });

    constexpr int n = 8;
    std::atomic<int> ok{0}, done{0};
    std::vector<std::thread> threads;
    for (int i = 0; i < n; ++i) {
        threads.emplace_back([&, i] {
            SingleInstance s(key, dir.path());
            if (s.sendToPrimary({QString("/f%1.txt").arg(i)}, 5000)) ++ok;
            ++done;
        });
    }
    pumpUntil([&] { return done.load() == n; }, 15000);
    for (auto& t : threads) t.join();
    EXPECT_EQ(ok.load(), n);
    pumpUntil([&] { return got.size() == n; });
    got.sort();
    QStringList expected;
    for (int i = 0; i < n; ++i) expected << QString("/f%1.txt").arg(i);
    expected.sort();
    EXPECT_EQ(got, expected);
}

// 主實例存在（鎖被持有）但沒有回應：次實例逾時回報失敗，呼叫端改為自行開窗，檔案不會遺失
TEST(SingleInstanceTest, UnresponsivePrimaryMakesSendFail) {
    QTemporaryDir dir;
    const QString key = uniqueKey();
    QLockFile heldElsewhere(SingleInstance::lockPathFor(key, dir.path()));
    ASSERT_TRUE(heldElsewhere.tryLock(0));             // 有人持有鎖，但沒有在聽

    QElapsedTimer t;
    t.start();
    EXPECT_FALSE(sendFromAnotherThread(key, dir.path(), {"/x.txt"}, 600));
    EXPECT_LT(t.elapsed(), 4000);
}

// 主實例結束後鎖與通道都釋放，下一次啟動可成為新的主實例並收得到檔案
TEST(SingleInstanceTest, NewPrimaryAfterPreviousExits) {
    QTemporaryDir dir;
    const QString key = uniqueKey();
    {
        SingleInstance first(key, dir.path());
        ASSERT_TRUE(first.tryBecomePrimary());
    }
    SingleInstance second(key, dir.path());
    ASSERT_TRUE(second.tryBecomePrimary());
    QSignalSpy spy(&second, &SingleInstance::filesReceived);
    EXPECT_TRUE(sendFromAnotherThread(key, dir.path(), {"/y.txt"}));
    EXPECT_TRUE(pumpUntil([&] { return spy.count() == 1; }));
}

// 不同 key（如可攜版與安裝版各自的資料目錄）互不干擾
TEST(SingleInstanceTest, DifferentKeysAreIndependent) {
    QTemporaryDir dir;
    SingleInstance a(uniqueKey(), dir.path());
    SingleInstance b(uniqueKey(), dir.path());
    EXPECT_TRUE(a.tryBecomePrimary());
    EXPECT_TRUE(b.tryBecomePrimary());
}

// 主實例收到檔案：各開成分頁；已開啟的不重複開；視窗由最小化恢復
TEST(SingleInstanceTest, MainWindowOpensReceivedFilesAsTabs) {
    QStandardPaths::setTestModeEnabled(true);
    QDir(Portable::dataDir()).removeRecursively();
    { AppSettings s; s.setValue("session/restore", false); s.sync(); }
    QTemporaryDir dir;
    auto make = [&](const QString& n, const QByteArray& d) {
        QFile f(dir.filePath(n)); f.open(QIODevice::WriteOnly); f.write(d); return dir.filePath(n);
    };
    const QString a = make("a.txt", "A\n"), b = make("中文 b.txt", "B\n");
    {
        MainWindow w;
        w.showMinimized();
        auto* tabs = w.findChild<QTabWidget*>();
        ASSERT_TRUE(tabs);
        const int before = tabs->count();
        w.openFilesFromOtherInstance({a, b});
        w.openFilesFromOtherInstance({a});                  // 再次雙擊同一檔：不重複開
        QStringList paths;
        for (int i = 0; i < tabs->count(); ++i) paths << tabs->widget(i)->property("filePath").toString();
        EXPECT_EQ(paths.count(a), 1);
        EXPECT_EQ(paths.count(b), 1);
        EXPECT_EQ(tabs->count(), before + 2);
        EXPECT_EQ(tabs->currentWidget()->property("filePath").toString(), a);   // 切到剛要求的檔案
        EXPECT_FALSE(w.isMinimized());
    }
    QStandardPaths::setTestModeEnabled(false);
}
