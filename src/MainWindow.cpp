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
#include <QVBoxLayout>
#include <QLabel>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent), findDialog(nullptr) {
    setupUI();
    resize(800, 600);
}

CodeEditor* MainWindow::activeEditor() {
    return qobject_cast<CodeEditor*>(tabWidget->currentWidget());
}

void MainWindow::setupUI() {
    tabWidget = new QTabWidget(this);
    tabWidget->setTabsClosable(true);
    connect(tabWidget, &QTabWidget::tabCloseRequested, this, [this](int index) {
        auto widget = tabWidget->widget(index);
        tabWidget->removeTab(index);
        widget->deleteLater();
    });
    connect(tabWidget, &QTabWidget::currentChanged, this, [this](int) {
        if (resultsList) resultsList->clear();
    });
    setCentralWidget(tabWidget);

    QMenuBar* menuBar = new QMenuBar(this);
    setMenuBar(menuBar);

    // File Menu
    QMenu* fileMenu = menuBar->addMenu("File");
    
    QAction* newAction = new QAction("New File", this);
    newAction->setShortcut(QKeySequence::New);
    fileMenu->addAction(newAction);
    connect(newAction, &QAction::triggered, this, [this]() {
        CodeEditor* editor = new CodeEditor(this);
        tabWidget->addTab(editor, "Untitled");
        tabWidget->setCurrentWidget(editor);
    });

    QAction* openAction = new QAction("Open", this);
    openAction->setShortcut(QKeySequence::Open);
    fileMenu->addAction(openAction);
    connect(openAction, &QAction::triggered, this, &MainWindow::openFile);

    QAction* saveAction = new QAction("Save", this);
    saveAction->setShortcut(QKeySequence::Save);
    fileMenu->addAction(saveAction);
    connect(saveAction, &QAction::triggered, this, &MainWindow::saveFile);

    // Edit Menu
    QMenu* editMenu = menuBar->addMenu("Edit");
    
    QAction* undoAction = new QAction("Undo", this);
    undoAction->setShortcut(QKeySequence::Undo);
    editMenu->addAction(undoAction);
    connect(undoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->undo();
    });

    QAction* redoAction = new QAction("Redo", this);
    redoAction->setShortcut(QKeySequence::Redo);
    editMenu->addAction(redoAction);
    connect(redoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->redo();
    });
    
    editMenu->addSeparator();

    QAction* cutAction = new QAction("Cut", this);
    cutAction->setShortcut(QKeySequence::Cut);
    editMenu->addAction(cutAction);
    connect(cutAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->cut();
    });

    QAction* copyAction = new QAction("Copy", this);
    copyAction->setShortcut(QKeySequence::Copy);
    editMenu->addAction(copyAction);
    connect(copyAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->copy();
    });

    QAction* pasteAction = new QAction("Paste", this);
    pasteAction->setShortcut(QKeySequence::Paste);
    editMenu->addAction(pasteAction);
    connect(pasteAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->paste();
    });

    editMenu->addSeparator();

    QAction* findAction = new QAction("Find", this);
    findAction->setShortcut(QKeySequence::Find);
    editMenu->addAction(findAction);
    connect(findAction, &QAction::triggered, this, &MainWindow::showFindDialog);

    // View Menu
    QMenu* viewMenu = menuBar->addMenu("View");
    QAction* wrapAction = new QAction("Word Wrap", this);
    wrapAction->setCheckable(true);
    wrapAction->setChecked(true); // default QPlainTextEdit wraps
    viewMenu->addAction(wrapAction);
    connect(wrapAction, &QAction::triggered, this, [this](bool checked) {
        if (auto editor = activeEditor()) {
            editor->setLineWrapMode(checked ? QPlainTextEdit::WidgetWidth : QPlainTextEdit::NoWrap);
        }
    });

    // Toolbar
    QToolBar* toolbar = addToolBar("Filter");
    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText("Enter keywords separated by |");
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    filterBtn = new QPushButton("Filter", this);

    toolbar->addWidget(filterInput);
    toolbar->addWidget(logicCombo);
    toolbar->addWidget(filterBtn);

    // Filter Results Dock
    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);

    connect(filterBtn, &QPushButton::clicked, this, &MainWindow::runFilter);
    connect(resultsList, &QListWidget::itemDoubleClicked, this, &MainWindow::onResultDoubleClicked);

    CodeEditor* initialEditor = new CodeEditor(this);
    tabWidget->addTab(initialEditor, "Untitled");
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

    CodeEditor* newEditor = new CodeEditor(this);
    newEditor->setPlainText(text);
    
    QFileInfo fileInfo(fileName);
    int tabIndex = tabWidget->addTab(newEditor, fileInfo.fileName());
    newEditor->setProperty("filePath", fileName);
    tabWidget->setTabToolTip(tabIndex, fileName);
    tabWidget->setCurrentIndex(tabIndex);
}

void MainWindow::saveFile() {
    CodeEditor* currentEditor = activeEditor();
    if (!currentEditor) return;

    QString fileName = currentEditor->property("filePath").toString();
    if (fileName.isEmpty()) {
        fileName = QFileDialog::getSaveFileName(this, "Save File");
        if (fileName.isEmpty()) return;
        
        QFileInfo fileInfo(fileName);
        tabWidget->setTabText(tabWidget->currentIndex(), fileInfo.fileName());
        currentEditor->setProperty("filePath", fileName);
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

void MainWindow::showFindDialog() {
    if (!findDialog) {
        findDialog = new QDialog(this);
        findDialog->setWindowTitle("Find");
        QVBoxLayout* layout = new QVBoxLayout(findDialog);
        findInput = new QLineEdit(findDialog);
        QPushButton* findNextBtn = new QPushButton("Find Next", findDialog);
        layout->addWidget(new QLabel("Find:"));
        layout->addWidget(findInput);
        layout->addWidget(findNextBtn);
        connect(findNextBtn, &QPushButton::clicked, this, &MainWindow::performFind);
    }
    findDialog->show();
    findDialog->raise();
    findDialog->activateWindow();
    findInput->setFocus();
}

void MainWindow::performFind() {
    if (auto editor = activeEditor()) {
        QString textToFind = findInput->text();
        if (!editor->find(textToFind)) {
            // Restart from top if not found
            QTextCursor cursor = editor->textCursor();
            cursor.movePosition(QTextCursor::Start);
            editor->setTextCursor(cursor);
            if (!editor->find(textToFind)) {
                QMessageBox::information(this, "Find", "Cannot find \"" + textToFind + "\"");
            }
        }
    }
}

void MainWindow::runFilter() {
    resultsList->clear();
    QStringList keywords = filterInput->text().split("|", Qt::SkipEmptyParts);
    for (int i = 0; i < keywords.size(); ++i) keywords[i] = keywords[i].trimmed();
    
    engine.setKeywords(keywords);
    engine.setLogic(logicCombo->currentText() == "AND" ? FilterLogic::AND : FilterLogic::OR);
    
    CodeEditor* editor = activeEditor();
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
    CodeEditor* editor = activeEditor();
    if (!editor) return;

    int line = item->data(Qt::UserRole).toInt();
    QTextCursor cursor(editor->document()->findBlockByNumber(line));
    editor->setTextCursor(cursor);
    editor->centerCursor();
    editor->setFocus();
}