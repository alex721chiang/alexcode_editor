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
#include <algorithm>
#include <QTextDocumentFragment>
#include <QXmlStreamReader>
#include <QXmlStreamWriter>
#include <QCryptographicHash>
#include <QTextBrowser>
#include <QSpinBox>
#include <QFormLayout>
#include <QDialogButtonBox>
#include "AICompletionProvider.h"
#include "LspManager.h"
#include "GitGutter.h"
#include "TimelineBar.h"
#include "TerminalWidget.h"
#include "Theme.h"
#include "Portable.h"

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
    : QMainWindow(parent), findDialog(nullptr), findInFilesDialog(nullptr)
{
    defaultEditorFont = loadFont();
    isFontSet = true;
    loadRecentFiles();
    loadSnippets();
    setupLsp();
    setupUI();
    setupStatusBar();
    setAcceptDrops(true);
    setWindowTitle("AlexCode — Neon Edition");
    resize(1100, 720);

    AppSettings settings;
    if (settings.value("session/restore", true).toBool())
        restoreSession();

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
    lsp = new LspManager(this);

    // didChange 防抖：編輯停頓 400ms 後送出完整內容
    lspChangeTimer = new QTimer(this);
    lspChangeTimer->setSingleShot(true);
    lspChangeTimer->setInterval(400);
    connect(lspChangeTimer, &QTimer::timeout, this, [this]() {
        for (const QPointer<CodeEditor>& e : lspDirtyEditors) {
            if (!e) continue;
            const QString path = e->property("filePath").toString();
            if (!path.isEmpty()) lsp->documentChanged(path, e->toPlainText());
        }
        lspDirtyEditors.clear();
    });

    // Git gutter 重算防抖：編輯停頓 600ms 後比對 HEAD
    gitGutterTimer = new QTimer(this);
    gitGutterTimer->setSingleShot(true);
    gitGutterTimer->setInterval(600);
    connect(gitGutterTimer, &QTimer::timeout, this, [this]() {
        recomputeGitGutter(activeEditor());
    });

    connect(lsp, &LspManager::diagnosticsReceived, this,
            [this](const QString& path, const QList<LspProtocol::Diagnostic>& diags) {
        if (CodeEditor* e = editorForPath(path)) {
            e->setDiagnostics(diags);
            if (e == activeEditor()) updateLspStatus();
        }
    });
    connect(lsp, &LspManager::completionReady, this,
            [this](const QString& path, const QStringList& items) {
        CodeEditor* e = activeEditor();
        if (e && e->property("filePath").toString() == path)
            e->showLspCompletions(items);
    });
    connect(lsp, &LspManager::definitionReady, this,
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
    connect(lsp, &LspManager::hoverReady, this,
            [this](const QString& path, const QString& text) {
        CodeEditor* e = activeEditor();
        if (e && e->property("filePath").toString() == path)
            e->showHoverText(text);
    });
    connect(lsp, &LspManager::statusChanged, this, &MainWindow::updateLspStatus);
    connect(lsp, &LspManager::serverFailed, this, [this](const QString& reason) {
        statusBar()->showMessage(tr("LSP：") + reason, 6000);
    });

    // ---- v2：全部引用結果面板（雙擊跳轉，模式同 Find in Files）----
    refsDock = new QDockWidget(tr("REFERENCES — 全部引用"), this);
    refsList = new QListWidget(this);
    refsDock->setWidget(refsList);
    addDockWidget(Qt::BottomDockWidgetArea, refsDock);
    refsDock->hide();
    connect(refsList, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem* item) {
        const QVariantMap data = item->data(Qt::UserRole).toMap();
        if (data.isEmpty()) return;
        openFileByPath(data.value("filePath").toString());
        if (CodeEditor* e = activeEditor()) e->gotoLine(data.value("lineNum").toInt());
    });

    connect(lsp, &LspManager::referencesReady, this,
            [this](const QString&, const QList<LspProtocol::Location>& locations) {
        refsList->clear();
        QHash<QString, QStringList> fileLines;               // 每檔讀一次，顯示該行內容
        for (const LspProtocol::Location& loc : locations) {
            QString lineText;
            if (CodeEditor* e = editorForPath(loc.path)) {
                lineText = e->document()->findBlockByNumber(loc.line).text();
            } else {
                if (!fileLines.contains(loc.path)) {
                    QFile f(loc.path);
                    if (f.size() < 4 * 1024 * 1024 && f.open(QIODevice::ReadOnly))
                        fileLines.insert(loc.path, QString::fromUtf8(f.readAll()).split('\n'));
                    else
                        fileLines.insert(loc.path, QStringList());
                }
                lineText = fileLines[loc.path].value(loc.line);
            }
            auto* item = new QListWidgetItem(QStringLiteral("%1:%2:  %3")
                .arg(QFileInfo(loc.path).fileName()).arg(loc.line + 1).arg(lineText.trimmed()));
            item->setToolTip(loc.path);
            item->setData(Qt::UserRole, QVariantMap{
                {"filePath", loc.path}, {"lineNum", loc.line + 1}});
            refsList->addItem(item);
        }
        refsDock->setWindowTitle(tr("REFERENCES — 全部引用（%1 處）").arg(locations.size()));
        if (locations.isEmpty())
            statusBar()->showMessage(tr("LSP：找不到引用"), 4000);
        else
            refsDock->show();
    });

    connect(lsp, &LspManager::renameReady, this,
            [this](const QHash<QString, QList<LspProtocol::TextEdit>>& edits) {
        if (edits.isEmpty()) {
            statusBar()->showMessage(tr("LSP：無法重新命名（伺服器未回傳編輯）"), 4000);
            return;
        }
        int editCount = 0;
        for (auto it = edits.begin(); it != edits.end(); ++it) {
            CodeEditor* e = editorForPath(it.key());
            if (!e) {                                        // 未開啟的檔案：開進分頁再套用（保留 Undo）
                openFileByPath(it.key());
                e = editorForPath(it.key());
            }
            if (!e) continue;
            applyTextEditsToEditor(e, it.value());
            editCount += it.value().size();
        }
        statusBar()->showMessage(tr("重新命名完成：%1 個檔案、%2 處變更")
                                     .arg(edits.size()).arg(editCount), 5000);
    });

    connect(lsp, &LspManager::formattingReady, this,
            [this](const QString& path, const QList<LspProtocol::TextEdit>& edits) {
        if (edits.isEmpty()) {
            statusBar()->showMessage(tr("LSP：文件已符合格式（無變更）"), 4000);
            return;
        }
        if (CodeEditor* e = editorForPath(path)) {
            applyTextEditsToEditor(e, edits);
            statusBar()->showMessage(tr("格式化完成：%1 處變更").arg(edits.size()), 4000);
        }
    });
}

