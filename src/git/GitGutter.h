#pragma once

#include <QStringList>
#include <QHash>

// Git gutter 行狀態計算：比較 HEAD 版本與目前緩衝區，輸出每行標示。
// 純邏輯、不依賴 git 行程與 GUI（見 tests/GitGutterTest.cpp）。
namespace GitGutter {

// 位元旗標：一行可同時是 Modified 且上方有刪除
enum State {
    Added        = 1,   // 新增行（青色條）
    Modified     = 2,   // 修改行（黃色條）
    DeletedAbove = 4    // 此行上方有刪除（洋紅三角）
};

// 回傳 { 0-based 緩衝區行號 → State 旗標組合 }；無變更的行不在結果中。
// 演算法：共同前綴/後綴修剪後對中段做 LCS；中段超過 maxMiddle 行時
// 整段粗略標為 Modified（避免大檔 O(n·m) 記憶體爆炸）。
QHash<int, int> diffLineStates(const QStringList& oldLines,
                               const QStringList& newLines,
                               int maxMiddle = 3000);

} // namespace GitGutter
