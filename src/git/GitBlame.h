#pragma once
#include <QString>
#include <QHash>
#include <QByteArray>

// git blame --line-porcelain 輸出解析（純邏輯，可單元測試）。
// 每行格式：header「<sha> <orig> <final> [count]」＋ 屬性行（author/author-time/summary…）
// ＋ 一行 tab 開頭的內容。--line-porcelain 對每行都重複完整屬性，免管 commit 快取。
namespace GitBlame {

struct LineInfo {
    QString shortSha;    // 前 8 碼
    QString author;
    QString date;        // yyyy-MM-dd（author-time 秒級時間戳轉換）
    QString summary;
    bool uncommitted = false;   // 全零 sha = 尚未提交的本地變更
};

// 回傳 0-based 行號 → 資訊
QHash<int, LineInfo> parsePorcelain(const QByteArray& out);

// 狀態列顯示字串："⎇ abcd1234 作者 2026-07-09 · 摘要"；未提交顯示「未提交變更」
QString statusText(const LineInfo& info);

} // namespace GitBlame
