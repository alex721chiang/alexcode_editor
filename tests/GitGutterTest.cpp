#include <gtest/gtest.h>
#include "../src/GitGutter.h"

using namespace GitGutter;

static QStringList L(std::initializer_list<const char*> lines) {
    QStringList out;
    for (const char* s : lines) out << QString::fromUtf8(s);
    return out;
}

TEST(GitGutterTest, NoChangesReturnsEmpty) {
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a", "b", "c"}));
    EXPECT_TRUE(s.isEmpty());
}

TEST(GitGutterTest, PureAddition) {
    // 在 b 後插入 x, y
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a", "b", "x", "y", "c"}));
    EXPECT_EQ(s.value(2), Added);
    EXPECT_EQ(s.value(3), Added);
    EXPECT_EQ(s.size(), 2);
}

TEST(GitGutterTest, PureDeletionMarksLineBelow) {
    // 刪掉 b：c（新版第 1 行）上方有刪除
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a", "c"}));
    EXPECT_EQ(s.value(1), DeletedAbove);
    EXPECT_EQ(s.size(), 1);
}

TEST(GitGutterTest, DeletionAtEndMarksLastLine) {
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a"}));
    EXPECT_TRUE(s.value(0) & DeletedAbove);
    EXPECT_EQ(s.size(), 1);
}

TEST(GitGutterTest, ModificationPairsOldAndNew) {
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a", "B!", "c"}));
    EXPECT_EQ(s.value(1), Modified);
    EXPECT_EQ(s.size(), 1);
}

TEST(GitGutterTest, ReplaceOneLineWithTwoIsModifiedPlusAdded) {
    const auto s = diffLineStates(L({"a", "b", "c"}), L({"a", "x", "y", "c"}));
    EXPECT_EQ(s.value(1), Modified);
    EXPECT_EQ(s.value(2), Added);
    EXPECT_EQ(s.size(), 2);
}

TEST(GitGutterTest, ReplaceTwoLinesWithOneIsModifiedPlusDeletedAbove) {
    // b,c → x：x 標 Modified；淨刪 1 行 → 下一行 d 標 DeletedAbove
    const auto s = diffLineStates(L({"a", "b", "c", "d"}), L({"a", "x", "d"}));
    EXPECT_TRUE(s.value(1) & Modified);
    EXPECT_TRUE(s.value(2) & DeletedAbove);
}

TEST(GitGutterTest, MultipleSeparatedChangeRuns) {
    const auto s = diffLineStates(L({"a", "b", "c", "d", "e"}),
                                  L({"a", "B", "c", "d", "E"}));
    EXPECT_EQ(s.value(1), Modified);
    EXPECT_EQ(s.value(4), Modified);
    EXPECT_EQ(s.size(), 2);
}

TEST(GitGutterTest, EmptyOldMeansAllAdded) {
    const auto s = diffLineStates(QStringList{}, L({"a", "b"}));
    EXPECT_EQ(s.value(0), Added);
    EXPECT_EQ(s.value(1), Added);
}

TEST(GitGutterTest, OversizedMiddleFallsBackToCoarseModified) {
    QStringList oldL, newL;
    for (int i = 0; i < 50; ++i) oldL << QString("old%1").arg(i);
    for (int i = 0; i < 50; ++i) newL << QString("new%1").arg(i);
    const auto s = diffLineStates(oldL, newL, /*maxMiddle=*/10);   // 強制走粗略路徑
    EXPECT_EQ(s.size(), 50);
    for (int i = 0; i < 50; ++i) EXPECT_TRUE(s.value(i) & Modified);
}

TEST(GitGutterTest, PrefixSuffixTrimKeepsLineNumbersCorrect) {
    QStringList oldL, newL;
    for (int i = 0; i < 1000; ++i) { oldL << QString::number(i); newL << QString::number(i); }
    newL[500] = "changed";
    const auto s = diffLineStates(oldL, newL);
    EXPECT_EQ(s.size(), 1);
    EXPECT_EQ(s.value(500), Modified);
}
