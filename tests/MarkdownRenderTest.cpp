#include <gtest/gtest.h>
#include "MarkdownRender.h"

using MarkdownRender::preprocessWikilinks;

TEST(MarkdownRender, PlainWikilinkBecomesLink) {
    EXPECT_EQ(preprocessWikilinks("see [[Note]]"),
              QString("see [Note](alexcode:Note)"));
}

TEST(MarkdownRender, AliasUsedAsLabel) {
    EXPECT_EQ(preprocessWikilinks("[[Target|Display]]"),
              QString("[Display](alexcode:Target)"));
}

TEST(MarkdownRender, HeadingStrippedFromTarget) {
    EXPECT_EQ(preprocessWikilinks("[[Target#Section]]"),
              QString("[Target#Section](alexcode:Target)"));
}

TEST(MarkdownRender, SpacesPercentEncodedInUrl) {
    EXPECT_EQ(preprocessWikilinks("[[Project Alpha]]"),
              QString("[Project Alpha](alexcode:Project%20Alpha)"));
}

TEST(MarkdownRender, MultipleOnOneLine) {
    EXPECT_EQ(preprocessWikilinks("[[A]] and [[B]]"),
              QString("[A](alexcode:A) and [B](alexcode:B)"));
}

TEST(MarkdownRender, InlineCodeUntouched) {
    EXPECT_EQ(preprocessWikilinks("text `[[NotALink]]` and [[Real]]"),
              QString("text `[[NotALink]]` and [Real](alexcode:Real)"));
}

TEST(MarkdownRender, FencedCodeUntouched) {
    QString in = "before [[A]]\n```\n[[B]] in code\n```\nafter [[C]]";
    QString out = "before [A](alexcode:A)\n```\n[[B]] in code\n```\nafter [C](alexcode:C)";
    EXPECT_EQ(preprocessWikilinks(in), out);
}

TEST(MarkdownRender, NoWikilinksUnchanged) {
    EXPECT_EQ(preprocessWikilinks("# Heading\n\nplain text"),
              QString("# Heading\n\nplain text"));
}

TEST(MarkdownRender, PreservesLineCount) {
    QString in = "a\nb\nc";
    EXPECT_EQ(preprocessWikilinks(in), in);
}

TEST(MarkdownRender, StyleSheetNonEmpty) {
    EXPECT_FALSE(MarkdownRender::styleSheet().isEmpty());
}
