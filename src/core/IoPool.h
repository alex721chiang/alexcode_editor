#pragma once
#include <QThread>
#include <QThreadPool>

// 背景檔案 / 網路 I/O 專用執行緒池（取代 QThreadPool::globalInstance()）。
// 原因：Qt 的軟體繪圖（QRasterPaintEngine 的多執行緒填色）也使用全域執行緒池；
// 若全域池的執行緒都被「卡在網路分享上的讀檔/掃描」佔住，主執行緒的繪圖會在
// QSemaphore::acquire 等待空出的執行緒 → 視窗 "Not Responding"。
// 刻意不釋放：程式結束時不必等卡在網路 I/O 的執行緒返回。
namespace IoPool {
inline QThreadPool* instance() {
    static QThreadPool* pool = [] {
        auto* p = new QThreadPool;
        p->setObjectName(QStringLiteral("AlexCodeIoPool"));
        p->setMaxThreadCount(qMax(4, QThread::idealThreadCount()));
        return p;
    }();
    return pool;
}
} // namespace IoPool
