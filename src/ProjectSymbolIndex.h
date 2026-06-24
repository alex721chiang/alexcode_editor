#pragma once
#include <QString>
#include <QStringList>
#include <QByteArray>
#include <QVector>
#include <QHash>
#include "TsSymbols.h"

// 專案級符號索引（Source Insight 風）：掃描資料夾、用 tree-sitter 擷取每個檔的符號，
// 提供跨檔快速搜尋。索引與搜尋為純邏輯（不依賴 tree-sitter / 檔案系統），可單元測試；
// 只有 build()/updateFileFromDisk() 會用到 TsSymbolParser 與檔案系統。
class ProjectSymbolIndex {
public:
    struct Entry {
        QString name;
        QString kind;     // class / function / method …
        QString file;     // 絕對路徑
        int line = 0;     // 0-based 起始行
    };

    // ---- 純邏輯（可單元測試）----
    void clear();
    void setFileSymbols(const QString& file, const QVector<TsSymbols::Symbol>& syms,
                        qint64 mtime = -1);                          // 取代該檔符號（mtime>=0 時記錄）
    void removeFile(const QString& file);
    int fileCount() const { return m_byFile.size(); }
    int symbolCount() const;
    QStringList files() const;                                       // 已索引的檔（絕對路徑）
    qint64 mtimeOf(const QString& file) const;                       // -1 = 未知
    QVector<Entry> all() const;
    QVector<Entry> exact(const QString& name) const;                  // 同名（找定義）
    QVector<Entry> search(const QString& query, int limit = 200) const;   // 模糊排序

    // ---- 磁碟快取序列化（純邏輯，可測）----
    QByteArray serialize() const;                                    // → JSON
    void deserialize(const QByteArray& json);                        // ← JSON

    // ---- 需要 tree-sitter / 檔案系統 ----
    void build(const QString& folder);                               // 依 mtime 增量重建 + 讀寫 .alexcode 快取
    bool updateFileFromDisk(const QString& file);                    // 重新解析單一檔（false = 已移除/不支援）

private:
    QHash<QString, QVector<Entry>> m_byFile;     // 絕對檔路徑 → 該檔符號
    QHash<QString, qint64> m_mtime;              // 絕對檔路徑 → 上次索引時的 mtime（秒）
};
