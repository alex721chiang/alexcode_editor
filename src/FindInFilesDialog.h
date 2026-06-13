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

class FindInFilesDialog : public QDialog {
    Q_OBJECT
public:
    explicit FindInFilesDialog(QWidget *parent = nullptr);
    QListWidget* getResultsList() const { return resultsList; }

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

    void setupUI();
};
