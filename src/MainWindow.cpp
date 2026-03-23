#include "MainWindow.h"
#include <QToolBar>
#include <QDockWidget>
#include <QPushButton>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setupUI();
    resize(800, 600);
}

void MainWindow::setupUI() {
    editor = new QPlainTextEdit(this);
    setCentralWidget(editor);

    QToolBar* toolbar = addToolBar("Filter");
    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText("Enter keywords separated by |");
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    QPushButton* filterBtn = new QPushButton("Filter", this);

    toolbar->addWidget(filterInput);
    toolbar->addWidget(logicCombo);
    toolbar->addWidget(filterBtn);

    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);
}