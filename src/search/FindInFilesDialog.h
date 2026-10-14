#pragma once

#include <QDialog>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QFileDialog>
#include <QDirIterator>
#include <QTextStream>
#include <QFileInfo>
#include <QVariantMap>
#include <memory>

class QTimer;

class FindInFilesDialog : public QDialog {
    Q_OBJECT
public:
    explicit FindInFilesDialog(QWidget *parent = nullptr);
    ~FindInFilesDialog() override;
    QListWidget* getResultsList() const { return resultsList; }
    bool isBusy() const { return m_busy; }        // 搜尋/取代進行中（背景執行）

signals:
    void resultDoubleClicked(QListWidgetItem* item);

private slots:
    void browseDirectory();
    void performSearch();
    void performReplaceAll();          // 2.8 Replace in Files

private:
    QLineEdit* dirInput;
    QPushButton* browseBtn;
    QLineEdit* filterInput;
    QLineEdit* searchInput;
    QLineEdit* replaceInput;
    QListWidget* resultsList;
    QPushButton* searchBtn;
    QPushButton* replaceBtn;
    QLabel* statusLabel = nullptr;
    QTimer* drainTimer = nullptr;                  // 定期把背景找到的結果加入清單

    struct Job;                                    // 背景工作的共享狀態（取消旗標、待加入結果）
    std::shared_ptr<Job> m_job;
    bool m_busy = false;

    void setupUI();
    void setBusy(bool busy, const QString& status);
    void drainResults();
    QStringList nameFilters() const;
};
