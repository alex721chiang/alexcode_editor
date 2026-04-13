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
#include <QFontDialog>
#include <QGridLayout>
#include <QMessageBox>
#include <QFileInfo>
#include <QVBoxLayout>
#include <QLabel>
#include <QStyle>
#include <QSettings>
#include <QProcess>
#include <QPointer>
#include <QTimer>
#include "AICompletionProvider.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), findDialog(nullptr), findInFilesDialog(nullptr)
{
    defaultEditorFont = loadFont();
    isFontSet = true;
    setupUI();
    resize(800, 600);
}

CodeEditor* MainWindow::activeEditor() {
    return qobject_cast<CodeEditor*>(tabWidget->currentWidget());
}

void MainWindow::saveFont(const QFont& font) {
    QSettings settings("AlexCode", "AlexCodeEditor");
    settings.setValue("font/family", font.family());
    settings.setValue("font/pointSize", font.pointSize());
    settings.setValue("font/bold", font.bold());
    settings.setValue("font/italic", font.italic());
}

QFont MainWindow::loadFont() {
    QSettings settings("AlexCode", "AlexCodeEditor");
    QFont font;
    font.setFamily(settings.value("font/family", "Courier New").toString());
    font.setPointSize(settings.value("font/pointSize", 11).toInt());
    font.setBold(settings.value("font/bold", false).toBool());
    font.setItalic(settings.value("font/italic", false).toBool());
    return font;
}

void MainWindow::applyFontToAllTabs(const QFont& font) {
    for (int i = 0; i < tabWidget->count(); ++i) {
        if (auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
            editor->setFont(font);
    }
}

void MainWindow::applyHighlighterForPath(CodeEditor* editor, const QString& filePath) {
    if (!editor) return;
    SyntaxHighlighter::Language lang = SyntaxHighlighter::detectLanguage(filePath);
    if (lang == SyntaxHighlighter::Language::Unknown) return;
    SyntaxHighlighter* highlighter = new SyntaxHighlighter(editor->document());
    highlighter->setLanguage(lang);
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

    newAction = new QAction("New File", this);
    newAction->setShortcut(QKeySequence::New);
    newAction->setIcon(style()->standardIcon(QStyle::SP_FileIcon));
    connect(newAction, &QAction::triggered, this, [this]() {
        CodeEditor* editor = new CodeEditor(this);
        editor->setFont(defaultEditorFont);
        editor->setAIProvider(aiProvider);
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

    findInFilesAction = new QAction("Find in Files", this);
    findInFilesAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_F));
    findInFilesAction->setIcon(style()->standardIcon(QStyle::SP_DirIcon));
    connect(findInFilesAction, &QAction::triggered, this, &MainWindow::showFindInFilesDialog);

    fontAction = new QAction("Font...", this);
    connect(fontAction, &QAction::triggered, this, &MainWindow::showFontDialog);

    wrapAction = new QAction("Word Wrap", this);
    wrapAction->setCheckable(true);
    wrapAction->setChecked(true);
    connect(wrapAction, &QAction::triggered, this, [this](bool checked) {
        if (auto editor = activeEditor())
            editor->setLineWrapMode(checked ? QPlainTextEdit::WidgetWidth : QPlainTextEdit::NoWrap);
    });

    QMenuBar* menuBar = new QMenuBar(this);
    menuBar->setNativeMenuBar(false);
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
    editMenu->addAction(findInFilesAction);

    QMenu* viewMenu = menuBar->addMenu("View");
    viewMenu->addAction(wrapAction);
    viewMenu->addAction(fontAction);

    setupToolBar();

    aiProvider = new AICompletionProvider(this);

    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);
    connect(resultsList, &QListWidget::itemDoubleClicked, this, &MainWindow::onResultDoubleClicked);

    CodeEditor* initialEditor = new CodeEditor(this);
    initialEditor->setFont(defaultEditorFont);
    initialEditor->setAIProvider(aiProvider);
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
    openFileByPath(fileName);
}

