#include <gtest/gtest.h>
#include "FilterEngine.h"

TEST(FilterEngine, OrLogic) {
    FilterEngine e;
    e.setKeywords({"error", "warn"});
    e.setLogic(FilterLogic::OR);
    EXPECT_TRUE(e.matchLine("an ERROR occurred"));
    EXPECT_TRUE(e.matchLine("just a warning"));
    EXPECT_FALSE(e.matchLine("all good"));
}

TEST(FilterEngine, AndLogic) {
    FilterEngine e;
    e.setKeywords({"error", "timeout"});
    e.setLogic(FilterLogic::AND);
    EXPECT_TRUE(e.matchLine("error: connection timeout"));
    EXPECT_FALSE(e.matchLine("error: refused"));
}

TEST(FilterEngine, EmptyKeywordsMatchesAll) {
    FilterEngine e;
    e.setKeywords({});
    EXPECT_TRUE(e.matchLine("anything"));
}

TEST(FilterEngine, ParseQueryDoublePipe) {
    QStringList kws = FilterEngine::parseQuery("error || timeout ||  fail ");
    EXPECT_EQ(kws, QStringList({"error", "timeout", "fail"}));
}

TEST(FilterEngine, ParseQueryLegacySinglePipe) {
    QStringList kws = FilterEngine::parseQuery("a|b|c");
    EXPECT_EQ(kws, QStringList({"a", "b", "c"}));
}

TEST(FilterEngine, FuzzyMatch) {
    EXPECT_TRUE(FilterEngine::fuzzyContains("MainWindow.cpp", "mwin"));
    EXPECT_TRUE(FilterEngine::fuzzyContains("FilterEngine", "ftre"));
    EXPECT_FALSE(FilterEngine::fuzzyContains("abc", "abd"));

    FilterEngine e;
    e.setKeywords({"mwin"});
    e.setFuzzy(true);
    EXPECT_TRUE(e.matchLine("class MainWindow : public QMainWindow"));
    e.setFuzzy(false);
    EXPECT_FALSE(e.matchLine("class MainWindow : public QMainWindow"));
}

TEST(FilterEngine, AdvancedSyntax) {
    FilterEngine e;
    e.compile("error && !heartbeat");
    EXPECT_TRUE(e.matchCompiled("ERROR: payment failed"));
    EXPECT_FALSE(e.matchCompiled("error heartbeat check"));
    EXPECT_FALSE(e.matchCompiled("all good"));

    e.compile("error && timeout || fatal");
    EXPECT_TRUE(e.matchCompiled("error: timeout on db"));
    EXPECT_FALSE(e.matchCompiled("error: refused"));
    EXPECT_TRUE(e.matchCompiled("FATAL crash"));

    e.compile("a||b");                      // 舊式仍可用
    EXPECT_TRUE(e.matchCompiled("has b"));

    e.compile("error && !heartbeat");
    EXPECT_EQ(e.positiveKeywords(), QStringList({"error"}));
}

TEST(FilterEngine, RegexPrefix) {
    FilterEngine e;
    e.compile("re:err(or)?s?");
    EXPECT_TRUE(e.matchCompiled("ERRORS detected"));
    EXPECT_TRUE(e.matchCompiled("an err in line 3"));
    EXPECT_FALSE(e.matchCompiled("all good"));

    e.compile("re:timeout=\\d{3,} && !debug");          // regex 與排除組合
    EXPECT_TRUE(e.matchCompiled("conn timeout=1500ms"));
    EXPECT_FALSE(e.matchCompiled("conn timeout=15ms"));
    EXPECT_FALSE(e.matchCompiled("debug: timeout=1500"));

    e.compile("re:^\\[WARN\\] || fatal");               // regex OR 字面詞
    EXPECT_TRUE(e.matchCompiled("[WARN] disk almost full"));
    EXPECT_TRUE(e.matchCompiled("FATAL: oom"));
    EXPECT_FALSE(e.matchCompiled("note: [WARN] quoted mid-line"));

    e.compile("re:err.* && info");
    EXPECT_EQ(e.positiveKeywords(), QStringList({"info"}));   // regex 詞不入多色標示

    e.compile("re:[invalid(");                          // 無效 regex：退回字面比對不崩潰
    EXPECT_FALSE(e.matchCompiled("anything"));
    EXPECT_TRUE(e.matchCompiled("re:[invalid("));
}
