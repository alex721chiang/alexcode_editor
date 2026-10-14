// 分割檢視顯示另一份文件（並排比對）、分頁拖出成獨立視窗。
#include <gtest/gtest.h>
#include <QAction>
#include <QComboBox>
#include <QCoreApplication>
#include <QDir>
#include <QDockWidget>
#include <QElapsedTimer>
#include <QFile>
#include <QStandardPaths>
#include <QTabBar>
#include <QTabWidget>
#include <QTemporaryDir>
#include <QTest>
#include <QThread>
#include <QToolButton>
#include "CodeEditor.h"
#include "EditorWindows.h"
#include "MainWindow.h"
#include "Portable.h"

namespace {
void pump(int ms = 50) {
    QElapsedTimer t;
    t.start();
    while (t.elapsed() < ms) { QCoreApplication::processEvents(QEventLoop::AllEvents, 10); QThread::msleep(1); }
}

class EditorWindowsTest : public ::testing::Test {
protected:
    QTemporaryDir dir;
    QString a, b;
    void SetUp() override {
        QStandardPaths::setTestModeEnabled(true);
        QDir(Portable::dataDir()).removeRecursively();
        AppSettings s;
        s.setValue("session/restore", false);
        s.sync();
        a = write("a.txt", "AAA\n");
        b = write("b.txt", "BBB\n");
    }
    void TearDown() override { QStandardPaths::setTestModeEnabled(false); }
    QString write(const QString& n, const QByteArray& d) {
        QFile f(dir.filePath(n));
        f.open(QIODevice::WriteOnly);
        f.write(d);
        return dir.filePath(n);
    }
    static QTabWidget* tabs(MainWindow& w) { return w.findChild<QTabWidget*>(); }
    static CodeEditor* tabFor(MainWindow& w, const QString& path) {
        for (int i = 0; i < tabs(w)->count(); ++i)
            if (tabs(w)->widget(i)->property("filePath").toString() == path)
                return qobject_cast<CodeEditor*>(tabs(w)->widget(i));
        return nullptr;
    }
    static void showSplit(MainWindow& w) {
        for (QAction* act : w.findChildren<QAction*>())
            if (act->isCheckable() && act->shortcut() == QKeySequence(Qt::CTRL | Qt::Key_Backslash) && !act->isChecked())
                act->trigger();
        pump();
    }
};
}

// 分割檢視可固定顯示另一個分頁的文件：切換分頁時不跟著換（兩份文件並排）
TEST_F(EditorWindowsTest, SplitViewCanPinAnotherDocument) {
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({a, b});                   // 目前分頁：b
    showSplit(w);
    auto* combo = w.findChild<QComboBox*>("splitSourceCombo");
    auto* split = w.findChild<CodeEditor*>("splitEditor");
    ASSERT_TRUE(combo && split);
    EXPECT_EQ(split->document(), tabFor(w, b)->document());   // 預設：跟隨目前分頁

    const int idx = combo->findData(QVariant::fromValue<QObject*>(tabFor(w, a)));
    ASSERT_GE(idx, 0);
    combo->setCurrentIndex(idx);                             // 固定顯示 a
    pump();
    EXPECT_EQ(split->document(), tabFor(w, a)->document());
    tabs(w)->setCurrentWidget(tabFor(w, b));
    pump();
    EXPECT_EQ(split->document(), tabFor(w, a)->document());   // 換分頁仍是 a

    tabs(w)->setCurrentWidget(tabFor(w, a));
    QMetaObject::invokeMethod(tabs(w), "tabCloseRequested", Q_ARG(int, tabs(w)->indexOf(tabFor(w, a))));
    pump();
    ASSERT_EQ(tabFor(w, a), nullptr);
    EXPECT_EQ(split->document(), tabFor(w, b)->document());   // 固定的分頁關閉 → 恢復跟隨
    EXPECT_EQ(combo->currentIndex(), 0);
}

// 拖出：分頁拖到分頁列外放開 → 開獨立視窗，與分頁共用同一份文件（編輯即同步）
TEST_F(EditorWindowsTest, DraggingTabOutOpensDetachedWindowSharingDocument) {
    MainWindow w;
    w.resize(900, 600);
    w.show();
    w.openFilesFromOtherInstance({a, b});
    auto* ctrl = w.findChild<EditorWindowsController*>();
    ASSERT_TRUE(ctrl);
    QTabBar* bar = tabs(w)->tabBar();
    const int ia = tabs(w)->indexOf(tabFor(w, a));
    const QPoint start = bar->tabRect(ia).center();
    const QPoint outside = start + QPoint(0, 250);
    QTest::mousePress(bar, Qt::LeftButton, {}, start);
    QTest::mouseMove(bar, start + QPoint(0, 60));
    QTest::mouseMove(bar, outside);
    QTest::mouseRelease(bar, Qt::LeftButton, {}, outside);
    pump();

    ASSERT_EQ(ctrl->detachedWindows().size(), 1);
    CodeEditor* view = ctrl->viewOf(ctrl->detachedWindows().first());
    ASSERT_TRUE(view);
    EXPECT_EQ(view->document(), tabFor(w, a)->document());
    view->moveCursor(QTextCursor::End);
    view->insertPlainText("typed in window\n");
    EXPECT_TRUE(tabFor(w, a)->toPlainText().contains("typed in window"));
    EXPECT_TRUE(tabFor(w, a)->document()->isModified());
}

