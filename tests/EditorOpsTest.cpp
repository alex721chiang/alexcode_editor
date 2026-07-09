// CodeEditor 書籤批次操作的回歸測試（widget 測試：main 在 WidgetTestMain.cpp，
// QApplication 以 offscreen 平台啟動，CI 無桌面也能跑）。
// 這些操作的游標數學有多個易錯邊界（最後一行、唯一一行、連續行），
// 開發時就抓過「刪最後一行位置溢位」與「迭代器混用 segfault」兩個 bug，務必保留覆蓋。
#include <gtest/gtest.h>
#include <QTest>
#include <QClipboard>
#include <QApplication>
#include "../src/CodeEditor.h"

namespace {

CodeEditor* makeEditor(const QString& text) {
    auto* e = new CodeEditor();          // 交給呼叫端 delete；測試內用 unique_ptr 包
    e->setPlainText(text);
    return e;
}

} // namespace

TEST(EditorBookmarkOps, MarkMatchingPlainText) {
    std::unique_ptr<CodeEditor> e(makeEditor("alpha\nbeta\nALPHA\ngamma"));
    const int added = e->bookmarkMatchingLines("alpha", /*caseSensitive=*/false, /*useRegex=*/false);
    EXPECT_EQ(added, 2);
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0, 2}));
}

TEST(EditorBookmarkOps, MarkMatchingCaseSensitive) {
    std::unique_ptr<CodeEditor> e(makeEditor("alpha\nbeta\nALPHA\ngamma"));
    const int added = e->bookmarkMatchingLines("alpha", /*caseSensitive=*/true, /*useRegex=*/false);
    EXPECT_EQ(added, 1);
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0}));
}

TEST(EditorBookmarkOps, MarkMatchingRegex) {
    std::unique_ptr<CodeEditor> e(makeEditor("error: x\nwarning: y\nerror: z\nok"));
    const int added = e->bookmarkMatchingLines("^error:", false, /*useRegex=*/true);
    EXPECT_EQ(added, 2);
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0, 2}));
}

TEST(EditorBookmarkOps, MarkMatchingInvalidRegexReturnsMinusOne) {
    std::unique_ptr<CodeEditor> e(makeEditor("abc"));
    EXPECT_EQ(e->bookmarkMatchingLines("([", false, /*useRegex=*/true), -1);
    EXPECT_TRUE(e->bookmarkedLines().isEmpty());
}

TEST(EditorBookmarkOps, MarkMatchingKeepsExistingBookmarks) {
    std::unique_ptr<CodeEditor> e(makeEditor("aa\nbb\ncc"));
    e->setBookmarkedLines({1});
    const int added = e->bookmarkMatchingLines("aa", false, false);
    EXPECT_EQ(added, 1);
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0, 1}));
}

TEST(EditorBookmarkOps, BookmarkedLinesTextSortedByLine) {
    std::unique_ptr<CodeEditor> e(makeEditor("L0\nL1\nL2\nL3"));
    e->setBookmarkedLines({3, 0});
    EXPECT_EQ(e->bookmarkedLinesText(), QString("L0\nL3"));
}

TEST(EditorBookmarkOps, DeleteBookmarkedMiddleLine) {
    std::unique_ptr<CodeEditor> e(makeEditor("L0\nL1\nL2"));
    e->setBookmarkedLines({1});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("L0\nL2"));
    EXPECT_TRUE(e->bookmarkedLines().isEmpty());
}

TEST(EditorBookmarkOps, DeleteBookmarkedLastLine) {
    // 抓過的 bug：最後一個 block 的 length() 仍 +1，位置會溢位一格
    std::unique_ptr<CodeEditor> e(makeEditor("L0\nL1\nL2"));
    e->setBookmarkedLines({2});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("L0\nL1"));
}

TEST(EditorBookmarkOps, DeleteBookmarkedOnlyLine) {
    std::unique_ptr<CodeEditor> e(makeEditor("only"));
    e->setBookmarkedLines({0});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString(""));
}

TEST(EditorBookmarkOps, DeleteBookmarkedAllLines) {
    std::unique_ptr<CodeEditor> e(makeEditor("a\nb\nc"));
    e->setBookmarkedLines({0, 1, 2});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString(""));
}

TEST(EditorBookmarkOps, DeleteBookmarkedConsecutiveTail) {
    std::unique_ptr<CodeEditor> e(makeEditor("keep\nx\ny"));
    e->setBookmarkedLines({1, 2});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("keep"));
}

TEST(EditorBookmarkOps, DeleteBookmarkedIsSingleUndoStep) {
    std::unique_ptr<CodeEditor> e(makeEditor("a\nb\nc"));
    e->setBookmarkedLines({0, 2});
    e->deleteBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("b"));
    e->undo();
    EXPECT_EQ(e->toPlainText(), QString("a\nb\nc"));
}

TEST(EditorBookmarkOps, DeleteNonBookmarkedKeepsMarkedLines) {
    std::unique_ptr<CodeEditor> e(makeEditor("a\nb\nc\nd"));
    e->setBookmarkedLines({1, 3});
    e->deleteNonBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("b\nd"));
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0, 1}));   // 書籤重新對齊到剩餘行
}

