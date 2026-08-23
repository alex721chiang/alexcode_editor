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
#include <QFontDatabase>
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
#include <QStatusBar>
#include <QInputDialog>
#include <QCloseEvent>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QMimeData>
#include <QShortcut>
#include <QTreeView>
#include <QFileSystemModel>
#include <QFileSystemWatcher>
#include <QtConcurrent>
#include <QStandardPaths>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QDir>
#include <QScrollBar>
#include <QTextCodec>
#include <QUrl>
#include <QClipboard>
#include <QApplication>
#include <QDateTime>
#include <functional>
#include <memory>
#include <QVector>
#include <algorithm>
#include <QTextDocumentFragment>
#include <QXmlStreamReader>
#include <QXmlStreamWriter>
#include <QCryptographicHash>
#include <QTextBrowser>
#include <QListWidget>
#include <QSpinBox>
#include <QFormLayout>
#include <QDialogButtonBox>
#include <QDialog>
#include <QLineEdit>
#include <QCheckBox>
#include "AICompletionProvider.h"
#include "LspManager.h"
#include "MarkdownLinkController.h"
#include "RecentFilesController.h"
#include "SnippetsController.h"
#include "SymbolDialog.h"
#include "CommandPalette.h"
#include "ProjectSymbolController.h"
#include "ProjectSymbolDialog.h"
#include "TextRefs.h"
#include "FileTier.h"
#include "MarkdownRender.h"
#include <QDesktopServices>
#include "GitGutterController.h"
#include "LspStatusController.h"
#include "LspSyncController.h"
#include "LspController.h"
#include "FunctionListController.h"
#include "FindController.h"
#include "NeonIcons.h"
#include "DiffCalc.h"
#include "DiffViewer.h"
#include "PluginManager.h"
#ifdef Q_OS_WIN
#include <windows.h>
#include <dwmapi.h>
#endif
#include "TimelineBar.h"
#include "TerminalWidget.h"
#include "HudOverlay.h"
#include "TextTools.h"
#include "SettingsDialog.h"
#include "Theme.h"
#include "Portable.h"
#include "Version.h"

// 2.3c 檔案樹 git 染色：修改＝黃、未追蹤＝青（色票同 git gutter）
class GitFileSystemModel : public QFileSystemModel {
public:
    using QFileSystemModel::QFileSystemModel;
    QHash<QString, QChar> gitStates;                 // 絕對路徑（'/' 分隔）→ 'M' / '?'
    QVariant data(const QModelIndex& idx, int role) const override {
        if (role == Qt::ForegroundRole && !gitStates.isEmpty() && !isDir(idx)) {
            const auto it = gitStates.constFind(filePath(idx));
            if (it != gitStates.constEnd())
                return QColor(*it == QLatin1Char('?') ? Theme::GIT_ADDED : Theme::GIT_MODIFIED);
        }
        return QFileSystemModel::data(idx, role);
    }
};

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), findInFilesDialog(nullptr)
{
    defaultEditorFont = loadFont();
    isFontSet = true;
    snippetsController = new SnippetsController(this);
    setupLsp();
    setupUI();
    setupStatusBar();
    setAcceptDrops(true);
    setWindowTitle(QStringLiteral("AlexCode v%1 — Neon Edition").arg(ALEXCODE_VERSION));
    resize(1100, 720);
    applyTitleBarTheme();                   // 標題列跟主題同色（Win11 DWM；Win10 退回深色模式）

    AppSettings settings;
    if (settings.value("session/restore", true).toBool())
        // 先讓空的主視窗即時繪出，再於事件迴圈啟動後串流還原分頁；
        // 避免 N 個分頁的 tree-sitter 解析 / clangd 啟動 / git 子行程全塞在首次繪製前。
        QTimer::singleShot(0, this, &MainWindow::restoreSession);

    applyKeymap();                          // 6.2：套用使用者自訂快捷鍵（首次輸出模板）

    // 自動快照（當機復原）：預設每 2 分鐘
    autosaveTimer = new QTimer(this);
    connect(autosaveTimer, &QTimer::timeout, this, &MainWindow::saveSession);
    const int mins = settings.value("session/autosaveMinutes", 2).toInt();
    if (mins > 0) autosaveTimer->start(mins * 60 * 1000);
}

CodeEditor* MainWindow::activeEditor() {
    return qobject_cast<CodeEditor*>(tabWidget->currentWidget());
}

// 目前作用中分頁的檔案路徑；MarkdownLinkController 不認識 CodeEditor，靠這個取值傳進去。
QString MainWindow::currentFilePath() {
    CodeEditor* e = activeEditor();
    return e ? e->property("filePath").toString() : QString();
}

CodeEditor* MainWindow::editorForPath(const QString& path) {
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (e && e->property("filePath").toString() == path) return e;
    }
    return nullptr;
}

// ----------------------------------------------------------------
// LSP（語言伺服器）：診斷 / 補全 / 跳至定義 / hover
// ----------------------------------------------------------------
void MainWindow::setupLsp() {
    // resolveEditor：找已開啟的分頁；openIfMissing 時找不到就先開分頁。
    // 讓 LspController 不需要認識 tabWidget，維持與其他 controller 一致的解耦模式。
    auto resolveEditor = [this](const QString& path, bool openIfMissing) -> CodeEditor* {
        CodeEditor* e = editorForPath(path);
        if (!e && openIfMissing) { openFileByPath(path); e = editorForPath(path); }
        return e;
    };
    lspController = new LspController(resolveEditor, this, this);
    connect(lspController, &LspController::statusMessage, this,
            [this](const QString& text, int ms) { statusBar()->showMessage(text, ms); });
    connect(lspController, &LspController::referenceActivated, this,
            [this](const QString& path, int line) {
                openFileByPath(path);
                if (CodeEditor* e = activeEditor()) e->gotoLine(line);
            });

    lspStatusController = new LspStatusController(lspController->lsp(), this, this);   // 第三個 this：QObject parent

    // LSP 文件同步（didChange 防抖 + 套用 TextEdit）
    lspSyncController = new LspSyncController(lspController->lsp(), this);

    // Git gutter：非同步抓 HEAD 版本、與緩衝區 diff 後標示行狀態
    gitGutterController = new GitGutterController(this);
    connect(gitGutterController, &GitGutterController::headReady, this,
            [this](const QString& path) {
                if (CodeEditor* e = editorForPath(path)) gitGutterController->recompute(e);
            });
    connect(gitGutterController, &GitGutterController::markedUntracked, this,
            [this](const QString& path) {
                if (CodeEditor* e = editorForPath(path)) e->setGitLineStates({});
            });
    connect(gitGutterController, &GitGutterController::blameReady, this,
            [this](const QString&) { updateStatusBar(); });   // 游標行 blame 上線後刷新

    // Git gutter 重算防抖：編輯停頓 600ms 後比對 HEAD
    gitGutterTimer = new QTimer(this);
    gitGutterTimer->setSingleShot(true);
    gitGutterTimer->setInterval(600);
    connect(gitGutterTimer, &QTimer::timeout, this, [this]() {
        gitGutterController->recompute(activeEditor());
    });

    // 以下幾個天生要看「目前作用中分頁是誰」，留在這裡（透過 lspController->lsp() 存取底層 LspManager）
    connect(lspController->lsp(), &LspManager::diagnosticsReceived, this,
            [this](const QString& path, const QList<LspProtocol::Diagnostic>& diags) {
        if (CodeEditor* e = editorForPath(path)) {
            e->setDiagnostics(diags);
            if (e == activeEditor()) lspStatusController->update(e);
        }
    });
    connect(lspController->lsp(), &LspManager::completionReady, this,
            [this](const QString& path, const QStringList& items) {
        CodeEditor* e = activeEditor();
        if (e && e->property("filePath").toString() == path)
            e->showLspCompletions(items);
    });
    connect(lspController->lsp(), &LspManager::definitionReady, this,
            [this](const QString& path, int line, int character) {
        openFileByPath(path);                       // 已開啟則切換分頁；導覽歷史自動記錄
        if (CodeEditor* e = activeEditor()) {
            e->gotoLine(line + 1);
            QTextCursor c = e->textCursor();
            c.movePosition(QTextCursor::StartOfBlock);
            c.movePosition(QTextCursor::Right, QTextCursor::MoveAnchor,
                           qMin(character, int(c.block().length()) - 1));
            e->setTextCursor(c);
        }
    });
    connect(lspController->lsp(), &LspManager::hoverReady, this,
            [this](const QString& path, const QString& text) {
        CodeEditor* e = activeEditor();
        if (e && e->property("filePath").toString() == path)
            e->showHoverText(text);
    });
    connect(lspController->lsp(), &LspManager::statusChanged, this,
            [this]() { lspStatusController->update(activeEditor()); });
}

// ----------------------------------------------------------------
// Git gutter 行標示：緩衝區 vs HEAD 版本（實作已搬到 GitGutterController）
// ----------------------------------------------------------------


// ----------------------------------------------------------------
// 設定持久化
// ----------------------------------------------------------------
void MainWindow::saveFont(const QFont& font) {
    AppSettings settings;
    settings.setValue("font/family", font.family());
    settings.setValue("font/pointSize", font.pointSize());
    settings.setValue("font/bold", font.bold());
    settings.setValue("font/italic", font.italic());
}

QFont MainWindow::loadFont() {
    AppSettings settings;
    QFont font;
    // 未設定字型時，依序挑可用的程式設計字型（多數附帶連字），最後退回 Consolas/等寬。
    QString family = settings.value("font/family").toString();
    if (family.isEmpty()) {
        const QStringList prefer = {
            "Cascadia Code", "Cascadia Mono", "JetBrains Mono",
            "Fira Code", "Source Code Pro", "Consolas"
        };
        const QStringList installed = QFontDatabase::families();
        for (const QString& f : prefer) {
            if (installed.contains(f, Qt::CaseInsensitive)) { family = f; break; }
        }
        if (family.isEmpty()) family = "Consolas";
    }
    font.setFamily(family);
    font.setPointSize(settings.value("font/pointSize", 11).toInt());
    font.setBold(settings.value("font/bold", false).toBool());
    font.setItalic(settings.value("font/italic", false).toBool());
    font.setStyleHint(QFont::Monospace, QFont::PreferAntialias);     // 等寬偏好 + 抗鋸齒
    font.setFixedPitch(true);
    font.setHintingPreference(QFont::PreferFullHinting);             // 提升清晰度
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
    const SyntaxHighlighter::Language lang = SyntaxHighlighter::detectLanguage(filePath);

    // 註解前綴（給 Toggle Comment 用）：# 系列 vs // 系列
    QString commentPrefix = "//";
    if (lang == SyntaxHighlighter::Language::Python ||
        lang == SyntaxHighlighter::Language::CMakeLang ||
        lang == SyntaxHighlighter::Language::Bash)
        commentPrefix = "#";
    editor->setProperty("language", SyntaxHighlighter::languageName(lang));
    editor->setCommentPrefix(commentPrefix);
    editor->setSyntaxLanguage(lang, filePath);      // 支援語言用 tree-sitter，其餘 regex（含 Unknown 清空）
    snippetsController->applyTo(editor);            // 語言確定後注入對應 snippet
    updateStatusBar();
    updateBreadcrumb();
    if (functionListController->dock()->isVisible())
        functionListController->refresh(editor);
}

