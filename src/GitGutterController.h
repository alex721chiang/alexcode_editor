#pragma once
#include <QObject>
#include <QHash>
#include <QSet>
#include <QString>

class CodeEditor;

// 從 MainWindow 拆出的 Git gutter 子系統：非同步抓 HEAD 版本、與目前緩衝區 diff
// 後標示行狀態。不認識 tabWidget/editorForPath——用 signal 讓 MainWindow 自己找
// 對應的 CodeEditor 來套用結果，維持解耦。
class GitGutterController : public QObject {
    Q_OBJECT
public:
    explicit GitGutterController(QObject* parent = nullptr);

    void fetchHead(const QString& path);          // 非同步 `git show HEAD:./檔名`
    void recompute(CodeEditor* editor) const;      // 用快取幫這個編輯器重算 gutter（未快取則不動作）
    bool hasHead(const QString& path) const { return m_headCache.contains(path); }

signals:
    void headReady(const QString& path);        // 抓到 HEAD 內容；MainWindow 可接著呼叫 recompute()
    void markedUntracked(const QString& path);  // 確認不在版控；MainWindow 該把該檔 gutter 清空

private:
    QHash<QString, QString> m_headCache;   // path → HEAD 內容
    QSet<QString> m_untracked;             // 不在版控中的檔案（不重試）
};
