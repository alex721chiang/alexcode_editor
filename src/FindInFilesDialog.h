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

private:
    QLineEdit* dirInput;
    QPushButton* browseBtn;
    QLineEdit* filterInput;
    QLineEdit* searchInput;
    QListWidget* resultsList;
    QPushButton* searchBtn;

    void setupUI();
};
