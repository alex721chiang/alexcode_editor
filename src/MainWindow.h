#pragma once
#include <QMainWindow>
#include <QTabWidget>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QDialog>
#include <QAction>
#include <QToolBar>
#include <QLabel>
#include <QFont>
#include <QSettings>
#include <QPointer>
#include <QTimer>
#include <QProcess>
#include <QPlainTextEdit>
#include "FilterEngine.h"
#include "CodeEditor.h"
#include "FindInFilesDialog.h"
#include "SyntaxHighlighter.h"
#include "QuickOpenDialog.h"

class QTreeView;
class QFileSystemModel;
class GitFileSystemModel;
class QFileSystemWatcher;
class QDockWidget;
class LspManager;

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);

protected:
    void closeEvent(QCloseEvent* event) override;
    void dragEnterEvent(QDragEnterEvent* event) override;
    void dropEvent(QDropEvent* event) override;
    bool eventFilter(QObject* obj, QEvent* event) override;

private slots:
    void runFilter();
    void onResultDoubleClicked(QListWidgetItem* item);
    void newFile();
    void openFile();
    void openFileByPath(const QString& filePath);
    void saveFile();
    void saveFileAs();
    void saveAllFiles();
    bool closeTab(int index);
    void closeAllTabs();
    void showFindDialog();
    void performFind();
    void performFindPrev();
    void performReplace();
    void performReplaceAll();
    void showGotoLineDialog();
    void showFontDialog();
    void showFindInFilesDialog();
    void onFindInFilesResultDoubleClicked(QListWidgetItem* item);
    void openRecentFile();
    void updateStatusBar();
    void onModificationChanged(bool modified);
    void openFolder();
    void showQuickOpen();
    void onFileChangedExternally(const QString& path);
    void navigateBack();
    void navigateForward();
    void showPreferences();
    void showDiff(const QString& titleA, const QString& a,
                  const QString& titleB, const QString& b);
    void runExternalTool();
    void runBuildTask();
    void runCommand(const QString& cmd);
    void updateGitStatus();
    void showSymbolList();

