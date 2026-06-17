#include <gtest/gtest.h>
#include "../src/MarkdownLinkIndex.h"
#include <QHash>

static MarkdownLinkIndex makeVault() {
    QHash<QString, QString> v;
    v["/vault/A.md"] = "Links to [[B]] and [[C]].";
    v["/vault/B.md"] = "Back to [[A]]. Also [[C|see C]].";
    v["/vault/C.md"] = "A leaf note, no outgoing links.";
    v["/vault/D.md"] = "Points to [[B]] via [text](B.md) too.";
    MarkdownLinkIndex idx;
    idx.buildFromContents(v);
    return idx;
}

TEST(MarkdownLinkIndex, FileCount) {
    EXPECT_EQ(makeVault().fileCount(), 4);
}

TEST(MarkdownLinkIndex, OutLinksResolved) {
    auto idx = makeVault();
    auto out = idx.outLinks("/vault/A.md");
    EXPECT_EQ(out.size(), 2);
    EXPECT_TRUE(out.contains("/vault/B.md"));
    EXPECT_TRUE(out.contains("/vault/C.md"));
}

TEST(MarkdownLinkIndex, Backlinks) {
    auto idx = makeVault();
    auto back = idx.backlinks("/vault/C.md");
    EXPECT_EQ(back.size(), 2);                  // A and B link to C
    EXPECT_TRUE(back.contains("/vault/A.md"));
    EXPECT_TRUE(back.contains("/vault/B.md"));
}

TEST(MarkdownLinkIndex, BacklinksToB) {
    auto idx = makeVault();
    auto back = idx.backlinks("/vault/B.md");   // A and D link to B
    EXPECT_EQ(back.size(), 2);
    EXPECT_TRUE(back.contains("/vault/A.md"));
    EXPECT_TRUE(back.contains("/vault/D.md"));
}

TEST(MarkdownLinkIndex, DuplicateLinkCountedOnce) {
    auto idx = makeVault();
    // D links to B twice ([[B]] and [text](B.md)) → still one edge
    EXPECT_EQ(idx.outLinks("/vault/D.md"), (QStringList{"/vault/B.md"}));
}

TEST(MarkdownLinkIndex, LeafHasNoOutLinks) {
    EXPECT_TRUE(makeVault().outLinks("/vault/C.md").isEmpty());
}

TEST(MarkdownLinkIndex, Resolve) {
    auto idx = makeVault();
    EXPECT_EQ(idx.resolve("B"), QString("/vault/B.md"));
    EXPECT_EQ(idx.resolve("b.md"), QString("/vault/B.md"));
    EXPECT_TRUE(idx.resolve("Nonexistent").isEmpty());
}

TEST(MarkdownLinkIndex, EdgesCount) {
    auto idx = makeVault();
    // A->B, A->C, B->A, B->C, D->B = 5 edges
    EXPECT_EQ(idx.edges().size(), 5);
}