void MainWindow::openFileByPath(const QString& fileName) {
    QFile file(fileName);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QMessageBox::warning(this, "Error", "Cannot open file:\n" + file.errorString());
        return;
    }
    QTextStream in(&file);
    QString text = in.readAll();
    file.close();

    for (int i = 0; i < tabWidget->count(); ++i) {
        CodeEditor* editor = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (editor && editor->property("filePath").toString() == fileName) {
            tabWidget->setCurrentIndex(i);
            return;
        }
    }

    CodeEditor* newEditor = new CodeEditor(this);
    newEditor->setFont(defaultEditorFont);
    newEditor->setAIProvider(aiProvider);
    newEditor->setPlainText(text);

    QFileInfo fileInfo(fileName);
    int tabIndex = tabWidget->addTab(newEditor, fileInfo.fileName());
    newEditor->setProperty("filePath", fileName);
    tabWidget->setTabToolTip(tabIndex, fileName);
    tabWidget->setCurrentIndex(tabIndex);

    applyHighlighterForPath(newEditor, fileName);
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
        applyHighlighterForPath(currentEditor, fileName);
    }

    QString originalText = currentEditor->toPlainText();

    auto doSave = [this](const QString& fileToSave, const QString& textToSave) {
        QFile file(fileToSave);
        if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QMessageBox::warning(this, "Error", "Cannot save file:\n" + file.errorString());
            return;
        }
        QTextStream out(&file);
        out << textToSave;
        file.close();
    };

    if (fileName.endsWith(".cpp") || fileName.endsWith(".h") || fileName.endsWith(".hpp") || fileName.endsWith(".c")) {
        QProcess* formatProcess = new QProcess(this);
        formatProcess->setProgram("clang-format");

        QPointer<CodeEditor> safeEditor(currentEditor);

        connect(formatProcess, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
                this, [this, safeEditor, fileName, originalText, formatProcess](int exitCode, QProcess::ExitStatus exitStatus) {
            
            QString textToSave = originalText;
            
            if (exitStatus == QProcess::NormalExit && exitCode == 0) {
                QString formattedText = QString::fromUtf8(formatProcess->readAllStandardOutput());
                if (!formattedText.isEmpty() && formattedText != originalText) {
                    if (safeEditor) {
                        QTextCursor cursor = safeEditor->textCursor();
                        cursor.beginEditBlock();
                        int position = cursor.position();
                        cursor.select(QTextCursor::Document);
                        cursor.insertText(formattedText);
                        cursor.setPosition(qMin(position, safeEditor->document()->characterCount() - 1));
                        safeEditor->setTextCursor(cursor);
                        cursor.endEditBlock();
                        textToSave = safeEditor->toPlainText();
                    } else {
                        textToSave = formattedText;
                    }
                } else if (safeEditor) {
                    textToSave = safeEditor->toPlainText();
                }
            } else if (safeEditor) {
                textToSave = safeEditor->toPlainText();
            }

            QFile file(fileName);
            if (file.open(QIODevice::WriteOnly | QIODevice::Text)) {
                QTextStream out(&file);
                out << textToSave;
                file.close();
            } else {
                qWarning() << "Cannot save file:" << file.errorString();
            }

            formatProcess->deleteLater();
        });

        // 處理超時保護
        QTimer* timeoutTimer = new QTimer(formatProcess);
        timeoutTimer->setSingleShot(true);
        connect(timeoutTimer, &QTimer::timeout, formatProcess, [formatProcess]() {
            if (formatProcess->state() == QProcess::Running) {
                formatProcess->kill();
            }
        });

        formatProcess->start();
        if (formatProcess->waitForStarted(500)) {
            formatProcess->write(originalText.toUtf8());
            formatProcess->closeWriteChannel();
            timeoutTimer->start(2000); // 2秒超時
        } else {
            doSave(fileName, originalText);
            formatProcess->deleteLater();
        }
    } else {
        doSave(fileName, originalText);
    }
}

