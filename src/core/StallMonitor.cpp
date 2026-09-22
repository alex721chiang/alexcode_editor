#include "StallMonitor.h"
#include <QCoreApplication>
#include <QDateTime>
#include <QFile>
#include <QTimer>
#include <QMap>
#include <algorithm>
#include <atomic>
#include <chrono>
#include <mutex>
#include <thread>
#include <vector>

namespace StallMonitor {
namespace {

using Clock = std::chrono::steady_clock;
qint64 nowMs() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now().time_since_epoch()).count();
}

std::atomic<qint64> g_lastBeat{0};
std::atomic<bool> g_running{false};
std::thread g_watchdog;
QTimer* g_beatTimer = nullptr;

std::mutex g_mutex;                      // 保護下列狀態
std::vector<const char*> g_stack;        // 主執行緒目前的區段堆疊
QMap<QString, int> g_samples;            // 本次卡頓期間各區段被取樣到的次數（每 25ms 一次）
QString g_last;                          // 最近一次卡頓紀錄
QString g_logPath;
std::atomic<std::thread::id> g_mainThread{};

QString stackString() {                  // 呼叫端持有 g_mutex
    QString s;
    for (const char* n : g_stack) {
        if (!s.isEmpty()) s += QStringLiteral(" > ");
        s += QLatin1String(n);
    }
    return s.isEmpty() ? QStringLiteral("(未標記區段)") : s;
}

void writeLine(const QString& line) {
    QFile f(g_logPath);
    if (f.size() > 1024 * 1024) f.remove();          // 上限 1MB，超過即重來
    if (f.open(QIODevice::Append | QIODevice::Text))
        f.write((QDateTime::currentDateTime().toString(Qt::ISODateWithMs) + ' ' + line + '\n').toUtf8());
}

void watchdogLoop(int thresholdMs) {
    bool inStall = false;
    qint64 stallStart = 0;
    while (g_running) {
        std::this_thread::sleep_for(std::chrono::milliseconds(25));
        const qint64 gap = nowMs() - g_lastBeat.load();
        std::lock_guard<std::mutex> lock(g_mutex);
        if (gap > thresholdMs) {
            if (!inStall) { inStall = true; stallStart = g_lastBeat.load(); g_samples.clear(); }
            ++g_samples[stackString()];                   // 取樣：卡頓期間主執行緒停在哪個區段
        } else if (inStall) {
            inStall = false;
            int total = 0;
            QList<QPair<int, QString>> parts;
            for (auto it = g_samples.cbegin(); it != g_samples.cend(); ++it) {
                total += it.value();
                parts.append({it.value(), it.key()});
            }
            std::sort(parts.begin(), parts.end(), [](const auto& a, const auto& b) { return a.first > b.first; });
            QStringList where;
            for (const auto& p : parts)
                where << QStringLiteral("%1 (%2%)").arg(p.second).arg(p.first * 100 / qMax(total, 1));
            g_last = QStringLiteral("stall %1 ms @ %2").arg(nowMs() - stallStart).arg(where.join(QStringLiteral("; ")));
            writeLine(g_last);
        }
    }
}

} // namespace

void start(const QString& logPath, int thresholdMs) {
    if (g_running) return;
    g_logPath = logPath;
    g_mainThread = std::this_thread::get_id();
    g_lastBeat = nowMs();
    g_beatTimer = new QTimer(QCoreApplication::instance());
    g_beatTimer->setInterval(20);
    QObject::connect(g_beatTimer, &QTimer::timeout, [] { g_lastBeat = nowMs(); });
    g_beatTimer->start();
    g_running = true;
    g_watchdog = std::thread(watchdogLoop, thresholdMs);
}

void stop() {
    if (!g_running) return;
    g_running = false;
    if (g_watchdog.joinable()) g_watchdog.join();
    delete g_beatTimer;
    g_beatTimer = nullptr;
}

bool isRunning() { return g_running; }

QString lastStall() {
    std::lock_guard<std::mutex> lock(g_mutex);
    return g_last;
}

Scope::Scope(const char* name) {
    if (!g_running || std::this_thread::get_id() != g_mainThread.load()) return;
    m_active = true;
    std::lock_guard<std::mutex> lock(g_mutex);
    g_stack.push_back(name);
}

Scope::~Scope() {
    if (!m_active) return;
    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_stack.empty()) g_stack.pop_back();
}

} // namespace StallMonitor
