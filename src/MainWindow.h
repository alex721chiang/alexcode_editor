#pragma once
#include <QMainWindow>
#include <QTabWidget>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QDialog>
#include "FilterEngine.h"
#include "CodeEditor.h"

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
private slots:
    void runFilter();
    void onResultDoubleClicked(QListWidgetItem* item);
    void openFile();
    void saveFile();
    void showFindDialog();
    void performFind();
private:
    QTabWidget* tabWidget;
    QListWidget* resultsList = nullptr;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    QPushButton* filterBtn;
    FilterEngine engine;
    
    QDialog* findDialog;
    QLineEdit* findInput;
    
    void setupUI();
    CodeEditor* activeEditor();
};

