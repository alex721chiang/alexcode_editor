// 無頭 ConPTY 煙霧測試：用 PtySession 跑 cmd 的 echo，確認真的能拿到子行程輸出。
// 通過條件：在 PTY 輸出串流中看到我們 echo 的標記字串。
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
            qInfo() << "[pty_smoke] 收到標記，ConPTY 正常";
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

    if (!pty.start("cmd.exe /c echo ALEXCODE_PTY_OK", QString(), 80, 24)) {
        qCritical() << "[pty_smoke] 無法啟動 ConPTY";
        return 3;
    }
    return app.exec();
}