// ----------------------------------------------------------------
// 建立編輯器分頁（統一入口，集中接線）
// ----------------------------------------------------------------
CodeEditor* MainWindow::createEditorTab(const QString& title) {
    CodeEditor* editor = new CodeEditor(this);
    editor->setFont(defaultEditorFont);
    editor->setAIProvider(aiProvider);
    editor->setLineWrapMode(wrapAction && wrapAction->isChecked()
                                ? QPlainTextEdit::WidgetWidth : QPlainTextEdit::NoWrap);
    editor->setProperty("language", "Plain Text");
    editor->setAutoPairEnabled(AppSettings().value("editor/autoPair", true).toBool());
    editor->setShowWhitespace(AppSettings().value("editor/showWhitespace", false).toBool());
    editor->setStickyScrollEnabled(AppSettings().value("editor/stickyScroll", true).toBool());
    editor->setMinimapEnabled(AppSettings().value("editor/minimap", true).toBool());
    snippetsController->applyTo(editor);            // 通用（language 為空）snippet

    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, &MainWindow::updateStatusBar);
    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, &MainWindow::recordNavLocation);
    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, [this, editor]() {
        if (editor == activeEditor()) updateBreadcrumb();
    });
    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, [this, editor]() {
        // Function List 游標追蹤：面板可見時同步高亮游標所在的符號（不搶焦點）
        if (editor == activeEditor() && functionListController->dock()->isVisible())
            functionListController->highlightLine(editor->textCursor().blockNumber());
    });
    connect(editor, &QPlainTextEdit::selectionChanged, this, &MainWindow::updateStatusBar);
    connect(editor->document(), &QTextDocument::modificationChanged,
            this, &MainWindow::onModificationChanged);

    // LSP：請求轉發與內容變更同步
    connect(editor, &CodeEditor::lspCompletionRequested, this, [this, editor](int line, int ch) {
        lspController->lsp()->requestCompletion(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspDefinitionRequested, this, [this, editor](int line, int ch) {
        lspController->lsp()->requestDefinition(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspHoverRequested, this, [this, editor](int line, int ch) {
        lspController->lsp()->requestHover(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspReferencesRequested, this, [this, editor](int line, int ch) {
        lspController->lsp()->requestReferences(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspRenameRequested, this, [this, editor](int line, int ch) {
        // 取游標下字詞作為預設名稱
        QTextCursor c = editor->textCursor();
        c.select(QTextCursor::WordUnderCursor);
        bool ok = false;
        const QString newName = QInputDialog::getText(this, tr("重新命名符號"),
            tr("新名稱（套用至專案內所有引用）："),
            QLineEdit::Normal, c.selectedText(), &ok);
        if (ok && !newName.trimmed().isEmpty())
            lspController->lsp()->requestRename(editor->property("filePath").toString(), line, ch, newName.trimmed());
    });
    connect(editor, &CodeEditor::lspFormatRequested, this, [this, editor]() {
        AppSettings settings;
        lspController->lsp()->requestFormatting(editor->property("filePath").toString(),
                               settings.value("editor/tabWidth", 4).toInt(), true);
    });
    connect(editor, &CodeEditor::wikilinkActivated, this,
            [this](const QString& target) { mdLinkController->openOrCreateWikilink(target, projectFolder); });
    connect(editor, &CodeEditor::projectReferencesRequested, this,
            [this](const QString& name) { symbolController->findReferences(name); });
    connect(editor, &QPlainTextEdit::textChanged, this, [this, editor]() {
        if (editor->lspEnabled()) {
            lspSyncController->markDirty(editor);
        }
        if (editor->assistEnabled() && !editor->property("filePath").toString().isEmpty())
            gitGutterTimer->start();                    // Git gutter 重算（防抖；大檔停用）
        if (mdDock && mdDock->isVisible() && editor == activeEditor())
            mdTimer->start();                           // Markdown 預覽刷新（防抖）
        if (editor == activeEditor())
            functionListTimer->start();                 // 函式清單重新整理（防抖）
        if (editor == activeEditor() && aiProvider->config().autoTrigger)
            aiGhostTimer->start(aiProvider->config().autoDelayMs);   // AI ghost 補全（設定檔開啟時）
    });

    int idx = tabWidget->addTab(editor, title);
    editor->setProperty("baseTitle", title);
    tabWidget->setCurrentIndex(idx);
    updateStatusBar();
    return editor;
}

void MainWindow::setupUI() {
    aiProvider = new AICompletionProvider(this);
    connect(aiProvider, &AICompletionProvider::suggestionsReady, this, [this](const QStringList& s) {
        statusBar()->clearMessage();
        if (s.isEmpty() || s.first().trimmed().isEmpty()) {
            statusBar()->showMessage(tr("AI 沒有建議"), 3000);
            return;
        }
        if (CodeEditor* e = activeEditor()) e->setGhostText(s.first());
    });
    connect(aiProvider, &AICompletionProvider::chatReady, this, &MainWindow::onAiChatReady);
    connect(aiProvider, &AICompletionProvider::errorOccurred, this, [this](const QString& err) {
        statusBar()->showMessage(tr("AI 錯誤：%1（工具→AI 輔助→編輯 AI 設定檔）").arg(err), 6000);
    });
    aiGhostTimer = new QTimer(this);                     // 自動觸發（設定檔 autoTrigger 開啟時）
    aiGhostTimer->setSingleShot(true);
    connect(aiGhostTimer, &QTimer::timeout, this, &MainWindow::triggerAiCompletion);

    tabWidget = new QTabWidget(this);
    tabWidget->setTabsClosable(true);
    tabWidget->setMovable(true);
    tabWidget->setDocumentMode(true);
    connect(tabWidget, &QTabWidget::tabCloseRequested, this, [this](int index) {
        closeTab(index);
    });
    connect(tabWidget, &QTabWidget::currentChanged, this, [this](int) {
        if (resultsList) resultsList->clear();
        if (filterCountLabel) filterCountLabel->setText("");
        // 分頁切到時才啟用其 LSP / Git gutter（延遲載入）；串流還原期間不啟用，
        // 避免每個還原分頁在建立時因短暫成為當前分頁而被連帶啟用。
        if (!restoringSession) ensureEditorActivated(activeEditor());
        updateStatusBar();
        updateBreadcrumb();
        if (functionListController->dock()->isVisible())
            functionListController->refresh(activeEditor());
    });

    // 麵包屑列 + 分頁（容器置中）
    breadcrumbLabel = new QLabel(this);
    breadcrumbLabel->setTextFormat(Qt::RichText);
    breadcrumbLabel->setContentsMargins(10, 3, 10, 3);
    breadcrumbLabel->setText(QString());
    connect(breadcrumbLabel, &QLabel::linkActivated, this, [this](const QString& href) {
        if (CodeEditor* e = activeEditor()) e->gotoLine(href.toInt() + 1);   // href = 0-based line
    });
    centralWrap = new QWidget(this);
    auto* wrapLay = new QVBoxLayout(centralWrap);
    wrapLay->setContentsMargins(0, 0, 0, 0);
    wrapLay->setSpacing(0);
    wrapLay->addWidget(breadcrumbLabel);
    wrapLay->addWidget(tabWidget);
    setCentralWidget(centralWrap);

    // Neon HUD 四角角標覆蓋層：置於中央容器之上、滑鼠穿透；隨容器大小變化（eventFilter）。
    hudOverlay = new HudOverlay(centralWrap);
    hudOverlay->setGeometry(centralWrap->rect());
    hudOverlay->raise();
    centralWrap->installEventFilter(this);

    // ---------- File actions ----------
    newAction = new QAction(tr("New File"), this);
    newAction->setShortcut(QKeySequence::New);
    // 圖示統一由 applyActionIcons() 設定（自繪霓虹線條圖示，主題切換時重設）
    connect(newAction, &QAction::triggered, this, &MainWindow::newFile);

    openAction = new QAction(tr("Open..."), this);
    openAction->setShortcut(QKeySequence::Open);
    connect(openAction, &QAction::triggered, this, &MainWindow::openFile);

    saveAction = new QAction(tr("Save"), this);
    saveAction->setShortcut(QKeySequence::Save);
    connect(saveAction, &QAction::triggered, this, &MainWindow::saveFile);

    saveAsAction = new QAction(tr("Save As..."), this);
    saveAsAction->setShortcut(QKeySequence::SaveAs);
    connect(saveAsAction, &QAction::triggered, this, &MainWindow::saveFileAs);

    saveAllAction = new QAction(tr("Save All"), this);
    saveAllAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_S));
    connect(saveAllAction, &QAction::triggered, this, &MainWindow::saveAllFiles);

    closeTabAction = new QAction(tr("Close Tab"), this);
    closeTabAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_W));
    connect(closeTabAction, &QAction::triggered, this, [this]() {
        if (tabWidget->count() > 0) closeTab(tabWidget->currentIndex());
    });

    closeAllAction = new QAction(tr("Close All"), this);
    closeAllAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_W));
    connect(closeAllAction, &QAction::triggered, this, &MainWindow::closeAllTabs);

    // ---------- Edit actions ----------
    undoAction = new QAction(tr("Undo"), this);
    undoAction->setShortcut(QKeySequence::Undo);
    connect(undoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->undo();
    });

    redoAction = new QAction(tr("Redo"), this);
    redoAction->setShortcut(QKeySequence::Redo);
    connect(redoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->redo();
    });

    cutAction = new QAction(tr("Cut"), this);
    cutAction->setShortcut(QKeySequence::Cut);
    connect(cutAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->cut();
    });

    copyAction = new QAction(tr("Copy"), this);
    copyAction->setShortcut(QKeySequence::Copy);
    connect(copyAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->copy();
    });

    pasteAction = new QAction(tr("Paste"), this);
    pasteAction->setShortcut(QKeySequence::Paste);
    connect(pasteAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->paste();
    });

    selectAllAction = new QAction(tr("Select All"), this);
    selectAllAction->setShortcut(QKeySequence::SelectAll);
    connect(selectAllAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->selectAll();
    });

    duplicateLineAction = new QAction(tr("Duplicate Line"), this);
    duplicateLineAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_D));
    connect(duplicateLineAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->duplicateCurrentLine();
    });

    deleteLineAction = new QAction(tr("Delete Line"), this);
    deleteLineAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_L));
    connect(deleteLineAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->deleteCurrentLine();
    });

    moveLineUpAction = new QAction(tr("Move Line Up"), this);
    moveLineUpAction->setShortcut(QKeySequence(Qt::ALT | Qt::Key_Up));
    connect(moveLineUpAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->moveLineUp();
    });

    moveLineDownAction = new QAction(tr("Move Line Down"), this);
    moveLineDownAction->setShortcut(QKeySequence(Qt::ALT | Qt::Key_Down));
    connect(moveLineDownAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->moveLineDown();
    });

    toggleCommentAction = new QAction(tr("Toggle Comment"), this);
    toggleCommentAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_Slash));
    connect(toggleCommentAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->toggleComment();
    });

    upperCaseAction = new QAction(tr("UPPERCASE"), this);
    upperCaseAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_U));
    connect(upperCaseAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->selectionToUpper();
    });

    lowerCaseAction = new QAction(tr("lowercase"), this);
    lowerCaseAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_U));
    connect(lowerCaseAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->selectionToLower();
    });

    // ---------- Search actions ----------
    // findController 在 setupUI 稍後（FILTER RESULTS 面板建好後）才建立；
    // lambda 延遲取值，觸發時必已存在
    findAction = new QAction(tr("Find / Replace..."), this);
    findAction->setShortcut(QKeySequence::Find);
    connect(findAction, &QAction::triggered, this, [this]() { findController->showDialog(); });

    findNextAction = new QAction(tr("Find Next"), this);
    findNextAction->setShortcut(QKeySequence(Qt::Key_F3));
    connect(findNextAction, &QAction::triggered, this, [this]() { findController->findNext(); });

    findPrevAction = new QAction(tr("Find Previous"), this);
    findPrevAction->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F3));
    connect(findPrevAction, &QAction::triggered, this, [this]() { findController->findPrev(); });

    gotoLineAction = new QAction(tr("Go to Line..."), this);
    gotoLineAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_G));
    connect(gotoLineAction, &QAction::triggered, this, &MainWindow::showGotoLineDialog);

    findInFilesAction = new QAction(tr("Find in Files..."), this);
    findInFilesAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_F));
    connect(findInFilesAction, &QAction::triggered, this, &MainWindow::showFindInFilesDialog);

    applyActionIcons();   // 自繪霓虹圖示（工具列/選單共用；主題切換時會重呼叫換色）

    openFolderAction = new QAction(tr("Open Folder..."), this);
    openFolderAction->setShortcut(QKeySequence(Qt::CTRL | Qt::ALT | Qt::Key_O));   // 讓出 Ctrl+Shift+O 給 Go to Symbol
    connect(openFolderAction, &QAction::triggered, this, &MainWindow::openFolder);

    quickOpenAction = new QAction(tr("Quick Open..."), this);
    quickOpenAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_P));
    connect(quickOpenAction, &QAction::triggered, this, &MainWindow::showQuickOpen);

    QAction* gotoSymbolAction = new QAction(tr("Go to Symbol..."), this);
    gotoSymbolAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_O));
    connect(gotoSymbolAction, &QAction::triggered, this, &MainWindow::showGoToSymbol);
    addAction(gotoSymbolAction);                              // 全域快捷鍵

    QAction* cmdPaletteAction = new QAction(tr("Command Palette..."), this);
    cmdPaletteAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_P));
    connect(cmdPaletteAction, &QAction::triggered, this, &MainWindow::showCommandPalette);
    addAction(cmdPaletteAction);                              // 全域快捷鍵

    QAction* projSymbolAction = new QAction(tr("Go to Symbol in Project..."), this);
    projSymbolAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_T));
    connect(projSymbolAction, &QAction::triggered, this,
            [this]() { symbolController->showSearch(projectFolder); });
    addAction(projSymbolAction);                              // 全域快捷鍵

    toggleBookmarkAction = new QAction(tr("Toggle Bookmark"), this);
    toggleBookmarkAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_F2));
    connect(toggleBookmarkAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->toggleBookmark();
    });

    nextBookmarkAction = new QAction(tr("Next Bookmark"), this);
    nextBookmarkAction->setShortcut(QKeySequence(Qt::Key_F2));
    connect(nextBookmarkAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->nextBookmark();
    });

    prevBookmarkAction = new QAction(tr("Previous Bookmark"), this);
    prevBookmarkAction->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F2));
    connect(prevBookmarkAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->prevBookmark();
    });

    // 書籤批次操作（Notepad++ 的 Search→Bookmark 選單）
    auto* bookmarkMatchAction = new QAction(tr("標記符合的行為書籤…"), this);
    connect(bookmarkMatchAction, &QAction::triggered, this, &MainWindow::showBookmarkMatchingDialog);

    auto* copyBookmarkedAction = new QAction(tr("複製書籤行"), this);
    connect(copyBookmarkedAction, &QAction::triggered, this, [this]() {
        if (auto* editor = activeEditor())
            QApplication::clipboard()->setText(editor->bookmarkedLinesText());
    });

    auto* cutBookmarkedAction = new QAction(tr("剪下書籤行"), this);
    connect(cutBookmarkedAction, &QAction::triggered, this, [this]() {
        if (auto* editor = activeEditor()) {
            QApplication::clipboard()->setText(editor->bookmarkedLinesText());
            editor->deleteBookmarkedLines();
        }
    });

    auto* deleteBookmarkedAction = new QAction(tr("刪除書籤行"), this);
    connect(deleteBookmarkedAction, &QAction::triggered, this, [this]() {
        if (auto* editor = activeEditor()) editor->deleteBookmarkedLines();
    });

    auto* deleteNonBookmarkedAction = new QAction(tr("刪除非書籤行"), this);
    connect(deleteNonBookmarkedAction, &QAction::triggered, this, [this]() {
        if (auto* editor = activeEditor()) editor->deleteNonBookmarkedLines();
    });

    auto* invertBookmarksAction = new QAction(tr("反轉書籤"), this);
    connect(invertBookmarksAction, &QAction::triggered, this, [this]() {
        if (auto* editor = activeEditor()) editor->invertBookmarks();
    });

    navBackAction = new QAction(tr("Navigate Back"), this);
    navBackAction->setShortcut(QKeySequence(Qt::ALT | Qt::Key_Left));
    connect(navBackAction, &QAction::triggered, this, &MainWindow::navigateBack);

    navForwardAction = new QAction(tr("Navigate Forward"), this);
    navForwardAction->setShortcut(QKeySequence(Qt::ALT | Qt::Key_Right));
    connect(navForwardAction, &QAction::triggered, this, &MainWindow::navigateForward);

    QAction* buildAction = new QAction(tr("Run Build Task"), this);
    buildAction->setShortcut(QKeySequence(Qt::Key_F5));
    connect(buildAction, &QAction::triggered, this, &MainWindow::runBuildTask);
    addAction(buildAction);

    QAction* symbolAction = new QAction(tr("Document Symbols..."), this);
    symbolAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_M));
    connect(symbolAction, &QAction::triggered, this, &MainWindow::showSymbolList);
    addAction(symbolAction);

    prefsAction = new QAction(tr("Preferences..."), this);
    connect(prefsAction, &QAction::triggered, this, &MainWindow::showPreferences);

    // ---------- View actions ----------
    fontAction = new QAction(tr("Font..."), this);
    connect(fontAction, &QAction::triggered, this, &MainWindow::showFontDialog);

    wrapAction = new QAction(tr("Word Wrap"), this);
    wrapAction->setCheckable(true);
    wrapAction->setChecked(true);
    connect(wrapAction, &QAction::triggered, this, [this](bool checked) {
        for (int i = 0; i < tabWidget->count(); ++i) {
            if (auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
                editor->setLineWrapMode(checked ? QPlainTextEdit::WidgetWidth
                                                : QPlainTextEdit::NoWrap);
        }
    });

    zoomInAction = new QAction(tr("Zoom In"), this);
    zoomInAction->setShortcut(QKeySequence::ZoomIn);
    connect(zoomInAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->zoomEditorIn();
    });

    zoomOutAction = new QAction(tr("Zoom Out"), this);
    zoomOutAction->setShortcut(QKeySequence::ZoomOut);
    connect(zoomOutAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->zoomEditorOut();
    });

    tailAction = new QAction(tr("Tail Mode (follow file)"), this);
    tailAction->setCheckable(true);
    tailAction->setToolTip(tr("檔案被外部寫入時自動重新載入並捲到底（log 追蹤）"));

    zoomResetAction = new QAction(tr("Reset Zoom"), this);
    zoomResetAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_0));
    connect(zoomResetAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->zoomEditorReset();
    });

    // ---------- Menus ----------
    QMenuBar* menuBar = new QMenuBar(this);
    menuBar->setNativeMenuBar(false);
    setMenuBar(menuBar);

    QMenu* fileMenu = menuBar->addMenu(tr("File"));
    fileMenu->addAction(newAction);
    fileMenu->addAction(openAction);

    recentFilesController = new RecentFilesController(fileMenu, this);
    connect(recentFilesController, &RecentFilesController::fileOpenRequested, this,
            [this](const QString& path) { openFileByPath(path); });

    fileMenu->addAction(openFolderAction);
    fileMenu->addAction(quickOpenAction);
    fileMenu->addAction(gotoSymbolAction);
    fileMenu->addAction(projSymbolAction);
    fileMenu->addAction(cmdPaletteAction);
    fileMenu->addSeparator();
    fileMenu->addAction(saveAction);
    fileMenu->addAction(saveAsAction);
    fileMenu->addAction(saveAllAction);
    fileMenu->addSeparator();
    fileMenu->addAction(closeTabAction);
    fileMenu->addAction(closeAllAction);

    QMenu* editMenu = menuBar->addMenu(tr("Edit"));
    editMenu->addAction(undoAction);
    editMenu->addAction(redoAction);
    editMenu->addSeparator();
    editMenu->addAction(cutAction);
    editMenu->addAction(copyAction);
    editMenu->addAction(pasteAction);
    editMenu->addAction(selectAllAction);
    editMenu->addSeparator();
    editMenu->addAction(duplicateLineAction);
    editMenu->addAction(deleteLineAction);
    editMenu->addAction(moveLineUpAction);
    editMenu->addAction(moveLineDownAction);
    editMenu->addSeparator();
    editMenu->addAction(toggleCommentAction);
    editMenu->addAction(upperCaseAction);
    editMenu->addAction(lowerCaseAction);

    QMenu* searchMenu = menuBar->addMenu(tr("Search"));
    searchMenu->addAction(findAction);
    searchMenu->addAction(findNextAction);
    searchMenu->addAction(findPrevAction);
    searchMenu->addSeparator();
    searchMenu->addAction(gotoLineAction);
    searchMenu->addAction(findInFilesAction);
    searchMenu->addAction(symbolAction);
    searchMenu->addSeparator();
    searchMenu->addAction(navBackAction);
    searchMenu->addAction(navForwardAction);
    searchMenu->addSeparator();
    searchMenu->addAction(toggleBookmarkAction);
    searchMenu->addAction(nextBookmarkAction);
    searchMenu->addAction(prevBookmarkAction);
    QMenu* bookmarkBatchMenu = searchMenu->addMenu(tr("書籤批次"));
    bookmarkBatchMenu->addAction(bookmarkMatchAction);
    bookmarkBatchMenu->addSeparator();
    bookmarkBatchMenu->addAction(copyBookmarkedAction);
    bookmarkBatchMenu->addAction(cutBookmarkedAction);
    bookmarkBatchMenu->addAction(deleteBookmarkedAction);
    bookmarkBatchMenu->addAction(deleteNonBookmarkedAction);
    bookmarkBatchMenu->addSeparator();
    bookmarkBatchMenu->addAction(invertBookmarksAction);

    // 多游標（快捷鍵由編輯器內部處理，這裡的選單項為可發現性入口）
    searchMenu->addSeparator();
    auto* addOccurrenceAction = new QAction(tr("加選下一個相同字串"), this);
    addOccurrenceAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_D));
    connect(addOccurrenceAction, &QAction::triggered, this, [this]() {
        if (auto* e = activeEditor()) e->addNextOccurrence();
    });
    searchMenu->addAction(addOccurrenceAction);
    auto* selectAllOccAction = new QAction(tr("全選所有相同字串"), this);
    selectAllOccAction->setShortcut(QKeySequence(Qt::ALT | Qt::Key_F3));
    connect(selectAllOccAction, &QAction::triggered, this, [this]() {
        if (auto* e = activeEditor()) e->selectAllOccurrences();
    });
    searchMenu->addAction(selectAllOccAction);

    // ---------- Tools 工具箱 ----------
    QMenu* toolsMenu = menuBar->addMenu(tr("Tools"));
    auto editorOp = [this](std::function<QString(const QString&)> fn, bool selectionOnly = false) {
        CodeEditor* e = activeEditor();
        if (!e) return;
        QTextCursor c = e->textCursor();
        if (selectionOnly && !c.hasSelection()) {
            statusBar()->showMessage(tr("請先選取文字"), 2500);
            return;
        }
        if (c.hasSelection()) {
            QString sel = c.selectedText();
            sel.replace(QChar(0x2029), QChar('\n'));
            c.insertText(fn(sel));
        } else {
            c.select(QTextCursor::Document);
            c.insertText(fn(e->toPlainText()));
        }
    };
    QMenu* lineMenu = toolsMenu->addMenu(tr("行整理"));
    lineMenu->addAction(tr("排序（遞增）"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::sortLines(t, false); }); });
    lineMenu->addAction(tr("排序（遞減）"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::sortLines(t, true); }); });
    lineMenu->addAction(tr("移除重複行"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::removeDuplicateLines(t); }); });
    lineMenu->addAction(tr("移除空白行"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::removeBlankLines(t); }); });
    lineMenu->addAction(tr("反轉行順序"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::reverseLines(t); }); });
    lineMenu->addAction(tr("修剪行尾空白"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::trimTrailingWhitespace(t); }); });

    QMenu* jsonMenu = toolsMenu->addMenu(tr("JSON"));
    jsonMenu->addAction(tr("格式化（Pretty）"), this, [=]() {
        editorOp([this](const QString& t) {
            QJsonParseError err;
            QJsonDocument d = QJsonDocument::fromJson(t.toUtf8(), &err);
            if (err.error != QJsonParseError::NoError) {
                statusBar()->showMessage(tr("JSON 錯誤（位移 %1）：%2")
                    .arg(err.offset).arg(err.errorString()), 5000);
                return t;
            }
            return QString::fromUtf8(d.toJson(QJsonDocument::Indented)); }); });
    jsonMenu->addAction(tr("壓縮（Minify）"), this, [=]() {
        editorOp([this](const QString& t) {
            QJsonParseError err;
            QJsonDocument d = QJsonDocument::fromJson(t.toUtf8(), &err);
            if (err.error != QJsonParseError::NoError) {
                statusBar()->showMessage(tr("JSON 錯誤（位移 %1）：%2")
                    .arg(err.offset).arg(err.errorString()), 5000);
                return t;
            }
            return QString::fromUtf8(d.toJson(QJsonDocument::Compact)); }); });

    QMenu* encMenu = toolsMenu->addMenu(tr("編解碼（選取文字）"));
    encMenu->addAction(tr("Base64 Encode"), this, [=]() {
        editorOp([](const QString& t) { return QString::fromLatin1(t.toUtf8().toBase64()); }, true); });
    encMenu->addAction(tr("Base64 Decode"), this, [=]() {
        editorOp([](const QString& t) { return QString::fromUtf8(QByteArray::fromBase64(t.toUtf8())); }, true); });
    encMenu->addAction(tr("URL Encode"), this, [=]() {
        editorOp([](const QString& t) { return QString::fromLatin1(QUrl::toPercentEncoding(t)); }, true); });
    encMenu->addAction(tr("URL Decode"), this, [=]() {
        editorOp([](const QString& t) { return QUrl::fromPercentEncoding(t.toLatin1()); }, true); });
    encMenu->addAction(tr("HTML Entity Encode"), this, [=]() {
        editorOp([](const QString& t) { return t.toHtmlEscaped(); }, true); });
    encMenu->addAction(tr("HTML Entity Decode"), this, [=]() {
        editorOp([](const QString& t) {
            return QTextDocumentFragment::fromHtml(t).toPlainText(); }, true); });
    encMenu->addAction(tr("Unicode Escape (\\uXXXX)"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::unicodeEscape(t); }, true); });
    encMenu->addAction(tr("Unicode Unescape"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::unicodeUnescape(t); }, true); });

    toolsMenu->addAction(tr("時間戳 ↔ 時間（選取）"), this, [=]() {
        editorOp([](const QString& t) {
            const QString s = t.trimmed();
            bool ok = false;
            qint64 epoch = s.toLongLong(&ok);
            if (ok) {  // epoch → 可讀時間（自動判斷秒/毫秒）
                QDateTime dt = (epoch > 100000000000LL)
                    ? QDateTime::fromMSecsSinceEpoch(epoch) : QDateTime::fromSecsSinceEpoch(epoch);
                return dt.toString(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
            }
            QDateTime dt = QDateTime::fromString(s, QStringLiteral("yyyy-MM-dd HH:mm:ss"));
            if (dt.isValid()) return QString::number(dt.toSecsSinceEpoch());
            return t; }, true); });

    // XML 工具：QXmlStream 重排（不需額外模組）
    QMenu* xmlMenu = toolsMenu->addMenu(tr("XML"));
    xmlMenu->addAction(tr("格式化（Pretty）"), this, [=]() {
        editorOp([this](const QString& t) {
            QXmlStreamReader reader(t);
            QString out;
            QXmlStreamWriter writer(&out);
            writer.setAutoFormatting(true);
            writer.setAutoFormattingIndent(2);
            while (!reader.atEnd()) {
                reader.readNext();
                if (reader.hasError()) break;
                if (reader.isWhitespace() || reader.tokenType() == QXmlStreamReader::Invalid)
                    continue;
                writer.writeCurrentToken(reader);
            }
            if (reader.hasError()) {
                statusBar()->showMessage(tr("XML 錯誤（行 %1 欄 %2）：%3")
                    .arg(reader.lineNumber()).arg(reader.columnNumber())
                    .arg(reader.errorString()), 5000);
                return t;
            }
            return out.trimmed() + QStringLiteral("\n"); }); });
    xmlMenu->addAction(tr("驗證"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        QXmlStreamReader reader(e->toPlainText());
        while (!reader.atEnd()) reader.readNext();
        statusBar()->showMessage(reader.hasError()
            ? tr("XML 錯誤（行 %1 欄 %2）：%3").arg(reader.lineNumber())
                  .arg(reader.columnNumber()).arg(reader.errorString())
            : tr("XML 格式正確 ✓"), 5000);
    });

    // 雜湊（結果顯示於可複製的對話框）
    QMenu* hashMenu = toolsMenu->addMenu(tr("雜湊（選取或全文）"));
    auto hashAction = [this](QCryptographicHash::Algorithm algo, const QString& name) {
        CodeEditor* e = activeEditor();
        if (!e) return;
        QTextCursor c = e->textCursor();
        QString text = c.hasSelection() ? c.selectedText().replace(QChar(0x2029), QChar('\n'))
                                        : e->toPlainText();
        const QString hex = QString::fromLatin1(
            QCryptographicHash::hash(text.toUtf8(), algo).toHex());
        QInputDialog dlg(this);
        dlg.setWindowTitle(name);
        dlg.setLabelText(tr("%1（UTF-8，%2 字元）：").arg(name).arg(text.size()));
        dlg.setTextValue(hex);
        dlg.setOption(QInputDialog::NoButtons, false);
        dlg.exec();
    };
    hashMenu->addAction(tr("MD5"), this, [=]() { hashAction(QCryptographicHash::Md5, "MD5"); });
    hashMenu->addAction(tr("SHA-1"), this, [=]() { hashAction(QCryptographicHash::Sha1, "SHA-1"); });
    hashMenu->addAction(tr("SHA-256"), this, [=]() { hashAction(QCryptographicHash::Sha256, "SHA-256"); });

    // 數字底數轉換
    toolsMenu->addAction(tr("數字底數轉換（選取）"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e || !e->textCursor().hasSelection()) {
            statusBar()->showMessage(tr("請先選取數字"), 2500);
            return;
        }
        const QString s = e->textCursor().selectedText().trimmed();
        qlonglong v = 0;
        if (!TextTools::parseInteger(s, &v)) { statusBar()->showMessage(tr("無法解析數字：") + s, 3000); return; }
        QInputDialog dlg(this);
        dlg.setWindowTitle(tr("底數轉換"));
        dlg.setLabelText(tr("十進位 / 十六進位 / 二進位 / 八進位："));
        dlg.setTextValue(QStringLiteral("%1  |  0x%2  |  0b%3  |  0%4")
            .arg(v).arg(QString::number(v, 16).toUpper())
            .arg(QString::number(v, 2)).arg(QString::number(v, 8)));
        dlg.exec();
    });

    // 全形 ↔ 半形
    QMenu* widthMenu = toolsMenu->addMenu(tr("全形半形轉換"));
    widthMenu->addAction(tr("全形 → 半形"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::toHalfWidth(t); }); });
    widthMenu->addAction(tr("半形 → 全形"), this, [=]() {
        editorOp([](const QString& t) { return TextTools::toFullWidth(t); }); });

    // 摘要統計
    toolsMenu->addAction(tr("摘要統計（選取或全文）"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        QTextCursor c = e->textCursor();
        QString text = c.hasSelection() ? c.selectedText().replace(QChar(0x2029), QChar('\n'))
                                        : e->toPlainText();
        const int lines = text.isEmpty() ? 0 : text.count('\n') + 1;
        int nonSpace = 0;
        for (const QChar& ch : text) if (!ch.isSpace()) ++nonSpace;
        static const QRegularExpression wordRe(QStringLiteral("[\\w\\p{Han}]+"));
        int words = 0;
        auto it = wordRe.globalMatch(text);
        while (it.hasNext()) { it.next(); ++words; }
        QMessageBox::information(this, tr("摘要統計"),
            tr("行數：%1\n字元數：%2（不含空白 %3）\n字數：%4\nUTF-8 位元組：%5")
                .arg(lines).arg(text.size()).arg(nonSpace).arg(words)
                .arg(text.toUtf8().size()));
    });

    // 匯出 HTML（.md 以 Markdown 轉換，其餘以 <pre> 包裝）
    toolsMenu->addAction(tr("匯出 HTML…"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        const QString srcPath = e->property("filePath").toString();
        const QString suggest = (srcPath.isEmpty() ? QStringLiteral("export")
                                                   : QFileInfo(srcPath).completeBaseName()) + ".html";
        const QString outPath = QFileDialog::getSaveFileName(
            this, tr("匯出 HTML"), suggest, "HTML (*.html *.htm)");
        if (outPath.isEmpty()) return;
        QString html;
        const QString ext = QFileInfo(srcPath).suffix().toLower();
        if (ext == "md" || ext == "markdown") {
            QTextDocument tmp;
            tmp.setMarkdown(e->toPlainText());
            html = tmp.toHtml();
        } else {
            html = QStringLiteral("<!DOCTYPE html><html><head><meta charset=\"utf-8\">"
                       "<style>body{background:#0b0e16;color:#d6e2ff;}"
                       "pre{font-family:Consolas,monospace;font-size:13px;}</style></head>"
                       "<body><pre>%1</pre></body></html>")
                       .arg(e->toPlainText().toHtmlEscaped());
        }
        QFile f(outPath);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(html.toUtf8());
            statusBar()->showMessage(tr("已匯出：") + outPath, 4000);
        }
    });

    toolsMenu->addAction(buildAction);
    toolsMenu->addSeparator();
    QMenu* diffMenu = toolsMenu->addMenu(tr("比較目前分頁"));
    diffMenu->addAction(tr("與磁碟版本比較"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        const QString path = e->property("filePath").toString();
        if (path.isEmpty()) { statusBar()->showMessage(tr("此分頁尚未存檔"), 2500); return; }
        QFile f(path);
        if (!f.open(QIODevice::ReadOnly)) return;
        QString disk = QString::fromUtf8(f.readAll());
        disk.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
        showDiff(tr("磁碟版本"), disk, tr("目前內容"), e->toPlainText());
    });
    diffMenu->addAction(tr("與剪貼簿比較"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        showDiff(tr("剪貼簿"), QApplication::clipboard()->text(),
                 tr("目前內容"), e->toPlainText());
    });
    diffMenu->addSeparator();
    diffMenu->addAction(tr("與 Git HEAD 並排比較"), this, &MainWindow::compareActiveWithGitHead);
    diffMenu->addAction(tr("與磁碟版本並排比較"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        const QString path = e->property("filePath").toString();
        if (path.isEmpty()) { statusBar()->showMessage(tr("此分頁尚未存檔"), 2500); return; }
        QFile f(path);
        if (!f.open(QIODevice::ReadOnly)) return;
        QString disk = QString::fromUtf8(f.readAll());
        disk.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
        showSideBySideDiff(tr("磁碟版本"), disk, tr("目前內容"), e->toPlainText());
    });

    QMenu* macroMenu = toolsMenu->addMenu(tr("巨集"));
    QAction* recAct = macroMenu->addAction(tr("開始/停止錄製"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        if (e->isMacroRecording()) {
            e->stopMacroRecording();
            statusBar()->showMessage(tr("巨集錄製完成"), 3000);
        } else {
            e->startMacroRecording();
            statusBar()->showMessage(tr("● 巨集錄製中…（再按一次停止）"), 0);
        }
    });
    recAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_R));
    QAction* playAct = macroMenu->addAction(tr("重播一次"), this, [this]() {
        if (auto e = activeEditor()) e->playMacro(1);
    });
    playAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_M));   // 讓出 Ctrl+Shift+P 給命令面板
    macroMenu->addAction(tr("重播 N 次…"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        bool ok = false;
        int n = QInputDialog::getInt(this, tr("重播巨集"),
                                     tr("次數："), 10, 1, 10000, 1, &ok);
        if (ok) e->playMacro(n);
    });

    toolsMenu->addAction(tr("設定中心（LSP / Snippet / 快捷鍵）…"), this, [this]() { showSettingsCenter(); });

    // ---------- AI 輔助（OpenAI 相容端點：本機 LM Studio/Ollama 或雲端）----------
    QMenu* aiMenu = toolsMenu->addMenu(tr("AI 輔助"));
    QAction* aiCompleteAction = new QAction(tr("AI 補全（游標處）"), this);
    aiCompleteAction->setShortcut(QKeySequence(Qt::CTRL | Qt::ALT | Qt::Key_A));
    connect(aiCompleteAction, &QAction::triggered, this, &MainWindow::triggerAiCompletion);
    aiMenu->addAction(aiCompleteAction);
    aiMenu->addAction(tr("AI：解釋選取"), this, [this]() { runAiOnSelection(false); });
    aiMenu->addAction(tr("AI：重構選取"), this, [this]() { runAiOnSelection(true); });
    aiMenu->addSeparator();
    aiMenu->addAction(tr("編輯 AI 設定檔"), this, [this]() {
        openFileByPath(AIConfig::configFilePath());      // 存檔後自動重新載入
    });

    // ---------- 腳本外掛（QJSEngine；資料夾內 *.js 啟動時載入）----------
    pluginManager = new PluginManager(
        [this]() { return activeEditor(); },
        [this](const QString& path) { openFileByPath(path); },
        QString(), this);
    connect(pluginManager, &PluginManager::statusRequested, this,
            [this](const QString& msg, int ms) { statusBar()->showMessage(msg, ms); });
    pluginMenu = toolsMenu->addMenu(tr("腳本外掛"));
    connect(pluginManager, &PluginManager::commandsChanged, this, &MainWindow::rebuildPluginMenu);
    pluginManager->reload();

    QMenu* extMenu = toolsMenu->addMenu(tr("外部工具"));
    extMenu->addAction(tr("執行外部工具…"), this, [this]() { runExternalTool(); });
    extMenu->addAction(tr("編輯 LSP 設定檔"), this, [this]() {
        openFileByPath(LspManager::configFilePath());   // 首次啟動已寫入預設（clangd / pylsp）
    });
    extMenu->addAction(tr("編輯 Snippet 設定檔"), this, [this]() {
        openFileByPath(SnippetsController::configPath());  // 存檔後自動重新載入
    });
    extMenu->addAction(tr("編輯快捷鍵設定檔"), this, [this]() {
        applyKeymap();                                  // 確保模板已產生
        openFileByPath(keymapConfigPath());             // 存檔後自動重新載入
    });
    extMenu->addAction(tr("編輯工具設定檔"), this, [this]() {
        const QString cfg = sessionDir() + "/external_tools.json";
        if (!QFileInfo::exists(cfg)) {
            QFile f(cfg);
            if (f.open(QIODevice::WriteOnly))
                f.write(tr("[\n  {\"name\": \"\u7528\u8a18\u4e8b\u672c\u958b\u555f\", \"command\": \"notepad %FILE%\"},\n  {\"name\": \"Python \u57f7\u884c\", \"command\": \"python %FILE%\"}\n]\n").toUtf8());
        }
        openFileByPath(cfg);
    });

    QMenu* viewMenu = menuBar->addMenu(tr("View"));
    viewMenu->addAction(wrapAction);
    viewMenu->addAction(tailAction);
    viewMenu->addAction(fontAction);
    viewMenu->addSeparator();
    splitAction = new QAction(tr("分割視窗（同文件雙視圖）"), this);
    splitAction->setCheckable(true);
    splitAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_Backslash));
    connect(splitAction, &QAction::toggled, this, [this](bool on) {
        splitDock->setVisible(on);
        if (on) syncSplitView();
    });
    viewMenu->addAction(splitAction);
    QAction* mdAction = new QAction(tr("Markdown 預覽"), this);
    mdAction->setCheckable(true);
    connect(mdAction, &QAction::toggled, this, [this](bool on) {
        mdDock->setVisible(on);
        if (on) refreshMarkdownPreview();
    });
    viewMenu->addAction(mdAction);
    // Backlinks/Graph 的 dock 必須在這裡（用到之前）就建好，見 MarkdownLinkController 開頭註解：
    // 原本的寫法是 dock 晚點才 new，但下面 visibilityChanged 的 connect() 馬上就要用到，
    // 導致 connect 在 dock 還是 nullptr 時執行、Qt 印警告且完全沒接上。
    mdLinkController = new MarkdownLinkController(this, this);
    connect(mdLinkController, &MarkdownLinkController::statusMessage, this,
            [this](const QString& text, int ms) { statusBar()->showMessage(text, ms); });
    connect(mdLinkController, &MarkdownLinkController::fileOpenRequested, this,
            [this](const QString& path) { openFileByPath(path); });

    QAction* backlinksAction = new QAction(tr("Backlinks（反向連結）"), this);
    backlinksAction->setCheckable(true);
    connect(backlinksAction, &QAction::toggled, this, [this](bool on) {
        mdLinkController->backlinksDock()->setVisible(on);
        if (on) { mdLinkController->rebuildIndex(projectFolder, currentFilePath());
                  mdLinkController->refreshBacklinks(currentFilePath()); }
    });
    connect(mdLinkController->backlinksDock(), &QDockWidget::visibilityChanged, this,
            [backlinksAction](bool v) {
        if (backlinksAction->isChecked() != v) backlinksAction->setChecked(v);
    });
    viewMenu->addAction(backlinksAction);
    QAction* graphAction = new QAction(tr("關係圖（Graph）"), this);
    graphAction->setCheckable(true);
    connect(graphAction, &QAction::toggled, this, [this](bool on) {
        mdLinkController->graphDock()->setVisible(on);
        if (on) { mdLinkController->rebuildIndex(projectFolder, currentFilePath());
                  mdLinkController->showGraphView(currentFilePath()); }
    });
    connect(mdLinkController->graphDock(), &QDockWidget::visibilityChanged, this,
            [graphAction](bool v) {
        if (graphAction->isChecked() != v) graphAction->setChecked(v);
    });
    viewMenu->addAction(graphAction);

    // Function List 常駐面板（Notepad++ 風）：dock 一樣要先建好才能接 visibilityChanged
    functionListController = new FunctionListController(this, this);
    connect(functionListController, &FunctionListController::lineActivated, this,
            [this](int line) { if (CodeEditor* e = activeEditor()) e->gotoLine(line + 1); });
    QAction* functionListAction = new QAction(tr("函式清單（Function List）"), this);
    functionListAction->setCheckable(true);
    connect(functionListAction, &QAction::toggled, this, [this](bool on) {
        functionListController->dock()->setVisible(on);
        if (on) functionListController->refresh(activeEditor());
    });
    connect(functionListController->dock(), &QDockWidget::visibilityChanged, this,
            [functionListAction](bool v) {
        if (functionListAction->isChecked() != v) functionListAction->setChecked(v);
    });
    viewMenu->addAction(functionListAction);
    QAction* whitespaceAction = new QAction(tr("顯示空白字元"), this);
    whitespaceAction->setCheckable(true);
    whitespaceAction->setChecked(AppSettings().value("editor/showWhitespace", false).toBool());
    connect(whitespaceAction, &QAction::toggled, this, [this](bool on) {
        AppSettings().setValue("editor/showWhitespace", on);
        for (int i = 0; i < tabWidget->count(); ++i)
            if (auto* e = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
                e->setShowWhitespace(on);
    });
    viewMenu->addAction(whitespaceAction);
    QAction* stickyAction = new QAction(tr("Sticky Scroll（固定標頭）"), this);
    stickyAction->setCheckable(true);
    stickyAction->setChecked(AppSettings().value("editor/stickyScroll", true).toBool());
    connect(stickyAction, &QAction::toggled, this, [this](bool on) {
        AppSettings().setValue("editor/stickyScroll", on);
        for (int i = 0; i < tabWidget->count(); ++i)
            if (auto* e = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
                e->setStickyScrollEnabled(on);
    });
    viewMenu->addAction(stickyAction);
    QAction* minimapAction = new QAction(tr("Minimap 縮圖"), this);
    minimapAction->setCheckable(true);
    minimapAction->setChecked(AppSettings().value("editor/minimap", true).toBool());
    connect(minimapAction, &QAction::toggled, this, [this](bool on) {
        AppSettings().setValue("editor/minimap", on);
        for (int i = 0; i < tabWidget->count(); ++i)
            if (auto* e = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
                e->setMinimapEnabled(on);
    });
    viewMenu->addAction(minimapAction);
    QAction* termAction = new QAction(tr("終端機"), this);
    termAction->setCheckable(true);
    termAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_QuoteLeft));   // Ctrl+`
    connect(termAction, &QAction::toggled, this, [this](bool on) {
        termDock->setVisible(on);
        if (on) {
            terminal->startShell(projectFolder);   // 首次顯示時才啟動 shell
            terminal->setFocus();
        }
    });
    viewMenu->addAction(termAction);
    viewMenu->addSeparator();
    viewMenu->addAction(zoomInAction);
    viewMenu->addAction(zoomOutAction);
    viewMenu->addAction(zoomResetAction);
    viewMenu->addSeparator();
    viewMenu->addAction(prefsAction);

    QMenu* helpMenu = menuBar->addMenu(tr("Help"));
    helpMenu->addAction(tr("關於 AlexCode"), this, [this]() { showAbout(); });

    setupToolBar();

    // ---------- Filter Results dock（含 5.6 命中密度條）----------
    QDockWidget* dock = new QDockWidget("FILTER RESULTS", this);
    filterResultsDock = dock;
    dock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetClosable);
    QWidget* filterWrap = new QWidget(this);
    auto* filterLay = new QVBoxLayout(filterWrap);
    filterLay->setContentsMargins(0, 0, 0, 0);
    filterLay->setSpacing(0);
    timelineBar = new TimelineBar(this);
    resultsList = new QListWidget(this);
    filterLay->addWidget(timelineBar);
    filterLay->addWidget(resultsList);
    dock->setWidget(filterWrap);
    addDockWidget(Qt::BottomDockWidgetArea, dock);
    connect(resultsList, &QListWidget::itemDoubleClicked, this, &MainWindow::onResultDoubleClicked);
    connect(timelineBar, &TimelineBar::jumpToLine, this, [this](int line) {
        if (CodeEditor* e = activeEditor()) {
            e->gotoLine(line + 1);
            e->setFocus();
        }
    });

    // ---------- 專案檔案樹 ----------
    projectDock = new QDockWidget("EXPLORER", this);
    projectDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetClosable);
    fsModel = new GitFileSystemModel(this);
    fsModel->setFilter(QDir::AllDirs | QDir::Files | QDir::NoDotAndDotDot);
    fsTree = new QTreeView(this);
    fsTree->setModel(fsModel);
    fsTree->setHeaderHidden(true);
    for (int c = 1; c < fsModel->columnCount(); ++c) fsTree->hideColumn(c); // 只留檔名
    connect(fsTree, &QTreeView::doubleClicked, this, [this](const QModelIndex& idx) {
        if (!fsModel->isDir(idx)) openFileByPath(fsModel->filePath(idx));
    });
    projectDock->setWidget(fsTree);
    addDockWidget(Qt::LeftDockWidgetArea, projectDock);
    projectDock->hide(); // 開啟資料夾後才顯示

    quickOpen = new QuickOpenDialog(this);
    connect(quickOpen, &QuickOpenDialog::fileChosen, this, &MainWindow::openFileByPath);

    fileWatcher = new QFileSystemWatcher(this);
    connect(fileWatcher, &QFileSystemWatcher::fileChanged,
            this, &MainWindow::onFileChangedExternally);

    // ---------- Markdown 預覽 ----------
    mdDock = new QDockWidget("MARKDOWN PREVIEW", this);
    mdView = new QTextBrowser(this);
    mdView->setOpenLinks(false);                        // 自行處理連結（區分內部 wikilink / 外部）
    mdView->setOpenExternalLinks(false);
    connect(mdView, &QTextBrowser::anchorClicked, this, [this](const QUrl& url) {
        if (url.scheme() == QLatin1String("alexcode"))
            mdLinkController->openOrCreateWikilink(url.path(), projectFolder);   // path() 已解碼，如 "Project Alpha"
        else
            QDesktopServices::openUrl(url);
    });
    mdDock->setWidget(mdView);
    addDockWidget(Qt::RightDockWidgetArea, mdDock);
    mdDock->hide();
    mdTimer = new QTimer(this);
    mdTimer->setSingleShot(true);
    mdTimer->setInterval(500);
    connect(mdTimer, &QTimer::timeout, this, &MainWindow::refreshMarkdownPreview);

    functionListTimer = new QTimer(this);
    functionListTimer->setSingleShot(true);
    functionListTimer->setInterval(500);
    connect(functionListTimer, &QTimer::timeout, this, [this]() {
        if (functionListController->dock()->isVisible())
            functionListController->refresh(activeEditor());
    });

    // ---------- 專案符號索引（Source Insight 風）----------
    // 拆到 ProjectSymbolController：索引本身、Ctrl+T 對話框、找引用邏輯都搬過去了，
    // 這裡只接 statusBar()/開檔跳行 兩條 signal。
    symbolController = new ProjectSymbolController(this, lspController->refsList(), lspController->refsDock(), this);
    connect(symbolController, &ProjectSymbolController::statusMessage, this,
            [this](const QString& text, int ms) { statusBar()->showMessage(text, ms); });
    connect(symbolController, &ProjectSymbolController::symbolChosen, this,
            [this](const QString& file, int line) {
                openFileByPath(file);
                if (CodeEditor* e = activeEditor()) e->gotoLine(line + 1);   // line 0-based
            });

    // ---------- Backlinks / 關係圖 的 dock 已在 MarkdownLinkController 建構子建立 ----------
    // 這裡只留計時器：需要在觸發當下取「目前」的 projectFolder / 作用中檔案，
    // 所以留在 MainWindow（controller 維持不認識目前 UI 狀態，方法都吃參數）。
    mdIndexTimer = new QTimer(this);
    mdIndexTimer->setSingleShot(true);
    mdIndexTimer->setInterval(800);
    connect(mdIndexTimer, &QTimer::timeout, this, [this]() {
        mdLinkController->rebuildIndex(projectFolder, currentFilePath());
    });

    // ---------- 互動式終端機（ConPTY；首次顯示時才啟動 shell）----------
    termDock = new QDockWidget("TERMINAL", this);
    terminal = new TerminalWidget(this);
    termDock->setWidget(terminal);
    addDockWidget(Qt::BottomDockWidgetArea, termDock);
    termDock->hide();

    // ---------- 分割視窗（共用 QTextDocument 的第二視圖，編輯即時同步）----------
    splitDock = new QDockWidget("SPLIT VIEW", this);
    splitEditor = new CodeEditor(this);
    splitEditor->setFont(defaultEditorFont);
    splitOwnDoc = splitEditor->document();           // 後備空白文件（editor 自有）
    splitDock->setWidget(splitEditor);
    addDockWidget(Qt::RightDockWidgetArea, splitDock);
    splitDock->hide();
    connect(splitDock, &QDockWidget::visibilityChanged, this, [this](bool visible) {
        if (splitAction && splitAction->isChecked() != visible)
            splitAction->setChecked(visible);        // 點 X 關閉時同步選單勾選
    });
    connect(tabWidget, &QTabWidget::currentChanged, this, [this](int) {
        syncSplitView();
        refreshMarkdownPreview();
        mdLinkController->refreshBacklinks(currentFilePath());
    });

    // ---------- 輸出面板（終端機 v1：指令執行 + 輸出 + 點錯誤跳行）----------
    outputDock = new QDockWidget("OUTPUT / TERMINAL", this);
    QWidget* outWrap = new QWidget(this);
    auto* outLay = new QVBoxLayout(outWrap);
    outLay->setContentsMargins(4, 4, 4, 4);
    outputView = new QPlainTextEdit(this);
    outputView->setReadOnly(true);
    outputView->setMaximumBlockCount(5000);
    cmdInput = new QLineEdit(this);
    cmdInput->setPlaceholderText(tr("輸入指令並按 Enter（工作目錄＝專案資料夾）；雙擊錯誤訊息可跳至該行"));
    outLay->addWidget(outputView);
    outLay->addWidget(cmdInput);
    outputDock->setWidget(outWrap);
    addDockWidget(Qt::BottomDockWidgetArea, outputDock);
    outputDock->hide();
    connect(cmdInput, &QLineEdit::returnPressed, this, [this]() {
        const QString c = cmdInput->text().trimmed();
        if (!c.isEmpty()) { runCommand(c); cmdInput->clear(); }
    });
    outputView->viewport()->installEventFilter(this);

    // ---------- Git 狀態 ----------
    gitTimer = new QTimer(this);
    gitTimer->setInterval(30000);
    connect(gitTimer, &QTimer::timeout, this, &MainWindow::updateGitStatus);

    createEditorTab("Untitled");
}

