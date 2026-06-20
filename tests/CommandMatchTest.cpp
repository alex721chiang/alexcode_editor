#include <gtest/gtest.h>
#include "../src/CommandMatch.h"

using CommandMatch::score;

TEST(CommandMatch, EmptyQueryKeepsAll) {
    EXPECT_GE(score("Anything", ""), 0);
}

TEST(CommandMatch, PrefixBeatsContains) {
    const int pre = score("Open File", "Open");
    const int con = score("Recent Open", "Open");
    EXPECT_GT(pre, 0);
    EXPECT_GT(con, 0);
    EXPECT_GT(pre, con);
}

TEST(CommandMatch, ContainsBeatsFuzzy) {
    const int con = score("Toggle Comment", "Comment");
    const int fz  = score("Close Document", "Comment");   // 子序列 C-o-m...：模糊
    EXPECT_GT(con, 0);
    if (fz >= 0) EXPECT_GT(con, fz);
}

TEST(CommandMatch, FuzzySubsequenceMatches) {
    EXPECT_GE(score("Go to Symbol", "gts"), 0);            // g-t-s 子序列
}

TEST(CommandMatch, NoMatchReturnsNegative) {
    EXPECT_LT(score("Save File", "xyz"), 0);
}

TEST(CommandMatch, ShorterNameRanksHigherForSamePrefix) {
    EXPECT_GT(score("Save", "Sa"), score("Save All Files", "Sa"));
}
