#include <gtest/gtest.h>
#include <chrono>
#include <QStringList>
#include "../src/FilterEngine.h"
#include "../src/ProjectSymbolIndex.h"

TEST(PerformanceTest, MatchLine100kLines) {
    FilterEngine engine;
    QStringList keywords = {"performance", "test", "keyword"};
    engine.setKeywords(keywords);
    engine.setLogic(FilterLogic::OR);

    // Generate 100,000 lines
    QString line = "This is a line that might or might not contain the performance keyword to test the filter engine.";
    
    int numLines = 100000;
    
    auto start = std::chrono::high_resolution_clock::now();

    for (int i = 0; i < numLines; ++i) {
        engine.matchLine(line);
    }

    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count();

    // Assert that execution time is under 500ms
    EXPECT_LT(duration, 500) << "Performance test failed, took " << duration << " ms for 100,000 lines";
}

// api_index_search_plan Phase 2：5 萬符號規模下驗證記憶體索引的查詢延遲。
// 目標 <5ms/次；斷言用寬鬆上限（CI 共享機器波動大），實測值印出來供人工追蹤。
namespace {

ProjectSymbolIndex makeBigIndex(int files, int symbolsPerFile) {
    ProjectSymbolIndex idx;
    // 名稱刻意多樣化（前綴 get/set/on/handle + 序號），模擬真實 API 命名分布
    static const char* prefixes[] = {"get", "set", "on", "handle", "compute", "render"};
    for (int f = 0; f < files; ++f) {
        QVector<TsSymbols::Symbol> syms;
        syms.reserve(symbolsPerFile);
        for (int s = 0; s < symbolsPerFile; ++s) {
            const int n = f * symbolsPerFile + s;
            TsSymbols::Symbol sym;
            sym.name = QStringLiteral("%1Item%2").arg(prefixes[n % 6]).arg(n);
            sym.kind = QStringLiteral("function");
            sym.line = s;
            syms.append(sym);
        }
        idx.setFileSymbols(QStringLiteral("/proj/file%1.cpp").arg(f), syms);
    }
    return idx;
}

} // namespace

TEST(PerformanceTest, SymbolSearch50kPrefixFastPath) {
    ProjectSymbolIndex idx = makeBigIndex(500, 100);   // 50,000 符號

    // 暖身一次（首次呼叫可能觸及冷快取）
    (void)idx.search(QStringLiteral("getItem1"), 50);

    constexpr int kRuns = 100;
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < kRuns; ++i) {
        const auto hits = idx.search(QStringLiteral("getItem1"), 50);   // 前綴命中 → 快速路徑
        ASSERT_EQ(hits.size(), 50);
    }
    auto end = std::chrono::high_resolution_clock::now();
    const double avgUs =
        std::chrono::duration_cast<std::chrono::microseconds>(end - start).count() / double(kRuns);
    std::cout << "[perf] search() prefix fast path @50k symbols: " << avgUs << " us/query\n";
    EXPECT_LT(avgUs, 5000.0) << "prefix search should be well under 5ms per query";
}

TEST(PerformanceTest, SymbolExact50k) {
    ProjectSymbolIndex idx = makeBigIndex(500, 100);

    constexpr int kRuns = 1000;
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < kRuns; ++i) {
        const auto hits = idx.exact(QStringLiteral("setItem49999"));   // 49999 % 6 = 1 → "set" 前綴
        ASSERT_EQ(hits.size(), 1);
    }
    auto end = std::chrono::high_resolution_clock::now();
    const double avgUs =
        std::chrono::duration_cast<std::chrono::microseconds>(end - start).count() / double(kRuns);
    std::cout << "[perf] exact() @50k symbols: " << avgUs << " us/query\n";
    EXPECT_LT(avgUs, 1000.0) << "exact lookup should be O(1)-ish, far under 1ms";
}

TEST(PerformanceTest, SymbolSearchFallbackStillBounded) {
    ProjectSymbolIndex idx = makeBigIndex(500, 100);

    // 非前綴 query → 走全表回退路徑；驗證 50k 規模仍在可互動範圍（單次 <100ms）
    auto start = std::chrono::high_resolution_clock::now();
    const auto hits = idx.search(QStringLiteral("tem123"), 50);   // 子字串命中
    auto end = std::chrono::high_resolution_clock::now();
    const double ms =
        std::chrono::duration_cast<std::chrono::microseconds>(end - start).count() / 1000.0;
    std::cout << "[perf] search() full-scan fallback @50k symbols: " << ms << " ms\n";
    EXPECT_FALSE(hits.isEmpty());
    EXPECT_LT(ms, 100.0);
}