void MainWindow::setupToolBar() {
    QToolBar* mainToolBar = addToolBar("Main");
    mainToolBar->setMovable(false);
    mainToolBar->addAction(newAction);
    mainToolBar->addAction(openAction);
    mainToolBar->addAction(saveAction);
    mainToolBar->addSeparator();
    mainToolBar->addAction(undoAction);
    mainToolBar->addAction(redoAction);
    mainToolBar->addSeparator();
    mainToolBar->addAction(findAction);

    // 篩選工具列：關鍵字以 || 分隔，支援 AND/OR 與 Fuzzy 模糊比對
    QToolBar* filterToolBar = addToolBar("Filter");
    filterToolBar->setMovable(false);

    QLabel* filterIcon = new QLabel(tr("  \xE2\x9A\xA1 FILTER "), this);
    filterIcon->setStyleSheet(QStringLiteral("color:%1; font-weight:700; letter-spacing:1px;")
                                  .arg(Theme::ACCENT2));
    filterToolBar->addWidget(filterIcon);

    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText(tr("error && !heartbeat || fatal （||=或、&&=且、!=排除）"));
    filterInput->setClearButtonEnabled(true);
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    fuzzyCheck = new QCheckBox("Fuzzy", this);
    fuzzyCheck->setToolTip(tr("模糊比對：子序列匹配，例如 mwin 可比對 MainWindow"));
    filterBtn = new QPushButton("Filter", this);
    filterCountLabel = new QLabel("", this);
    filterCountLabel->setStyleSheet(QStringLiteral("color:%1; padding:0 8px;").arg(Theme::ACCENT));

    // ---------- Find & Replace 子系統（拆到 FindController）----------
    // 依賴 resultsList / filterCountLabel / filterResultsDock（Find All 沿用 FILTER RESULTS 面板），
    // 所以在這裡（皆已建好）才建立；Search 選單的 action 用 lambda 延遲呼叫
    findController = new FindController(
        this,
        [this]() { return activeEditor(); },
        [this]() {
            QList<CodeEditor*> editors;
            for (int i = 0; i < tabWidget->count(); ++i)
                if (auto* e = qobject_cast<CodeEditor*>(tabWidget->widget(i))) editors.append(e);
            return editors;
        },
        resultsList, filterCountLabel, filterResultsDock, this);
    connect(findController, &FindController::statusMessage, this,
            [this](const QString& text, int ms) { statusBar()->showMessage(text, ms); });

    filterToolBar->addWidget(filterInput);
    filterToolBar->addWidget(logicCombo);
    filterToolBar->addWidget(fuzzyCheck);
    filterToolBar->addWidget(filterBtn);
    QPushButton* extractBtn = new QPushButton(tr("→ Tab"), this);
    extractBtn->setToolTip(tr("把篩選結果抽取成新分頁（可再次篩選做漏斗分析）"));
    filterToolBar->addWidget(extractBtn);

    // 5.4 篩選預設集：選擇即套用；💾 存目前條件、🗑 刪除選取的預設
    presetCombo = new QComboBox(this);
    presetCombo->setMinimumWidth(120);
    presetCombo->setToolTip(tr("篩選預設集（選擇即套用）"));
    loadFilterPresets();
    QPushButton* savePresetBtn = new QPushButton(tr("💾"), this);
    savePresetBtn->setFixedWidth(28);
    savePresetBtn->setToolTip(tr("將目前篩選條件存為預設"));
    QPushButton* delPresetBtn = new QPushButton(tr("🗑"), this);
    delPresetBtn->setFixedWidth(28);
    delPresetBtn->setToolTip(tr("刪除選取的預設"));
    filterToolBar->addWidget(presetCombo);
    filterToolBar->addWidget(savePresetBtn);
    filterToolBar->addWidget(delPresetBtn);
    filterToolBar->addWidget(filterCountLabel);

    connect(presetCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int idx) {
        const QVariantMap m = presetCombo->itemData(idx).toMap();
        if (m.isEmpty()) return;
        filterInput->setText(m.value("query").toString());
        fuzzyCheck->setChecked(m.value("fuzzy").toBool());
        logicCombo->setCurrentText(m.value("logic", "OR").toString());
        runFilter();
    });
    connect(savePresetBtn, &QPushButton::clicked, this, [this]() {
        const QString query = filterInput->text().trimmed();
        if (query.isEmpty()) return;
        bool ok = false;
        const QString name = QInputDialog::getText(this, tr("儲存篩選預設"),
            tr("預設名稱："), QLineEdit::Normal, query.left(24), &ok).trimmed();
        if (!ok || name.isEmpty()) return;
        AppSettings settings;
        QVariantMap presets = settings.value("filterPresets").toMap();
        presets.insert(name, QVariantMap{
            {"query", query}, {"fuzzy", fuzzyCheck->isChecked()},
            {"logic", logicCombo->currentText()}});
        settings.setValue("filterPresets", presets);
        loadFilterPresets();
        presetCombo->setCurrentText(name);
    });
    connect(delPresetBtn, &QPushButton::clicked, this, [this]() {
        const QString name = presetCombo->currentText();
        if (name.isEmpty() || presetCombo->currentIndex() < 0) return;
        AppSettings settings;
        QVariantMap presets = settings.value("filterPresets").toMap();
        if (presets.remove(name)) {
            settings.setValue("filterPresets", presets);
            loadFilterPresets();
        }
    });
    connect(extractBtn, &QPushButton::clicked, this, [this]() {
        if (resultsList->count() == 0) return;
        QStringList lines;
        for (int i = 0; i < resultsList->count(); ++i)
            lines << resultsList->item(i)->text();
        CodeEditor* e = createEditorTab(tr("Filtered (%1)").arg(resultsList->count()));
        e->setPlainText(lines.join(QStringLiteral("\n")));
        e->document()->setModified(false);
    });
    connect(filterBtn, &QPushButton::clicked, this, &MainWindow::runFilter);
    connect(filterInput, &QLineEdit::returnPressed, this, &MainWindow::runFilter);
}

