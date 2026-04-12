#include <gtest/gtest.h>
#include "../src/FilterEngine.h"

TEST(FilterEngineTest, FilterOrLogic) {
    FilterEngine engine;
    engine.setKeywords({"error", "fail"});
    engine.setLogic(FilterLogic::OR);
    
    EXPECT_TRUE(engine.matchLine("This is an error log"));
    EXPECT_TRUE(engine.matchLine("Process failed successfully"));
    EXPECT_FALSE(engine.matchLine("Everything is fine"));
}

TEST(FilterEngineTest, FilterAndLogic) {
    FilterEngine engine;
    engine.setKeywords({"error", "timeout"});
    engine.setLogic(FilterLogic::AND);
    
    EXPECT_TRUE(engine.matchLine("error: connection timeout occurred"));
    EXPECT_FALSE(engine.matchLine("This is an error log"));
}
