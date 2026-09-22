// 工作階段還原 / 開檔的「不卡主執行緒」測試。
// 以 FIFO（具名管線）模擬卡住的網路檔：open() 會阻塞到有寫入端為止，
// 任何在主執行緒上的讀取都會讓測試永遠卡住——正好重現 SMB 伺服器慢/離線時的症狀。
#include <gtest/gtest.h>
#include <QtGlobal>

#ifndef Q_OS_WIN
#include <QTemporaryDir>
#include <QStandardPaths>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QElapsedTimer>
#include <QThread>
#include <QFile>
#include <QDir>
#include <QTabWidget>
#include <QCoreApplication>
#include <QThreadPool>
#include <QtConcurrent>
#include <atomic>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include "MainWindow.h"
#include "Portable.h"

class MainWindowSessionTest : public ::testing::Test {
protected:
    QTemporaryDir tmp;
    QStringList fifos;

    void SetUp() override {
        QStandardPaths::setTestModeEnabled(true);
        QDir(Portable::dataDir()).removeRecursively();
        AppSettings s;
        s.setValue("session/restore", false);   // 建構子不自動還原；由測試明確呼叫
        s.sync();
    }
    void TearDown() override {
        for (const QString& f : fifos) unblockFifo(f, QByteArray(), 200);   // 保證背景讀取執行緒不殘留
        QStandardPaths::setTestModeEnabled(false);
    }

    QString makeFile(const QString& name, const QByteArray& data) {
        const QString p = tmp.filePath(name);
        QFile f(p);
        f.open(QIODevice::WriteOnly);
        f.write(data);
        return p;
    }
    QString makeFifo(const QString& name) {
        const QString p = tmp.filePath(name);
        EXPECT_EQ(::mkfifo(QFile::encodeName(p).constData(), 0600), 0);
        fifos << p;
        return p;
    }
    static bool isFifo(const QString& p) {
        struct stat st {};
        return ::stat(QFile::encodeName(p).constData(), &st) == 0 && S_ISFIFO(st.st_mode);
    }
    // 讀取端就緒後寫入資料並關閉（讀取端收到 EOF）
    static bool unblockFifo(const QString& p, const QByteArray& data, int timeoutMs = 5000) {
        QElapsedTimer t;
        t.start();
        while (t.elapsed() < timeoutMs) {
            const int fd = ::open(QFile::encodeName(p).constData(), O_WRONLY | O_NONBLOCK);
            if (fd >= 0) {
                if (!data.isEmpty()) (void)!::write(fd, data.constData(), data.size());
                ::close(fd);
                return true;
            }
            QCoreApplication::processEvents();
            QThread::msleep(10);
        }
        return false;
    }
    template <class F> static bool spinUntil(F cond, int timeoutMs = 5000) {
        QElapsedTimer t;
        t.start();
        while (!cond()) {
            if (t.elapsed() > timeoutMs) return false;
            QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
            QThread::msleep(5);
        }
        return true;
    }
    void writeSession(const QJsonArray& tabs, int current) {
        QJsonObject root;
        root["tabs"] = tabs;
        root["currentIndex"] = current;
        QFile f(Portable::dataDir() + "/session.json");
        ASSERT_TRUE(f.open(QIODevice::WriteOnly));
        f.write(QJsonDocument(root).toJson());
    }
    static QJsonObject tab(const QString& path, int cursor = 0) {
        QJsonObject t;
        t["filePath"] = path;
        t["baseTitle"] = QFileInfo(path).fileName();
        t["cursor"] = cursor;
        return t;
    }

    // friend 存取 MainWindow 私有成員
    static void restore(MainWindow& w) { w.restoreSession(); }
    static void save(MainWindow& w) { w.saveSession(); }
    static void open(MainWindow& w, const QString& p) { w.openFileByPath(p); }
    static void saveCurrent(MainWindow& w) { w.saveFile(); }
    static bool restoring(MainWindow& w) { return w.restoringSession; }
    static QTabWidget* tabs(MainWindow& w) { return w.tabWidget; }
    static CodeEditor* editorAt(MainWindow& w, int i) {
        return qobject_cast<CodeEditor*>(w.tabWidget->widget(i));
    }
    static bool pending(CodeEditor* e) { return e && e->property("pendingLoad").toBool(); }
    static void startLoad(MainWindow& w, CodeEditor* e) { w.startPendingLoad(e); }
    static void externalChange(MainWindow& w, const QString& p) { w.onFileChangedExternally(p); }
};

