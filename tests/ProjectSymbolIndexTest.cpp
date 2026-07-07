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

TEST(ProjectSymbolIndex, FilesAndMtime) {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/p/a.cpp", { sym("X", "class", 0) }, 42);
    EXPECT_EQ(idx.files(), QStringList{ "/p/a.cpp" });
    EXPECT_EQ(idx.mtimeOf("/p/a.cpp"), 42);
    EXPECT_EQ(idx.mtimeOf("/nope"), -1);
}

TEST(ProjectSymbolIndex, SerializeRoundTrip) {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/p/a.cpp", { sym("Foo", "class", 1), sym("bar", "function", 5) }, 1000);
    idx.setFileSymbols("/p/b.py", { sym("Baz", "class", 2) }, 2000);
    idx.setFileSymbols("/p/empty.cpp", {}, 3000);   // 無符號但記了 mtime
    const QByteArray blob = idx.serialize();

    ProjectSymbolIndex idx2;
    idx2.deserialize(blob);
    EXPECT_EQ(idx2.fileCount(), 2);                 // empty.cpp 無符號 → 不計 fileCount
    EXPECT_EQ(idx2.symbolCount(), 3);
    EXPECT_EQ(idx2.mtimeOf("/p/a.cpp"), 1000);
    EXPECT_EQ(idx2.mtimeOf("/p/empty.cpp"), 3000);  // 無符號檔 mtime 仍保留（避免重解析）
    ASSERT_EQ(idx2.exact("Foo").size(), 1);
    EXPECT_EQ(idx2.exact("Foo").first().kind, "class");
    EXPECT_EQ(idx2.exact("bar").first().line, 5);
}

// --- 記憶體內索引（Phase 1）：exact()/search() 改走索引後，行為需與舊版線性掃描完全一致 ---

TEST(ProjectSymbolIndex, ExactAfterRemoveUsesIndex) {
    auto idx = makeIndex();
    idx.removeFile("/proj/a.cpp");
    EXPECT_TRUE(idx.exact("Foo").isEmpty());       // a.cpp 的 Foo 已移除，索引需同步移除
    EXPECT_EQ(idx.exact("doWork").size(), 1);      // b.cpp 的 doWork 仍在
}

TEST(ProjectSymbolIndex, ExactAfterReplaceUsesIndex) {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/p/a.cpp", { sym("Old", "class", 1) });
    idx.setFileSymbols("/p/a.cpp", { sym("New", "class", 1) });   // 取代同一檔的符號
    EXPECT_TRUE(idx.exact("Old").isEmpty());       // 舊索引項需一併清掉，否則會殘留幽靈結果
    EXPECT_EQ(idx.exact("New").size(), 1);
}

TEST(ProjectSymbolIndex, SearchFallbackFindsSubstringMatch) {
    auto idx = makeIndex();
    // "or" 不是任何符號名稱的前綴，前綴快速路徑找不到 → 必須退回全表比對子字串
    auto hits = idx.search("or");
    ASSERT_FALSE(hits.isEmpty());
    for (auto& e : hits) EXPECT_EQ(e.name, "doWork");
}

TEST(ProjectSymbolIndex, SearchPrefixFastPathRespectsLimit) {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/p/a.cpp", { sym("getAlpha", "function", 1), sym("getBeta", "function", 2),
                                     sym("getGamma", "function", 3) });
    // 三個符號都以 get 開頭，命中數(3) >= limit(2) → 觸發快速路徑，只回傳 2 筆
    auto hits = idx.search("get", 2);
    EXPECT_EQ(hits.size(), 2);
}

TEST(ProjectSymbolIndex, SearchPrefixFastPathMatchesLinearScanOrder) {
    ProjectSymbolIndex idx;
    idx.setFileSymbols("/p/a.cpp", { sym("getX", "function", 1), sym("getLongerName", "function", 2) });
    // 前綴分數 = 400 - 長度，短名稱分數較高，應排在前面（與舊版線性掃描排序規則一致）
    auto hits = idx.search("get", 10);
    ASSERT_EQ(hits.size(), 2);
    EXPECT_EQ(hits.first().name, "getX");
}
