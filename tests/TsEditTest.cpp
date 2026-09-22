#include <gtest/gtest.h>
#include "TsEdit.h"

using TsEdit::compute;
using TsEdit::ByteEdit;

TEST(TsEdit, InsertSingleAsciiChar) {
    // "abc" -> "aXbc" : insert 'X' at pos 1
    ByteEdit e = compute("abc", "aXbc", 1, 0, 1);
    EXPECT_EQ(e.startByte, 1u);
    EXPECT_EQ(e.oldEndByte, 1u);     // nothing removed
    EXPECT_EQ(e.newEndByte, 2u);     // one byte added
    EXPECT_EQ(e.startRow, 0u);
    EXPECT_EQ(e.startCol, 1u);
    EXPECT_EQ(e.newEndCol, 2u);
}

TEST(TsEdit, DeleteSingleChar) {
    // "aXbc" -> "abc" : remove 'X' at pos 1
    ByteEdit e = compute("aXbc", "abc", 1, 1, 0);
    EXPECT_EQ(e.startByte, 1u);
    EXPECT_EQ(e.oldEndByte, 2u);     // one byte removed
    EXPECT_EQ(e.newEndByte, 1u);     // nothing added
}

TEST(TsEdit, InsertMultibyteChar) {
    // "a" -> "a中" : '中' is 3 UTF-8 bytes, 1 UTF-16 unit
    ByteEdit e = compute("a", "a中", 1, 0, 1);
    EXPECT_EQ(e.startByte, 1u);
    EXPECT_EQ(e.oldEndByte, 1u);
    EXPECT_EQ(e.newEndByte, 4u);     // 1 + 3 bytes
    EXPECT_EQ(e.newEndCol, 4u);
}

TEST(TsEdit, ReplaceRange) {
    // "hello" -> "hiyo" : replace "ell" (pos 1, removed 3) with "iy" (added 2)
    ByteEdit e = compute("hello", "hiyo", 1, 3, 2);
    EXPECT_EQ(e.startByte, 1u);
    EXPECT_EQ(e.oldEndByte, 4u);     // 1 + len("ell")=3
    EXPECT_EQ(e.newEndByte, 3u);     // 1 + len("iy")=2
}

TEST(TsEdit, InsertNewlineUpdatesRows) {
    // "ab" -> "a\nb" : insert '\n' at pos 1
    ByteEdit e = compute("ab", "a\nb", 1, 0, 1);
    EXPECT_EQ(e.startRow, 0u);
    EXPECT_EQ(e.startCol, 1u);
    EXPECT_EQ(e.newEndRow, 1u);      // after the inserted newline we are on row 1
    EXPECT_EQ(e.newEndCol, 0u);      // column 0 of the new line
}

TEST(TsEdit, EditOnSecondLine) {
    // "ab\ncd" -> "ab\ncXd" : insert 'X' at pos 4 (second line)
    ByteEdit e = compute("ab\ncd", "ab\ncXd", 4, 0, 1);
    EXPECT_EQ(e.startRow, 1u);
    EXPECT_EQ(e.startCol, 1u);       // after 'c' on line 1
    EXPECT_EQ(e.startByte, 4u);      // "ab\nc" = 4 bytes
}

TEST(TsEdit, EmptyInsertAtStart) {
    ByteEdit e = compute("", "x", 0, 0, 1);
    EXPECT_EQ(e.startByte, 0u);
    EXPECT_EQ(e.oldEndByte, 0u);
    EXPECT_EQ(e.newEndByte, 1u);
}