// 非作用中分頁只建佔位、完全不讀檔；作用中分頁於背景載入
TEST_F(MainWindowSessionTest, NonCurrentTabsAreNotReadOnRestore) {
    const QString a = makeFile("a.txt", "AAA\n");
    const QString slow = makeFifo("slow.log");
    QJsonObject slowTab = tab(slow, 0);
    slowTab["bookmarks"] = QJsonArray{0};
    writeSession(QJsonArray{tab(a, 2), slowTab}, 0);

    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    ASSERT_EQ(tabs(w)->count(), 2);
    EXPECT_TRUE(pending(editorAt(w, 1)));
    EXPECT_TRUE(editorAt(w, 1)->toPlainText().isEmpty());

    ASSERT_TRUE(spinUntil([&] { return !pending(editorAt(w, 0)); }));
    EXPECT_EQ(editorAt(w, 0)->toPlainText(), "AAA\n");
    EXPECT_EQ(editorAt(w, 0)->textCursor().position(), 2);   // 游標狀態於載入後套用

    // 佔位分頁的狀態在存檔時原樣保留
    save(w);
    QFile f(Portable::dataDir() + "/session.json");
    ASSERT_TRUE(f.open(QIODevice::ReadOnly));
    const QJsonArray saved = QJsonDocument::fromJson(f.readAll()).object()["tabs"].toArray();
    ASSERT_EQ(saved.size(), 2);
    EXPECT_EQ(saved[1].toObject()["filePath"].toString(), slow);
    EXPECT_EQ(saved[1].toObject()["bookmarks"].toArray(), QJsonArray{0});
    EXPECT_FALSE(saved[1].toObject().contains("backup"));
}

// 作用中分頁的檔案卡住：主執行緒照樣回應；載入前唯讀且不可存檔（避免空內容覆寫原檔）
TEST_F(MainWindowSessionTest, CurrentTabLoadsInBackgroundAndIsProtectedUntilLoaded) {
    const QString slow = makeFifo("slow.txt");
    writeSession(QJsonArray{tab(slow)}, 0);

    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    CodeEditor* e = editorAt(w, 0);
    ASSERT_TRUE(pending(e));
    EXPECT_TRUE(e->isReadOnly());
    saveCurrent(w);                       // 必須是 no-op
    EXPECT_TRUE(isFifo(slow));

    ASSERT_TRUE(unblockFifo(slow, "hello\n"));
    ASSERT_TRUE(spinUntil([&] { return !pending(e); }));
    EXPECT_EQ(e->toPlainText(), "hello\n");
    EXPECT_FALSE(e->isReadOnly());
    EXPECT_FALSE(e->document()->isModified());
}

// 讀不到（伺服器離線/檔已刪）：分頁保留並提示，之後切回可重試
TEST_F(MainWindowSessionTest, UnreadableFileKeepsPlaceholderTab) {
    writeSession(QJsonArray{tab(tmp.filePath("missing.txt"))}, 0);
    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    CodeEditor* e = editorAt(w, 0);
    ASSERT_TRUE(spinUntil([&] { return e && !e->property("loading").toBool(); }));
    EXPECT_EQ(tabs(w)->count(), 1);
    EXPECT_TRUE(pending(e));
    EXPECT_FALSE(e->placeholderText().isEmpty());
}

// 有未存檔快照的分頁：從本機快照還原，不碰（可能在網路上的）原檔
TEST_F(MainWindowSessionTest, BackupTabRestoresWithoutReadingOriginal) {
    const QString slow = makeFifo("draft.txt");
    const QString backup = makeFile("tab_0.txt", "draft\n");
    QJsonObject t = tab(slow);
    t["backup"] = backup;
    t["modified"] = true;
    t["encoding"] = "Big5";
    t["eol"] = "CRLF";
    writeSession(QJsonArray{t}, 0);

    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    CodeEditor* e = editorAt(w, 0);
    ASSERT_TRUE(e);
    EXPECT_FALSE(pending(e));
    EXPECT_EQ(e->toPlainText(), "draft\n");
    EXPECT_TRUE(e->document()->isModified());
    EXPECT_EQ(e->property("filePath").toString(), slow);
    EXPECT_EQ(e->property("encoding").toString(), "Big5");   // 原編碼保留，存檔不會被改成 UTF-8
    EXPECT_EQ(e->property("eol").toString(), "CRLF");
}

