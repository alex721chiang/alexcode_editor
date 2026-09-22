#pragma once
#include <QString>

// 文字工具箱的純轉換邏輯（不依賴 GUI，方便單元測試）。
// 由 MainWindow 的 Tools 選單呼叫；抽離自原本內嵌於 setupUI 的 lambda。
namespace TextTools {

QString sortLines(const QString& text, bool descending = false);
QString removeDuplicateLines(const QString& text);
QString removeBlankLines(const QString& text);
QString reverseLines(const QString& text);
QString trimTrailingWhitespace(const QString& text);

QString toHalfWidth(const QString& text);     // 全形 → 半形
QString toFullWidth(const QString& text);     // 半形 → 全形

QString unicodeEscape(const QString& text);   // 非 ASCII → \uXXXX
QString unicodeUnescape(const QString& text); // \uXXXX → 字元

// 解析整數字面值（支援 0x 十六進位、0b 二進位、0 開頭八進位、十進位）。成功回傳 true。
bool parseInteger(const QString& s, qlonglong* out);

} // namespace TextTools
