#pragma once
#include <QObject>
#include <QByteArray>
#include <QString>
#include <thread>
#include <atomic>
#ifdef Q_OS_WIN
#include <windows.h>
#endif

// Windows ConPTY 會話：開一個虛擬主控台接 PowerShell/cmd，雙向 I/O。
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

signals:
    void dataReceived(const QByteArray& data);   // queued 至 GUI 執行緒
    void exited();

private:
    void readerLoop();
    std::thread m_reader;
    std::atomic<bool> m_running{false};
    std::atomic<bool> m_stopping{false};   // 關閉中：抑制 reader 對外發訊號，避免事件迴圈拆除時的競態
#ifdef Q_OS_WIN
    HPCON  m_hPC      = nullptr;
    HANDLE m_inWrite  = nullptr;   // 我們寫 → 子行程 stdin
    HANDLE m_outRead  = nullptr;   // 我們讀 ← 子行程 stdout
    HANDLE m_hProcess = nullptr;
    HANDLE m_hThread  = nullptr;
    LPPROC_THREAD_ATTRIBUTE_LIST m_attrList = nullptr;
#endif
};
