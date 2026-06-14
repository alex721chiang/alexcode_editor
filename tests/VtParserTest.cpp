#include <gtest/gtest.h>
#include "../src/VtParser.h"

static QByteArray b(const char* s) { return QByteArray(s); }

TEST(VtParser, PlainTextAndCursor) {
    VtParser p(5, 20);
    p.feed(b("hello"));
    EXPECT_EQ(p.lineText(0), "hello");
    EXPECT_EQ(p.cursorRow(), 0);
    EXPECT_EQ(p.cursorCol(), 5);
}

TEST(VtParser, CrLfMoveCursor) {
    VtParser p(5, 20);
    p.feed(b("ab\r\ncd"));
    EXPECT_EQ(p.lineText(0), "ab");
    EXPECT_EQ(p.lineText(1), "cd");
    EXPECT_EQ(p.cursorRow(), 1);
    EXPECT_EQ(p.cursorCol(), 2);
}

TEST(VtParser, BackspaceAndOverwrite) {
    VtParser p(3, 20);
    p.feed(b("abc\b\bX"));         // 退兩格後覆寫 b → "aXc"
    EXPECT_EQ(p.lineText(0), "aXc");
}

TEST(VtParser, TabAdvancesToMultipleOf8) {
    VtParser p(3, 40);
    p.feed(b("a\tb"));
    EXPECT_EQ(p.cellAt(0, 0).ch, QChar('a'));
    EXPECT_EQ(p.cellAt(0, 8).ch, QChar('b'));
}

TEST(VtParser, AutoWrapAtEol) {
    VtParser p(3, 4);
    p.feed(b("abcde"));            // 4 欄滿後換行
    EXPECT_EQ(p.lineText(0), "abcd");
    EXPECT_EQ(p.lineText(1), "e");
    EXPECT_EQ(p.cursorRow(), 1);
}

TEST(VtParser, ScrollUpPushesScrollback) {
    VtParser p(2, 10);
    p.feed(b("L0\r\nL1\r\nL2"));   // 第三行使第一行捲離
    EXPECT_EQ(p.scrollbackCount(), 1);
    EXPECT_EQ(p.scrollbackLine(0), "L0");
    EXPECT_EQ(p.lineText(0), "L1");
    EXPECT_EQ(p.lineText(1), "L2");
}

TEST(VtParser, CsiCursorPosition) {
    VtParser p(10, 20);
    p.feed(b("\x1b[3;5HX"));       // CUP 第3列第5欄（1-based）
    EXPECT_EQ(p.cellAt(2, 4).ch, QChar('X'));
}

TEST(VtParser, CsiCursorMoves) {
    VtParser p(10, 20);
    p.feed(b("\x1b[5;5H"));        // 移到 (4,4)
    p.feed(b("\x1b[2A"));          // 上 2 → row 2
    p.feed(b("\x1b[3C"));          // 右 3 → col 7
    EXPECT_EQ(p.cursorRow(), 2);
    EXPECT_EQ(p.cursorCol(), 7);
}

TEST(VtParser, EraseLineAndScreen) {
    VtParser p(3, 10);
    p.feed(b("hello\r\nworld"));
    p.feed(b("\x1b[H"));           // 回左上
    p.feed(b("\x1b[2J"));          // 清全螢幕
    EXPECT_EQ(p.lineText(0), "");
    EXPECT_EQ(p.lineText(1), "");
}

TEST(VtParser, EraseToEndOfLine) {
    VtParser p(3, 10);
    p.feed(b("abcdef"));
    p.feed(b("\x1b[3G"));          // 移到第 3 欄（col 2）
    p.feed(b("\x1b[K"));           // 清到行尾
    EXPECT_EQ(p.lineText(0), "ab");
}

TEST(VtParser, SgrColorAndBold) {
    VtParser p(3, 20);
    p.feed(b("\x1b[1;31mR\x1b[0mN"));   // 紅色粗體 R，重置後 N
    const auto& r = p.cellAt(0, 0);
    EXPECT_EQ(r.ch, QChar('R'));
    EXPECT_EQ(r.fg, 1);
    EXPECT_TRUE(r.bold);
    const auto& n = p.cellAt(0, 1);
    EXPECT_EQ(n.fg, -1);
    EXPECT_FALSE(n.bold);
}

TEST(VtParser, BrightForegroundAndBackground) {
    VtParser p(3, 20);
    p.feed(b("\x1b[92;44mX"));     // 亮綠前景(8+2=10)、藍背景(4)
    const auto& x = p.cellAt(0, 0);
    EXPECT_EQ(x.fg, 10);
    EXPECT_EQ(x.bg, 4);
}

TEST(VtParser, SplitEscapeAcrossFeeds) {
    VtParser p(3, 20);
    p.feed(b("\x1b["));            // 序列被切斷
    p.feed(b("31m"));
    p.feed(b("Z"));
    EXPECT_EQ(p.cellAt(0, 0).ch, QChar('Z'));
    EXPECT_EQ(p.cellAt(0, 0).fg, 1);   // 紅，證明跨 feed 狀態保留
}

TEST(VtParser, IgnoresUnknownAndOscSequences) {
    VtParser p(3, 20);
    p.feed(b("\x1b]0;window title\x07OK"));   // OSC 設定標題，應被吞掉
    EXPECT_EQ(p.lineText(0), "OK");
    p.feed(b("\r\n\x1b[?25lY"));              // 私有模式 ?25l（隱藏游標）應略過
    EXPECT_EQ(p.lineText(1), "Y");
}
