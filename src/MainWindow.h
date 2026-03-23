#pragma once
#include <QMainWindow>
#include <QPlainTextEdit>
#include <QTabWidget>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include "FilterEngine.h"

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
private slots:
    void runFilter();
    void onResultDoubleClicked(QListWidgetItem* item);
    void openFile();
    void saveFile();
private:
    QTabWidget* tabWidget;
    QListWidget* resultsList = nullptr;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    QPushButton* filterBtn;
    FilterEngine engine;
    void setupUI();
};