void MainWindow::loadFilterPresets() {
    presetCombo->clear();
    const QVariantMap presets =
        AppSettings().value("filterPresets").toMap();
    for (auto it = presets.begin(); it != presets.end(); ++it)
        presetCombo->addItem(it.key(), it.value());
    presetCombo->setCurrentIndex(-1);
}

void MainWindow::setupStatusBar() {
    QStatusBar* sb = statusBar();
    statusLineCol  = new QLabel("Ln 1, Col 1", this);
    statusSel      = new QLabel("Sel 0", this);
    statusLines    = new QLabel("1 lines", this);
    statusLang     = new QLabel("Plain Text", this);
    statusGit = new QLabel("", this);
    statusBlame = new QLabel("", this);
    statusBlame->setStyleSheet(QStringLiteral("color:%1;").arg(Theme::SYN_COMMENT));   // 低調的行內 blame
    statusEncoding = new QLabel("UTF-8", this);
    statusEol      = new QLabel("LF", this);
    statusEncoding->setToolTip(tr("點擊切換編碼（重新載入或轉換）"));
    statusEol->setToolTip(tr("點擊切換換行符（儲存時生效）"));
    statusEncoding->setCursor(Qt::PointingHandCursor);
    statusEol->setCursor(Qt::PointingHandCursor);
    statusEncoding->installEventFilter(this);
    statusEol->installEventFilter(this);
    sb->addPermanentWidget(statusBlame);                     // 游標行 blame（一般訊息區右側）
    sb->addPermanentWidget(lspStatusController->label());   // QLabel 已在 setupLsp() 建好，這裡只負責排位置
    sb->addPermanentWidget(statusGit);
    sb->addPermanentWidget(statusLineCol);
    sb->addPermanentWidget(statusSel);
    sb->addPermanentWidget(statusLines);
    sb->addPermanentWidget(statusLang);
    sb->addPermanentWidget(statusEncoding);
    sb->addPermanentWidget(statusEol);
    sb->showMessage("Ready");
}