void MainWindow::showFindDialog() {
    if (!findDialog) {
        findDialog = new QDialog(this);
        findDialog->setWindowTitle("Find & Replace");
        QGridLayout* layout = new QGridLayout(findDialog);
        layout->addWidget(new QLabel("Find:", findDialog), 0, 0);
        findInput = new QLineEdit(findDialog);
        layout->addWidget(findInput, 0, 1);
        layout->addWidget(new QLabel("Replace:", findDialog), 1, 0);
        replaceInput = new QLineEdit(findDialog);
        layout->addWidget(replaceInput, 1, 1);
        QPushButton* findNextBtn   = new QPushButton("Find Next", findDialog);
        QPushButton* replaceBtn    = new QPushButton("Replace", findDialog);
        QPushButton* replaceAllBtn = new QPushButton("Replace All", findDialog);
        QHBoxLayout* btnLayout = new QHBoxLayout();
        btnLayout->addWidget(findNextBtn);
        btnLayout->addWidget(replaceBtn);
        btnLayout->addWidget(replaceAllBtn);
        layout->addLayout(btnLayout, 2, 0, 1, 2);
        connect(findNextBtn,   &QPushButton::clicked, this, &MainWindow::performFind);
        connect(replaceBtn,    &QPushButton::clicked, this, &MainWindow::performReplace);
        connect(replaceAllBtn, &QPushButton::clicked, this, &MainWindow::performReplaceAll);
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
            QTextCursor cursor = editor->textCursor();
            cursor.movePosition(QTextCursor::Start);
            editor->setTextCursor(cursor);
            if (!editor->find(textToFind))
                QMessageBox::information(this, "Find", "Cannot find \"" + textToFind + "\"");
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
            QListWidgetItem* item = new QListWidgetItem(
                QString("Line %1: %2").arg(lineIndex + 1).arg(block.text()));
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

void MainWindow::showFontDialog() {
    bool ok;
    QFont font = QFontDialog::getFont(&ok, defaultEditorFont, this);
    if (ok) {
        defaultEditorFont = font;
        isFontSet = true;
        applyFontToAllTabs(font);
        saveFont(font);
    }
}

void MainWindow::performReplace() {
    if (auto editor = activeEditor()) {
        QString findText    = findInput->text();
        QString replaceText = replaceInput->text();
        if (findText.isEmpty()) return;
        QTextCursor cursor = editor->textCursor();
        if (cursor.hasSelection() && cursor.selectedText() == findText)
            cursor.insertText(replaceText);
        performFind();
    }
}

void MainWindow::performReplaceAll() {
    if (auto editor = activeEditor()) {
        QString findText    = findInput->text();
        QString replaceText = replaceInput->text();
        if (findText.isEmpty()) return;
        QTextCursor cursor = editor->textCursor();
        cursor.beginEditBlock();
        cursor.movePosition(QTextCursor::Start);
        editor->setTextCursor(cursor);
        int count = 0;
        while (editor->find(findText)) {
            editor->textCursor().insertText(replaceText);
            count++;
        }
        cursor.endEditBlock();
        QMessageBox::information(this, "Replace All",
            QString::number(count) + " replacements made.");
    }
}

void MainWindow::showFindInFilesDialog() {
    if (!findInFilesDialog) {
        findInFilesDialog = new FindInFilesDialog(this);
        connect(findInFilesDialog, &FindInFilesDialog::resultDoubleClicked,
                this, &MainWindow::onFindInFilesResultDoubleClicked);
    }
    findInFilesDialog->show();
    findInFilesDialog->raise();
    findInFilesDialog->activateWindow();
}

void MainWindow::onFindInFilesResultDoubleClicked(QListWidgetItem* item) {
    QVariantMap data = item->data(Qt::UserRole).toMap();
    if (data.isEmpty()) return;
    QString filePath = data.value("filePath").toString();
    int lineNum      = data.value("lineNum").toInt();
    openFileByPath(filePath);
    if (auto editor = activeEditor()) {
        QTextBlock block = editor->document()->findBlockByNumber(lineNum - 1);
        if (block.isValid()) {
            QTextCursor cursor(block);
            editor->setTextCursor(cursor);
            editor->ensureCursorVisible();
            editor->setFocus();
        }
    }
}
