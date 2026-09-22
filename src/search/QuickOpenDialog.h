#pragma once
#include <QDialog>
#include <QLineEdit>
#include <QListWidget>
#include <QStringList>
#include <QFutureWatcher>

// Ctrl+P 快速開檔：背景建立檔名索引，FilterEngine::fuzzyContains 模糊比對
class QuickOpenDialog : public QDialog {
    Q_OBJECT
public:
    explicit QuickOpenDialog(QWidget* parent = nullptr);
    void setRootFolder(const QString& folder);   // 設定/重建索引
    void open() ;                                 // 顯示並聚焦

signals:
    void fileChosen(const QString& filePath);

protected:
    bool eventFilter(QObject* obj, QEvent* event) override;

private slots:
    void refreshResults(const QString& query);
    void acceptCurrent();

private:
    static QStringList buildIndex(const QString& folder);

    QLineEdit* input;
    QListWidget* list;
    QString rootFolder;
    QStringList fileIndex;                       // 相對路徑
    QFutureWatcher<QStringList>* indexWatcher;
};
