#pragma once
#include <QObject>
#include <QJSValue>
#include <QStringList>
#include <QHash>
#include <functional>

class CodeEditor;
class QJSEngine;

// JS 外掛的 API 橋接物件：以全域 `alexcode` 為名暴露給腳本，
// Q_INVOKABLE 方法即腳本可呼叫的介面。不直接認識 MainWindow——
// 編輯器/開檔都透過建構時傳入的回呼、狀態列用 signal（與 controller 拆分同一模式）。
class PluginApi : public QObject {
    Q_OBJECT
public:
    using EditorFn = std::function<CodeEditor*()>;
    using OpenFn   = std::function<void(const QString&)>;
    using RegisterFn = std::function<void(const QString&, const QJSValue&)>;
    PluginApi(EditorFn activeEditor, OpenFn openFile, QObject* parent = nullptr);
    void setRegisterCallback(RegisterFn cb) { m_register = std::move(cb); }

    // ---- 指令註冊（外掛選單入口）----
    Q_INVOKABLE void registerCommand(const QString& name, const QJSValue& fn);

    // ---- 編輯器 ----
    Q_INVOKABLE QString text() const;
    Q_INVOKABLE void setText(const QString& t);
    Q_INVOKABLE QString selectedText() const;
    Q_INVOKABLE void insertText(const QString& t);   // 取代選取或於游標插入
    Q_INVOKABLE int currentLine() const;             // 0-based
    Q_INVOKABLE int lineCount() const;
    Q_INVOKABLE QString line(int n) const;           // 0-based；越界回傳空字串
    Q_INVOKABLE void gotoLine(int n);                // 1-based（與「跳至行」一致）

    // ---- 應用 ----
    Q_INVOKABLE QString currentFilePath() const;
    Q_INVOKABLE void openFile(const QString& path);
    Q_INVOKABLE void statusMessage(const QString& msg, int timeoutMs = 3000);
    Q_INVOKABLE QString prompt(const QString& title, const QString& label,
                               const QString& defaultText = QString());

signals:
    void statusRequested(const QString& msg, int timeoutMs);

private:
    EditorFn m_activeEditor;
    OpenFn m_openFile;
    RegisterFn m_register;
};

// 腳本外掛管理：掃描外掛資料夾的 *.js 以 QJSEngine 執行；腳本用
// alexcode.registerCommand(名稱, fn) 註冊指令，MainWindow 據此建外掛選單。
// 選 JS 而非 C++ DLL：無 ABI/編譯器相容問題（MinGW/MSVC 混用會炸），
// 使用者存個 .js 檔就能擴充。首次啟動寫入範例（與 Snippet/LSP 設定同模式）。
class PluginManager : public QObject {
    Q_OBJECT
public:
    // dirOverride 供測試指定暫存資料夾；正式使用留空 → dataDir()/plugins
    PluginManager(PluginApi::EditorFn activeEditor, PluginApi::OpenFn openFile,
                  const QString& dirOverride = QString(), QObject* parent = nullptr);

    QString pluginsDir() const { return m_dir; }
    void reload();                                   // 重建引擎、重新執行所有腳本
    QStringList commandNames() const { return m_order; }
    bool runCommand(const QString& name);            // false = 指令不存在
    int scriptCount() const { return m_scriptCount; }
    QStringList errors() const { return m_errors; }  // 載入/執行錯誤（檔名: 訊息）

signals:
    void commandsChanged();                          // 重載後選單需要重建
    void statusRequested(const QString& msg, int timeoutMs);

private:
    void writeExamplesIfMissing() const;

    PluginApi* m_api = nullptr;
    QJSEngine* m_engine = nullptr;
    QString m_dir;
    QHash<QString, QJSValue> m_commands;
    QStringList m_order;                             // 依註冊順序（選單穩定）
    QStringList m_errors;
    int m_scriptCount = 0;
};
