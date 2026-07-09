// 無頭 PTY 煙霧測試（Windows=ConPTY、POSIX=forkpty）：用 PtySession 跑 echo，
// 確認真的能拿到子行程輸出。通過條件：輸出串流中看到我們 echo 的標記字串。
// Linux 有註冊進 ctest（CI 驗證 forkpty 行為）；Windows 桌面鎖定/背景環境下
// ConPTY 行為不穩定，僅供手動執行。
#include <QCoreApplication>
#include <QTimer>
#include <QByteArray>
#include <QDebug>
#include "../src/PtySession.h"

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    PtySession pty;
    QByteArray acc;
    const QByteArray needle = "ALEXCODE_PTY_OK";

    QObject::connect(&pty, &PtySession::dataReceived, [&](const QByteArray& d) {
        acc += d;
        if (acc.contains(needle)) {
            qInfo() << "[pty_smoke] 收到標記，PTY 正常";
            QCoreApplication::exit(0);
        }
    });
    QObject::connect(&pty, &PtySession::exited, [&]() {
        QCoreApplication::exit(acc.contains(needle) ? 0 : 1);
    });
    QTimer::singleShot(15000, [&]() {
        qCritical() << "[pty_smoke] 逾時，未收到標記。已收 bytes=" << acc.size();
        QCoreApplication::exit(2);
    });

#ifdef Q_OS_WIN
    const QString cmd = QStringLiteral("cmd.exe /c echo ALEXCODE_PTY_OK");
#else
    const QString cmd = QStringLiteral("echo ALEXCODE_PTY_OK");   // 經 sh -c 執行
#endif
    if (!pty.start(cmd, QString(), 80, 24)) {
        qCritical() << "[pty_smoke] 無法啟動 PTY";
        return 3;
    }
    return app.exec();
}
