#include "PluginManager.h"
#include "CodeEditor.h"
#include "Portable.h"
#include <QJSEngine>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTextCursor>
#include <QTextBlock>
#include <QInputDialog>

// ============================================================
// PluginApi
// ============================================================
PluginApi::PluginApi(EditorFn activeEditor, OpenFn openFile, QObject* parent)
    : QObject(parent), m_activeEditor(std::move(activeEditor)), m_openFile(std::move(openFile)) {}

void PluginApi::registerCommand(const QString& name, const QJSValue& fn) {
    if (m_register && !name.trimmed().isEmpty() && fn.isCallable())
        m_register(name.trimmed(), fn);
}

QString PluginApi::text() const {
    CodeEditor* e = m_activeEditor();
    return e ? e->toPlainText() : QString();
}

void PluginApi::setText(const QString& t) {
    if (CodeEditor* e = m_activeEditor()) {
        QTextCursor c(e->document());                 // 經游標走 undo 堆疊（setPlainText 會清空）
        c.select(QTextCursor::Document);
        c.insertText(t);
    }
}

QString PluginApi::selectedText() const {
    CodeEditor* e = m_activeEditor();
    if (!e) return QString();
    return e->textCursor().selectedText().replace(QChar(0x2029), QChar('\n'));
}

void PluginApi::insertText(const QString& t) {
    if (CodeEditor* e = m_activeEditor()) e->textCursor().insertText(t);
}

int PluginApi::currentLine() const {
    CodeEditor* e = m_activeEditor();
    return e ? e->textCursor().blockNumber() : 0;
}

int PluginApi::lineCount() const {
    CodeEditor* e = m_activeEditor();
    return e ? e->blockCount() : 0;
}

QString PluginApi::line(int n) const {
    CodeEditor* e = m_activeEditor();
    if (!e) return QString();
    const QTextBlock b = e->document()->findBlockByNumber(n);
    return b.isValid() ? b.text() : QString();
}

void PluginApi::gotoLine(int n) {
    if (CodeEditor* e = m_activeEditor()) e->gotoLine(n);
}

QString PluginApi::currentFilePath() const {
    CodeEditor* e = m_activeEditor();
    return e ? e->property("filePath").toString() : QString();
}

void PluginApi::openFile(const QString& path) {
    if (m_openFile) m_openFile(path);
}

void PluginApi::statusMessage(const QString& msg, int timeoutMs) {
    emit statusRequested(msg, timeoutMs);
}

QString PluginApi::prompt(const QString& title, const QString& label, const QString& defaultText) {
    return QInputDialog::getText(nullptr, title, label, QLineEdit::Normal, defaultText);
}

// ============================================================
// PluginManager
// ============================================================
PluginManager::PluginManager(PluginApi::EditorFn activeEditor, PluginApi::OpenFn openFile,
                             const QString& dirOverride, QObject* parent)
    : QObject(parent) {
    m_dir = dirOverride.isEmpty() ? Portable::dataDir() + QStringLiteral("/plugins") : dirOverride;
    m_api = new PluginApi(std::move(activeEditor), std::move(openFile), this);
    connect(m_api, &PluginApi::statusRequested, this, &PluginManager::statusRequested);
    m_api->setRegisterCallback([this](const QString& name, const QJSValue& fn) {
        if (!m_commands.contains(name)) m_order.append(name);
        m_commands.insert(name, fn);
    });
}

void PluginManager::writeExamplesIfMissing() const {
    QDir dir(m_dir);
    if (dir.exists()) return;                        // 已有資料夾就完全不動（含使用者清空的情況）
    QDir().mkpath(m_dir);
    QFile f(m_dir + QStringLiteral("/examples.js"));
    if (!f.open(QIODevice::WriteOnly)) return;
    f.write(QByteArray(
"// AlexCode 腳本外掛範例 — 放在這個資料夾的 *.js 會在啟動時載入（工具→腳本外掛→重新載入）\n"
"//\n"
"// API（全域物件 alexcode）：\n"
"//   registerCommand(名稱, fn)  註冊指令（出現在 工具→腳本外掛 選單）\n"
"//   text() / setText(t)        全文讀寫（setText 可 Ctrl+Z 復原）\n"
"//   selectedText() / insertText(t)  選取文字 / 於游標插入（取代選取）\n"
"//   currentLine() / lineCount() / line(n) / gotoLine(n)\n"
"//   currentFilePath() / openFile(path)\n"
"//   statusMessage(訊息, 毫秒) / prompt(標題, 說明, 預設值)\n"
"\n"
"alexcode.registerCommand(\"插入日期時間\", function() {\n"
"    const now = new Date();\n"
"    const pad = n => String(n).padStart(2, \"0\");\n"
"    alexcode.insertText(now.getFullYear() + \"-\" + pad(now.getMonth() + 1) + \"-\" + pad(now.getDate())\n"
"        + \" \" + pad(now.getHours()) + \":\" + pad(now.getMinutes()));\n"
"});\n"
"\n"
"alexcode.registerCommand(\"為選取文字加上引號\", function() {\n"
"    const s = alexcode.selectedText();\n"
"    if (s.length === 0) { alexcode.statusMessage(\"請先選取文字\", 2500); return; }\n"
"    alexcode.insertText('\"' + s + '\"');\n"
"});\n"));
}

void PluginManager::reload() {
    writeExamplesIfMissing();
    // QJSValue 持有引擎參照，換引擎前必須先清掉
    m_commands.clear();
    m_order.clear();
    m_errors.clear();
    m_scriptCount = 0;
    delete m_engine;
    m_engine = new QJSEngine(this);
    m_engine->installExtensions(QJSEngine::ConsoleExtension);   // console.log 進 qDebug
    m_engine->globalObject().setProperty(QStringLiteral("alexcode"),
                                         m_engine->newQObject(m_api));   // api 有 parent → C++ 持有

    const QFileInfoList scripts =
        QDir(m_dir).entryInfoList({ QStringLiteral("*.js") }, QDir::Files, QDir::Name);
    for (const QFileInfo& fi : scripts) {
        QFile f(fi.absoluteFilePath());
        if (!f.open(QIODevice::ReadOnly)) continue;
        const QJSValue r = m_engine->evaluate(QString::fromUtf8(f.readAll()), fi.fileName());
        if (r.isError()) {
            m_errors.append(QStringLiteral("%1:%2: %3")
                                .arg(fi.fileName(), r.property("lineNumber").toString(),
                                     r.toString()));
        } else {
            ++m_scriptCount;
        }
    }
    if (!m_errors.isEmpty())
        emit statusRequested(tr("外掛載入有錯誤：%1").arg(m_errors.join(QStringLiteral("；"))), 6000);
    emit commandsChanged();
}

bool PluginManager::runCommand(const QString& name) {
    auto it = m_commands.find(name);
    if (it == m_commands.end()) return false;
    const QJSValue r = it->call();
    if (r.isError())
        emit statusRequested(tr("外掛「%1」執行錯誤：%2").arg(name, r.toString()), 6000);
    return true;
}
