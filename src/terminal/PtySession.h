#pragma once
#include <QObject>
#include <QByteArray>
#include <QString>
#include <thread>
#include <atomic>
#ifdef Q_OS_WIN
#include <windows.h>
#else
#include <sys/types.h>   // pid_t
#endif

// 互動式終端機會話：Windows 用 ConPTY（虛擬主控台接 PowerShell/cmd），
// Linux/macOS 用 forkpty()（pty 接 $SHELL）。雙向 I/O；
// 讀取在背景執行緒進行，透過 queued signal 把資料丟回 GUI 執行緒。
class PtySession : public QObject {
    Q_OBJECT
public:
    explicit PtySession(QObject* parent = nullptr);
    ~PtySession() override;

    bool start(const QString& program, const QString& workingDir, int cols, int rows);
    void writeData(const QByteArray& data);
    void resize(int cols, int rows);
    void stop();
    bool isRunning() const { return m_running.load(); }

    // Windows（ConPTY）與 Linux/macOS（forkpty）皆有實作。
    // 讓呼叫端（TerminalWidget）能區分「此平台不支援」與「真的啟動失敗」，顯示正確訊息。
    static bool isPlatformSupported();

signals:
    void dataReceived(const QByteArray& data);   // queued 至 GUI 執行緒
    void exited();

private:
    void readerLoop();
    std::thread m_reader;
    std::atomic<bool> m_running{false};
    std::atomic<bool> m_stopping{false};   // 關閉中：抑制 reader 對外發訊號，避免事件迴圈拆除時的競態
#ifdef Q_OS_WIN
    std::atomic<bool> m_readerDone{false}; // reader 執行緒真正跑完才會設 true；stop() 靠這個判斷何時可安全 join
    std::atomic<DWORD> m_readerTid{0};     // reader 的 Win32 thread id（0 = 尚未啟動）；
                                           // stop() 用 OpenThread 換 HANDLE 給 CancelSynchronousIo
    HPCON  m_hPC      = nullptr;
    HANDLE m_inWrite  = nullptr;   // 我們寫 → 子行程 stdin
    HANDLE m_outRead  = nullptr;   // 我們讀 ← 子行程 stdout
    HANDLE m_hProcess = nullptr;
    HANDLE m_hThread  = nullptr;
    LPPROC_THREAD_ATTRIBUTE_LIST m_attrList = nullptr;
#else
    int   m_masterFd    = -1;      // pty master 端（讀寫子行程）
    pid_t m_childPid    = -1;
    int   m_wakePipe[2] = {-1, -1};   // stop() 喚醒 select() 用（避免 close-while-read 競態）
#endif
};
