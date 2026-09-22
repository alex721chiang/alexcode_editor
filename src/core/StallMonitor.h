#pragma once
#include <QString>

// UI 卡頓偵測（診斷 "Not Responding"）：主執行緒以計時器打心跳，看門狗執行緒發現心跳
// 中斷超過門檻時，記下當下主執行緒正在執行的區段（StallScope 標記），恢復後寫入
// perf.log：「stall 1234 ms @ setProjectFolder > fsModel.setRootPath」。
// 設定 debug/stallLog=false 可關閉；只在真的卡頓時寫檔，平時開銷可忽略。
namespace StallMonitor {

void start(const QString& logPath, int thresholdMs = 400);   // 主執行緒呼叫
void stop();
QString lastStall();                                         // 最近一次卡頓紀錄（測試用）
bool isRunning();

// RAII 區段標記：只記錄主執行緒（其他執行緒呼叫時為 no-op，例如 notify() 在背景執行緒的事件）
class Scope {
public:
    explicit Scope(const char* name);
    ~Scope();
private:
    bool m_active = false;
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
};

} // namespace StallMonitor

#define STALL_SCOPE_CAT2(a, b) a##b
#define STALL_SCOPE_CAT(a, b) STALL_SCOPE_CAT2(a, b)
#define STALL_SCOPE(name) StallMonitor::Scope STALL_SCOPE_CAT(_stallScope, __LINE__)(name)