void MainWindow::updateStatusBar() {
    CodeEditor* editor = activeEditor();
    if (!editor || !statusLineCol) return;
    QTextCursor c = editor->textCursor();
    statusLineCol->setText(QString("Ln %1, Col %2")
                               .arg(c.blockNumber() + 1)
                               .arg(c.positionInBlock() + 1));
    statusSel->setText(QString("Sel %1").arg(c.selectedText().length()));
    statusLines->setText(QString("%1 lines").arg(editor->blockCount()));
    statusLang->setText(editor->property("language").toString());
    const QString enc = editor->property("encoding").toString();
    statusEncoding->setText(enc.isEmpty() ? "UTF-8" : enc);
    const QString eol = editor->property("eol").toString();
    statusEol->setText(eol.isEmpty() ? "LF" : eol);
    lspStatusController->update(editor);
    if (statusBlame) {                                       // 游標行的 git blame（快取查詢，無 I/O）
        const QString path = editor->property("filePath").toString();
        statusBlame->setText(path.isEmpty() ? QString()
            : gitGutterController->blameTextForLine(path, c.blockNumber()));
    }
}

// ----------------------------------------------------------------
// 分頁標題：未儲存以 ● 標示
// ----------------------------------------------------------------
void MainWindow::updateTabTitle(CodeEditor* editor) {
    if (!editor) return;
    int idx = tabWidget->indexOf(editor);
    if (idx < 0) return;
    QString base = editor->property("baseTitle").toString();
    if (base.isEmpty()) base = "Untitled";
    const bool modified = editor->document()->isModified();
    tabWidget->setTabText(idx, modified ? tr("\xE2\x97\x8F ") + base : base);
}

void MainWindow::onModificationChanged(bool /*modified*/) {
    // sender 是 QTextDocument，找到所屬編輯器
    QTextDocument* doc = qobject_cast<QTextDocument*>(sender());
    if (!doc) return;
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (editor && editor->document() == doc) {
            updateTabTitle(editor);
            return;
        }
    }
}

// ----------------------------------------------------------------
// 檔案操作
// ----------------------------------------------------------------
void MainWindow::newFile() {
    createEditorTab("Untitled");
}

void MainWindow::openTerminalForShot(const QString& cmd) {
    termDock->show();
    terminal->startShell(projectFolder);
    if (!cmd.isEmpty()) terminal->sendText((cmd + QStringLiteral("\r")).toUtf8());
}

void MainWindow::syncSplitView() {
    if (!splitDock || !splitDock->isVisible()) return;
    CodeEditor* e = activeEditor();
    splitEditor->setDocument(e ? e->document() : splitOwnDoc);
    splitEditor->setFont(defaultEditorFont);
}

void MainWindow::refreshMarkdownPreview() {
    if (!mdDock || !mdDock->isVisible()) return;
    CodeEditor* e = activeEditor();
    const QString raw = e ? e->toPlainText() : QString();
    // 高擬真：套用 GitHub 風 CSS，並把 [[wikilink]] 轉成可點擊的內部連結
    mdView->document()->setDefaultStyleSheet(MarkdownRender::styleSheet());
    mdView->document()->setMarkdown(MarkdownRender::preprocessWikilinks(raw));
}

// ----------------------------------------------------------------
// Snippet 樣板（與 LSP / 外部工具相同模式：首次啟動寫入預設 JSON）
// ----------------------------------------------------------------
// ----------------------------------------------------------------
// 6.2 快捷鍵自訂：以動作顯示名稱為鍵的 JSON；空字串 = 移除快捷鍵
// 僅涵蓋選單動作（編輯器內建鍵如 F12 / Ctrl+Space 不在此列）
// ----------------------------------------------------------------
QString MainWindow::keymapConfigPath() {
    return Portable::dataDir() + QStringLiteral("/alexcode-keys.json");
}

void MainWindow::applyKeymap() {
    QHash<QString, QAction*> registry;
    for (QAction* a : findChildren<QAction*>()) {
        if (a->shortcut().isEmpty() || a->text().isEmpty()) continue;
        registry.insert(QString(a->text()).remove(QLatin1Char('&')), a);
    }

    const QString cfgPath = keymapConfigPath();
    if (!QFileInfo::exists(cfgPath)) {                   // 首次：輸出現況為模板
        QJsonObject o;
        for (auto it = registry.begin(); it != registry.end(); ++it)
            o.insert(it.key(), it.value()->shortcut().toString());
        QFile f(cfgPath);
        if (f.open(QIODevice::WriteOnly))
            f.write(QJsonDocument(o).toJson(QJsonDocument::Indented));
        return;                                          // 模板即現況，無需套用
    }

    QFile f(cfgPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    const QJsonObject o = QJsonDocument::fromJson(f.readAll()).object();
    QHash<QString, QString> bySeq;                       // 衝突偵測：快捷鍵 → 動作名
    QStringList conflicts;
    for (auto it = o.begin(); it != o.end(); ++it) {
        QAction* a = registry.value(it.key());
        if (!a) continue;
        const QKeySequence seq(it.value().toString());
        a->setShortcut(seq);
        const QString s = seq.toString();
        if (s.isEmpty()) continue;
        if (bySeq.contains(s))
            conflicts << QStringLiteral("%1 ⇄ %2（%3）").arg(bySeq.value(s), it.key(), s);
        else
            bySeq.insert(s, it.key());
    }
    if (!conflicts.isEmpty())
        statusBar()->showMessage(
            tr("⚠ 快捷鍵衝突：") + conflicts.join(QStringLiteral("；")), 8000);
}

void MainWindow::openFile() {
    QString fileName = QFileDialog::getOpenFileName(this, "Open File");
    if (fileName.isEmpty()) return;
    openFileByPath(fileName);
}

void MainWindow::openFileByPath(const QString& fileName) {
    QFile file(fileName);
    if (!file.open(QIODevice::ReadOnly)) {
        QMessageBox::warning(this, "Error", "Cannot open file:\n" + file.errorString());
        return;
    }
    const QByteArray raw = file.readAll();
    file.close();

    // 換行符偵測
    const QString eol = raw.contains("\r\n") ? QStringLiteral("CRLF") : QStringLiteral("LF");
    // 編碼偵測：UTF-8 驗證失敗 → 嘗試 Big5
    QString encName = QStringLiteral("UTF-8");
    auto utf8 = QStringDecoder(QStringDecoder::Utf8);
    QString text = utf8.decode(raw);
    if (utf8.hasError()) {
        QTextCodec* big5 = QTextCodec::codecForName("Big5");
        if (big5) {
            QTextCodec::ConverterState st;
            const QString t2 = big5->toUnicode(raw.constData(), raw.size(), &st);
            if (st.invalidChars == 0) { text = t2; encName = QStringLiteral("Big5"); }
        }
    }
    text.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));

    for (int i = 0; i < tabWidget->count(); ++i) {
        CodeEditor* editor = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (editor && editor->property("filePath").toString() == fileName) {
            tabWidget->setCurrentIndex(i);
            return;
        }
    }

    QFileInfo fileInfo(fileName);
    CodeEditor* newEditor = createEditorTab(fileInfo.fileName());

    // 大檔分級降級（閾值可在偏好設定調整，取代過去單一 50MB 門檻）：
    //   > highlightMaxMB（預設 10）→ 連語法高亮一併關閉（完整大檔模式）
    //   > assistMaxMB   （預設 2） → 關閉自動補全 / LSP / 即時 Git gutter，但保留高亮
    AppSettings sizeSettings;
    const double mb = raw.size() / (1024.0 * 1024.0);
    const int hiMaxMB = sizeSettings.value("editor/highlightMaxMB", 10).toInt();
    const int asMaxMB = sizeSettings.value("editor/assistMaxMB", 2).toInt();
    const FileTier::Level tier = FileTier::forSizeMB(mb, asMaxMB, hiMaxMB);
    const bool highlightOff = tier == FileTier::Level::HighlightOff;
    const bool assistOff = tier != FileTier::Level::Full;

    if (highlightOff) {
        newEditor->setLargeFileMode(true);
        newEditor->setLineWrapMode(QPlainTextEdit::NoWrap);
        statusBar()->showMessage(
            tr("大檔（%1 MB）：已停用語法高亮與自動完成以確保流暢").arg(qRound(mb)), 5000);
    } else if (assistOff) {
        statusBar()->showMessage(
            tr("較大檔（%1 MB）：已停用自動完成 / LSP / 即時 Git 標示，保留語法高亮").arg(qRound(mb)), 5000);
    }
    newEditor->setAssistEnabled(!assistOff);
    newEditor->setProperty("bigFile", highlightOff);
    newEditor->setPlainText(text);
    newEditor->document()->setModified(false);

    newEditor->setProperty("filePath", fileName);
    newEditor->setProperty("baseTitle", fileInfo.fileName());
    newEditor->setProperty("encoding", encName);
    newEditor->setProperty("eol", eol);
    newEditor->setShowWhitespace(AppSettings().value("editor/showWhitespace", false).toBool());
    tabWidget->setTabToolTip(tabWidget->indexOf(newEditor), fileName);
    updateTabTitle(newEditor);

    if (!highlightOff) applyHighlighterForPath(newEditor, fileName);
    // LSP didOpen 與 Git gutter 延到分頁實際成為作用中才做（見 ensureEditorActivated）。
    // 前景開檔時該分頁已是當前分頁，立即啟用；串流還原期間則跳過，等切到才啟用。
    if (!restoringSession && newEditor == tabWidget->currentWidget())
        ensureEditorActivated(newEditor);
    recentFilesController->addFile(fileName);
    if (fileWatcher) fileWatcher->addPath(fileName);
    statusBar()->showMessage("Opened " + fileName, 3000);
}

// 分頁首次成為作用中時才啟用重量級服務：LSP didOpen（含首個 clangd 啟動）+ Git gutter
// （每檔 2 個 git 子行程）。以 "activated" 屬性守衛，確保每分頁只做一次。
void MainWindow::ensureEditorActivated(CodeEditor* editor) {
    if (!editor) return;
    if (editor->property("activated").toBool()) return;
    const QString path = editor->property("filePath").toString();
    if (path.isEmpty()) return;                      // Untitled / 僅備份的分頁：無事可做，且不標記已啟用
    editor->setProperty("activated", true);
    if (!editor->assistEnabled()) return;            // 大檔降級：維持停用 LSP / Git（與開檔時一致）
    if (!lspController->lsp()->languageIdForFile(path).isEmpty()) {
        editor->setLspEnabled(true);
        lspController->lsp()->documentOpened(path, editor->toPlainText());
    }
    gitGutterController->fetchHead(path);             // Git gutter 行狀態
    gitGutterController->fetchBlame(path);            // 狀態列行內 blame
}

void MainWindow::saveFileAs() {
    CodeEditor* currentEditor = activeEditor();
    if (!currentEditor) return;
    currentEditor->setProperty("filePath", QString());
    saveFile();
}

void MainWindow::saveAllFiles() {
    const int current = tabWidget->currentIndex();
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (editor && editor->document()->isModified()) {
            tabWidget->setCurrentIndex(i);
            saveFile();
        }
    }
    tabWidget->setCurrentIndex(current);
}

void MainWindow::saveFile() {
    CodeEditor* currentEditor = activeEditor();
    if (!currentEditor) return;

    QString fileName = currentEditor->property("filePath").toString();
    if (fileName.isEmpty()) {
        fileName = QFileDialog::getSaveFileName(this, "Save File");
        if (fileName.isEmpty()) return;
        QFileInfo fileInfo(fileName);
        currentEditor->setProperty("filePath", fileName);
        currentEditor->setProperty("baseTitle", fileInfo.fileName());
        tabWidget->setTabToolTip(tabWidget->currentIndex(), fileName);
        applyHighlighterForPath(currentEditor, fileName);
        recentFilesController->addFile(fileName);
    }

    QString originalText = currentEditor->toPlainText();

    auto doSave = [this, currentEditor](const QString& fileToSave, const QString& textToSave) {
        QString text = textToSave;
        // 存檔時修剪行尾空白（設定）
        if (AppSettings().value("editor/trimTrailing", false).toBool()) {
            static const QRegularExpression trailing(QStringLiteral("[ \\t]+(?=\\n)|[ \\t]+$"));
            text.remove(trailing);
        }
        if (currentEditor->property("eol").toString() == "CRLF")
            text.replace(QStringLiteral("\n"), QStringLiteral("\r\n"));
        QByteArray bytes;
        if (currentEditor->property("encoding").toString() == "Big5") {
            if (QTextCodec* big5 = QTextCodec::codecForName("Big5")) bytes = big5->fromUnicode(text);
            else bytes = text.toUtf8();
        } else {
            bytes = text.toUtf8();
        }
        QFile file(fileToSave);
        if (!file.open(QIODevice::WriteOnly)) {
            QMessageBox::warning(this, "Error", "Cannot save file:\n" + file.errorString());
            return;
        }
        file.write(bytes);
        file.close();
        currentEditor->document()->setModified(false);
        updateTabTitle(currentEditor);
        statusBar()->showMessage("Saved " + fileToSave, 3000);
        // 存檔 Markdown → 重建連結索引（debounce），讓 backlinks / 關係圖不過期
        if (mdIndexTimer && !projectFolder.isEmpty()
            && (fileToSave.endsWith(".md", Qt::CaseInsensitive)
                || fileToSave.endsWith(".markdown", Qt::CaseInsensitive)))
            mdIndexTimer->start();
        symbolController->updateFile(projectFolder, fileToSave);   // 存檔 → 增量更新專案符號索引
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
                if (safeEditor) {
                    safeEditor->document()->setModified(false);
                    updateTabTitle(safeEditor);
                }
                statusBar()->showMessage("Saved " + fileName, 3000);
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

    // LSP：另存新檔後若語言有對應伺服器則啟用；通知 didSave
    if (!currentEditor->lspEnabled() && !currentEditor->property("bigFile").toBool()
        && !lspController->lsp()->languageIdForFile(fileName).isEmpty()) {
        currentEditor->setLspEnabled(true);
        lspController->lsp()->documentOpened(fileName, currentEditor->toPlainText());
        lspStatusController->update(currentEditor);
    }
    lspController->lsp()->documentSaved(fileName);

    // Snippet 設定檔存檔 → 立即重載並套用至所有分頁
    if (fileName == SnippetsController::configPath()) {
        snippetsController->load();
        for (int i = 0; i < tabWidget->count(); ++i)
            snippetsController->applyTo(qobject_cast<CodeEditor*>(tabWidget->widget(i)));
        statusBar()->showMessage(tr("Snippet 設定已重新載入（%1 個）")
                                     .arg(snippetsController->count()), 3000);
    }
    // AI 設定檔存檔 → 立即重載
    if (fileName == AIConfig::configFilePath()) {
        aiProvider->reloadConfig();
        statusBar()->showMessage(tr("AI 設定已重新載入"), 3000);
    }
    // 快捷鍵設定檔存檔 → 立即重新套用
    if (fileName == keymapConfigPath()) {
        applyKeymap();
        statusBar()->showMessage(tr("快捷鍵設定已重新套用"), 3000);
    }

    // Git gutter：另存的新路徑可能在版控中，重抓 HEAD（已知未版控者不重試）
    if (!gitGutterController->hasHead(fileName)) gitGutterController->fetchHead(fileName);
    else gitGutterController->recompute(currentEditor);
    gitGutterController->invalidateBlame(fileName);   // 存檔後行號位移，blame 重抓
    gitGutterController->fetchBlame(fileName);
}

// ----------------------------------------------------------------
// 關閉分頁 / 視窗（未儲存提示）
// ----------------------------------------------------------------
bool MainWindow::maybeSave(int index) {
    auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(index));
    if (!editor || !editor->document()->isModified()) return true;

    QString name = editor->property("baseTitle").toString();
    if (name.isEmpty()) name = "Untitled";
    auto ret = QMessageBox::warning(this, "Unsaved Changes",
        tr("\"%1\" 尚未儲存，要儲存變更嗎？").arg(name),
        QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel);
    if (ret == QMessageBox::Cancel) return false;
    if (ret == QMessageBox::Save) {
        tabWidget->setCurrentIndex(index);
        saveFile();
        return !editor->document()->isModified();
    }
    return true;
}

bool MainWindow::closeTab(int index) {
    if (!maybeSave(index)) return false;
    auto widget = tabWidget->widget(index);
    if (auto e = qobject_cast<CodeEditor*>(widget)) {
        const QString path = e->property("filePath").toString();
        if (!path.isEmpty()) lspController->lsp()->documentClosed(path);
        lspSyncController->editorClosed(e);
        if (splitEditor && splitEditor->document() == e->document())
            splitEditor->setDocument(splitOwnDoc);   // 文件即將隨分頁銷毀
    }
    tabWidget->removeTab(index);
    widget->deleteLater();
    syncSplitView();
    return true;
}

void MainWindow::closeAllTabs() {
    while (tabWidget->count() > 0) {
        if (!closeTab(tabWidget->count() - 1)) return;
    }
}

void MainWindow::closeEvent(QCloseEvent* event) {
    // Notepad++ 風格：關閉時不逐一詢問，整個工作階段（含未儲存內容）快照保存，
    // 下次啟動完整還原。個別關閉分頁 (Ctrl+W) 仍會提示儲存。
    saveSession();
    event->accept();
}

