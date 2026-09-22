#pragma once
#include <QObject>
#include <QHash>
#include <QSet>
#include <QString>
#include "GitBlame.h"

class CodeEditor;

// 從 MainWindow 拆出的 Git gutter 子系統：非同步抓 HEAD 版本、與目前緩衝區 diff
// 後標示行狀態。不認識 tabWidget/editorForPath——用 signal 讓 MainWindow 自己找
// 對應的 CodeEditor 來套用結果，維持解耦。
// 另提供：HEAD 內容存取（並排 diff 檢視器用）、行級 blame（狀態列顯示用）。
class GitGutterController : public QObject {
    Q_OBJECT
public:
    explicit GitGutterController(QObject* parent = nullptr);

    void fetchHead(const QString& path);          // 非同步 `git show HEAD:./檔名`
    void recompute(CodeEditor* editor) const;      // 用快取幫這個編輯器重算 gutter（未快取則不動作）
    bool hasHead(const QString& path) const { return m_headCache.contains(path); }
    QString headText(const QString& path) const { return m_headCache.value(path); }

    // ---- 行級 blame（git blame --line-porcelain，非同步 + 快取）----
    void fetchBlame(const QString& path);          // 完成後發 blameReady；存檔後重呼叫刷新
    void invalidateBlame(const QString& path) { m_blameCache.remove(path); }
    // 游標行的狀態列文字；空字串 = 無資料（未在版控 / 尚未抓完）
    QString blameTextForLine(const QString& path, int line) const {
        const auto it = m_blameCache.constFind(path);
        if (it == m_blameCache.constEnd()) return QString();
        const auto lit = it->constFind(line);
        return lit == it->constEnd() ? QString() : GitBlame::statusText(*lit);
    }

signals:
    void headReady(const QString& path);        // 抓到 HEAD 內容；MainWindow 可接著呼叫 recompute()
    void markedUntracked(const QString& path);  // 確認不在版控；MainWindow 該把該檔 gutter 清空
    void blameReady(const QString& path);       // blame 解析完成；MainWindow 可刷新狀態列

private:
    QHash<QString, QString> m_headCache;   // path → HEAD 內容
    QSet<QString> m_untracked;             // 不在版控中的檔案（不重試）
    QHash<QString, QHash<int, GitBlame::LineInfo>> m_blameCache;   // path → 行 → 資訊
    QSet<QString> m_blameInFlight;         // 進行中的 blame，避免重複啟動
};
