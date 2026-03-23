#include "MainWindow.h"
#include <QToolBar>
#include <QDockWidget>
#include <QTextCursor>
#include <QTextBlock>

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

void MainWindow::runFilter() {
    resultsList->clear();
    QStringList keywords = filterInput->text().split("|", Qt::SkipEmptyParts);
    for (int i = 0; i < keywords.size(); ++i) keywords[i] = keywords[i].trimmed();
    
    engine.setKeywords(keywords);
    engine.setLogic(logicCombo->currentText() == "AND" ? FilterLogic::AND : FilterLogic::OR);
    
    QString text = editor->toPlainText();
    QStringList lines = text.split("\n");
    for (int i = 0; i < lines.size(); ++i) {
        if (engine.matchLine(lines[i])) {
            QListWidgetItem* item = new QListWidgetItem(QString("Line %1: %2").arg(i + 1).arg(lines[i]));
            item->setData(Qt::UserRole, i); // Store line number (0-indexed) for safe click-to-jump
            resultsList->addItem(item);
        }
    }
}

void MainWindow::onResultDoubleClicked(QListWidgetItem* item) {
    int line = item->data(Qt::UserRole).toInt();
    QTextCursor cursor(editor->document()->findBlockByNumber(line));
    editor->setTextCursor(cursor);
    editor->centerCursor();
    editor->setFocus();
}
