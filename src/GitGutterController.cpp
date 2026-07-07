#include "GitGutterController.h"
#include "GitGutter.h"
#include "CodeEditor.h"
#include <QProcess>
#include <QFileInfo>

GitGutterController::GitGutterController(QObject* parent) : QObject(parent) {}

void GitGutterController::fetchHead(const QString& path) {
    if (path.isEmpty() || m_untracked.contains(path)) return;
    const QFileInfo fi(path);
    auto* p = new QProcess(this);
    p->setWorkingDirectory(fi.absolutePath());
    connect(p, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, [this, p, path](int code, QProcess::ExitStatus) {
        if (code == 0) {
            QString head = QString::fromUtf8(p->readAllStandardOutput());
            head.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
            m_headCache.insert(path, head);
            emit headReady(path);
        } else {
            m_untracked.insert(path);               // 不在版控（或非 git 目錄）：不重試
            emit markedUntracked(path);
        }
        p->deleteLater();
    });
    // "HEAD:./檔名" 相對於工作目錄解析，免去計算 repo 內相對路徑
    p->start("git", {"show", "HEAD:./" + fi.fileName()});
}

void GitGutterController::recompute(CodeEditor* editor) const {
    if (!editor || editor->property("bigFile").toBool()) return;
    const QString path = editor->property("filePath").toString();
    const auto it = m_headCache.constFind(path);
    if (path.isEmpty() || it == m_headCache.constEnd()) return;
    editor->setGitLineStates(GitGutter::diffLineStates(
        it.value().split('\n'), editor->toPlainText().split('\n')));
}
