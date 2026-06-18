#include <gtest/gtest.h>
#include "../src/AutoPair.h"

using namespace AutoPair;
static const QChar NUL = QChar();

TEST(AutoPair, ClosingForMap) {
    EXPECT_EQ(closingFor('('), QChar(')'));
    EXPECT_EQ(closingFor('['), QChar(']'));
    EXPECT_EQ(closingFor('{'), QChar('}'));
    EXPECT_EQ(closingFor('"'), QChar('"'));
    EXPECT_EQ(closingFor('x'), QChar());
}

TEST(AutoPair, OpenBracketAutoCloses) {
    auto d = decide('(', QChar(' '), NUL, false);
    EXPECT_EQ(d.action, Action::AutoClose);
    EXPECT_EQ(d.open, QChar('('));
    EXPECT_EQ(d.close, QChar(')'));
}

TEST(AutoPair, OpenBracketBeforeWordDoesNotAutoClose) {
    auto d = decide('(', QChar(' '), QChar('x'), false);
    EXPECT_EQ(d.action, Action::Insert);
}

TEST(AutoPair, CloserSkipsOverMatching) {
    auto d = decide(')', NUL, QChar(')'), false);
    EXPECT_EQ(d.action, Action::SkipOver);
}

TEST(AutoPair, CloserInsertsWhenNoMatchAhead) {
    auto d = decide(')', NUL, QChar('x'), false);
    EXPECT_EQ(d.action, Action::Insert);
}

TEST(AutoPair, QuoteAutoClosesInEmptyContext) {
    auto d = decide('"', QChar(' '), NUL, false);
    EXPECT_EQ(d.action, Action::AutoClose);
    EXPECT_EQ(d.close, QChar('"'));
}

TEST(AutoPair, QuoteSkipsOverMatching) {
    auto d = decide('"', NUL, QChar('"'), false);
    EXPECT_EQ(d.action, Action::SkipOver);
}

TEST(AutoPair, QuoteInWordDoesNotAutoClose) {
    // apostrophe inside "don't"
    auto d = decide('\'', QChar('n'), QChar('t'), false);
    EXPECT_EQ(d.action, Action::Insert);
}

TEST(AutoPair, SurroundOnSelection) {
    auto d = decide('(', NUL, NUL, true);
    EXPECT_EQ(d.action, Action::Surround);
    EXPECT_EQ(d.open, QChar('('));
    EXPECT_EQ(d.close, QChar(')'));
}

TEST(AutoPair, SurroundWithQuote) {
    auto d = decide('"', NUL, NUL, true);
    EXPECT_EQ(d.action, Action::Surround);
    EXPECT_EQ(d.open, QChar('"'));
    EXPECT_EQ(d.close, QChar('"'));
}

TEST(AutoPair, NonPairCharInserts) {
    EXPECT_EQ(decide('a', NUL, NUL, false).action, Action::Insert);
}

TEST(AutoPair, DeletePairBetweenEmptyPair) {
    EXPECT_TRUE(shouldDeletePair(QChar('('), QChar(')')));
    EXPECT_TRUE(shouldDeletePair(QChar('"'), QChar('"')));
    EXPECT_FALSE(shouldDeletePair(QChar('('), QChar(']')));
    EXPECT_FALSE(shouldDeletePair(QChar('x'), QChar('y')));
}
