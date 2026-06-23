#pragma once
#include <QVector>
#include <QByteArray>
#include <QString>
#include "TsSymbols.h"

struct TSTree;

// 以 tree-sitter 從「任意檔內容」擷取符號（不依賴開啟中的編輯器），供專案級符號索引使用。
namespace TsSymbolParser {

bool supports(const QString& ext);                                       // 是否為 tree-sitter 支援語言
QVector<TsSymbols::Symbol> parse(const QByteArray& utf8, const QString& ext);   // 自行解析檔內容
QVector<TsSymbols::Symbol> fromTree(TSTree* tree, const QByteArray& utf8);      // 用既有語法樹（重用解析結果）

} // namespace TsSymbolParser