// 只是點選或在分頁列內移動，不會開視窗
TEST_F(EditorWindowsTest, ClickOrMoveWithinTabBarDoesNotDetach) {
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({a, b});
    auto* ctrl = w.findChild<EditorWindowsController*>();
    QTabBar* bar = tabs(w)->tabBar();
    const QPoint p = bar->tabRect(0).center();
    QTest::mouseClick(bar, Qt::LeftButton, {}, p);
    QTest::mousePress(bar, Qt::LeftButton, {}, p);
    QTest::mouseMove(bar, p + QPoint(30, 0));
    QTest::mouseRelease(bar, Qt::LeftButton, {}, p + QPoint(30, 0));
    pump();
    EXPECT_TRUE(ctrl->detachedWindows().isEmpty());
}

// 「回到主視窗」：關閉獨立視窗並切到該分頁
TEST_F(EditorWindowsTest, BackToMainWindowClosesAndActivatesTab) {
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({a, b});                   // 目前分頁：b
    auto* ctrl = w.findChild<EditorWindowsController*>();
    QWidget* win = ctrl->detach(tabFor(w, a));
    ASSERT_TRUE(win);
    EXPECT_EQ(ctrl->detach(tabFor(w, a)), win);             // 同一分頁不重複開
    auto* back = win->findChild<QToolButton*>("backToMainButton");
    ASSERT_TRUE(back);
    back->click();
    pump();
    EXPECT_TRUE(ctrl->detachedWindows().isEmpty());
    EXPECT_EQ(tabs(w)->currentWidget(), tabFor(w, a));
}

// 分頁關閉（文件隨之銷毀）→ 對應的獨立視窗先關閉，不得存取已刪除的文件
TEST_F(EditorWindowsTest, ClosingTabClosesItsDetachedWindow) {
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({a, b});
    auto* ctrl = w.findChild<EditorWindowsController*>();
    ctrl->detach(tabFor(w, a));
    ctrl->detach(tabFor(w, b));
    ASSERT_EQ(ctrl->detachedWindows().size(), 2);
    QMetaObject::invokeMethod(tabs(w), "tabCloseRequested", Q_ARG(int, tabs(w)->indexOf(tabFor(w, a))));
    pump(100);
    ASSERT_EQ(ctrl->detachedWindows().size(), 1);
    EXPECT_EQ(ctrl->viewOf(ctrl->detachedWindows().first())->document(), tabFor(w, b)->document());
}

// 主視窗結束時仍有獨立視窗開著：安全關閉（不崩潰）
TEST_F(EditorWindowsTest, MainWindowDestructionWithDetachedWindowsIsSafe) {
    {
        MainWindow w;
        w.show();
        w.openFilesFromOtherInstance({a, b});
        auto* ctrl = w.findChild<EditorWindowsController*>();
        ctrl->detach(tabFor(w, a));
        ctrl->detach(tabFor(w, b));
        pump();
    }
    pump();
    SUCCEED();
}

// 既有缺陷回歸：分割視窗顯示目前分頁時關閉該分頁 → 原本會崩潰
//（備用空白文件是編輯器的子物件，QPlainTextEdit::setDocument 換文件時已將它刪除）
TEST_F(EditorWindowsTest, ClosingTabShownInSplitViewDoesNotCrash) {
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({a});
    showSplit(w);
    auto* split = w.findChild<CodeEditor*>("splitEditor");
    ASSERT_EQ(split->document(), tabFor(w, a)->document());
    QMetaObject::invokeMethod(tabs(w), "tabCloseRequested", Q_ARG(int, tabs(w)->indexOf(tabFor(w, a))));
    pump();
    EXPECT_NE(split->document(), nullptr);
    EXPECT_TRUE(split->toPlainText().isEmpty() || tabs(w)->count() > 0);
}

// 換成另一份文件後，行號區寬度須依新文件重算（原本只在行數變動時更新 → 3 位數行號的文件
// 在分割檢視中，每行第一個字被行號/摺疊區蓋住）
TEST_F(EditorWindowsTest, SharedViewGutterMatchesNewDocument) {
    QByteArray many;
    for (int i = 0; i < 150; ++i) many += "line\n";
    const QString c = write("c.txt", many);
    MainWindow w;
    w.show();
    w.openFilesFromOtherInstance({c});
    showSplit(w);
    auto* split = w.findChild<CodeEditor*>("splitEditor");
    ASSERT_EQ(split->document(), tabFor(w, c)->document());
    EXPECT_EQ(split->viewport()->geometry().left() - split->frameWidth(), split->lineNumberAreaWidth());
    auto* ctrl = w.findChild<EditorWindowsController*>();
    CodeEditor* view = ctrl->viewOf(ctrl->detach(tabFor(w, c)));
    pump();
    EXPECT_EQ(view->viewport()->geometry().left() - view->frameWidth(), view->lineNumberAreaWidth());
}
