#include <gtest/gtest.h>
#include "../src/TextRefs.h"

using TextRefs::findWholeWord;

TEST(TextRefs, FindsWholeWordOccurrences) {
    const QString src = "int foo();\nfoo();\nint x = foofoo;\n  foo  ();";
    auto hits = findWholeWord(src, "foo");
    ASSERT_EQ(hits.size(), 3);              // line 0, 1, 3（line 2 的 foofoo 不算）
    EXPECT_EQ(hits[0].line, 0);
    EXPECT_EQ(hits[1].line, 1);
    EXPECT_EQ(hits[2].line, 3);
}

TEST(TextRefs, IgnoresSubstring) {
    auto hits = findWholeWord("doWorker(); workerDo();", "Worker");
    EXPECT_TRUE(hits.isEmpty());            // Worker 只出現在更長字詞內
}

TEST(TextRefs, OnePerLineEvenIfRepeated) {
    auto hits = findWholeWord("foo + foo + foo", "foo");
    ASSERT_EQ(hits.size(), 1);
    EXPECT_EQ(hits[0].text, "foo + foo + foo");
}

TEST(TextRefs, TrimmedLineText) {
    auto hits = findWholeWord("    bar = 1;", "bar");
    ASSERT_EQ(hits.size(), 1);
    EXPECT_EQ(hits[0].text, "bar = 1;");
}

TEST(TextRefs, EmptyName) {
    EXPECT_TRUE(findWholeWord("anything", "").isEmpty());
}

TEST(TextRefs, BoundaryAtLineEdges) {
    auto hits = findWholeWord("foo", "foo");
    ASSERT_EQ(hits.size(), 1);
    EXPECT_EQ(hits[0].line, 0);
}
