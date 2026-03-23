#include <gtest/gtest.h>
#include <chrono>
#include <QStringList>
#include "../src/FilterEngine.h"

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