// ----------------------------------------------------------------
// AI 輔助：ghost 補全（游標前文脈）與選取指令（解釋/重構）
// ----------------------------------------------------------------
void MainWindow::triggerAiCompletion() {
    CodeEditor* e = activeEditor();
    if (!e) return;
    QTextCursor c = e->textCursor();
    const int startBlock = qMax(0, c.blockNumber() - 200);   // 至多帶 200 行前文
    QTextCursor ctx(e->document());
    ctx.setPosition(e->document()->findBlockByNumber(startBlock).position());
    ctx.setPosition(c.position(), QTextCursor::KeepAnchor);
    const QString context = ctx.selectedText().replace(QChar(0x2029), QChar('\n'));
    if (context.trimmed().isEmpty()) return;
    statusBar()->showMessage(tr("AI 補全請求中…"), 0);
    aiProvider->requestCompletion(context, e->property("language").toString());
}

void MainWindow::runAiOnSelection(bool refactor) {
    CodeEditor* e = activeEditor();
    if (!e) return;
    const QString sel = e->textCursor().selectedText().replace(QChar(0x2029), QChar('\n'));
    if (sel.trimmed().isEmpty()) { statusBar()->showMessage(tr("請先選取程式碼"), 3000); return; }
    m_aiRefactorMode = refactor;
    m_aiSelection = sel;
    m_aiCursor = e->textCursor();                        // 持久游標：套用時仍指向原選取
    statusBar()->showMessage(refactor ? tr("AI 重構中…") : tr("AI 解釋中…"), 0);
    const QString lang = e->property("language").toString();
    if (refactor) {
        aiProvider->requestChat(
            QStringLiteral("You are a senior %1 developer. Refactor the given code to be cleaner "
                           "and more idiomatic while preserving behavior. Output ONLY the "
                           "refactored code, no fences, no commentary.").arg(lang), sel);
    } else {
        aiProvider->requestChat(
            QStringLiteral("You are a senior %1 developer. Explain the given code concisely in "
                           "Traditional Chinese: purpose, key logic, pitfalls.").arg(lang), sel);
    }
}

void MainWindow::onAiChatReady(const QString& content) {
    statusBar()->clearMessage();
    if (content.trimmed().isEmpty()) { statusBar()->showMessage(tr("AI 沒有回應內容"), 3000); return; }
    if (m_aiRefactorMode) {
        showSideBySideDiff(tr("目前選取"), m_aiSelection, tr("AI 重構建議"), content);
        if (QMessageBox::question(this, tr("AI 重構"),
                                  tr("要以 AI 建議取代原選取內容嗎？（可 Ctrl+Z 復原）"))
                == QMessageBox::Yes) {
            if (!m_aiCursor.isNull()) m_aiCursor.insertText(content);
        }
    } else {
        CodeEditor* e = createEditorTab(tr("AI 解釋"));
        e->setPlainText(content);
        e->document()->setModified(false);
    }
}

// 外掛選單重建：指令列表（依註冊順序）+ 管理項
void MainWindow::rebuildPluginMenu() {
    if (!pluginMenu) return;
    pluginMenu->clear();
    const QStringList names = pluginManager->commandNames();
    for (const QString& name : names) {
        pluginMenu->addAction(name, this, [this, name]() { pluginManager->runCommand(name); });
    }
    if (!names.isEmpty()) pluginMenu->addSeparator();
    pluginMenu->addAction(tr("重新載入外掛"), this, [this]() {
        pluginManager->reload();
        statusBar()->showMessage(tr("外掛已重新載入：%1 個腳本、%2 個指令")
                                     .arg(pluginManager->scriptCount())
                                     .arg(pluginManager->commandNames().size()), 4000);
    });
    pluginMenu->addAction(tr("開啟外掛資料夾"), this, [this]() {
        QDesktopServices::openUrl(QUrl::fromLocalFile(pluginManager->pluginsDir()));
    });
    pluginMenu->addAction(tr("編輯範例外掛"), this, [this]() {
        openFileByPath(pluginManager->pluginsDir() + QStringLiteral("/examples.js"));
    });
}

// 標題列著色：原生標題列預設亮色，與深色主題格格不入。Win11（build 22000+）可直接
// 指定標題列底色/文字色/邊框色；Win10 沒有這些屬性（呼叫失敗無害），退回「沉浸式深色
// 模式」讓標題列變深灰。Paper Light 主題時底色是亮的，深色模式旗標會自動關閉。
void MainWindow::applyTitleBarTheme() {
#ifdef Q_OS_WIN
    const QColor bg(Theme::LINE_NUM_BG);        // 與選單列/面板同色
    const QColor fg(Theme::EDITOR_FG);
    const HWND hwnd = reinterpret_cast<HWND>(winId());
    BOOL dark = bg.lightness() < 128;
    DwmSetWindowAttribute(hwnd, 20 /*DWMWA_USE_IMMERSIVE_DARK_MODE*/, &dark, sizeof(dark));
    COLORREF caption = RGB(bg.red(), bg.green(), bg.blue());
    COLORREF text    = RGB(fg.red(), fg.green(), fg.blue());
    DwmSetWindowAttribute(hwnd, 35 /*DWMWA_CAPTION_COLOR*/, &caption, sizeof(caption));
    DwmSetWindowAttribute(hwnd, 34 /*DWMWA_BORDER_COLOR*/,  &caption, sizeof(caption));
    DwmSetWindowAttribute(hwnd, 36 /*DWMWA_TEXT_COLOR*/,    &text, sizeof(text));
#endif
}

// 自繪霓虹線條圖示：呼叫當下取 Theme 色，主題切換後重呼叫即可換色
void MainWindow::applyActionIcons() {
    newAction->setIcon(NeonIcons::icon(QStringLiteral("new")));
    openAction->setIcon(NeonIcons::icon(QStringLiteral("open")));
    saveAction->setIcon(NeonIcons::icon(QStringLiteral("save")));
    undoAction->setIcon(NeonIcons::icon(QStringLiteral("undo")));
    redoAction->setIcon(NeonIcons::icon(QStringLiteral("redo")));
    findAction->setIcon(NeonIcons::icon(QStringLiteral("find")));
    findInFilesAction->setIcon(NeonIcons::icon(QStringLiteral("find-in-files")));
}

// ----------------------------------------------------------------
// 拖放開檔
// ----------------------------------------------------------------
void MainWindow::dragEnterEvent(QDragEnterEvent* event) {
    if (event->mimeData()->hasUrls()) event->acceptProposedAction();
}

void MainWindow::dropEvent(QDropEvent* event) {
    const QList<QUrl> urls = event->mimeData()->urls();
    for (const QUrl& url : urls) {
        if (url.isLocalFile()) openFileByPath(url.toLocalFile());
    }
}


void MainWindow::showGotoLineDialog() {
    auto editor = activeEditor();
    if (!editor) return;
    bool ok = false;
    int line = QInputDialog::getInt(this, "Go to Line",
                                    QString("Line (1 - %1):").arg(editor->blockCount()),
                                    editor->textCursor().blockNumber() + 1,
                                    1, editor->blockCount(), 1, &ok);
    if (ok) editor->gotoLine(line);
}

// 「標記符合的行為書籤」小對話框（Notepad++ 的 Search→Bookmark→Bookmark Line... 對應功能）
void MainWindow::showBookmarkMatchingDialog() {
    CodeEditor* editor = activeEditor();
    if (!editor) return;

    QDialog dlg(this);
    dlg.setWindowTitle(tr("標記符合的行為書籤"));
    auto* layout = new QVBoxLayout(&dlg);
    layout->addWidget(new QLabel(tr("尋找目標："), &dlg));
    auto* patternEdit = new QLineEdit(&dlg);
    layout->addWidget(patternEdit);
    auto* caseBox = new QCheckBox(tr("大小寫相符"), &dlg);
    auto* regexBox = new QCheckBox(tr("正規表達式"), &dlg);
    layout->addWidget(caseBox);
    layout->addWidget(regexBox);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    patternEdit->setFocus();

    if (dlg.exec() != QDialog::Accepted) return;
    const QString pattern = patternEdit->text();
    if (pattern.isEmpty()) return;

    const int added = editor->bookmarkMatchingLines(pattern, caseBox->isChecked(), regexBox->isChecked());
    if (added < 0) {
        statusBar()->showMessage(tr("正規表達式錯誤"), 4000);
    } else {
        statusBar()->showMessage(tr("已將 %1 行符合的內容加入書籤").arg(added), 4000);
    }
}

// ----------------------------------------------------------------
// 行篩選（|| 多關鍵字 + Fuzzy + AND/OR）
// ----------------------------------------------------------------
void MainWindow::runFilter() {
    resultsList->clear();
    engine.setFuzzy(fuzzyCheck->isChecked());
    QString query = filterInput->text();
    // 沒有進階運算子時，沿用 AND/OR 下拉選擇
    if (logicCombo->currentText() == "AND" && !query.contains("&&") && !query.contains('!'))
        query = FilterEngine::parseQuery(query).join(QStringLiteral(" && "));
    engine.compile(query);

    CodeEditor* editor = activeEditor();
    if (!editor) return;

    // 多色標示：每個正向關鍵字配一個霓虹色
    static const QList<QColor> palette = {
        QColor("#ff5370"), QColor("#ffd166"), QColor("#00e5ff"),
        QColor("#ff2d95"), QColor("#c3f73a"), QColor("#b388ff") };
    QList<QPair<QString, QColor>> kwColors;
    const QStringList kws = engine.positiveKeywords();
    for (int i = 0; i < kws.size(); ++i)
        kwColors.append({kws[i], palette[i % palette.size()]});
    editor->setKeywordHighlights(query.trimmed().isEmpty()
                                     ? QList<QPair<QString, QColor>>() : kwColors);

    resultsList->setUpdatesEnabled(false);
    QTextBlock block = editor->document()->begin();
    int lineIndex = 0;
    int matchCount = 0;
    QList<int> matchedLines;                            // 5.6 密度條資料
    while (block.isValid()) {
        if (engine.matchCompiled(block.text())) {
            QListWidgetItem* item = new QListWidgetItem(
                QString("Line %1: %2").arg(lineIndex + 1).arg(block.text()));
            item->setData(Qt::UserRole, lineIndex);
            resultsList->addItem(item);
            matchedLines.append(lineIndex);
            matchCount++;
        }
        block = block.next();
        lineIndex++;
    }
    resultsList->setUpdatesEnabled(true);
    if (query.trimmed().isEmpty())
        timelineBar->clearData();
    else
        timelineBar->setData(matchedLines, lineIndex);
    filterCountLabel->setText(tr("%1 hits").arg(matchCount));
    statusBar()->showMessage(QString("Filter: %1 matching lines").arg(matchCount), 4000);
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
        editor->gotoLine(lineNum);
    }
}

// ----------------------------------------------------------------
// 專案資料夾 / Ctrl+P 快速開檔
// ----------------------------------------------------------------
void MainWindow::openFolder() {
    QString dir = QFileDialog::getExistingDirectory(this, "Open Folder", projectFolder);
    if (dir.isEmpty()) return;
    setProjectFolder(dir);
}

void MainWindow::setProjectFolder(const QString& folder) {
    projectFolder = folder;
    fsModel->setRootPath(folder);
    fsTree->setRootIndex(fsModel->index(folder));
    projectDock->setWindowTitle("EXPLORER — " + QFileInfo(folder).fileName().toUpper());
    projectDock->show();
    quickOpen->setRootFolder(folder);
    updateGitStatus();
    if (gitTimer) gitTimer->start();

    // LSP：根目錄改變 → 伺服器重啟，已開啟的文件重新 didOpen
    lspController->lsp()->setRootPath(folder);
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (!e || !e->lspEnabled()) continue;
        const QString path = e->property("filePath").toString();
        if (!path.isEmpty()) lspController->lsp()->documentOpened(path, e->toPlainText());
    }
    lspStatusController->update(activeEditor());

    mdLinkController->rebuildIndex(projectFolder, currentFilePath());   // 重建 Markdown 連結關係圖
    symbolController->rebuild(projectFolder);            // 建立專案符號索引（Source Insight 風）
    symbolController->enableAutoRefresh(projectFolder);  // 外部變更（git pull 等）自動增量更新索引
    statusBar()->showMessage("Folder: " + folder, 3000);
}

void MainWindow::openVaultForShot(const QString& folder) {
    setProjectFolder(folder);
    mdLinkController->backlinksDock()->show();
    mdLinkController->refreshBacklinks(currentFilePath());
}

void MainWindow::showMarkdownPreviewForShot() {
    if (mdDock) { mdDock->show(); mdDock->resize(560, 700); refreshMarkdownPreview(); }
}

void MainWindow::openGraphForShot(const QString& folder) {
    setProjectFolder(folder);
    mdLinkController->graphDock()->show();
    mdLinkController->graphDock()->resize(560, 520);
    mdLinkController->showGraphView(currentFilePath());
}

void MainWindow::showQuickOpen() {
    if (projectFolder.isEmpty()) {
        openFolder();
        if (projectFolder.isEmpty()) return;
    }
    quickOpen->open();
}

void MainWindow::gotoLineForShot(int line) {
    if (CodeEditor* e = activeEditor()) { e->gotoLine(line); updateBreadcrumb(); }
}

void MainWindow::setShowWhitespaceAll(bool on) {
    for (int i = 0; i < tabWidget->count(); ++i)
        if (auto* e = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
            e->setShowWhitespace(on);
}

static QVector<CommandPalette::Command> gatherCommands(MainWindow* w) {
    QVector<CommandPalette::Command> cmds;
    QSet<QString> seen;
    const QList<QAction*> actions = w->findChildren<QAction*>();
    for (QAction* a : actions) {
        if (a->isSeparator() || a->menu()) continue;         // 跳過分隔線與子選單
        QString name = a->text();
        name.remove(QLatin1Char('&'));                        // 去掉助記符
        name = name.trimmed();
        if (name.isEmpty() || seen.contains(name)) continue;
        seen.insert(name);
        cmds.append({ name, a->shortcut().toString(QKeySequence::NativeText), a });
    }
    std::sort(cmds.begin(), cmds.end(),
              [](const CommandPalette::Command& a, const CommandPalette::Command& b) {
                  return a.name.localeAwareCompare(b.name) < 0;
              });
    return cmds;
}

void MainWindow::showCommandPalette() {
    if (!commandPalette) commandPalette = new CommandPalette(this);
    commandPalette->openWith(gatherCommands(this));
}

void MainWindow::openMenuForShot(int index, const QString& outPng) {
    const QList<QAction*> acts = menuBar()->actions();
    if (index < 0 || index >= acts.size() || !acts[index]->menu()) return;
    QMenu* m = acts[index]->menu();
    m->popup(mapToGlobal(QPoint(40 + index * 70, 30)));
    QTimer::singleShot(700, this, [m, outPng]() { if (m) m->grab().save(outPng); });
}

void MainWindow::openCallGraphForShot(const QString& outPng) {
    CodeEditor* e = activeEditor();
    if (!e) return;
    QWidget* dlg = e->showCallGraphAt(e->textCursor().blockNumber());
    QTimer::singleShot(700, this, [dlg, outPng]() { if (dlg) dlg->grab().save(outPng); });
}

void MainWindow::findRefsForShot(const QString& name) {
    symbolController->buildSync(projectFolder);   // 截圖：同步建索引（避開背景非同步）
    symbolController->findReferences(name);
}

void MainWindow::openCommandPaletteForShot(const QString& outPng) {
    if (!commandPalette) commandPalette = new CommandPalette(this);
    commandPalette->openWith(gatherCommands(this), QStringLiteral("go"));
    QTimer::singleShot(700, this, [this, outPng]() {
        if (commandPalette) commandPalette->grab().save(outPng);
    });
}

void MainWindow::openProjectSymbolForShot(const QString& outPng) {
    symbolController->rebuild(projectFolder);
    symbolController->showSearch(projectFolder);
    QTimer::singleShot(700, this, [this, outPng]() {
        if (symbolController->dialog()) symbolController->dialog()->grab().save(outPng);
    });
}

void MainWindow::showGoToSymbol() {
    CodeEditor* e = activeEditor();
    if (!e) return;
    const QVector<TsSymbols::Symbol> syms = e->documentSymbols();
    if (syms.isEmpty()) {
        statusBar()->showMessage(
            tr("此檔無可用符號（Go to Symbol 目前支援 tree-sitter 語言：C/C++、Python、JS、JSON）"), 3500);
        return;
    }
    if (!symbolDialog) {
        symbolDialog = new SymbolDialog(this);
        connect(symbolDialog, &SymbolDialog::symbolChosen, this, [this](int line) {
            if (CodeEditor* ed = activeEditor()) ed->gotoLine(line + 1);   // line 為 0-based
        });
    }
    symbolDialog->openWith(syms);
}

// 麵包屑：檔 > 包含游標所在行的（巢狀）類別/函式，可點擊跳轉。
void MainWindow::updateBreadcrumb() {
    if (!breadcrumbLabel) return;
    CodeEditor* e = activeEditor();
    if (!e) { breadcrumbLabel->clear(); return; }
    const QString path = e->property("filePath").toString();
    QString fileName = path.isEmpty() ? tr("Untitled") : QFileInfo(path).fileName();

    QStringList parts;
    parts << QStringLiteral("<span style='color:%1'>%2</span>")
                 .arg(Theme::LINE_NUM_FG, fileName.toHtmlEscaped());

    const int curLine = e->textCursor().blockNumber();
    QVector<TsSymbols::Symbol> chain;
    for (const TsSymbols::Symbol& s : e->documentSymbols())
        if (s.line <= curLine && curLine <= s.endLine) chain.append(s);
    std::sort(chain.begin(), chain.end(),
              [](const TsSymbols::Symbol& a, const TsSymbols::Symbol& b) { return a.line < b.line; });
    for (const TsSymbols::Symbol& s : chain)
        parts << QStringLiteral("<a href='%1' style='color:%2; text-decoration:none'>%3</a>")
                     .arg(QString::number(s.line), Theme::ACCENT, s.name.toHtmlEscaped());

    breadcrumbLabel->setText(parts.join(QStringLiteral("  ›  ")));
}

// ----------------------------------------------------------------
// 外部變更偵測 + tail 模式
// ----------------------------------------------------------------
void MainWindow::onFileChangedExternally(const QString& path) {
    // 找到對應分頁
    CodeEditor* editor = nullptr;
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (e && e->property("filePath").toString() == path) { editor = e; break; }
    }
    // 檔案可能被覆寫重建，重新加回監看
    if (QFileInfo::exists(path) && !fileWatcher->files().contains(path))
        fileWatcher->addPath(path);
    symbolController->updateFile(projectFolder, path);   // 外部變更 → 增量更新符號索引
    if (!editor || !QFileInfo::exists(path)) return;

    const bool tail = tailAction && tailAction->isChecked();
    if (editor->document()->isModified() && !tail) {
        auto ret = QMessageBox::question(this, "File Changed",
            tr("\"%1\" 已被外部程式修改，且你有未儲存的變更。\n要放棄變更並重新載入嗎？")
                .arg(QFileInfo(path).fileName()),
            QMessageBox::Yes | QMessageBox::No);
        if (ret != QMessageBox::Yes) return;
    }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;
    const QString text = QTextStream(&file).readAll();
    file.close();

    // 保留游標與捲動位置（tail 模式則捲到底）
    const int cursorPos = editor->textCursor().position();
    const int scrollVal = editor->verticalScrollBar()->value();
    editor->setPlainText(text);
    editor->document()->setModified(false);
    updateTabTitle(editor);
    if (tail) {
        QTextCursor c = editor->textCursor();
        c.movePosition(QTextCursor::End);
        editor->setTextCursor(c);
        editor->verticalScrollBar()->setValue(editor->verticalScrollBar()->maximum());
        // 5.5 即時篩選聯動：tail 重載後，若篩選條件存在則重套（新進行立即反映於結果與密度條）
        if (editor == activeEditor() && !filterInput->text().trimmed().isEmpty())
            runFilter();
    } else {
        QTextCursor c = editor->textCursor();
        c.setPosition(qMin(cursorPos, editor->document()->characterCount() - 1));
        editor->setTextCursor(c);
        editor->verticalScrollBar()->setValue(scrollVal);
    }
    statusBar()->showMessage("Reloaded " + QFileInfo(path).fileName(), 2000);
}

