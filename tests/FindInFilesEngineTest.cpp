#include <gtest/gtest.h>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
#include <atomic>
#include "FindInFilesEngine.h"

namespace {
void write(const QString& p, const QByteArray& d) {
    QDir().mkpath(QFileInfo(p).absolutePath());
    QFile f(p);
    ASSERT_TRUE(f.open(QIODevice::WriteOnly));
    f.write(d);
}
QByteArray read(const QString& p) {
    QFile f(p);
    f.open(QIODevice::ReadOnly);
    return f.readAll();
}
FindInFiles::Stats run(const FindInFiles::Options& o, QVector<FindInFiles::Match>& out,
                       std::atomic<bool>* cancel = nullptr) {
    std::atomic<bool> none{false};
    return FindInFiles::search(o, cancel ? *cancel : none,
                               [&](const QVector<FindInFiles::Match>& b) { out += b; });
}
}

TEST(FindInFilesEngineTest, FindsLinesWithOrQueryAndNameFilters) {
    QTemporaryDir d;
    write(d.filePath("a.cpp"), "int foo;\nint bar;\nnothing\n");
    write(d.filePath("sub/b.h"), "BAR here\n");
    write(d.filePath("c.md"), "foo in markdown\n");                  // 被檔名過濾排除
    QVector<FindInFiles::Match> m;
    const auto st = run({d.path(), {"*.cpp", "*.h"}, "foo || bar"}, m);
    EXPECT_FALSE(st.canceled);
    EXPECT_EQ(st.matches, 3);
    ASSERT_EQ(m.size(), 3);
    QStringList got;
    for (const auto& x : m) got << QFileInfo(x.filePath).fileName() + ":" + QString::number(x.line);
    got.sort();
    EXPECT_EQ(got, (QStringList{"a.cpp:1", "a.cpp:2", "b.h:1"}));
}

TEST(FindInFilesEngineTest, SnippetIsTrimmedAndTruncated) {
    QTemporaryDir d;
    write(d.filePath("x.txt"), "    key " + QByteArray(200, 'z') + "\n");
    QVector<FindInFiles::Match> m;
    run({d.path(), {"*.txt"}, "key"}, m);
    ASSERT_EQ(m.size(), 1);
    EXPECT_TRUE(m[0].snippet.startsWith("key "));
    EXPECT_EQ(m[0].snippet.size(), 100);
    EXPECT_TRUE(m[0].snippet.endsWith("..."));
}

TEST(FindInFilesEngineTest, CancelStopsEarly) {
    QTemporaryDir d;
    for (int i = 0; i < 300; ++i) write(d.filePath(QString("f%1.txt").arg(i)), "hit\n");
    std::atomic<bool> cancel{false};
    int batches = 0;
    const auto st = FindInFiles::search({d.path(), {"*.txt"}, "hit"}, cancel,
        [&](const QVector<FindInFiles::Match>&) { if (++batches == 1) cancel = true; }, 100000, 10);
    EXPECT_TRUE(st.canceled);
    EXPECT_LT(st.filesScanned, 300);
}

TEST(FindInFilesEngineTest, ResultCapTruncates) {
    QTemporaryDir d;
    write(d.filePath("many.txt"), QByteArray("x\n").repeated(50));
    QVector<FindInFiles::Match> m;
    std::atomic<bool> none{false};
    const auto st = FindInFiles::search({d.path(), {"*.txt"}, "x"}, none,
        [&](const QVector<FindInFiles::Match>& b) { m += b; }, 20);
    EXPECT_TRUE(st.truncated);
    EXPECT_EQ(m.size(), 20);
}

TEST(FindInFilesEngineTest, ReplacePlanAndApplyAreLiteralAndCaseInsensitive) {
    QTemporaryDir d;
    write(d.filePath("a.txt"), "a.b A.B axb\n");                    // "." 必須視為字面，不可當萬用字元
    write(d.filePath("b.txt"), "none\n");
    std::atomic<bool> none{false};
    const auto plan = FindInFiles::planReplace({d.path(), {"*.txt"}, "a.b"}, none);
    ASSERT_EQ(plan.size(), 1);
    EXPECT_EQ(plan[0].count, 2);
    const auto res = FindInFiles::applyReplace(plan, "a.b", "Q", none);
    EXPECT_EQ(res.filesWritten, 1);
    EXPECT_EQ(res.replacements, 2);
    EXPECT_EQ(read(d.filePath("a.txt")), "Q Q axb\n");
    EXPECT_EQ(read(d.filePath("b.txt")), "none\n");
}

// 確認後、寫入前檔案又被改：以最新內容為準重新計算，不覆蓋掉新的修改
TEST(FindInFilesEngineTest, ApplyRereadsFilesChangedAfterPlan) {
    QTemporaryDir d;
    write(d.filePath("a.txt"), "old\n");
    std::atomic<bool> none{false};
    const auto plan = FindInFiles::planReplace({d.path(), {"*.txt"}, "old"}, none);
    write(d.filePath("a.txt"), "old\nnew line added later\nold\n");
    const auto res = FindInFiles::applyReplace(plan, "old", "X", none);
    EXPECT_EQ(res.replacements, 2);
    EXPECT_EQ(read(d.filePath("a.txt")), "X\nnew line added later\nX\n");
}
