#pragma once
#include <QMainWindow>
#include <QPlainTextEdit>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
private:
    QPlainTextEdit* editor;
    QListWidget* resultsList;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    void setupUI();
};