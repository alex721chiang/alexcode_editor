#pragma once

// 依檔案大小決定降級層級（純邏輯，可單元測試）。
namespace FileTier {

enum class Level {
    Full,          // 全功能
    AssistOff,     // 關自動完成 / LSP / 即時 Git，保留語法高亮
    HighlightOff   // 連語法高亮一併關閉（完整大檔模式）
};

inline Level forSizeMB(double mb, int assistMaxMB, int highlightMaxMB) {
    if (mb > highlightMaxMB) return Level::HighlightOff;
    if (mb > assistMaxMB)    return Level::AssistOff;
    return Level::Full;
}

} // namespace FileTier
