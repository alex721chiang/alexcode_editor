#include <gtest/gtest.h>
#include "LocalCompletion.h"

using LocalCompletion::suggest;

TEST(LocalCompletion, PrefixMatchesRankAboveFuzzy) {
    // "counter" 前綴符合 "co"；"colorPick" 僅模糊符合
    const QString text = "counter counter colorPick";
    const auto r = suggest(text, 0, "co", 10);
    ASSERT_FALSE(r.isEmpty());
    EXPECT_EQ(r.first(), "counter");
    EXPECT_TRUE(r.contains("colorPick"));
    EXPECT_LT(r.indexOf("counter"), r.indexOf("colorPick"));
}

TEST(LocalCompletion, HigherFrequencyRanksHigher) {
    // 兩者皆前綴符合；apple 出現 3 次、apply 1 次
    const QString text = "apple apple apple apply";
    const auto r = suggest(text, 0, "app", 10);
    EXPECT_LT(r.indexOf("apple"), r.indexOf("apply"));
}

TEST(LocalCompletion, CloserOccurrenceRanksHigher) {
    // 同頻率（各 1 次）；nearVar 靠近游標、farVar 在最前面（遠離游標）
    const QString text = "farVar ............................................ nearVar";
    const int cursor = text.size();      // 游標在尾端，靠近 nearVar
    const auto r = suggest(text, cursor, "", 10);
    EXPECT_LT(r.indexOf("nearVar"), r.indexOf("farVar"));
}

TEST(LocalCompletion, DeduplicatesIdentifiers) {
    const QString text = "value value value";
    const auto r = suggest(text, 0, "val", 10);
    EXPECT_EQ(r.count("value"), 1);
}

TEST(LocalCompletion, EmptyPrefixReturnsRankedIdentifiers) {
    const auto r = suggest("alpha beta beta gamma", 0, "", 10);
    EXPECT_FALSE(r.isEmpty());
    EXPECT_EQ(r.first(), "beta");        // 出現兩次 → 最前
}

TEST(LocalCompletion, ExcludesExactPrefixToken) {
    // 前綴本身正好是一個識別字，不應建議補成自己
    const QString text = "len length lengthy";
    const auto r = suggest(text, 0, "len", 10);
    EXPECT_FALSE(r.contains("len"));
    EXPECT_TRUE(r.contains("length"));
    EXPECT_TRUE(r.contains("lengthy"));
}

TEST(LocalCompletion, RespectsMaxItems) {
    const auto r = suggest("aa1 aa2 aa3 aa4 aa5", 0, "aa", 3);
    EXPECT_EQ(r.size(), 3);
}
