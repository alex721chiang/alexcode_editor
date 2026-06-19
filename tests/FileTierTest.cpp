#include <gtest/gtest.h>
#include "../src/FileTier.h"

using FileTier::Level;
using FileTier::forSizeMB;

TEST(FileTier, SmallFileIsFull) {
    EXPECT_EQ(forSizeMB(0.5, 2, 10), Level::Full);
    EXPECT_EQ(forSizeMB(2.0, 2, 10), Level::Full);     // exactly at threshold = still full
}

TEST(FileTier, MediumFileDisablesAssist) {
    EXPECT_EQ(forSizeMB(2.1, 2, 10), Level::AssistOff);
    EXPECT_EQ(forSizeMB(5.0, 2, 10), Level::AssistOff);
    EXPECT_EQ(forSizeMB(10.0, 2, 10), Level::AssistOff);
}

TEST(FileTier, LargeFileDisablesHighlight) {
    EXPECT_EQ(forSizeMB(10.1, 2, 10), Level::HighlightOff);
    EXPECT_EQ(forSizeMB(60.0, 2, 10), Level::HighlightOff);
}

TEST(FileTier, RespectsCustomThresholds) {
    EXPECT_EQ(forSizeMB(3.0, 5, 20), Level::Full);
    EXPECT_EQ(forSizeMB(8.0, 5, 20), Level::AssistOff);
    EXPECT_EQ(forSizeMB(25.0, 5, 20), Level::HighlightOff);
}
