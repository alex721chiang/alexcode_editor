#pragma once
#include <QObject>
#include <QList>
#include <QString>

class CodeEditor;

// 從 MainWindow 拆出的 Snippet 樣板子系統（trigger + Tab 展開；JSON 設定檔）。
// 純資料/邏輯，不持有任何 UI，建構子直接載入設定（首次執行會寫入預設樣板）。
class SnippetsController : public QObject {
    Q_OBJECT
public:
    explicit SnippetsController(QObject* parent = nullptr);

    static QString configPath();   // 設定檔路徑（Portable::dataDir()/alexcode-snippets.json）
    void load();                   // 讀設定；檔案不存在則先寫入預設樣板
    void applyTo(CodeEditor* editor) const;   // 依編輯器目前語言過濾後注入
    int count() const { return m_defs.size(); }

private:
    struct SnippetDef { QString trigger, language, body; };
    QList<SnippetDef> m_defs;
};
