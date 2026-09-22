#pragma once
#include <QVector>
#include <QByteArray>
#include <QString>
#include <QStringList>
#include "TsSymbols.h"

struct TSTree;

// 以 tree-sitter 從「任意檔內容」擷取符號（不依賴開啟中的編輯器），供專案級符號索引使用。
namespace TsSymbolParser {

bool supports(const QString& ext);                                       // 是否為 tree-sitter 支援語言
QVector<TsSymbols::Symbol> parse(const QByteArray& utf8, const QString& ext);   // 自行解析檔內容
QVector<TsSymbols::Symbol> fromTree(TSTree* tree, const QByteArray& utf8);      // 用既有語法樹（重用解析結果）

// 擷取被呼叫的函式名（去重、依出現順序）；startRow/endRow>=0 時只看該行範圍內（0-based）。
QStringList calleesFromTree(TSTree* tree, const QByteArray& utf8, int startRow, int endRow);

} // namespace TsSymbolParser
