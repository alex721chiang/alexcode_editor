#include <gtest/gtest.h>
#include "../src/TerminalSelection.h"

using TermSelection::extractText;

static QStringList grid() { return QStringList{ "hello world", "second line", "third row x" }; }

TEST(TermSelection, SingleRow) {
    EXPECT_EQ(extractText(grid(), 0, 0, 0, 5), "hello");
    EXPECT_EQ(extractText(grid(), 0, 6, 0, 11), "world");
}

TEST(TermSelection, MultiRow) {
    // 從第0行第6欄到第2行第5欄
    EXPECT_EQ(extractText(grid(), 0, 6, 2, 5), "world\nsecond line\nthird");
}

TEST(TermSelection, ReversedSelectionNormalised) {
    EXPECT_EQ(extractText(grid(), 2, 5, 0, 6), "world\nsecond line\nthird");  // 反向拖曳同結果
}

TEST(TermSelection, ColumnsBeyondLineClamp) {
    EXPECT_EQ(extractText(grid(), 0, 0, 0, 999), "hello world");   // 超界 clamp 到行尾
    EXPECT_EQ(extractText(grid(), 0, 6, 1, 999), "world\nsecond line");
}

TEST(TermSelection, EmptyAndDegenerate) {
    EXPECT_EQ(extractText(QStringList(), 0, 0, 2, 2), "");
    EXPECT_EQ(extractText(grid(), 1, 3, 1, 3), "");               // 零寬選取
}