TEST(EditorBookmarkOps, DeleteNonBookmarkedNoBookmarksIsNoOp) {
    // 防呆：沒有書籤時不可把整份文件清空
    std::unique_ptr<CodeEditor> e(makeEditor("a\nb"));
    e->deleteNonBookmarkedLines();
    EXPECT_EQ(e->toPlainText(), QString("a\nb"));
}

TEST(EditorBookmarkOps, InvertBookmarks) {
    std::unique_ptr<CodeEditor> e(makeEditor("a\nb\nc"));
    e->setBookmarkedLines({1});
    e->invertBookmarks();
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{0, 2}));
    e->invertBookmarks();
    EXPECT_EQ(e->bookmarkedLines(), (QList<int>{1}));
}

// ---- 多游標（Sublime 式核心）----

TEST(MultiCursor, SelectAllOccurrences) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo bar foo baz foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);   // 選取第一個 foo
    e->setTextCursor(c);
    e->selectAllOccurrences();
    EXPECT_EQ(e->cursorCount(), 3);
}

TEST(MultiCursor, TypingReplacesAllSelections) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo bar foo baz foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    QTest::keyClicks(e.get(), "X");
    EXPECT_EQ(e->toPlainText(), QString("X bar X baz X"));
}

TEST(MultiCursor, TypingIsSingleUndoStep) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    QTest::keyClicks(e.get(), "Y");
    EXPECT_EQ(e->toPlainText(), QString("Y Y"));
    e->undo();
    EXPECT_EQ(e->toPlainText(), QString("foo foo"));
}

TEST(MultiCursor, EscapeClearsExtraCursors) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    ASSERT_EQ(e->cursorCount(), 2);
    QTest::keyClick(e.get(), Qt::Key_Escape);
    EXPECT_EQ(e->cursorCount(), 1);
}

TEST(MultiCursor, EndKeyMovesAllCursors) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo bar\nfoo baz"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    ASSERT_EQ(e->cursorCount(), 2);
    QTest::keyClick(e.get(), Qt::Key_End);
    QTest::keyClicks(e.get(), "!");
    EXPECT_EQ(e->toPlainText(), QString("foo bar!\nfoo baz!"));
}

TEST(MultiCursor, PasteDistributesLinesWhenCountsMatch) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo A foo B foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    ASSERT_EQ(e->cursorCount(), 3);
    QApplication::clipboard()->setText("1\n2\n3");
    QTest::keyClick(e.get(), Qt::Key_V, Qt::ControlModifier);
    EXPECT_EQ(e->toPlainText(), QString("1 A 2 B 3"));
}

TEST(MultiCursor, PasteInsertsWholeTextWhenCountsDiffer) {
    std::unique_ptr<CodeEditor> e(makeEditor("foo foo"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(3, QTextCursor::KeepAnchor);
    e->setTextCursor(c);
    e->selectAllOccurrences();
    ASSERT_EQ(e->cursorCount(), 2);
    QApplication::clipboard()->setText("Z");
    QTest::keyClick(e.get(), Qt::Key_V, Qt::ControlModifier);
    EXPECT_EQ(e->toPlainText(), QString("Z Z"));
}

TEST(MultiCursor, MergeAfterMovementRemovesDuplicates) {
    // 同一行的兩個游標按 End 後重合，應自動合併為一個
    std::unique_ptr<CodeEditor> e(makeEditor("x x"));
    QTextCursor c = e->textCursor();
    c.setPosition(0);
    c.setPosition(1, QTextCursor::KeepAnchor);   // 選第一個 x
    e->setTextCursor(c);
    e->selectAllOccurrences();
    ASSERT_EQ(e->cursorCount(), 2);
    QTest::keyClick(e.get(), Qt::Key_End);
    EXPECT_EQ(e->cursorCount(), 1);
}

// QTextDocument::find() 的 QRegularExpression 多載只看 FindCaseSensitively 旗標，
// 不看 regex 自己的 CaseInsensitiveOption —— 「Match case」修復所依賴的 Qt 行為，
// 用測試釘住，Qt 升級若改變此行為會在這裡爆
TEST(FindFlagsBehavior, RegexOverloadHonorsFindCaseSensitivelyOnly) {
    QTextDocument doc;
    doc.setPlainText("Alpha alpha");
    const QRegularExpression re("alpha");   // 未帶 CaseInsensitiveOption

    // 無旗標：Qt 強制不分大小寫 → 從頭找會先命中 "Alpha"
    QTextCursor c1 = doc.find(re, 0);
    ASSERT_FALSE(c1.isNull());
    EXPECT_EQ(c1.selectedText(), QString("Alpha"));

    // 帶 FindCaseSensitively：只命中小寫 "alpha"
    QTextCursor c2 = doc.find(re, 0, QTextDocument::FindCaseSensitively);
    ASSERT_FALSE(c2.isNull());
    EXPECT_EQ(c2.selectedText(), QString("alpha"));
}
