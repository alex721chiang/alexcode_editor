#pragma once
#include <QString>
#include <QStringList>
#include <QVector>
#include <atomic>
#include <functional>

// Find / Replace in Files 的核心（原本寫在 FindInFilesDialog 裡、於 UI 執行緒同步掃描與讀檔，
// 大專案或網路分享上會讓視窗 Not Responding）。純函式、只依賴 Qt Core，供背景執行緒呼叫：
// 可取消（cancel 旗標）、分批回報（onBatch）、有結果上限。
namespace FindInFiles {

struct Options {
    QString root;               // 搜尋的資料夾
    QStringList nameFilters;    // 如 *.cpp *.h
    QString query;              // 搜尋：支援 "a || b"（OR）；取代：整段視為字面字串
};

struct Match {
    QString filePath;
    int line = 0;               // 1-based
    QString snippet;            // trim 後最長 100 字元
};

struct Stats {
    int filesScanned = 0;
    int matches = 0;
    bool canceled = false;
    bool truncated = false;     // 達到結果上限而停止
};

using BatchFn = std::function<void(const QVector<Match>&)>;

Stats search(const Options& opt, const std::atomic<bool>& cancel, const BatchFn& onBatch,
             int maxResults = 20000, int batchSize = 200);

struct ReplaceHit {
    QString path;
    int count = 0;
};
struct ReplaceResult {
    int filesWritten = 0;
    int replacements = 0;
    bool canceled = false;
};

// 取代分兩階段：先統計（供使用者確認），確認後才寫入；寫入時重新讀檔再取代，
// 不會用統計當下的舊內容覆蓋之後的修改。比對為字面字串、不分大小寫。
QVector<ReplaceHit> planReplace(const Options& opt, const std::atomic<bool>& cancel);
ReplaceResult applyReplace(const QVector<ReplaceHit>& plan, const QString& term,
                           const QString& replacement, const std::atomic<bool>& cancel);

} // namespace FindInFiles
