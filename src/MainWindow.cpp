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
#include <QStyle>

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

    // Initialize Actions
    newAction = new QAction("New File", this);
    newAction->setShortcut(QKeySequence::New);
    newAction->setIcon(style()->standardIcon(QStyle::SP_FileIcon));
    connect(newAction, &QAction::triggered, this, [this]() {
        CodeEditor* editor = new CodeEditor(this);
        tabWidget->addTab(editor, "Untitled");
        tabWidget->setCurrentWidget(editor);
    });

    openAction = new QAction("Open", this);
    openAction->setShortcut(QKeySequence::Open);
    openAction->setIcon(style()->standardIcon(QStyle::SP_DirOpenIcon));
    connect(openAction, &QAction::triggered, this, &MainWindow::openFile);

    saveAction = new QAction("Save", this);
    saveAction->setShortcut(QKeySequence::Save);
    saveAction->setIcon(style()->standardIcon(QStyle::SP_DialogSaveButton));
    connect(saveAction, &QAction::triggered, this, &MainWindow::saveFile);

    undoAction = new QAction("Undo", this);
    undoAction->setShortcut(QKeySequence::Undo);
    undoAction->setIcon(style()->standardIcon(QStyle::SP_ArrowBack));
    connect(undoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->undo();
    });

    redoAction = new QAction("Redo", this);
    redoAction->setShortcut(QKeySequence::Redo);
    redoAction->setIcon(style()->standardIcon(QStyle::SP_ArrowForward));
    connect(redoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->redo();
    });

    cutAction = new QAction("Cut", this);
    cutAction->setShortcut(QKeySequence::Cut);
    connect(cutAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->cut();
    });

    copyAction = new QAction("Copy", this);
    copyAction->setShortcut(QKeySequence::Copy);
    connect(copyAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->copy();
    });

    pasteAction = new QAction("Paste", this);
    pasteAction->setShortcut(QKeySequence::Paste);
    connect(pasteAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->paste();
    });

    findAction = new QAction("Find", this);
    findAction->setShortcut(QKeySequence::Find);
    findAction->setIcon(style()->standardIcon(QStyle::SP_FileDialogContentsView));
    connect(findAction, &QAction::triggered, this, &MainWindow::showFindDialog);

    wrapAction = new QAction("Word Wrap", this);
    wrapAction->setCheckable(true);
    wrapAction->setChecked(true);
    connect(wrapAction, &QAction::triggered, this, [this](bool checked) {
        if (auto editor = activeEditor()) {
            editor->setLineWrapMode(checked ? QPlainTextEdit::WidgetWidth : QPlainTextEdit::NoWrap);
        }
    });

    // Menu Bar
    QMenuBar* menuBar = new QMenuBar(this);
    menuBar->setNativeMenuBar(false); // Force menu bar to appear inside the window on macOS
    setMenuBar(menuBar);

    QMenu* fileMenu = menuBar->addMenu("File");
    fileMenu->addAction(newAction);
    fileMenu->addAction(openAction);
    fileMenu->addAction(saveAction);

    QMenu* editMenu = menuBar->addMenu("Edit");
    editMenu->addAction(undoAction);
    editMenu->addAction(redoAction);
    editMenu->addSeparator();
    editMenu->addAction(cutAction);
    editMenu->addAction(copyAction);
    editMenu->addAction(pasteAction);
    editMenu->addSeparator();
    editMenu->addAction(findAction);

    QMenu* viewMenu = menuBar->addMenu("View");
    viewMenu->addAction(wrapAction);

    // Toolbars
    setupToolBar();

    // Filter Results Dock
    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);

    connect(resultsList, &QListWidget::itemDoubleClicked, this, &MainWindow::onResultDoubleClicked);

    CodeEditor* initialEditor = new CodeEditor(this);
    tabWidget->addTab(initialEditor, "Untitled");
}

void MainWindow::setupToolBar() {
    QToolBar* mainToolBar = addToolBar("Main");
    mainToolBar->addAction(newAction);
    mainToolBar->addAction(openAction);
    mainToolBar->addAction(saveAction);
    mainToolBar->addSeparator();
    mainToolBar->addAction(undoAction);
    mainToolBar->addAction(redoAction);
    mainToolBar->addSeparator();
    mainToolBar->addAction(findAction);

    QToolBar* filterToolBar = addToolBar("Filter");
    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText("Enter keywords separated by |");
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    filterBtn = new QPushButton("Filter", this);

    filterToolBar->addWidget(filterInput);
    filterToolBar->addWidget(logicCombo);
    filterToolBar->addWidget(filterBtn);

    connect(filterBtn, &QPushButton::clicked, this, &MainWindow::runFilter);
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
