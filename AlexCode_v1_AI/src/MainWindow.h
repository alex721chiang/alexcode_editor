#pragma once
#include <QMainWindow>
#include <QTabWidget>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QDialog>
#include <QAction>
#include <QToolBar>
#include <QFont>
#include <QSettings>
#include "FilterEngine.h"
#include "CodeEditor.h"
#include "FindInFilesDialog.h"
#include "SyntaxHighlighter.h"

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
private slots:
    void runFilter();
    void onResultDoubleClicked(QListWidgetItem* item);
    void openFile();
    void openFileByPath(const QString& filePath);
    void saveFile();
    void showFindDialog();
    void performFind();
    void performReplace();
    void performReplaceAll();
    void showFontDialog();
    void showFindInFilesDialog();
    void onFindInFilesResultDoubleClicked(QListWidgetItem* item);
private:
    QTabWidget* tabWidget;
    QListWidget* resultsList = nullptr;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    QPushButton* filterBtn;
    FilterEngine engine;
    AICompletionProvider* aiProvider;

    QDialog* findDialog;
    QLineEdit* findInput;
    QLineEdit* replaceInput;
    FindInFilesDialog* findInFilesDialog;

    // Actions
    QAction* newAction;
    QAction* openAction;
    QAction* saveAction;
    QAction* undoAction;
    QAction* redoAction;
    QAction* cutAction;
    QAction* copyAction;
    QAction* pasteAction;
    QAction* findAction;
    QAction* findInFilesAction;
    QAction* wrapAction;
    QAction* fontAction;
    QFont defaultEditorFont;
    bool isFontSet = false;

    void setupUI();
    void setupToolBar();
    CodeEditor* activeEditor();

    // Feature 1: font persistence
    void saveFont(const QFont& font);
    QFont loadFont();
    void applyFontToAllTabs(const QFont& font);

    // Feature 2: auto syntax highlighting
    void applyHighlighterForPath(CodeEditor* editor, const QString& filePath);
};
