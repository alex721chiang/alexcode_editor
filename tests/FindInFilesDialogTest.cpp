// Find in Files 對話框：搜尋/取代在背景執行，UI 執行緒不做掃描與讀檔。
#include <gtest/gtest.h>
#include <QApplication>
#include <QElapsedTimer>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QTemporaryDir>
#include <QThread>
#include <QTimer>
#include <QFile>
#include "FindInFilesDialog.h"

namespace {
template <class F> bool pumpUntil(F cond, int timeoutMs = 10000) {
    QElapsedTimer t;
    t.start();
    while (!cond()) {
        if (t.elapsed() > timeoutMs) return false;
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        QThread::msleep(2);
    }
    return true;
}
void makeFiles(const QTemporaryDir& d, int n, const QByteArray& content) {
    for (int i = 0; i < n; ++i) {
        QFile f(d.filePath(QString("f%1.txt").arg(i)));
        f.open(QIODevice::WriteOnly);
        f.write(content);
    }
}
struct Ui {
    FindInFilesDialog dlg;
    QLineEdit* dir; QLineEdit* filter; QLineEdit* query; QLineEdit* repl;
    QPushButton* search; QPushButton* replace; QListWidget* results;
    Ui() {
        dir = dlg.findChild<QLineEdit*>("dirInput");
        filter = dlg.findChild<QLineEdit*>("filterInput");
        query = dlg.findChild<QLineEdit*>("searchInput");
        repl = dlg.findChild<QLineEdit*>("replaceInput");
        search = dlg.findChild<QPushButton*>("searchBtn");
        replace = dlg.findChild<QPushButton*>("replaceBtn");
        results = dlg.getResultsList();
    }
    bool running() const { return dlg.isBusy(); }
};
}

// 按下 Search 立即返回（結果於事件迴圈中陸續加入），完成後結果完整且帶開檔所需資料
TEST(FindInFilesDialogTest, SearchRunsInBackground) {
    QTemporaryDir d;
    makeFiles(d, 200, "alpha\nhit here\n");
    Ui ui;
    ASSERT_TRUE(ui.dir && ui.filter && ui.query && ui.search && ui.results);
    ui.dir->setText(d.path());
    ui.filter->setText("*.txt");
    ui.query->setText("hit");

    ui.search->click();
    EXPECT_TRUE(ui.running());
    EXPECT_EQ(ui.results->count(), 0);              // 舊版在此已同步完成全部 200 筆
    ASSERT_TRUE(pumpUntil([&] { return !ui.running(); }));
    ASSERT_EQ(ui.results->count(), 200);
    const QVariantMap data = ui.results->item(0)->data(Qt::UserRole).toMap();
    EXPECT_TRUE(data.value("filePath").toString().endsWith(".txt"));
    EXPECT_EQ(data.value("lineNum").toInt(), 2);
}

// 搜尋中再按一次（Cancel）即停止
TEST(FindInFilesDialogTest, SearchCanBeCanceled) {
    QTemporaryDir d;
    makeFiles(d, 3000, QByteArray("hit\n").repeated(20));
    Ui ui;
    ui.dir->setText(d.path());
    ui.filter->setText("*.txt");
    ui.query->setText("hit");
    ui.search->click();
    ASSERT_TRUE(ui.running());
    ui.search->click();                             // Cancel
    ASSERT_TRUE(pumpUntil([&] { return !ui.running(); }));
    EXPECT_LT(ui.results->count(), 3000 * 20);
}

// 取代：背景統計 → 確認 → 背景寫入 → 重新搜尋
TEST(FindInFilesDialogTest, ReplaceAllInBackgroundAfterConfirm) {
    QTemporaryDir d;
    makeFiles(d, 5, "foo bar foo\n");
    Ui ui;
    ui.dir->setText(d.path());
    ui.filter->setText("*.txt");
    ui.query->setText("foo");
    ui.repl->setText("baz");

    // 自動回答所有出現的訊息框（確認 → Yes；完成 → OK）
    QTimer answer;
    QObject::connect(&answer, &QTimer::timeout, [] {
        if (auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
            if (auto* yes = box->button(QMessageBox::Yes)) yes->click();
            else if (auto* ok = box->button(QMessageBox::Ok)) ok->click();
        }
    });
    answer.start(20);
    ui.replace->click();
    ASSERT_TRUE(pumpUntil([&] {
        QFile f(d.filePath("f4.txt"));
        f.open(QIODevice::ReadOnly);
        return f.readAll() == "baz bar baz\n" && !ui.running();
    }));
    for (int i = 0; i < 5; ++i) {
        QFile f(d.filePath(QString("f%1.txt").arg(i)));
        f.open(QIODevice::ReadOnly);
        EXPECT_EQ(f.readAll(), "baz bar baz\n");
    }
}
