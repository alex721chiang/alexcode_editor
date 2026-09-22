#pragma once
#include <QDialog>
#include <QString>
#include <QList>
#include <QPair>

class QTableWidget;

// 圖形設定中心：以表格編輯 LSP 伺服器、Snippet、快捷鍵，存檔即寫回對應 JSON。
// 不直接套用（交由 MainWindow 在 accept 後呼叫既有 load/apply）。
class SettingsDialog : public QDialog {
    Q_OBJECT
public:
    SettingsDialog(const QString& lspPath, const QString& snippetPath, const QString& keymapPath,
                   const QList<QPair<QString, QString>>& actions, QWidget* parent = nullptr);

    void save() const;          // 把三個表格寫回 JSON 檔（accept 後由 MainWindow 呼叫）

private:
    QTableWidget* makeTable(const QStringList& headers);
    void addRow(QTableWidget* t, const QStringList& values);
    void loadLsp();
    void loadSnippets();
    void loadKeys(const QList<QPair<QString, QString>>& actions);

    QString m_lspPath, m_snippetPath, m_keymapPath;
    QTableWidget* m_lspTable = nullptr;
    QTableWidget* m_snipTable = nullptr;
    QTableWidget* m_keyTable = nullptr;
};