// 前景開啟一個仍是佔位的分頁（如 Find in Files 跳行）：返回前內容已載入，後續 gotoLine 不會被覆蓋
TEST_F(MainWindowSessionTest, ForegroundOpenOfPlaceholderLoadsImmediately) {
    const QString a = makeFile("a.txt", "A\n");
    const QString b = makeFile("c.txt", "l1\nl2\nl3\n");
    writeSession(QJsonArray{tab(a), tab(b, 0)}, 0);
    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    ASSERT_TRUE(pending(editorAt(w, 1)));
    open(w, b);
    EXPECT_FALSE(pending(editorAt(w, 1)));
    EXPECT_EQ(editorAt(w, 1)->toPlainText(), "l1\nl2\nl3\n");
    EXPECT_EQ(tabs(w)->currentIndex(), 1);
}

// 已開啟的檔案再次開啟：直接切換分頁，不重讀
TEST_F(MainWindowSessionTest, ReopeningOpenFileDoesNotReadAgain) {
    const QString a = makeFile("b.txt", "B\n");
    MainWindow w;
    open(w, a);
    const int before = tabs(w)->count();
    QFile::remove(a);
    ASSERT_EQ(::mkfifo(QFile::encodeName(a).constData(), 0600), 0);   // 同路徑換成會卡住的 FIFO
    fifos << a;
    open(w, a);                                                       // 舊版會在此卡死
    EXPECT_EQ(tabs(w)->count(), before);
}
// 卡在網路 I/O 的背景讀檔不可佔用 Qt 全域執行緒池（Qt 繪圖會用它；被佔滿時 UI 卡死）
TEST_F(MainWindowSessionTest, BlockedLoadsDoNotStarveGlobalThreadPool) {
    const int n = QThreadPool::globalInstance()->maxThreadCount() + 1;
    QJsonArray tabsJson;
    for (int i = 0; i < n; ++i) tabsJson.append(tab(makeFifo(QString("hung%1.log").arg(i))));
    writeSession(tabsJson, 0);
    MainWindow w;
    restore(w);
    ASSERT_TRUE(spinUntil([&] { return !restoring(w); }));
    for (int i = 0; i < n; ++i) startLoad(w, editorAt(w, i));   // n 個讀檔全部卡住
    QThread::msleep(100);

    std::atomic<bool> ran{false};
    auto f = QtConcurrent::run([&ran] { ran = true; });         // 全域池仍有空位
    EXPECT_TRUE(spinUntil([&] { return ran.load(); }, 2000));
    f.waitForFinished();
}

// 外部變更（如 tail 模式的 log 追加）於背景重讀：檔案卡住時主執行緒仍即時返回
TEST_F(MainWindowSessionTest, ExternalChangeReloadsInBackground) {
    const QString p = makeFile("live.log", "one\n");
    MainWindow w;
    open(w, p);
    CodeEditor* e = editorAt(w, tabs(w)->count() - 1);
    ASSERT_EQ(e->toPlainText(), "one\n");

    QFile::remove(p);
    ASSERT_EQ(::mkfifo(QFile::encodeName(p).constData(), 0600), 0);   // 下次讀取會卡住
    fifos << p;
    QElapsedTimer t;
    t.start();
    externalChange(w, p);                                             // 舊版在此同步讀檔而卡死
    EXPECT_LT(t.elapsed(), 1000);
    // 移除檔案/建 FIFO 本身也會觸發監看而排入重讀；連續變更會合併為「讀完再讀最新一次」，
    // 所以持續供應內容直到套用
    EXPECT_TRUE(spinUntil([&] {
        unblockFifo(p, "one\ntwo\n", 20);
        return e->toPlainText() == "one\ntwo\n";
    }));
}
#endif
