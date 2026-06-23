#include <gtest/gtest.h>
#include "../src/ProjectSymbolIndex.h"

static TsSymbols::Symbol sym(const QString& name, const QString& kind, int line) {
    TsSymbols::Symbol s; s.name = name; s.kind = kind; s.line = line; return s;
}

static ProjectSymbolIndex makeIndex() {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/proj/a.cpp", { sym("Foo", "class", 1), sym("doWork", "function", 5) });
    idx.setFileSymbols("/proj/b.cpp", { sym("Bar", "class", 1), sym("doWork", "function", 9),
                                        sym("helper", "function", 20) });
    return idx;
}

TEST(ProjectSymbolIndex, Counts) {
    auto idx = makeIndex();
    EXPECT_EQ(idx.fileCount(), 2);
    EXPECT_EQ(idx.symbolCount(), 5);
}

TEST(ProjectSymbolIndex, ExactFindsAcrossFiles) {
    auto idx = makeIndex();
    auto hits = idx.exact("doWork");          // 兩個檔都有 doWork
    EXPECT_EQ(hits.size(), 2);
    QStringList files; for (auto& e : hits) files << e.file;
    EXPECT_TRUE(files.contains("/proj/a.cpp"));
    EXPECT_TRUE(files.contains("/proj/b.cpp"));
}

TEST(ProjectSymbolIndex, ExactNoMatch) {
    EXPECT_TRUE(makeIndex().exact("nope").isEmpty());
}

TEST(ProjectSymbolIndex, SearchFuzzy) {
    auto idx = makeIndex();
    auto hits = idx.search("dw");             // d-w 子序列 → doWork
    ASSERT_FALSE(hits.isEmpty());
    EXPECT_EQ(hits.first().name, "doWork");
}

TEST(ProjectSymbolIndex, SearchPrefixRanksFirst) {
    auto idx = makeIndex();
    auto hits = idx.search("he");
    ASSERT_FALSE(hits.isEmpty());
    EXPECT_EQ(hits.first().name, "helper");
}

TEST(ProjectSymbolIndex, SetFileSymbolsReplaces) {
    auto idx = makeIndex();
    idx.setFileSymbols("/proj/a.cpp", { sym("NewOnly", "class", 1) });   // 取代 a.cpp 的符號
    EXPECT_EQ(idx.symbolCount(), 4);          // 5 - 2 + 1
    EXPECT_TRUE(idx.exact("Foo").isEmpty());
    EXPECT_EQ(idx.exact("NewOnly").size(), 1);
}

TEST(ProjectSymbolIndex, RemoveFile) {
    auto idx = makeIndex();
    idx.removeFile("/proj/b.cpp");
    EXPECT_EQ(idx.fileCount(), 1);
    EXPECT_EQ(idx.exact("doWork").size(), 1);
}

TEST(ProjectSymbolIndex, EmptySymbolsClearsFile) {
    auto idx = makeIndex();
    idx.setFileSymbols("/proj/a.cpp", {});    // 空 → 視同移除該檔
    EXPECT_EQ(idx.fileCount(), 1);
}
