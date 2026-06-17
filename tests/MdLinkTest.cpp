#include <gtest/gtest.h>
#include "../src/MdLink.h"

static QStringList targets(const QString& text, bool wikiOnly = false) {
    QStringList out;
    for (const MdLink::Ref& r : MdLink::extractRefs(text))
        if (!wikiOnly || r.wiki) out << r.target;
    return out;
}

TEST(MdLink, PlainWikilink) {
    EXPECT_EQ(targets("see [[Other Note]] here"), (QStringList{"Other Note"}));
}

TEST(MdLink, WikilinkAliasStripped) {
    EXPECT_EQ(targets("[[Target|display text]]"), (QStringList{"Target"}));
}

TEST(MdLink, WikilinkHeadingStripped) {
    EXPECT_EQ(targets("[[Target#Section]]"), (QStringList{"Target"}));
    EXPECT_EQ(targets("[[Target#Section|alias]]"), (QStringList{"Target"}));
}

TEST(MdLink, MultipleWikilinks) {
    EXPECT_EQ(targets("[[A]] and [[B]] and [[C]]"), (QStringList{"A", "B", "C"}));
}

TEST(MdLink, MarkdownLocalLink) {
    auto t = targets("[label](notes/foo.md)");
    EXPECT_EQ(t, (QStringList{"notes/foo.md"}));
}

TEST(MdLink, IgnoresHttpLinks) {
    EXPECT_TRUE(targets("[site](https://example.com)").isEmpty());
    EXPECT_TRUE(targets("[mail](mailto:a@b.com)").isEmpty());
}

TEST(MdLink, IgnoresPureAnchor) {
    EXPECT_TRUE(targets("[jump](#heading)").isEmpty());
}

TEST(MdLink, IgnoresLinksInFencedCode) {
    QString text = "real [[Yes]]\n```\ncode [[No]] block\n```\nafter [[Also]]";
    EXPECT_EQ(targets(text, true), (QStringList{"Yes", "Also"}));
}

TEST(MdLink, IgnoresLinksInInlineCode) {
    EXPECT_EQ(targets("text `[[NotALink]]` and [[Real]]", true), (QStringList{"Real"}));
}

TEST(MdLink, NormalizeKeyBasenameLowerNoExt) {
    EXPECT_EQ(MdLink::normalizeKey("Notes/My File.md"), QString("my file"));
    EXPECT_EQ(MdLink::normalizeKey("My File"), QString("my file"));
    EXPECT_EQ(MdLink::normalizeKey("a\\b\\C.MD"), QString("c"));
}

TEST(MdLink, EmptyAndNoLinks) {
    EXPECT_TRUE(targets("").isEmpty());
    EXPECT_TRUE(targets("just plain text with [not a link]").isEmpty());
}
