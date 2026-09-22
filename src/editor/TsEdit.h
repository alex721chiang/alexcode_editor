#pragma once
#include <QString>
#include <cstdint>

// 從 QTextDocument::contentsChange(pos, removed, added)（皆為 UTF-16 字元偏移）
// 算出 tree-sitter 增量解析需要的位元組偏移與 (row, column) 點位（column 以 UTF-8 位元組計）。
// 純邏輯、不相依 tree-sitter，可單元測試。
namespace TsEdit {

struct ByteEdit {
    uint32_t startByte = 0, oldEndByte = 0, newEndByte = 0;
    uint32_t startRow = 0, startCol = 0;       // column 以位元組計
    uint32_t oldEndRow = 0, oldEndCol = 0;
    uint32_t newEndRow = 0, newEndCol = 0;
};

// 某字元位置（UTF-16 index）之前的 UTF-8 位元組數。
inline uint32_t utf8Bytes(const QString& text, int charPos) {
    charPos = qBound(0, charPos, int(text.size()));
    return static_cast<uint32_t>(QStringView(text).left(charPos).toUtf8().size());
}

// 某字元位置的 (row, column-in-bytes)。
inline void pointAt(const QString& text, int charPos, uint32_t& row, uint32_t& col) {
    charPos = qBound(0, charPos, int(text.size()));
    uint32_t r = 0;
    int lineStart = 0;
    for (int i = 0; i < charPos; ++i) {
        if (text.at(i) == QLatin1Char('\n')) { ++r; lineStart = i + 1; }
    }
    row = r;
    col = static_cast<uint32_t>(QStringView(text).mid(lineStart, charPos - lineStart).toUtf8().size());
}

inline ByteEdit compute(const QString& oldText, const QString& newText,
                        int pos, int removed, int added) {
    ByteEdit e;
    const uint32_t startByte = utf8Bytes(newText, pos);          // 前綴在新舊相同
    e.startByte  = startByte;
    e.oldEndByte = startByte + (utf8Bytes(oldText, pos + removed) - utf8Bytes(oldText, pos));
    e.newEndByte = startByte + (utf8Bytes(newText, pos + added)  - utf8Bytes(newText, pos));
    pointAt(newText, pos, e.startRow, e.startCol);
    pointAt(oldText, pos + removed, e.oldEndRow, e.oldEndCol);
    pointAt(newText, pos + added, e.newEndRow, e.newEndCol);
    return e;
}

} // namespace TsEdit
