#include <gtest/gtest.h>
#include "TextTools.h"

using namespace TextTools;

TEST(TextTools, SortLinesAscendingDescending) {
    EXPECT_EQ(sortLines("b\na\nc"), "a\nb\nc");
    EXPECT_EQ(sortLines("b\na\nc", true), "c\nb\na");
}

TEST(TextTools, RemoveDuplicateAndBlankLines) {
    EXPECT_EQ(removeDuplicateLines("a\nb\na\nc\nb"), "a\nb\nc");
    EXPECT_EQ(removeBlankLines("a\n\n  \nb"), "a\nb");
}

TEST(TextTools, ReverseAndTrim) {
    EXPECT_EQ(reverseLines("a\nb\nc"), "c\nb\na");
    EXPECT_EQ(trimTrailingWhitespace("a  \nb\t\nc"), "a\nb\nc");
}

TEST(TextTools, FullHalfWidthRoundTrip) {
    const QString full = QStringLiteral("ＡＢＣ１２３");   // 全形
    EXPECT_EQ(toHalfWidth(full), "ABC123");
    EXPECT_EQ(toFullWidth("ABC123"), full);
    EXPECT_EQ(toHalfWidth(QString(QChar(0x3000))), " ");   // 全形空白 → 半形空白
    EXPECT_EQ(toFullWidth(" "), QString(QChar(0x3000)));
}

TEST(TextTools, UnicodeEscapeUnescape) {
    const QString zh = QStringLiteral("Hi 中文");
    const QString esc = unicodeEscape(zh);
    EXPECT_TRUE(esc.startsWith("Hi "));
    EXPECT_TRUE(esc.contains("\\u"));
    EXPECT_EQ(unicodeUnescape(esc), zh);                   // 往返一致
    EXPECT_EQ(unicodeEscape("ascii"), "ascii");            // 純 ASCII 不變
}

TEST(TextTools, ParseInteger) {
    qlonglong v = 0;
    EXPECT_TRUE(parseInteger("255", &v));     EXPECT_EQ(v, 255);
    EXPECT_TRUE(parseInteger("0xFF", &v));    EXPECT_EQ(v, 255);
    EXPECT_TRUE(parseInteger("0b1010", &v));  EXPECT_EQ(v, 10);
    EXPECT_TRUE(parseInteger("0755", &v));    EXPECT_EQ(v, 493);   // 八進位
    EXPECT_FALSE(parseInteger("notanumber", &v));
}