// 以單一 Undo 步驟套用 LSP TextEdit（由後往前，避免位置位移）
void MainWindow::applyTextEditsToEditor(CodeEditor* editor, const QList<LspProtocol::TextEdit>& edits) {
    QTextDocument* doc = editor->document();
    const auto offsetOf = [doc](int line, int ch) {
        const QTextBlock b = doc->findBlockByNumber(qMin(line, doc->blockCount() - 1));
        return b.position() + qMin(ch, qMax(int(b.length()) - 1, 0));
    };
    struct Span { int start, end; QString newText; };
    QList<Span> spans;
    spans.reserve(edits.size());
    for (const LspProtocol::TextEdit& e : edits)
        spans.append({ offsetOf(e.startLine, e.startChar), offsetOf(e.endLine, e.endChar), e.newText });
    std::sort(spans.begin(), spans.end(),
              [](const Span& a, const Span& b) { return a.start > b.start; });

    QTextCursor c(doc);
    c.beginEditBlock();
    for (const Span& s : spans) {
        c.setPosition(s.start);
        c.setPosition(qMax(s.end, s.start), QTextCursor::KeepAnchor);
        c.insertText(s.newText);
    }
    c.endEditBlock();
}

void MainWindow::updateLspStatus() {
    if (!statusLsp) return;
    CodeEditor* e = activeEditor();
    const QString path = e ? e->property("filePath").toString() : QString();
    const QString server = path.isEmpty() ? QString() : lsp->serverNameForFile(path);
    if (server.isEmpty() || !e->lspEnabled()) {
        statusLsp->setText("");
        return;
    }
    if (!lsp->isReadyForFile(path)) {
        statusLsp->setText(QStringLiteral("LSP: %1 …").arg(server));
        return;
    }
    int errors = 0, warnings = 0;
    e->diagnosticCounts(&errors, &warnings);
    statusLsp->setText(QStringLiteral("LSP: %1 ✓ E%2 W%3").arg(server).arg(errors).arg(warnings));
    e->setLspCompletionTriggers(lsp->completionTriggersForFile(path));   // v2：自動補全觸發字元
}

// ----------------------------------------------------------------
// Git gutter 行標示：緩衝區 vs HEAD 版本
// ----------------------------------------------------------------
void MainWindow::fetchGitHead(const QString& path) {
    if (path.isEmpty() || gitUntracked.contains(path)) return;
    const QFileInfo fi(path);
    auto* p = new QProcess(this);
    p->setWorkingDirectory(fi.absolutePath());
    connect(p, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, [this, p, path](int code, QProcess::ExitStatus) {
        if (code == 0) {
            QString head = QString::fromUtf8(p->readAllStandardOutput());
            head.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
            gitHeadCache.insert(path, head);
            if (CodeEditor* e = editorForPath(path)) recomputeGitGutter(e);
        } else {
            gitUntracked.insert(path);               // 不在版控（或非 git 目錄）：不重試
            if (CodeEditor* e = editorForPath(path)) e->setGitLineStates({});
        }
        p->deleteLater();
    });
    // "HEAD:./檔名" 相對於工作目錄解析，免去計算 repo 內相對路徑
    p->start("git", {"show", "HEAD:./" + fi.fileName()});
}