// ----------------------------------------------------------------
// Session 還原
// ----------------------------------------------------------------
QString MainWindow::sessionDir() const {
    QString dir = Portable::dataDir();
    QDir().mkpath(dir + "/backup");
    return dir;
}

void MainWindow::saveSession() {
    QJsonArray tabs;
    const QString backupDir = sessionDir() + "/backup";
    // 清舊備份
    QDir bd(backupDir);
    for (const QString& f : bd.entryList(QDir::Files)) bd.remove(f);

    for (int i = 0; i < tabWidget->count(); ++i) {
        auto editor = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (!editor) continue;
        QJsonObject t;
        t["filePath"]  = editor->property("filePath").toString();
        t["baseTitle"] = editor->property("baseTitle").toString();
        t["cursor"]    = editor->textCursor().position();
        t["scroll"]    = editor->verticalScrollBar()->value();
        t["modified"]  = editor->document()->isModified();
        QJsonArray bms;
        for (int ln : editor->bookmarkedLines()) bms.append(ln);
        t["bookmarks"] = bms;
        QJsonArray folds;
        for (int ln : editor->foldedStartLines()) folds.append(ln);
        t["folds"] = folds;
        // 未儲存內容（含 Untitled）→ 快照
        if (editor->document()->isModified() || t["filePath"].toString().isEmpty()) {
            const QString backupFile = backupDir + QString("/tab_%1.txt").arg(i);
            QFile f(backupFile);
            if (f.open(QIODevice::WriteOnly | QIODevice::Text)) {
                QTextStream(&f) << editor->toPlainText();
                t["backup"] = backupFile;
            }
        }
        tabs.append(t);
    }
    QJsonObject root;
    root["tabs"] = tabs;
    root["currentIndex"] = tabWidget->currentIndex();
    root["projectFolder"] = projectFolder;
    root["terminalOpen"] = (termDock && termDock->isVisible());

    QFile f(sessionDir() + "/session.json");
    if (f.open(QIODevice::WriteOnly)) f.write(QJsonDocument(root).toJson());
}

void MainWindow::restoreSession() {
    QFile f(sessionDir() + "/session.json");
    if (!f.open(QIODevice::ReadOnly)) return;
    const QJsonObject root = QJsonDocument::fromJson(f.readAll()).object();
    const QJsonArray tabs = root["tabs"].toArray();
    if (tabs.isEmpty()) return;

    // 移除預設 Untitled 分頁
    if (tabWidget->count() == 1) {
        auto e = qobject_cast<CodeEditor*>(tabWidget->widget(0));
        if (e && e->toPlainText().isEmpty() && e->property("filePath").toString().isEmpty()) {
            tabWidget->removeTab(0);
            e->deleteLater();
        }
    }

    const QString folder = root["projectFolder"].toString();
    if (!folder.isEmpty() && QFileInfo(folder).isDir()) setProjectFolder(folder);

    // 逐分頁還原（單一分頁的完整狀態）；抽成 helper 讓串流與同步兩種路徑共用。
    auto restoreOneTab = [this](const QJsonObject& t) {
        const QString filePath = t["filePath"].toString();
        const QString backup   = t["backup"].toString();
        CodeEditor* editor = nullptr;

        if (!filePath.isEmpty() && QFileInfo::exists(filePath)) {
            openFileByPath(filePath);
            editor = activeEditor();
        } else if (!backup.isEmpty()) {
            QString title = t["baseTitle"].toString();
            editor = createEditorTab(title.isEmpty() ? "Untitled" : title);
        }
        if (!editor) return;

        // 還原未儲存內容
        if (!backup.isEmpty() && QFileInfo::exists(backup)) {
            QFile bf(backup);
            if (bf.open(QIODevice::ReadOnly | QIODevice::Text)) {
                editor->setPlainText(QTextStream(&bf).readAll());
                editor->document()->setModified(t["modified"].toBool(true));
                updateTabTitle(editor);
            }
        }
        // 游標 / 捲動 / 書籤
        QTextCursor c = editor->textCursor();
        c.setPosition(qMin(t["cursor"].toInt(), editor->document()->characterCount() - 1));
        editor->setTextCursor(c);
        editor->verticalScrollBar()->setValue(t["scroll"].toInt());
        QList<int> bms;
        for (const QJsonValue& b : t["bookmarks"].toArray()) bms << b.toInt();
        editor->setBookmarkedLines(bms);
        // 還原摺疊區域（內容已載入）
        QList<int> folds;
        for (const QJsonValue& fv : t["folds"].toArray()) folds << fv.toInt();
        if (!folds.isEmpty()) editor->applyFolds(folds);
    };

    // 串流還原：分頁依原順序逐一於事件迴圈的空檔開啟，避免所有檔案的
    // tree-sitter 解析 / clangd 啟動 / git 子行程一次塞爆首屏而卡住 UI。
    auto specs = std::make_shared<QVector<QJsonObject>>();
    for (const QJsonValue& v : tabs) specs->push_back(v.toObject());
    const int curIdx = root["currentIndex"].toInt();
    const bool termOpen = root["terminalOpen"].toBool();
    const int tabCount = tabs.size();

    restoringSession = true;   // 串流期間抑制逐分頁的即時 LSP / Git 啟用
    auto step = std::make_shared<std::function<void(int)>>();
    *step = [this, specs, curIdx, termOpen, tabCount, restoreOneTab, step](int i) {
        if (i >= specs->size()) {
            // 全部還原完成：解除抑制、定位作用中分頁（並確保其被啟用）、還原終端機、提示。
            restoringSession = false;
            if (curIdx >= 0 && curIdx < tabWidget->count())
                tabWidget->setCurrentIndex(curIdx);
            ensureEditorActivated(activeEditor());   // index 未變動時 currentChanged 不發，這裡補上
            if (termOpen && termDock) {
                termDock->show();
                terminal->startShell(projectFolder);
            }
            statusBar()->showMessage(
                tr("已還原上次工作階段（%1 個分頁）").arg(tabCount), 4000);
            return;
        }
        restoreOneTab((*specs)[i]);
        QTimer::singleShot(0, this, [step, i]() { (*step)(i + 1); });
    };
    (*step)(0);
}

// ----------------------------------------------------------------
// 導覽歷史（Alt+← / Alt+→）：游標大幅跳躍時自動入棧
// ----------------------------------------------------------------
void MainWindow::recordNavLocation() {
    if (navInProgress) return;
    auto editor = qobject_cast<CodeEditor*>(sender());
    if (!editor) editor = activeEditor();
    if (!editor) return;

    const int pos = editor->textCursor().position();
    const int line = editor->textCursor().blockNumber();
    if (navIndex >= 0 && navIndex < navHistory.size()) {
        const NavLoc& last = navHistory[navIndex];
        if (last.editor == editor) {
            QTextCursor c(editor->document());
            c.setPosition(qMin(last.pos, editor->document()->characterCount() - 1));
            if (qAbs(c.blockNumber() - line) < 10) return;  // 小幅移動不入棧
        }
    }
    // 截斷 forward 分支
    while (navHistory.size() > navIndex + 1) navHistory.removeLast();
    navHistory.append({editor, pos});
    if (navHistory.size() > 100) navHistory.removeFirst();
    navIndex = navHistory.size() - 1;
}

void MainWindow::navigateBack() {
    while (navIndex > 0) {
        --navIndex;
        const NavLoc& loc = navHistory[navIndex];
        if (!loc.editor) continue;
        navInProgress = true;
        tabWidget->setCurrentWidget(loc.editor);
        QTextCursor c = loc.editor->textCursor();
        c.setPosition(qMin(loc.pos, loc.editor->document()->characterCount() - 1));
        loc.editor->setTextCursor(c);
        loc.editor->centerCursor();
        loc.editor->setFocus();
        navInProgress = false;
        return;
    }
}

void MainWindow::navigateForward() {
    while (navIndex < navHistory.size() - 1) {
        ++navIndex;
        const NavLoc& loc = navHistory[navIndex];
        if (!loc.editor) continue;
        navInProgress = true;
        tabWidget->setCurrentWidget(loc.editor);
        QTextCursor c = loc.editor->textCursor();
        c.setPosition(qMin(loc.pos, loc.editor->document()->characterCount() - 1));
        loc.editor->setTextCursor(c);
        loc.editor->centerCursor();
        loc.editor->setFocus();
        navInProgress = false;
        return;
    }
}

// ----------------------------------------------------------------
// 狀態列點擊：編碼 / 換行符選單
// ----------------------------------------------------------------
bool MainWindow::eventFilter(QObject* obj, QEvent* event) {
    // HUD 覆蓋層跟隨中央容器大小；不消費事件。
    if (obj == centralWrap && event->type() == QEvent::Resize && hudOverlay) {
        hudOverlay->setGeometry(centralWrap->rect());
        hudOverlay->raise();
    }
    if (outputView && obj == outputView->viewport()
        && event->type() == QEvent::MouseButtonDblClick) {
        const QString line = outputView->textCursor().block().text();
        // gcc/clang/python 格式: path:line[:col] 或 File "path", line N
        static const QRegularExpression gccRe(QStringLiteral("([^\\s:]+\\.[A-Za-z]+):(\\d+)"));
        static const QRegularExpression pyRe(QStringLiteral("File \"([^\"]+)\", line (\\d+)"));
        QString path; int ln = 0;
        auto m = pyRe.match(line);
        if (m.hasMatch()) { path = m.captured(1); ln = m.captured(2).toInt(); }
        else { m = gccRe.match(line); if (m.hasMatch()) { path = m.captured(1); ln = m.captured(2).toInt(); } }
        if (!path.isEmpty()) {
            if (QFileInfo(path).isRelative() && !projectFolder.isEmpty())
                path = projectFolder + "/" + path;
            if (QFileInfo::exists(path)) {
                openFileByPath(path);
                if (auto e = activeEditor()) e->gotoLine(ln);
                return true;
            }
        }
    }
    if (event->type() == QEvent::MouseButtonRelease && activeEditor()) {
        if (obj == statusEncoding) {
            QMenu menu(this);
            QAction* reUtf8 = menu.addAction(tr("以 UTF-8 重新載入"));
            QAction* reBig5 = menu.addAction(tr("以 Big5 重新載入"));
            menu.addSeparator();
            QAction* cvUtf8 = menu.addAction(tr("轉換為 UTF-8（儲存時生效）"));
            QAction* cvBig5 = menu.addAction(tr("轉換為 Big5（儲存時生效）"));
            QAction* chosen = menu.exec(QCursor::pos());
            if (chosen == reUtf8) reloadWithEncoding("UTF-8");
            else if (chosen == reBig5) reloadWithEncoding("Big5");
            else if (chosen == cvUtf8) setEditorEncoding(activeEditor(), "UTF-8");
            else if (chosen == cvBig5) setEditorEncoding(activeEditor(), "Big5");
            return true;
        }
        if (obj == statusEol) {
            QMenu menu(this);
            QAction* lf   = menu.addAction(tr("LF（Unix）"));
            QAction* crlf = menu.addAction(tr("CRLF（Windows）"));
            QAction* chosen = menu.exec(QCursor::pos());
            if (chosen == lf || chosen == crlf) {
                activeEditor()->setProperty("eol", chosen == lf ? "LF" : "CRLF");
                activeEditor()->document()->setModified(true);
                updateTabTitle(activeEditor());
                updateStatusBar();
            }
            return true;
        }
    }
    return QMainWindow::eventFilter(obj, event);
}

void MainWindow::setEditorEncoding(CodeEditor* e, const QString& enc) {
    if (!e) return;
    e->setProperty("encoding", enc);
    e->document()->setModified(true);
    updateTabTitle(e);
    updateStatusBar();
}

void MainWindow::reloadWithEncoding(const QString& enc) {
    CodeEditor* editor = activeEditor();
    if (!editor) return;
    const QString path = editor->property("filePath").toString();
    if (path.isEmpty()) { setEditorEncoding(editor, enc); return; }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) return;
    const QByteArray raw = file.readAll();
    file.close();

    QString text;
    if (enc == "Big5") {
        QTextCodec* big5 = QTextCodec::codecForName("Big5");
        if (!big5) return;
        text = big5->toUnicode(raw);
    } else {
        text = QString::fromUtf8(raw);
    }
    text.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
    editor->setPlainText(text);
    editor->document()->setModified(false);
    editor->setProperty("encoding", enc);
    updateTabTitle(editor);
    updateStatusBar();
    statusBar()->showMessage(tr("已以 %1 重新載入").arg(enc), 3000);
}

// ----------------------------------------------------------------
// 設定中心（Phase 0 精簡版）
// ----------------------------------------------------------------
// 蒐集有快捷鍵的動作（名稱去重，與 applyKeymap 同邏輯）
static QList<QPair<QString, QString>> gatherShortcutActions(const QObject* w) {
    QList<QPair<QString, QString>> actions;
    QSet<QString> seen;
    for (QAction* a : w->findChildren<QAction*>()) {
        if (a->text().isEmpty() || a->shortcut().isEmpty()) continue;
        const QString name = QString(a->text()).remove(QLatin1Char('&'));
        if (seen.contains(name)) continue;
        seen.insert(name);
        actions.append({name, a->shortcut().toString()});
    }
    return actions;
}

void MainWindow::openSettingsForShot(const QString& outPng) {
    auto* dlg = new SettingsDialog(LspManager::configFilePath(), SnippetsController::configPath(),
                                   keymapConfigPath(), gatherShortcutActions(this), this);
    dlg->setAttribute(Qt::WA_DeleteOnClose);
    dlg->show();
    QTimer::singleShot(900, dlg, [dlg, outPng]() { dlg->grab().save(outPng); });
}

static QString aboutHtml() {
    // 字標以內嵌的 Orbitron 呈現（未來科技感）；未註冊成功時退回一般字型。
    return QObject::tr(
        "<div style=\"font-family:'Orbitron','Segoe UI',sans-serif; font-size:20px;"
        " font-weight:800; letter-spacing:3px; color:%3;\">ALEXCODE</div>"
        "<p style=\"color:%4; letter-spacing:2px;\">v%1 &nbsp;·&nbsp; NEON EDITION</p>"
        "<p>輕量級 Qt 程式碼編輯器：LSP、Git、互動式終端機、多語言語法高亮、"
        "log 分析、多套主題（含 Neon HUD）、繁中/英文介面。</p>"
        "<p>以 Qt %2 建置。</p>"
        "<p><a href=\"https://github.com/alex721chiang/alexcode_editor\">GitHub 專案</a></p>")
        .arg(QStringLiteral(ALEXCODE_VERSION), QStringLiteral(QT_VERSION_STR),
             Theme::ACCENT, Theme::ACCENT2);
}

void MainWindow::showAbout() {
    QMessageBox::about(this, tr("關於 AlexCode"), aboutHtml());
}

void MainWindow::openAboutForShot(const QString& outPng) {
    auto* box = new QMessageBox(QMessageBox::Information, tr("關於 AlexCode"), aboutHtml(),
                                QMessageBox::Ok, this);
    box->setTextFormat(Qt::RichText);
    box->setAttribute(Qt::WA_DeleteOnClose);
    box->show();
    QTimer::singleShot(700, box, [box, outPng]() { box->grab().save(outPng); });
}

void MainWindow::showSettingsCenter() {
    applyKeymap();          // 確保快捷鍵模板已存在
    SettingsDialog dlg(LspManager::configFilePath(), SnippetsController::configPath(),
                       keymapConfigPath(), gatherShortcutActions(this), this);
    if (dlg.exec() != QDialog::Accepted) return;
    dlg.save();
    lspController->lsp()->reloadConfig();
    snippetsController->load();
    for (int i = 0; i < tabWidget->count(); ++i)
        snippetsController->applyTo(qobject_cast<CodeEditor*>(tabWidget->widget(i)));
    applyKeymap();
    statusBar()->showMessage(tr("設定已套用"), 3000);
}

