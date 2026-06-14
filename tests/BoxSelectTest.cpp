#include <gtest/gtest.h>
#include "../src/BoxSelect.h"

using BoxSelect::Range;

TEST(BoxSelect, NormalRectangle) {
    QVector<int> len{10, 10, 10};
    auto r = BoxSelect::compute(0, 2, 2, 6, len);
    ASSERT_EQ(r.size(), 3);
    EXPECT_EQ(r[0], (Range{0, 2, 6}));
    EXPECT_EQ(r[1], (Range{1, 2, 6}));
    EXPECT_EQ(r[2], (Range{2, 2, 6}));
}

TEST(BoxSelect, ShortLineClamped) {
    QVector<int> len{10, 3, 10};          // 第 1 行只有 3 字
    auto r = BoxSelect::compute(0, 2, 2, 6, len);
    EXPECT_EQ(r[1], (Range{1, 2, 3}));    // end clamp 到行長 3
}

TEST(BoxSelect, VeryShortLineCollapsesToPoint) {
    QVector<int> len{10, 1, 10};          // 第 1 行只有 1 字，欄位 [2,6) 全超出
    auto r = BoxSelect::compute(0, 2, 2, 6, len);
    EXPECT_EQ(r[1], (Range{1, 1, 1}));    // start/end 都 clamp 到 1 → 零寬
}

TEST(BoxSelect, ZeroWidthInsertionColumn) {
    QVector<int> len{10, 10, 10};
    auto r = BoxSelect::compute(0, 4, 2, 4, len);   // 同欄 → 每行零寬插入點
    for (const auto& x : r) EXPECT_EQ(x.start, x.end);
    EXPECT_EQ(r.size(), 3);
}

TEST(BoxSelect, ReverseDragNormalised) {
    QVector<int> len{10, 10, 10};
    auto fwd = BoxSelect::compute(0, 2, 2, 6, len);
    auto rev = BoxSelect::compute(2, 6, 0, 2, len);  // 反向拖曳應得相同結果
    EXPECT_EQ(fwd, rev);
}

TEST(BoxSelect, SingleLineDegenerate) {
    QVector<int> len{20};
    auto r = BoxSelect::compute(0, 3, 0, 8, len);
    ASSERT_EQ(r.size(), 1);
    EXPECT_EQ(r[0], (Range{0, 3, 8}));
}