private:
    QTabWidget* tabWidget = nullptr;
    QListWidget* resultsList = nullptr;
    QLineEdit* filterInput = nullptr;
    QComboBox* logicCombo = nullptr;
    QCheckBox* fuzzyCheck = nullptr;
    QPushButton* filterBtn = nullptr;
    QLabel* filterCountLabel = nullptr;
    QComboBox* presetCombo = nullptr;                 // 5.4 篩選預設集
    class TimelineBar* timelineBar = nullptr;         // 5.6 命中密度條
    void loadFilterPresets();                         // 重建 presetCombo 項目
    FilterEngine engine;
    AICompletionProvider* aiProvider = nullptr;

    // Find & Replace
    QDialog* findDialog;
    QLineEdit* findInput = nullptr;
    QLineEdit* replaceInput = nullptr;
    QCheckBox* caseCheck = nullptr;
    QCheckBox* wholeWordCheck = nullptr;
    QCheckBox* regexCheck = nullptr;
    FindInFilesDialog* findInFilesDialog;

    // 狀態列
    QLabel* statusLineCol = nullptr;
    QLabel* statusSel = nullptr;
    QLabel* statusLines = nullptr;
    QLabel* statusLang = nullptr;
    QLabel* statusEncoding = nullptr;

    // Recent Files
    QMenu* recentFilesMenu;
    QAction* recentFileActions[15];
    QStringList recentFiles;
    void updateRecentFileActions();
    void saveRecentFiles();
    void loadRecentFiles();
    void addToRecentFiles(const QString& filePath);

    // Actions
    QAction* newAction;
    QAction* openAction;
    QAction* saveAction;
    QAction* saveAsAction;
    QAction* saveAllAction;
    QAction* closeTabAction;
    QAction* closeAllAction;
    QAction* undoAction;
    QAction* redoAction;
    QAction* cutAction;
    QAction* copyAction;
    QAction* pasteAction;
    QAction* selectAllAction;
    QAction* duplicateLineAction;
    QAction* deleteLineAction;
    QAction* moveLineUpAction;
    QAction* moveLineDownAction;
    QAction* toggleCommentAction;
    QAction* upperCaseAction;
    QAction* lowerCaseAction;
    QAction* findAction;
    QAction* findNextAction;
    QAction* findPrevAction;
    QAction* gotoLineAction;
    QAction* findInFilesAction;
    QAction* wrapAction = nullptr;
    QAction* fontAction;
    QAction* zoomInAction;
    QAction* zoomOutAction;
    QAction* zoomResetAction;
    QAction* openFolderAction = nullptr;
    QAction* quickOpenAction = nullptr;
    QAction* toggleBookmarkAction = nullptr;
    QAction* nextBookmarkAction = nullptr;
    QAction* prevBookmarkAction = nullptr;
    QAction* tailAction = nullptr;
    QAction* navBackAction = nullptr;
    QAction* navForwardAction = nullptr;
    QAction* prefsAction = nullptr;

    // 導覽歷史
    struct NavLoc { QPointer<CodeEditor> editor; int pos; };
    QList<NavLoc> navHistory;
    int navIndex = -1;
    bool navInProgress = false;
    void recordNavLocation();

    // 編碼 / 換行符
    QLabel* statusEol = nullptr;
    void reloadWithEncoding(const QString& enc);
    void setEditorEncoding(CodeEditor* e, const QString& enc);
    QTimer* autosaveTimer = nullptr;

    // 互動式終端機（ConPTY）
    QDockWidget* termDock = nullptr;
    class TerminalWidget* terminal = nullptr;

    // 輸出面板 / 建置任務 / Git
    QDockWidget* outputDock = nullptr;
    QPlainTextEdit* outputView = nullptr;
    QLineEdit* cmdInput = nullptr;
    QProcess* taskProcess = nullptr;
    QLabel* statusGit = nullptr;
    QTimer* gitTimer = nullptr;

    // LSP（語言伺服器）
    LspManager* lsp = nullptr;
    QLabel* statusLsp = nullptr;
    QTimer* lspChangeTimer = nullptr;                 // didChange 防抖
    QList<QPointer<CodeEditor>> lspDirtyEditors;
    QDockWidget* refsDock = nullptr;                  // 「全部引用」結果面板
    QListWidget* refsList = nullptr;
    void setupLsp();
    void updateLspStatus();
    CodeEditor* editorForPath(const QString& path);   // nullptr = 未開啟
    void applyTextEditsToEditor(CodeEditor* editor, const QList<LspProtocol::TextEdit>& edits);

    // Git gutter 行標示
    QHash<QString, QString> gitHeadCache;             // path → HEAD 內容
    QSet<QString> gitUntracked;                       // 不在版控中的檔案（不重試）
    QTimer* gitGutterTimer = nullptr;                 // 編輯後重算防抖
    void fetchGitHead(const QString& path);           // 非同步抓 HEAD 版本後重算
    void recomputeGitGutter(CodeEditor* editor);

    // 分割視窗（同文件雙視圖）
    QDockWidget* splitDock = nullptr;
    CodeEditor* splitEditor = nullptr;
    QTextDocument* splitOwnDoc = nullptr;             // 無分頁時的空白後備文件
    QAction* splitAction = nullptr;
    void syncSplitView();                             // 讓分割視圖跟隨目前分頁的文件

    // Markdown 預覽
    QDockWidget* mdDock = nullptr;
    class QTextBrowser* mdView = nullptr;
    QTimer* mdTimer = nullptr;                        // 編輯後刷新防抖
    void refreshMarkdownPreview();

    // Snippet 樣板（trigger + Tab 展開；JSON 設定）
    struct SnippetDef { QString trigger, language, body; };
    QList<SnippetDef> snippetDefs;
    static QString snippetConfigPath();
    void loadSnippets();                              // 讀設定（首次寫入預設）
    void applySnippetsToEditor(CodeEditor* editor);   // 依語言過濾後注入

    // 6.2 快捷鍵自訂（JSON：動作名稱 → 快捷鍵；含衝突偵測）
    static QString keymapConfigPath();
    void applyKeymap();                               // 首次寫入現況為模板

    // 專案檔案樹 / Ctrl+P / 檔案監看 / Session
    QDockWidget* projectDock = nullptr;
    QTreeView* fsTree = nullptr;
    GitFileSystemModel* fsModel = nullptr;            // 2.3c：含 git 狀態染色
    QuickOpenDialog* quickOpen = nullptr;
    QFileSystemWatcher* fileWatcher = nullptr;
    QString projectFolder;
    void setProjectFolder(const QString& folder);
    void saveSession();
    void restoreSession();
    QString sessionDir() const;

    QFont defaultEditorFont;
    bool isFontSet = false;

    void setupUI();
    void setupToolBar();
    void setupStatusBar();
    CodeEditor* activeEditor();
    CodeEditor* createEditorTab(const QString& title);
    bool maybeSave(int index);
    QRegularExpression buildFindRegex() const;
    QTextDocument::FindFlags buildFindFlags(bool backward = false) const;
    void updateTabTitle(CodeEditor* editor);

    // 字型持久化
    void saveFont(const QFont& font);
    QFont loadFont();
    void applyFontToAllTabs(const QFont& font);

    // 自動語法高亮 + 語言/註解設定
    void applyHighlighterForPath(CodeEditor* editor, const QString& filePath);
};
