#include <gtest/gtest.h>
#include "GitBlame.h"

namespace {

// 精簡但格式正確的 --line-porcelain 輸出：兩行、兩個 commit，第二個為未提交（全零 sha）
const char* kSample =
    "1234567890abcdef1234567890abcdef12345678 1 1 1\n"
    "author alex\n"
    "author-mail <a@b.c>\n"
    "author-time 1751980800\n"
    "author-tz -0700\n"
    "summary Fix the frobnicator\n"
    "filename foo.cpp\n"
    "\tint main() {\n"
    "0000000000000000000000000000000000000000 2 2 1\n"
    "author Not Committed Yet\n"
    "author-time 1751980800\n"
    "summary Version of foo.cpp from foo.cpp\n"
    "filename foo.cpp\n"
    "\t  return 0;\n";

} // namespace

TEST(GitBlame, ParsesShaAuthorDateSummary) {
    const auto map = GitBlame::parsePorcelain(QByteArray(kSample));
    ASSERT_TRUE(map.contains(0));
    const auto& l0 = map.value(0);
    EXPECT_EQ(l0.shortSha, QString("12345678"));
    EXPECT_EQ(l0.author, QString("alex"));
    EXPECT_EQ(l0.summary, QString("Fix the frobnicator"));
    EXPECT_FALSE(l0.uncommitted);
    EXPECT_FALSE(l0.date.isEmpty());
}

TEST(GitBlame, MarksUncommittedLines) {
    const auto map = GitBlame::parsePorcelain(QByteArray(kSample));
    ASSERT_TRUE(map.contains(1));
    EXPECT_TRUE(map.value(1).uncommitted);
}

TEST(GitBlame, StatusTextFormats) {
    const auto map = GitBlame::parsePorcelain(QByteArray(kSample));
    const QString s = GitBlame::statusText(map.value(0));
    EXPECT_TRUE(s.contains("12345678"));
    EXPECT_TRUE(s.contains("alex"));
    EXPECT_TRUE(s.contains("Fix the frobnicator"));
    EXPECT_TRUE(GitBlame::statusText(map.value(1)).contains(QStringLiteral("未提交")));
}

TEST(GitBlame, EmptyInputYieldsEmptyMap) {
    EXPECT_TRUE(GitBlame::parsePorcelain(QByteArray()).isEmpty());
}