void MainWindow::showPreferences() {
    AppSettings settings;
    QDialog dlg(this);
    dlg.setWindowTitle("Preferences");
    auto* form = new QFormLayout(&dlg);

    auto* tabWidthSpin = new QSpinBox(&dlg);
    tabWidthSpin->setRange(2, 8);
    tabWidthSpin->setValue(settings.value("editor/tabWidth", 4).toInt());
    form->addRow(tr("Tab 寬度（空格數）"), tabWidthSpin);

    auto* trimCheck = new QCheckBox(&dlg);
    trimCheck->setChecked(settings.value("editor/trimTrailing", false).toBool());
    form->addRow(tr("儲存時修剪行尾空白"), trimCheck);

    auto* sessionCheck = new QCheckBox(&dlg);
    sessionCheck->setChecked(settings.value("session/restore", true).toBool());
    form->addRow(tr("啟動時還原上次工作階段"), sessionCheck);

    auto* autosaveSpin = new QSpinBox(&dlg);
    autosaveSpin->setRange(0, 30);
    autosaveSpin->setSuffix(tr(" 分鐘（0 = 停用）"));
    autosaveSpin->setValue(settings.value("session/autosaveMinutes", 2).toInt());
    form->addRow(tr("自動快照間隔"), autosaveSpin);

    auto* assistMaxSpin = new QSpinBox(&dlg);
    assistMaxSpin->setRange(1, 200);
    assistMaxSpin->setSuffix(tr(" MB"));
    assistMaxSpin->setValue(settings.value("editor/assistMaxMB", 2).toInt());
    form->addRow(tr("超過此大小停用自動完成/LSP"), assistMaxSpin);

    auto* highlightMaxSpin = new QSpinBox(&dlg);
    highlightMaxSpin->setRange(1, 500);
    highlightMaxSpin->setSuffix(tr(" MB"));
    highlightMaxSpin->setValue(settings.value("editor/highlightMaxMB", 10).toInt());
    form->addRow(tr("超過此大小停用語法高亮"), highlightMaxSpin);

    auto* themeCombo = new QComboBox(&dlg);
    themeCombo->addItems(Theme::themeNames());
    themeCombo->setCurrentText(Theme::currentThemeName);
    form->addRow(tr("主題"), themeCombo);

    // 介面語言：值存 system/en/zh_TW，重新啟動後生效
    auto* langCombo = new QComboBox(&dlg);
    langCombo->addItem(tr("系統預設"), "system");
    langCombo->addItem(QStringLiteral("English"), "en");
    langCombo->addItem(QStringLiteral("繁體中文"), "zh_TW");
    const QString curLang = settings.value("ui/language", "system").toString();
    langCombo->setCurrentIndex(qMax(0, langCombo->findData(curLang)));
    form->addRow(tr("介面語言"), langCombo);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    form->addRow(buttons);

    if (dlg.exec() != QDialog::Accepted) return;

    const QString newLang = langCombo->currentData().toString();
    const bool langChanged = newLang != curLang;

    settings.setValue("editor/tabWidth", tabWidthSpin->value());
    settings.setValue("editor/trimTrailing", trimCheck->isChecked());
    settings.setValue("session/restore", sessionCheck->isChecked());
    settings.setValue("session/autosaveMinutes", autosaveSpin->value());
    settings.setValue("editor/assistMaxMB", assistMaxSpin->value());
    settings.setValue("editor/highlightMaxMB", highlightMaxSpin->value());
    settings.setValue("ui/theme", themeCombo->currentText());
    settings.setValue("ui/language", newLang);

    if (langChanged)
        QMessageBox::information(this, tr("介面語言"),
            tr("介面語言將於下次啟動 AlexCode 時生效。"));

    // 立即套用
    if (themeCombo->currentText() != Theme::currentThemeName) {
        Theme::setTheme(themeCombo->currentText());
        qApp->setStyleSheet(Theme::stylesheet());
        applyActionIcons();                           // 圖示線色/點綴色跟隨主題
        applyTitleBarTheme();                         // 標題列底色跟隨主題
        if (hudOverlay) hudOverlay->update();         // HUD 角標：切到/離開 Neon HUD 時重繪或隱藏
        for (int i = 0; i < tabWidget->count(); ++i)
            if (auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i))) {
                e->refreshSyntaxTheme();              // 語法高亮色票跟隨主題
                e->viewport()->update();              // 重繪 current line / gutter 等程式內取色
            }
    }
    for (int i = 0; i < tabWidget->count(); ++i)
        if (auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i)))
            e->setTabStopDistance(e->fontMetrics().horizontalAdvance(QLatin1Char(' ')) * tabWidthSpin->value());
    if (autosaveTimer) {
        if (autosaveSpin->value() > 0) autosaveTimer->start(autosaveSpin->value() * 60 * 1000);
        else autosaveTimer->stop();
    }
}

// ----------------------------------------------------------------
// Diff（LCS 行比較，輸出統一格式到新分頁）
// ----------------------------------------------------------------
void MainWindow::showDiff(const QString& titleA, const QString& a,
                          const QString& titleB, const QString& b) {
    QStringList la = a.split('\n');
    QStringList lb = b.split('\n');
    if (la.size() > 4000 || lb.size() > 4000) {
        statusBar()->showMessage(tr("Diff 上限 4000 行"), 3000);
        return;
    }
    // LCS 動態規劃
    const int n = la.size(), m = lb.size();
    QVector<QVector<int>> dp(n + 1, QVector<int>(m + 1, 0));
    for (int i = n - 1; i >= 0; --i)
        for (int j = m - 1; j >= 0; --j)
            dp[i][j] = (la[i] == lb[j]) ? dp[i+1][j+1] + 1 : qMax(dp[i+1][j], dp[i][j+1]);

    QStringList out;
    out << tr("--- %1").arg(titleA)
        << tr("+++ %1").arg(titleB) << "";
    int i = 0, j = 0, changes = 0;
    while (i < n && j < m) {
        if (la[i] == lb[j]) { out << "  " + la[i]; ++i; ++j; }
        else if (dp[i+1][j] >= dp[i][j+1]) { out << "- " + la[i]; ++i; ++changes; }
        else { out << "+ " + lb[j]; ++j; ++changes; }
    }
    while (i < n) { out << "- " + la[i++]; ++changes; }
    while (j < m) { out << "+ " + lb[j++]; ++changes; }

    CodeEditor* e = createEditorTab(tr("Diff (%1 處差異)").arg(changes));
    e->setPlainText(out.join('\n'));
    e->document()->setModified(false);
    // 用多色標示突顯 +/- 行首
    e->setKeywordHighlights({});
    statusBar()->showMessage(tr("比較完成：%1 處差異").arg(changes), 4000);
}

// 目前檔案 vs Git HEAD 並排比較（選單與 --gitdiff-shot 共用）
void MainWindow::compareActiveWithGitHead() {
    CodeEditor* e = activeEditor();
    if (!e) return;
    const QString path = e->property("filePath").toString();
    if (path.isEmpty()) { statusBar()->showMessage(tr("此分頁尚未存檔"), 2500); return; }
    QString head = gitGutterController->headText(path);
    if (head.isEmpty()) {                        // 尚未快取：同步抓一次（選單動作可接受）
        QProcess p;
        p.setWorkingDirectory(QFileInfo(path).absolutePath());
        p.start("git", {"show", "HEAD:./" + QFileInfo(path).fileName()});
        if (!p.waitForFinished(3000) || p.exitCode() != 0) {
            statusBar()->showMessage(tr("取不到 Git HEAD 版本（不在版控中？）"), 3500);
            return;
        }
        head = QString::fromUtf8(p.readAllStandardOutput());
        head.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
    }
    showSideBySideDiff(tr("Git HEAD"), head, tr("目前內容"), e->toPlainText());
}

// 截圖用：對目前檔案觸發 Git HEAD 並排比較後存整window圖
void MainWindow::openGitDiffForShot(const QString& outPng) {
    resize(1200, 800);
    compareActiveWithGitHead();
    QTimer::singleShot(700, this, [this, outPng]() { grab().save(outPng); });
}

// 並排 diff：DiffCalc 對齊 → DiffViewer 分頁（唯讀，關閉不提示儲存）
void MainWindow::showSideBySideDiff(const QString& titleA, const QString& a,
                                    const QString& titleB, const QString& b) {
    const QStringList la = a.split('\n');
    const QStringList lb = b.split('\n');
    if (la.size() > 4000 || lb.size() > 4000) {
        statusBar()->showMessage(tr("Diff 上限 4000 行"), 3000);
        return;
    }
    const QList<DiffCalc::Row> rows = DiffCalc::align(la, lb);
    auto* viewer = new DiffViewer(titleA, la, titleB, lb, rows, this);
    const int idx = tabWidget->addTab(viewer, tr("Diff ⇄ %1 處").arg(viewer->changeCount()));
    tabWidget->setCurrentIndex(idx);
    statusBar()->showMessage(tr("比較完成：%1 處差異").arg(viewer->changeCount()), 4000);
}

// ----------------------------------------------------------------
// 外部工具（%FILE% / %DIR% / %LINE% 變數替換）
// ----------------------------------------------------------------
void MainWindow::runExternalTool() {
    const QString cfg = sessionDir() + "/external_tools.json";
    QFile f(cfg);
    if (!f.open(QIODevice::ReadOnly)) {
        statusBar()->showMessage(tr("尚未設定外部工具，請先用「編輯工具設定檔」"), 4000);
        return;
    }
    const QJsonArray tools = QJsonDocument::fromJson(f.readAll()).array();
    if (tools.isEmpty()) return;

    QStringList names;
    for (const auto& t : tools) names << t.toObject()["name"].toString();
    bool ok = false;
    const QString pick = QInputDialog::getItem(this, tr("外部工具"),
                                               tr("選擇工具："), names, 0, false, &ok);
    if (!ok) return;

    CodeEditor* e = activeEditor();
    QString cmd;
    for (const auto& t : tools)
        if (t.toObject()["name"].toString() == pick) cmd = t.toObject()["command"].toString();
    if (e) {
        const QString path = e->property("filePath").toString();
        cmd.replace("%FILE%", "\"" + path + "\"");
        cmd.replace("%DIR%",  "\"" + QFileInfo(path).absolutePath() + "\"");
        cmd.replace("%LINE%", QString::number(e->textCursor().blockNumber() + 1));
    }
    QStringList parts = QProcess::splitCommand(cmd);
    if (parts.isEmpty()) return;
    const QString prog = parts.takeFirst();
    if (QProcess::startDetached(prog, parts))
        statusBar()->showMessage(tr("已執行：%1").arg(pick), 3000);
    else
        statusBar()->showMessage(tr("啟動失敗：%1").arg(cmd), 4000);
}

// ----------------------------------------------------------------
// Phase 2：建置任務 / 指令執行 / Git / 符號清單
// ----------------------------------------------------------------
void MainWindow::runCommand(const QString& cmd) {
    outputDock->show();
    outputView->appendPlainText(tr("$ %1").arg(cmd));
    if (taskProcess && taskProcess->state() == QProcess::Running) {
        outputView->appendPlainText(tr("[前一個任務仍在執行]"));
        return;
    }
    if (!taskProcess) {
        taskProcess = new QProcess(this);
        taskProcess->setProcessChannelMode(QProcess::MergedChannels);
        connect(taskProcess, &QProcess::readyRead, this, [this]() {
            outputView->moveCursor(QTextCursor::End);
            outputView->insertPlainText(QString::fromUtf8(taskProcess->readAll()));
            outputView->moveCursor(QTextCursor::End);
        });
        connect(taskProcess, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
                this, [this](int code, QProcess::ExitStatus) {
            outputView->appendPlainText(tr("[完成，exit code %1]\n").arg(code));
            updateGitStatus();
        });
        connect(taskProcess, &QProcess::errorOccurred, this, [this](QProcess::ProcessError err) {
            if (err == QProcess::FailedToStart)
                outputView->appendPlainText(tr("[無法啟動指令：找不到直譯器或權限不足]\n"));
        });
    }
    QString wd = projectFolder;
    if (wd.isEmpty() && activeEditor())
        wd = QFileInfo(activeEditor()->property("filePath").toString()).absolutePath();
    if (!wd.isEmpty()) taskProcess->setWorkingDirectory(wd);
#ifdef Q_OS_WIN
    taskProcess->start("cmd", {"/C", cmd});
#else
    taskProcess->start("/bin/sh", {"-c", cmd});
#endif
}

void MainWindow::runBuildTask() {
    if (projectFolder.isEmpty()) { openFolder(); if (projectFolder.isEmpty()) return; }
    const QString cfg = projectFolder + "/alexcode-tasks.json";
    if (!QFileInfo::exists(cfg)) {
        QFile f(cfg);
        if (f.open(QIODevice::WriteOnly))
            f.write(QByteArray("[\n  {\"name\": \"Build\", \"command\": \"cmake --build build\"},\n"
                               "  {\"name\": \"Run tests\", \"command\": \"ctest --test-dir build\"}\n]\n"));
        openFileByPath(cfg);
        statusBar()->showMessage(tr("已建立任務範本，編輯後再按 F5"), 5000);
        return;
    }
    QFile f(cfg);
    if (!f.open(QIODevice::ReadOnly)) return;
    const QJsonArray tasks = QJsonDocument::fromJson(f.readAll()).array();
    if (tasks.isEmpty()) return;
    QString cmd;
    if (tasks.size() == 1) {
        cmd = tasks[0].toObject()["command"].toString();
    } else {
        QStringList names;
        for (const auto& t : tasks) names << t.toObject()["name"].toString();
        bool ok = false;
        const QString pick = QInputDialog::getItem(this, "Build Task",
                                                   tr("選擇任務："), names, 0, false, &ok);
        if (!ok) return;
        for (const auto& t : tasks)
            if (t.toObject()["name"].toString() == pick) cmd = t.toObject()["command"].toString();
    }
    if (!cmd.isEmpty()) runCommand(cmd);
}

void MainWindow::updateGitStatus() {
    // Git gutter：順帶刷新當前分頁的 HEAD 快取（commit 後標示自動消除）
    if (CodeEditor* e = activeEditor()) {
        const QString p = e->property("filePath").toString();
        if (!p.isEmpty() && !e->property("bigFile").toBool()) gitGutterController->fetchHead(p);
    }
    if (projectFolder.isEmpty() || !statusGit) return;
    auto* p = new QProcess(this);
    connect(p, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, [this, p](int code, QProcess::ExitStatus) {
        if (code == 0) {
            // 兩行輸出：分支名 + repo 根目錄（給檔案樹染色組絕對路徑）
            const QStringList head = QString::fromUtf8(p->readAllStandardOutput())
                                         .trimmed().split('\n');
            const QString branch = head.value(0).trimmed();
            const QString repoRoot = head.value(1).trimmed();
            auto* p2 = new QProcess(this);
            connect(p2, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
                    this, [this, p2, branch, repoRoot](int, QProcess::ExitStatus) {
                const QStringList lines = QString::fromUtf8(p2->readAllStandardOutput())
                                              .split('\n', Qt::SkipEmptyParts);
                statusGit->setText(!lines.isEmpty()
                    ? tr("⎇ %1 ●%2").arg(branch).arg(lines.size())
                    : tr("⎇ %1").arg(branch));

                // 2.3c 檔案樹染色：porcelain "XY path"（rename 取 "→" 後的新路徑）
                QHash<QString, QChar> states;
                if (!repoRoot.isEmpty()) {
                    for (const QString& line : lines) {
                        if (line.size() < 4) continue;
                        QString path = line.mid(3);
                        const int arrow = path.indexOf(QStringLiteral(" -> "));
                        if (arrow >= 0) path = path.mid(arrow + 4);
                        if (path.startsWith('"') && path.endsWith('"'))
                            path = path.mid(1, path.size() - 2);   // 含空白的路徑
                        states.insert(QDir::cleanPath(repoRoot + QLatin1Char('/') + path),
                                      line.startsWith(QStringLiteral("??")) ? QLatin1Char('?')
                                                                            : QLatin1Char('M'));
                    }
                }
                if (fsModel && fsModel->gitStates != states) {
                    fsModel->gitStates = states;
                    if (fsTree) fsTree->viewport()->update();
                }
                p2->deleteLater();
            });
            p2->start("git", {"-C", projectFolder, "status", "--porcelain"});
        } else {
            statusGit->setText("");
            if (fsModel && !fsModel->gitStates.isEmpty()) {
                fsModel->gitStates.clear();
                if (fsTree) fsTree->viewport()->update();
            }
        }
        p->deleteLater();
    });
    p->start("git", {"-C", projectFolder, "rev-parse", "--abbrev-ref", "HEAD", "--show-toplevel"});
}

void MainWindow::showSymbolList() {
    CodeEditor* e = activeEditor();
    if (!e) return;
    static const QRegularExpression cppRe(
        QStringLiteral("^\\s*(?:[\\w:<>~*&,\\s]+?)\\s+([A-Za-z_]\\w*(?:::[A-Za-z_~]\\w*)?)\\s*\\([^;]*$"));
    static const QRegularExpression pyRe(QStringLiteral("^\\s*(?:def|class)\\s+(\\w+)"));
    const bool isPy = e->property("language").toString() == "Python";

    QStringList items;
    QList<int> lines;
    QTextBlock b = e->document()->begin();
    int ln = 0;
    while (b.isValid()) {
        const auto m = (isPy ? pyRe : cppRe).match(b.text());
        if (m.hasMatch())
            { items << QString("%1  (line %2)").arg(m.captured(1)).arg(ln + 1); lines << ln; }
        b = b.next(); ++ln;
    }
    if (items.isEmpty()) {
        statusBar()->showMessage(tr("沒有偵測到符號"), 2500);
        return;
    }
    bool ok = false;
    const QString pick = QInputDialog::getItem(this, "Document Symbols",
                                               tr("跳至符號："), items, 0, false, &ok);
    if (ok) e->gotoLine(lines[items.indexOf(pick)] + 1);
}