void MainWindow::recomputeGitGutter(CodeEditor* editor) {
    if (!editor || editor->property("bigFile").toBool()) return;
    const QString path = editor->property("filePath").toString();
    const auto it = gitHeadCache.constFind(path);
    if (path.isEmpty() || it == gitHeadCache.constEnd()) return;
    editor->setGitLineStates(GitGutter::diffLineStates(
        it.value().split('\n'), editor->toPlainText().split('\n')));
}


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
    font.setFamily(settings.value("font/family", "Consolas").toString());
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
    const SyntaxHighlighter::Language lang = SyntaxHighlighter::detectLanguage(filePath);

    // 註解前綴（給 Toggle Comment 用）：# 系列 vs // 系列
    QString commentPrefix = "//";
    if (lang == SyntaxHighlighter::Language::Python ||
        lang == SyntaxHighlighter::Language::CMakeLang ||
        lang == SyntaxHighlighter::Language::Bash)
        commentPrefix = "#";
    editor->setProperty("language", SyntaxHighlighter::languageName(lang));
    editor->setCommentPrefix(commentPrefix);
    editor->setSyntaxLanguage(lang);                // 重用編輯器自有 highlighter（含 Unknown 清空）
    applySnippetsToEditor(editor);                  // 語言確定後注入對應 snippet
    updateStatusBar();
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
    applySnippetsToEditor(editor);                  // 通用（language 為空）snippet

    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, &MainWindow::updateStatusBar);
    connect(editor, &QPlainTextEdit::cursorPositionChanged, this, &MainWindow::recordNavLocation);
    connect(editor, &QPlainTextEdit::selectionChanged, this, &MainWindow::updateStatusBar);
    connect(editor->document(), &QTextDocument::modificationChanged,
            this, &MainWindow::onModificationChanged);

    // LSP：請求轉發與內容變更同步
    connect(editor, &CodeEditor::lspCompletionRequested, this, [this, editor](int line, int ch) {
        lsp->requestCompletion(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspDefinitionRequested, this, [this, editor](int line, int ch) {
        lsp->requestDefinition(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspHoverRequested, this, [this, editor](int line, int ch) {
        lsp->requestHover(editor->property("filePath").toString(), line, ch);
    });
    connect(editor, &CodeEditor::lspReferencesRequested, this, [this, editor](int line, int ch) {
        lsp->requestReferences(editor->property("filePath").toString(), line, ch);
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
            lsp->requestRename(editor->property("filePath").toString(), line, ch, newName.trimmed());
    });
    connect(editor, &CodeEditor::lspFormatRequested, this, [this, editor]() {
        AppSettings settings;
        lsp->requestFormatting(editor->property("filePath").toString(),
                               settings.value("editor/tabWidth", 4).toInt(), true);
    });
    connect(editor, &QPlainTextEdit::textChanged, this, [this, editor]() {
        if (editor->lspEnabled()) {
            if (!lspDirtyEditors.contains(editor)) lspDirtyEditors.append(editor);
            lspChangeTimer->start();
        }
        if (!editor->property("filePath").toString().isEmpty())
            gitGutterTimer->start();                    // Git gutter 重算（防抖）
        if (mdDock && mdDock->isVisible() && editor == activeEditor())
            mdTimer->start();                           // Markdown 預覽刷新（防抖）
    });

    int idx = tabWidget->addTab(editor, title);
    editor->setProperty("baseTitle", title);
    tabWidget->setCurrentIndex(idx);
    updateStatusBar();
    return editor;
}

void MainWindow::setupUI() {
    aiProvider = new AICompletionProvider(this);

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
        updateStatusBar();
    });
    setCentralWidget(tabWidget);

    // ---------- File actions ----------
    newAction = new QAction(tr("New File"), this);
    newAction->setShortcut(QKeySequence::New);
    newAction->setIcon(style()->standardIcon(QStyle::SP_FileIcon));
    connect(newAction, &QAction::triggered, this, &MainWindow::newFile);

    openAction = new QAction(tr("Open..."), this);
    openAction->setShortcut(QKeySequence::Open);
    openAction->setIcon(style()->standardIcon(QStyle::SP_DirOpenIcon));
    connect(openAction, &QAction::triggered, this, &MainWindow::openFile);

    saveAction = new QAction(tr("Save"), this);
    saveAction->setShortcut(QKeySequence::Save);
    saveAction->setIcon(style()->standardIcon(QStyle::SP_DialogSaveButton));
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
    undoAction->setIcon(style()->standardIcon(QStyle::SP_ArrowBack));
    connect(undoAction, &QAction::triggered, this, [this]() {
        if (auto editor = activeEditor()) editor->undo();
    });

    redoAction = new QAction(tr("Redo"), this);
    redoAction->setShortcut(QKeySequence::Redo);
    redoAction->setIcon(style()->standardIcon(QStyle::SP_ArrowForward));
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
    findAction = new QAction(tr("Find / Replace..."), this);
    findAction->setShortcut(QKeySequence::Find);
    findAction->setIcon(style()->standardIcon(QStyle::SP_FileDialogContentsView));
    connect(findAction, &QAction::triggered, this, &MainWindow::showFindDialog);

    findNextAction = new QAction(tr("Find Next"), this);
    findNextAction->setShortcut(QKeySequence(Qt::Key_F3));
    connect(findNextAction, &QAction::triggered, this, &MainWindow::performFind);

    findPrevAction = new QAction(tr("Find Previous"), this);
    findPrevAction->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F3));
    connect(findPrevAction, &QAction::triggered, this, &MainWindow::performFindPrev);

    gotoLineAction = new QAction(tr("Go to Line..."), this);
    gotoLineAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_G));
    connect(gotoLineAction, &QAction::triggered, this, &MainWindow::showGotoLineDialog);

    findInFilesAction = new QAction(tr("Find in Files..."), this);
    findInFilesAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_F));
    findInFilesAction->setIcon(style()->standardIcon(QStyle::SP_DirIcon));
    connect(findInFilesAction, &QAction::triggered, this, &MainWindow::showFindInFilesDialog);

    openFolderAction = new QAction(tr("Open Folder..."), this);
    openFolderAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_O));
    connect(openFolderAction, &QAction::triggered, this, &MainWindow::openFolder);

    quickOpenAction = new QAction(tr("Quick Open..."), this);
    quickOpenAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_P));
    connect(quickOpenAction, &QAction::triggered, this, &MainWindow::showQuickOpen);

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

    recentFilesMenu = fileMenu->addMenu(tr("Open Recent"));
    for (int i = 0; i < 15; ++i) {
        recentFileActions[i] = new QAction(this);
        recentFileActions[i]->setVisible(false);
        connect(recentFileActions[i], &QAction::triggered, this, &MainWindow::openRecentFile);
        recentFilesMenu->addAction(recentFileActions[i]);
    }
    updateRecentFileActions();

    fileMenu->addAction(openFolderAction);
    fileMenu->addAction(quickOpenAction);
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
    auto splitLines = [](const QString& t) { return t.split('\n'); };

    QMenu* lineMenu = toolsMenu->addMenu(tr("行整理"));
    lineMenu->addAction(tr("排序（遞增）"), this, [=]() {
        editorOp([&](const QString& t) { auto L = splitLines(t); std::sort(L.begin(), L.end()); return L.join('\n'); }); });
    lineMenu->addAction(tr("排序（遞減）"), this, [=]() {
        editorOp([&](const QString& t) { auto L = splitLines(t); std::sort(L.begin(), L.end(), std::greater<QString>()); return L.join('\n'); }); });
    lineMenu->addAction(tr("移除重複行"), this, [=]() {
        editorOp([&](const QString& t) {
            auto L = splitLines(t); QStringList out; QSet<QString> seen;
            for (const auto& l : L) if (!seen.contains(l)) { seen.insert(l); out << l; }
            return out.join('\n'); }); });
    lineMenu->addAction(tr("移除空白行"), this, [=]() {
        editorOp([&](const QString& t) {
            auto L = splitLines(t); QStringList out;
            for (const auto& l : L) if (!l.trimmed().isEmpty()) out << l;
            return out.join('\n'); }); });
    lineMenu->addAction(tr("反轉行順序"), this, [=]() {
        editorOp([&](const QString& t) { auto L = splitLines(t); std::reverse(L.begin(), L.end()); return L.join('\n'); }); });
    lineMenu->addAction(tr("修剪行尾空白"), this, [=]() {
        editorOp([](const QString& t) {
            static const QRegularExpression re(QStringLiteral("[ \\t]+(?=\\n)|[ \\t]+$"));
            QString r = t; r.remove(re); return r; }); });

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
        editorOp([](const QString& t) {
            QString out;
            for (const QChar& c : t) {
                if (c.unicode() < 0x80) out += c;
                else out += QStringLiteral("\\u%1").arg(c.unicode(), 4, 16, QLatin1Char('0'));
            }
            return out; }, true); });
    encMenu->addAction(tr("Unicode Unescape"), this, [=]() {
        editorOp([](const QString& t) {
            static const QRegularExpression re(QStringLiteral("\\\\u([0-9a-fA-F]{4})"));
            QString out = t;
            int from = 0;
            QRegularExpressionMatch m;
            while ((m = re.match(out, from)).hasMatch()) {
                out.replace(m.capturedStart(), 6, QChar(ushort(m.captured(1).toUInt(nullptr, 16))));
                from = m.capturedStart() + 1;
            }
            return out; }, true); });

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
        QString s = e->textCursor().selectedText().trimmed();
        bool ok = false;
        qlonglong v = 0;
        if (s.startsWith("0x", Qt::CaseInsensitive))      v = s.mid(2).toLongLong(&ok, 16);
        else if (s.startsWith("0b", Qt::CaseInsensitive)) v = s.mid(2).toLongLong(&ok, 2);
        else if (s.startsWith('0') && s.size() > 1 && !s.contains('.'))
                                                          v = s.mid(1).toLongLong(&ok, 8);
        if (!ok) v = s.toLongLong(&ok, 10);
        if (!ok) { statusBar()->showMessage(tr("無法解析數字：") + s, 3000); return; }
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
        editorOp([](const QString& t) {
            QString out;
            for (QChar c : t) {
                const ushort u = c.unicode();
                if (u == 0x3000) out += QChar(' ');
                else if (u >= 0xFF01 && u <= 0xFF5E) out += QChar(ushort(u - 0xFEE0));
                else out += c;
            }
            return out; }); });
    widthMenu->addAction(tr("半形 → 全形"), this, [=]() {
        editorOp([](const QString& t) {
            QString out;
            for (QChar c : t) {
                const ushort u = c.unicode();
                if (u == ' ') out += QChar(0x3000);
                else if (u >= 0x21 && u <= 0x7E) out += QChar(ushort(u + 0xFEE0));
                else out += c;
            }
            return out; }); });

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
    playAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_P));
    macroMenu->addAction(tr("重播 N 次…"), this, [this]() {
        CodeEditor* e = activeEditor();
        if (!e) return;
        bool ok = false;
        int n = QInputDialog::getInt(this, tr("重播巨集"),
                                     tr("次數："), 10, 1, 10000, 1, &ok);
        if (ok) e->playMacro(n);
    });

    QMenu* extMenu = toolsMenu->addMenu(tr("外部工具"));
    extMenu->addAction(tr("執行外部工具…"), this, [this]() { runExternalTool(); });
    extMenu->addAction(tr("編輯 LSP 設定檔"), this, [this]() {
        openFileByPath(LspManager::configFilePath());   // 首次啟動已寫入預設（clangd / pylsp）
    });
    extMenu->addAction(tr("編輯 Snippet 設定檔"), this, [this]() {
        openFileByPath(snippetConfigPath());            // 存檔後自動重新載入
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

    setupToolBar();

    // ---------- Filter Results dock（含 5.6 命中密度條）----------
    QDockWidget* dock = new QDockWidget("FILTER RESULTS", this);
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
    mdView->setOpenExternalLinks(true);
    mdDock->setWidget(mdView);
    addDockWidget(Qt::RightDockWidgetArea, mdDock);
    mdDock->hide();
    mdTimer = new QTimer(this);
    mdTimer->setSingleShot(true);
    mdTimer->setInterval(500);
    connect(mdTimer, &QTimer::timeout, this, &MainWindow::refreshMarkdownPreview);

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
    filterIcon->setStyleSheet("color:#ff2d95; font-weight:700; letter-spacing:1px;");
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
    filterCountLabel->setStyleSheet("color:#00e5ff; padding:0 8px;");

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
    statusEncoding = new QLabel("UTF-8", this);
    statusEol      = new QLabel("LF", this);
    statusEncoding->setToolTip(tr("點擊切換編碼（重新載入或轉換）"));
    statusEol->setToolTip(tr("點擊切換換行符（儲存時生效）"));
    statusEncoding->setCursor(Qt::PointingHandCursor);
    statusEol->setCursor(Qt::PointingHandCursor);
    statusEncoding->installEventFilter(this);
    statusEol->installEventFilter(this);
    statusLsp = new QLabel("", this);
    statusLsp->setToolTip(tr("LSP 狀態（E=錯誤 W=警告）；設定檔：") + LspManager::configFilePath());
    sb->addPermanentWidget(statusLsp);
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
    updateLspStatus();
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
    mdView->document()->setMarkdown(e ? e->toPlainText() : QString());
}

// ----------------------------------------------------------------
// Snippet 樣板（與 LSP / 外部工具相同模式：首次啟動寫入預設 JSON）
// ----------------------------------------------------------------
QString MainWindow::snippetConfigPath() {
    return Portable::dataDir() + QStringLiteral("/alexcode-snippets.json");
}

void MainWindow::loadSnippets() {
    const QString cfgPath = snippetConfigPath();
    if (!QFileInfo::exists(cfgPath)) {
        QFile f(cfgPath);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(QByteArray(
"[\n"
"  {\"trigger\": \"forr\",  \"language\": \"cpp\",    \"body\": \"for (int i = 0; i < ${1:n}; ++i) {\\n    $0\\n}\"},\n"
"  {\"trigger\": \"main\",  \"language\": \"cpp\",    \"body\": \"int main(int argc, char** argv) {\\n    $0\\n    return 0;\\n}\"},\n"
"  {\"trigger\": \"deff\",  \"language\": \"python\", \"body\": \"def ${1:name}():\\n    $0\"},\n"
"  {\"trigger\": \"ifmain\",\"language\": \"python\", \"body\": \"if __name__ == \\\"__main__\\\":\\n    $0\"},\n"
"  {\"trigger\": \"todo\",  \"language\": \"\",       \"body\": \"TODO($0): \"}\n"
"]\n"));
        }
    }
    snippetDefs.clear();
    QFile f(cfgPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    for (const QJsonValue& v : QJsonDocument::fromJson(f.readAll()).array()) {
        const QJsonObject o = v.toObject();
        SnippetDef d{ o.value("trigger").toString(),
                      o.value("language").toString().toLower(),
                      o.value("body").toString() };
        if (!d.trigger.isEmpty() && !d.body.isEmpty()) snippetDefs.append(d);
    }
}

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

void MainWindow::applySnippetsToEditor(CodeEditor* editor) {
    if (!editor) return;
    const QString lang = editor->property("language").toString();   // "C/C++" / "Python" / …
    const QString key = lang == "C/C++" ? QStringLiteral("cpp")
                      : lang == "Python" ? QStringLiteral("python") : QString();
    QHash<QString, QString> map;
    for (const SnippetDef& d : snippetDefs)
        if (d.language.isEmpty() || d.language == key)
            map.insert(d.trigger, d.body);
    editor->setSnippets(map);
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
    const bool bigFile = raw.size() > 50 * 1024 * 1024;
    if (bigFile) {
        newEditor->setLargeFileMode(true);
        newEditor->setLineWrapMode(QPlainTextEdit::NoWrap);
        statusBar()->showMessage(tr("大檔案模式：已停用語法高亮與自動完成以確保流暢"), 5000);
    }
    newEditor->setProperty("bigFile", bigFile);
    newEditor->setPlainText(text);
    newEditor->document()->setModified(false);

    newEditor->setProperty("filePath", fileName);
    newEditor->setProperty("baseTitle", fileInfo.fileName());
    newEditor->setProperty("encoding", encName);
    newEditor->setProperty("eol", eol);
    tabWidget->setTabToolTip(tabWidget->indexOf(newEditor), fileName);
    updateTabTitle(newEditor);

    if (!bigFile) applyHighlighterForPath(newEditor, fileName);
    if (!bigFile && !lsp->languageIdForFile(fileName).isEmpty()) {
        newEditor->setLspEnabled(true);
        lsp->documentOpened(fileName, text);
    }
    if (!bigFile) fetchGitHead(fileName);               // Git gutter
    addToRecentFiles(fileName);
    if (fileWatcher) fileWatcher->addPath(fileName);
    statusBar()->showMessage("Opened " + fileName, 3000);
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
        addToRecentFiles(fileName);
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
        && !lsp->languageIdForFile(fileName).isEmpty()) {
        currentEditor->setLspEnabled(true);
        lsp->documentOpened(fileName, currentEditor->toPlainText());
        updateLspStatus();
    }
    lsp->documentSaved(fileName);

    // Snippet 設定檔存檔 → 立即重載並套用至所有分頁
    if (fileName == snippetConfigPath()) {
        loadSnippets();
        for (int i = 0; i < tabWidget->count(); ++i)
            applySnippetsToEditor(qobject_cast<CodeEditor*>(tabWidget->widget(i)));
        statusBar()->showMessage(tr("Snippet 設定已重新載入（%1 個）")
                                     .arg(snippetDefs.size()), 3000);
    }
    // 快捷鍵設定檔存檔 → 立即重新套用
    if (fileName == keymapConfigPath()) {
        applyKeymap();
        statusBar()->showMessage(tr("快捷鍵設定已重新套用"), 3000);
    }

    // Git gutter：另存的新路徑可能在版控中，重抓 HEAD（已知未版控者不重試）
    if (!gitHeadCache.contains(fileName)) fetchGitHead(fileName);
    else recomputeGitGutter(currentEditor);
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
        if (!path.isEmpty()) lsp->documentClosed(path);
        lspDirtyEditors.removeAll(QPointer<CodeEditor>(e));
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

// ----------------------------------------------------------------
// Find & Replace（大小寫 / 全字 / Regex / 全部標示 / F3）
// ----------------------------------------------------------------
void MainWindow::showFindDialog() {
    if (!findDialog) {
        findDialog = new QDialog(this);
        findDialog->setWindowTitle("Find & Replace");
        QGridLayout* layout = new QGridLayout(findDialog);
        layout->addWidget(new QLabel("Find:", findDialog), 0, 0);
        findInput = new QLineEdit(findDialog);
        layout->addWidget(findInput, 0, 1, 1, 3);
        layout->addWidget(new QLabel("Replace:", findDialog), 1, 0);
        replaceInput = new QLineEdit(findDialog);
        layout->addWidget(replaceInput, 1, 1, 1, 3);

        caseCheck      = new QCheckBox("Match case", findDialog);
        wholeWordCheck = new QCheckBox("Whole word", findDialog);
        regexCheck     = new QCheckBox("Regex", findDialog);
        layout->addWidget(caseCheck, 2, 1);
        layout->addWidget(wholeWordCheck, 2, 2);
        layout->addWidget(regexCheck, 2, 3);

        QPushButton* findNextBtn   = new QPushButton("Find Next", findDialog);
        QPushButton* findPrevBtn   = new QPushButton("Find Prev", findDialog);
        QPushButton* replaceBtn    = new QPushButton("Replace", findDialog);
        QPushButton* replaceAllBtn = new QPushButton("Replace All", findDialog);
        QHBoxLayout* btnLayout = new QHBoxLayout();
        btnLayout->addWidget(findNextBtn);
        btnLayout->addWidget(findPrevBtn);
        btnLayout->addWidget(replaceBtn);
        btnLayout->addWidget(replaceAllBtn);
        layout->addLayout(btnLayout, 3, 0, 1, 4);

        connect(findNextBtn,   &QPushButton::clicked, this, &MainWindow::performFind);
        connect(findPrevBtn,   &QPushButton::clicked, this, &MainWindow::performFindPrev);
        connect(replaceBtn,    &QPushButton::clicked, this, &MainWindow::performReplace);
        connect(replaceAllBtn, &QPushButton::clicked, this, &MainWindow::performReplaceAll);
        connect(findInput, &QLineEdit::returnPressed, this, &MainWindow::performFind);

        // 輸入時即時標示所有符合項目
        auto refreshHighlight = [this]() {
            if (auto editor = activeEditor())
                editor->setSearchHighlightPattern(buildFindRegex());
        };
        connect(findInput, &QLineEdit::textChanged, this, refreshHighlight);
        connect(caseCheck, &QCheckBox::toggled, this, refreshHighlight);
        connect(wholeWordCheck, &QCheckBox::toggled, this, refreshHighlight);
        connect(regexCheck, &QCheckBox::toggled, this, refreshHighlight);
        connect(findDialog, &QDialog::finished, this, [this](int) {
            if (auto editor = activeEditor())
                editor->setSearchHighlightPattern(QRegularExpression());
        });
    }
    // 預填目前選取文字
    if (auto editor = activeEditor()) {
        const QString sel = editor->textCursor().selectedText();
        if (!sel.isEmpty() && !sel.contains(QChar(0x2029)))
            findInput->setText(sel);
    }
    findDialog->show();
    findDialog->raise();
    findDialog->activateWindow();
    findInput->setFocus();
    findInput->selectAll();
}

QRegularExpression MainWindow::buildFindRegex() const {
    if (!findInput) return QRegularExpression();
    QString text = findInput->text();
    if (text.isEmpty()) return QRegularExpression();

    QString pattern = (regexCheck && regexCheck->isChecked())
                          ? text : QRegularExpression::escape(text);
    if (wholeWordCheck && wholeWordCheck->isChecked())
        pattern = QStringLiteral("\\b") + pattern + QStringLiteral("\\b");

    QRegularExpression::PatternOptions opts = QRegularExpression::NoPatternOption;
    if (!caseCheck || !caseCheck->isChecked())
        opts |= QRegularExpression::CaseInsensitiveOption;
    return QRegularExpression(pattern, opts);
}

QTextDocument::FindFlags MainWindow::buildFindFlags(bool backward) const {
    QTextDocument::FindFlags flags;
    if (backward) flags |= QTextDocument::FindBackward;
    return flags;
}

void MainWindow::performFind() {
    auto editor = activeEditor();
    if (!editor || !findDialog) { showFindDialog(); return; }
    QRegularExpression re = buildFindRegex();
    if (!re.isValid() || re.pattern().isEmpty()) return;

    if (!editor->find(re, buildFindFlags(false))) {
        QTextCursor cursor = editor->textCursor();
        cursor.movePosition(QTextCursor::Start);
        editor->setTextCursor(cursor);
        if (!editor->find(re, buildFindFlags(false)))
            statusBar()->showMessage("Cannot find \"" + findInput->text() + "\"", 3000);
    }
}

void MainWindow::performFindPrev() {
    auto editor = activeEditor();
    if (!editor || !findDialog) { showFindDialog(); return; }
    QRegularExpression re = buildFindRegex();
    if (!re.isValid() || re.pattern().isEmpty()) return;

    if (!editor->find(re, buildFindFlags(true))) {
        QTextCursor cursor = editor->textCursor();
        cursor.movePosition(QTextCursor::End);
        editor->setTextCursor(cursor);
        if (!editor->find(re, buildFindFlags(true)))
            statusBar()->showMessage("Cannot find \"" + findInput->text() + "\"", 3000);
    }
}

void MainWindow::performReplace() {
    if (auto editor = activeEditor()) {
        if (!findDialog) return;
        QRegularExpression re = buildFindRegex();
        if (!re.isValid() || re.pattern().isEmpty()) return;
        QString replaceText = replaceInput->text();
        QTextCursor cursor = editor->textCursor();
        if (cursor.hasSelection() && re.match(cursor.selectedText()).capturedLength() == cursor.selectedText().length())
            cursor.insertText(replaceText);
        performFind();
    }
}

void MainWindow::performReplaceAll() {
    if (auto editor = activeEditor()) {
        if (!findDialog) return;
        QRegularExpression re = buildFindRegex();
        if (!re.isValid() || re.pattern().isEmpty()) return;
        QString replaceText = replaceInput->text();

        QTextCursor cursor(editor->document());
        cursor.beginEditBlock();
        int count = 0;
        QTextCursor found = editor->document()->find(re, 0);
        while (!found.isNull()) {
            const int after = found.selectionEnd();
            found.insertText(replaceText);
            count++;
            int next = found.position();
            if (replaceText.isEmpty() && next == after) next++; // 避免空字串無限迴圈
            found = editor->document()->find(re, next);
        }
        cursor.endEditBlock();
        statusBar()->showMessage(QString::number(count) + " replacements made.", 4000);
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
// Recent Files
// ----------------------------------------------------------------
void MainWindow::openRecentFile() {
    QAction* action = qobject_cast<QAction*>(sender());
    if (action) {
        openFileByPath(action->data().toString());
    }
}

void MainWindow::updateRecentFileActions() {
    int numRecentFiles = qMin(recentFiles.size(), 15);
    for (int i = 0; i < numRecentFiles; ++i) {
        QString text = tr("&%1 %2").arg(i + 1).arg(QFileInfo(recentFiles[i]).fileName());
        recentFileActions[i]->setText(text);
        recentFileActions[i]->setData(recentFiles[i]);
        recentFileActions[i]->setVisible(true);
    }
    for (int j = numRecentFiles; j < 15; ++j) {
        recentFileActions[j]->setVisible(false);
    }
}

void MainWindow::saveRecentFiles() {
    AppSettings settings;
    settings.setValue("recentFileList", recentFiles);
}

void MainWindow::loadRecentFiles() {
    AppSettings settings;
    recentFiles = settings.value("recentFileList").toStringList();
}

void MainWindow::addToRecentFiles(const QString& filePath) {
    recentFiles.removeAll(filePath);
    recentFiles.prepend(filePath);
    while (recentFiles.size() > 15) {
        recentFiles.removeLast();
    }
    saveRecentFiles();
    updateRecentFileActions();
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
    lsp->setRootPath(folder);
    for (int i = 0; i < tabWidget->count(); ++i) {
        auto e = qobject_cast<CodeEditor*>(tabWidget->widget(i));
        if (!e || !e->lspEnabled()) continue;
        const QString path = e->property("filePath").toString();
        if (!path.isEmpty()) lsp->documentOpened(path, e->toPlainText());
    }
    updateLspStatus();

    statusBar()->showMessage("Folder: " + folder, 3000);
}

void MainWindow::showQuickOpen() {
    if (projectFolder.isEmpty()) {
        openFolder();
        if (projectFolder.isEmpty()) return;
    }
    quickOpen->open();
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

    for (const QJsonValue& v : tabs) {
        const QJsonObject t = v.toObject();
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
        if (!editor) continue;

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
    }
    const int idx = root["currentIndex"].toInt();
    if (idx >= 0 && idx < tabWidget->count()) tabWidget->setCurrentIndex(idx);
    statusBar()->showMessage(tr("已還原上次工作階段（%1 個分頁）").arg(tabs.size()), 4000);
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
    settings.setValue("ui/theme", themeCombo->currentText());
    settings.setValue("ui/language", newLang);

    if (langChanged)
        QMessageBox::information(this, tr("介面語言"),
            tr("介面語言將於下次啟動 AlexCode 時生效。"));

    // 立即套用
    if (themeCombo->currentText() != Theme::currentThemeName) {
        Theme::setTheme(themeCombo->currentText());
        qApp->setStyleSheet(Theme::stylesheet());
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
        if (!p.isEmpty() && !e->property("bigFile").toBool()) fetchGitHead(p);
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
