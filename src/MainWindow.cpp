#include "MainWindow.h"
#include <QToolBar>
#include <QDockWidget>
#include <QTextCursor>
#include <QTextBlock>
#include <QMenuBar>
#include <QMenu>
#include <QAction>
#include <QFileDialog>
#include <QFile>
#include <QTextStream>
#include <QMessageBox>
#include <QFileInfo>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setupUI();
    resize(800, 600);
}

void MainWindow::setupUI() {
    tabWidget = new QTabWidget(this);
    setCentralWidget(tabWidget);

    QMenuBar* menuBar = new QMenuBar(this);
    setMenuBar(menuBar);
    QMenu* fileMenu = menuBar->addMenu("File");
    
    QAction* openAction = new QAction("Open", this);
    fileMenu->addAction(openAction);
    connect(openAction, &QAction::triggered, this, &MainWindow::openFile);

    QAction* saveAction = new QAction("Save", this);
    fileMenu->addAction(saveAction);
    connect(saveAction, &QAction::triggered, this, &MainWindow::saveFile);

    QToolBar* toolbar = addToolBar("Filter");
    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText("Enter keywords separated by |");
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    filterBtn = new QPushButton("Filter", this);

    toolbar->addWidget(filterInput);
    toolbar->addWidget(logicCombo);
    toolbar->addWidget(filterBtn);

    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);

    connect(filterBtn, &QPushButton::clicked, this, &MainWindow::runFilter);
    connect(resultsList, &QListWidget::itemDoubleClicked, this, &MainWindow::onResultDoubleClicked);
}

void MainWindow::openFile() {
    QString fileName = QFileDialog::getOpenFileName(this, "Open File");
    if (fileName.isEmpty()) return;

    QFile file(fileName);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QMessageBox::warning(this, "Error", "Cannot open file:\n" + file.errorString());
        return;
    }

    QTextStream in(&file);
    QString text = in.readAll();
    file.close();

    QPlainTextEdit* newEditor = new QPlainTextEdit(this);
    newEditor->setPlainText(text);
    
    QFileInfo fileInfo(fileName);
    int tabIndex = tabWidget->addTab(newEditor, fileInfo.fileName());
    tabWidget->setTabToolTip(tabIndex, fileName);
    tabWidget->setCurrentIndex(tabIndex);
}

void MainWindow::saveFile() {
    QPlainTextEdit* currentEditor = qobject_cast<QPlainTextEdit*>(tabWidget->currentWidget());
    if (!currentEditor) return;

    QString fileName = tabWidget->tabToolTip(tabWidget->currentIndex());
    if (fileName.isEmpty()) {
        fileName = QFileDialog::getSaveFileName(this, "Save File");
        if (fileName.isEmpty()) return;
        
        QFileInfo fileInfo(fileName);
        tabWidget->setTabText(tabWidget->currentIndex(), fileInfo.fileName());
        tabWidget->setTabToolTip(tabWidget->currentIndex(), fileName);
    }

    QFile file(fileName);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, "Error", "Cannot save file:\n" + file.errorString());
        return;
    }

    QTextStream out(&file);
    out << currentEditor->toPlainText();
    file.close();
}

void MainWindow::runFilter() {
    resultsList->clear();
    QStringList keywords = filterInput->text().split("|", Qt::SkipEmptyParts);
    for (int i = 0; i < keywords.size(); ++i) keywords[i] = keywords[i].trimmed();
    
    engine.setKeywords(keywords);
    engine.setLogic(logicCombo->currentText() == "AND" ? FilterLogic::AND : FilterLogic::OR);
    
    QPlainTextEdit* editor = qobject_cast<QPlainTextEdit*>(tabWidget->currentWidget());
    if (!editor) return;

    resultsList->setUpdatesEnabled(false);
    
    QTextBlock block = editor->document()->begin();
    int lineIndex = 0;
    while (block.isValid()) {
        if (engine.matchLine(block.text())) {
            QListWidgetItem* item = new QListWidgetItem(QString("Line %1: %2").arg(lineIndex + 1).arg(block.text()));
            item->setData(Qt::UserRole, lineIndex);
            resultsList->addItem(item);
        }
        block = block.next();
        lineIndex++;
    }

    resultsList->setUpdatesEnabled(true);
}

void MainWindow::onResultDoubleClicked(QListWidgetItem* item) {
    QPlainTextEdit* editor = qobject_cast<QPlainTextEdit*>(tabWidget->currentWidget());
    if (!editor) return;

    int line = item->data(Qt::UserRole).toInt();
    QTextCursor cursor(editor->document()->findBlockByNumber(line));
    editor->setTextCursor(cursor);
    editor->centerCursor();
    editor->setFocus();
}