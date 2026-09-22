#include <gtest/gtest.h>
#include "StickyScroll.h"

using TsSymbols::Symbol;
using StickyScroll::headers;

// 模擬：namespace demo (0..30) > class C (2..28) > function f (10..16)
static QVector<Symbol> sample() {
    return {
        { "demo", "namespace", 0, 30, 0 },
        { "C",    "class",     2, 28, 1 },
        { "f",    "function", 10, 16, 2 },
        { "g",    "function", 18, 24, 2 },
    };
}

TEST(StickyScroll, InsideFunctionShowsFullChain) {
    auto h = headers(sample(), 13);     // 第 13 行在 f 內
    ASSERT_EQ(h.size(), 3);
    EXPECT_EQ(h[0].name, "demo");
    EXPECT_EQ(h[1].name, "C");
    EXPECT_EQ(h[2].name, "f");
}

TEST(StickyScroll, BetweenFunctionsShowsOuterOnly) {
    auto h = headers(sample(), 17);     // 第 17 行在 C 內、兩函式之間
    ASSERT_EQ(h.size(), 2);
    EXPECT_EQ(h[0].name, "demo");
    EXPECT_EQ(h[1].name, "C");
}

TEST(StickyScroll, OnHeaderLineNotPinned) {
    // firstVisible 正好在 f 的標頭行(10)：f 不固定（s.line < first 不成立），只固定外層
    auto h = headers(sample(), 10);
    ASSERT_EQ(h.size(), 2);
    EXPECT_EQ(h[1].name, "C");
}

TEST(StickyScroll, TopOfFileNoHeaders) {
    EXPECT_TRUE(headers(sample(), 0).isEmpty());
}

TEST(StickyScroll, SecondFunction) {
    auto h = headers(sample(), 20);     // g 內
    ASSERT_EQ(h.size(), 3);
    EXPECT_EQ(h[2].name, "g");
}

TEST(StickyScroll, MaxDepthKeepsInnermost) {
    auto h = headers(sample(), 13, 2);  // 限 2 層 → 留 C、f
    ASSERT_EQ(h.size(), 2);
    EXPECT_EQ(h[0].name, "C");
    EXPECT_EQ(h[1].name, "f");
}
