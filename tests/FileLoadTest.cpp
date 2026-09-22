#include <gtest/gtest.h>
#include <QTemporaryDir>
#include <QFile>
#include "FileLoad.h"

TEST(FileLoadTest, Utf8WithLfIsDetected) {
    const auto r = FileLoad::decode(QByteArray("a\nb\n"));
    EXPECT_TRUE(r.ok);
    EXPECT_EQ(r.encoding, "UTF-8");
    EXPECT_EQ(r.eol, "LF");
    EXPECT_EQ(r.text, "a\nb\n");
    EXPECT_EQ(r.bytes, 4);
}

TEST(FileLoadTest, CrlfIsNormalizedAndReported) {
    const auto r = FileLoad::decode(QByteArray("a\r\nb\r\n"));
    EXPECT_EQ(r.eol, "CRLF");
    EXPECT_EQ(r.text, "a\nb\n");
}

TEST(FileLoadTest, InvalidUtf8FallsBackToBig5) {
    const auto r = FileLoad::decode(QByteArray("\xA4\xA4\xA4\xE5"));   // 「中文」的 Big5
    EXPECT_EQ(r.encoding, "Big5");
    EXPECT_EQ(r.text, QString::fromUtf8("中文"));
}

TEST(FileLoadTest, LoadReadsFileFromDisk) {
    QTemporaryDir dir;
    const QString p = dir.filePath("x.txt");
    QFile f(p);
    ASSERT_TRUE(f.open(QIODevice::WriteOnly));
    f.write("hello\r\n");
    f.close();
    const auto r = FileLoad::load(p);
    EXPECT_TRUE(r.ok);
    EXPECT_EQ(r.text, "hello\n");
    EXPECT_EQ(r.eol, "CRLF");
}

TEST(FileLoadTest, MissingFileReportsError) {
    const auto r = FileLoad::load("/nonexistent/definitely/missing.txt");
    EXPECT_FALSE(r.ok);
    EXPECT_FALSE(r.error.isEmpty());
}
