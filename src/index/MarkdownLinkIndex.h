#pragma once
#include <QString>
#include <QStringList>
#include <QHash>
#include <QList>
#include <QPair>

// 掃描一個 vault（資料夾）下所有 .md，建立連結關係圖：
// 每個檔案的對外連結（outLinks）與反向連結（backlinks）。
// 供 backlinks 面板與關係圖共用。可用 buildFromContents 做無檔案系統單元測試。
class MarkdownLinkIndex {
public:
    void build(const QString& folder);                                   // 遞迴掃描 *.md
    void buildFromContents(const QHash<QString, QString>& fileToContent); // 測試/記憶體用

    QStringList files() const { return m_files; }
    int fileCount() const { return m_files.size(); }
    QStringList outLinks(const QString& file) const { return m_out.value(file); }
    QStringList backlinks(const QString& file) const { return m_back.value(file); }
    QList<QPair<QString, QString>> edges() const;        // (from,to) 已解析、去重
    // 解析單一目標（wikilink 或相對路徑）為絕對檔案路徑；找不到回空字串。
    QString resolve(const QString& target) const;

private:
    void finalize();
    QStringList m_files;                         // 絕對路徑，排序
    QHash<QString, QString> m_keyToFile;         // normalizeKey(basename) -> abs file
    QHash<QString, QStringList> m_rawRefs;       // file -> 原始目標字串
    QHash<QString, QStringList> m_out;           // file -> 解析後的目標檔
    QHash<QString, QStringList> m_back;          // file -> 連入的來源檔
};
