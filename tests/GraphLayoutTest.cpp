#include <gtest/gtest.h>
#include "GraphLayout.h"
#include <cmath>

TEST(GraphLayout, EmptyGraph) {
    EXPECT_TRUE(GraphLayout::compute(0, {}, 800, 600).isEmpty());
}

TEST(GraphLayout, SingleNodeCentered) {
    auto pos = GraphLayout::compute(1, {}, 800, 600);
    ASSERT_EQ(pos.size(), 1);
    EXPECT_DOUBLE_EQ(pos[0].x(), 400.0);
    EXPECT_DOUBLE_EQ(pos[0].y(), 300.0);
}

TEST(GraphLayout, AllPositionsWithinBounds) {
    QVector<QPair<int, int>> edges{{0, 1}, {1, 2}, {2, 3}, {3, 0}, {0, 2}};
    auto pos = GraphLayout::compute(4, edges, 800, 600);
    ASSERT_EQ(pos.size(), 4);
    for (const QPointF& p : pos) {
        EXPECT_GE(p.x(), 0.0);
        EXPECT_LE(p.x(), 800.0);
        EXPECT_GE(p.y(), 0.0);
        EXPECT_LE(p.y(), 600.0);
    }
}

TEST(GraphLayout, Deterministic) {
    QVector<QPair<int, int>> edges{{0, 1}, {1, 2}};
    auto a = GraphLayout::compute(3, edges, 500, 500);
    auto b = GraphLayout::compute(3, edges, 500, 500);
    ASSERT_EQ(a.size(), b.size());
    for (int i = 0; i < a.size(); ++i) {
        EXPECT_DOUBLE_EQ(a[i].x(), b[i].x());
        EXPECT_DOUBLE_EQ(a[i].y(), b[i].y());
    }
}

TEST(GraphLayout, ConnectedNodesCloserThanUnconnected) {
    // 0-1 connected; 2 isolated. After layout, 0 and 1 should be closer
    // to each other than 0 is to the far-pushed isolated node on average.
    QVector<QPair<int, int>> edges{{0, 1}};
    auto pos = GraphLayout::compute(3, edges, 600, 600);
    ASSERT_EQ(pos.size(), 3);
    auto dist = [](QPointF a, QPointF b) { return std::hypot(a.x() - b.x(), a.y() - b.y()); };
    const double d01 = dist(pos[0], pos[1]);
    const double d02 = dist(pos[0], pos[2]);
    const double d12 = dist(pos[1], pos[2]);
    EXPECT_LT(d01, d02);
    EXPECT_LT(d01, d12);
}

TEST(GraphLayout, IgnoresOutOfRangeEdges) {
    // Should not crash on invalid indices.
    QVector<QPair<int, int>> edges{{0, 99}, {-1, 1}, {1, 1}};
    auto pos = GraphLayout::compute(2, edges, 400, 400);
    EXPECT_EQ(pos.size(), 2);
}
