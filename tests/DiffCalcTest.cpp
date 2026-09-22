#include <gtest/gtest.h>
#include "DiffCalc.h"

using DiffCalc::Row;
using DiffCalc::align;
using DiffCalc::changeCount;

TEST(DiffCalc, IdenticalTextsAllSame) {
    const auto rows = align({"a", "b"}, {"a", "b"});
    ASSERT_EQ(rows.size(), 2);
    EXPECT_EQ(changeCount(rows), 0);
    EXPECT_EQ(rows[0].left, 0);  EXPECT_EQ(rows[0].right, 0);
    EXPECT_EQ(rows[1].type, DiffCalc::Same);
}

TEST(DiffCalc, PureAddition) {
    const auto rows = align({"a", "c"}, {"a", "b", "c"});
    ASSERT_EQ(rows.size(), 3);
    EXPECT_EQ(rows[1].type, DiffCalc::Added);
    EXPECT_EQ(rows[1].left, -1);          // 左側填充
    EXPECT_EQ(rows[1].right, 1);
    EXPECT_EQ(changeCount(rows), 1);
}

TEST(DiffCalc, PureRemoval) {
    const auto rows = align({"a", "b", "c"}, {"a", "c"});
    ASSERT_EQ(rows.size(), 3);
    EXPECT_EQ(rows[1].type, DiffCalc::Removed);
    EXPECT_EQ(rows[1].left, 1);
    EXPECT_EQ(rows[1].right, -1);         // 右側填充
}

TEST(DiffCalc, ChangedLineBecomesModifiedPair) {
    const auto rows = align({"a", "OLD", "c"}, {"a", "NEW", "c"});
    ASSERT_EQ(rows.size(), 3);
    EXPECT_EQ(rows[1].type, DiffCalc::Modified);   // 一刪一增配成同一列
    EXPECT_EQ(rows[1].left, 1);
    EXPECT_EQ(rows[1].right, 1);
    EXPECT_EQ(changeCount(rows), 1);
}

TEST(DiffCalc, UnbalancedBlockPairsThenFills) {
    // 兩行換成一行：一列 Modified + 一列 Removed
    const auto rows = align({"a", "x", "y", "c"}, {"a", "z", "c"});
    ASSERT_EQ(rows.size(), 4);
    EXPECT_EQ(rows[1].type, DiffCalc::Modified);
    EXPECT_EQ(rows[2].type, DiffCalc::Removed);
    EXPECT_EQ(rows[2].right, -1);
}

TEST(DiffCalc, EmptyVersusContent) {
    const auto rows = align({""}, {"a", "b"});
    // "" 與 a/b 皆不同 → "" 配 a 成 Modified、b 為 Added
    ASSERT_EQ(rows.size(), 2);
    EXPECT_EQ(rows[0].type, DiffCalc::Modified);
    EXPECT_EQ(rows[1].type, DiffCalc::Added);
}
