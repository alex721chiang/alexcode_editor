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

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
    void openTerminalForShot(const QString& cmd);   // 截圖模式：開終端機並（可選）執行指令
    void openSettingsForShot(const QString& outPng); // 截圖模式：開設定中心並存圖
    void openAboutForShot(const QString& outPng);    // 截圖模式：開關於對話框並存圖

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
    void showGotoLineDialog();
    void showBookmarkMatchingDialog();   // 「標記符合的行為書籤」小對話框（Notepad++ 風）
    void showFontDialog();
    void showFindInFilesDialog();
    void onFindInFilesResultDoubleClicked(QListWidgetItem* item);
    void updateStatusBar();
    void onModificationChanged(bool modified);
    void openFolder();
    void showQuickOpen();
    void onFileChangedExternally(const QString& path);
    void navigateBack();
    void navigateForward();
    void showPreferences();
    void showSettingsCenter();   // 圖形設定中心（LSP/Snippet/快捷鍵）
    void showAbout();            // 關於 AlexCode
    void showDiff(const QString& titleA, const QString& a,
                  const QString& titleB, const QString& b);
    void showSideBySideDiff(const QString& titleA, const QString& a,
                            const QString& titleB, const QString& b);   // 並排 diff 分頁
    void compareActiveWithGitHead();                                     // 目前檔 vs Git HEAD（並排）
    void runExternalTool();
    void runBuildTask();
    void runCommand(const QString& cmd);
    void updateGitStatus();
    void showSymbolList();

private:
    QTabWidget* tabWidget = nullptr;
    class QLabel* breadcrumbLabel = nullptr;          // 編輯器頂部麵包屑（檔 > 類別 > 函式）
    class SymbolDialog* symbolDialog = nullptr;       // Ctrl+Shift+O 跳至符號
    class CommandPalette* commandPalette = nullptr;   // Ctrl+Shift+P 命令面板
    class ProjectSymbolController* symbolController = nullptr;   // 專案符號索引子系統（拆自 MainWindow）
    void showGoToSymbol();
    void showCommandPalette();
    void updateBreadcrumb();
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

    // Find & Replace — 拆到 FindController（對話框、Find/Replace/Count/Mark All/Find All）
    class FindController* findController = nullptr;
    QDockWidget* filterResultsDock = nullptr;    // FILTER RESULTS dock（Find All 結果也共用這個面板）
    FindInFilesDialog* findInFilesDialog;

    // 狀態列
    QLabel* statusLineCol = nullptr;
    QLabel* statusSel = nullptr;
    QLabel* statusLines = nullptr;
    QLabel* statusLang = nullptr;
    QLabel* statusEncoding = nullptr;

    // Recent Files — 拆到 RecentFilesController
    class RecentFilesController* recentFilesController = nullptr;

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
    void applyActionIcons();     // 自繪霓虹圖示（主題切換時重呼叫換色）
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
    QLabel* statusBlame = nullptr;   // 游標行的 git blame（commit/作者/日期 · 摘要）
    QTimer* gitTimer = nullptr;

    // LSP（語言伺服器）
    class LspController* lspController = nullptr;                // LSP 子系統核心 + REFERENCES dock
    class LspStatusController* lspStatusController = nullptr;   // LSP 子系統拆分第 1 段：狀態列
    class LspSyncController* lspSyncController = nullptr;   // LSP 子系統拆分第 2 段：變更防抖同步
    void setupLsp();
    CodeEditor* editorForPath(const QString& path);   // nullptr = 未開啟

    // Git gutter 行標示
    QTimer* gitGutterTimer = nullptr;                 // 編輯後重算防抖(留在 MainWindow：
                                                       // 需要在觸發當下取目前 activeEditor())
    class GitGutterController* gitGutterController = nullptr;

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
    class FunctionListController* functionListController = nullptr;   // Function List 常駐面板
    QTimer* functionListTimer = nullptr;              // 編輯後重新整理防抖
    void refreshMarkdownPreview();

    // Markdown wikilink / backlinks / 關係圖（Obsidian 風）— 拆到 MarkdownLinkController
    class MarkdownLinkController* mdLinkController = nullptr;
    QTimer* mdIndexTimer = nullptr;                   // 存檔 .md 後重建索引（防抖，留在 MainWindow：
                                                       // 需要在計時器觸發當下取目前 projectFolder/作用中檔）
    QString currentFilePath();                        // activeEditor() 的檔案路徑(給 controller 呼叫用)
public:
    void openVaultForShot(const QString& folder);     // 截圖用：設資料夾並顯示 backlinks
    void openGraphForShot(const QString& folder);     // 截圖用：設資料夾並顯示關係圖
    void showMarkdownPreviewForShot();                // 截圖用：顯示 Markdown 預覽並刷新
    void gotoLineForShot(int line);                   // 截圖/CLI：跳至指定行（1-based）
    void setShowWhitespaceAll(bool on);               // 截圖/CLI：所有分頁切換顯示空白
    void openCommandPaletteForShot(const QString& outPng);   // 截圖：開命令面板並截圖
    void openProjectSymbolForShot(const QString& outPng);    // 截圖：開專案符號搜尋並截圖
    void findRefsForShot(const QString& name);               // 截圖/CLI：同步建索引並找引用
    void openCallGraphForShot(const QString& outPng);        // 截圖：開目前游標所在函式的 Call Graph
    void openMenuForShot(int index, const QString& outPng);  // 截圖：彈出某選單列選單以驗證 i18n
    void openGitDiffForShot(const QString& outPng);          // 截圖：目前檔 vs HEAD 並排 diff
private:

    // Snippet 樣板 — 拆到 SnippetsController
    class SnippetsController* snippetsController = nullptr;

    // 腳本外掛（QJSEngine）
    class PluginManager* pluginManager = nullptr;
    QMenu* pluginMenu = nullptr;
    void rebuildPluginMenu();

    // AI 輔助（ghost 補全 / 解釋 / 重構）
    QTimer* aiGhostTimer = nullptr;      // autoTrigger 開啟時的編輯停頓觸發
    bool m_aiRefactorMode = false;       // chatReady 的路由：重構 vs 解釋
    QString m_aiSelection;               // 重構 diff 的左側原文
    QTextCursor m_aiCursor;              // 原選取（持久游標，套用取代用）
    void triggerAiCompletion();
    void runAiOnSelection(bool refactor);
    void onAiChatReady(const QString& content);

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
