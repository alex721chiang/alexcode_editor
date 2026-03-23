#pragma once
#include <QMainWindow>
#include <QPlainTextEdit>
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
private:
    QPlainTextEdit* editor;
    QListWidget* resultsList;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    QPushButton* filterBtn;
    FilterEngine engine;
    void setupUI();
};
