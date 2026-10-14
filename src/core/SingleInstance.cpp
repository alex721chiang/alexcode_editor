#include "SingleInstance.h"
#include "Portable.h"
#include <QCryptographicHash>
#include <QDataStream>
#include <QDir>
#include <QElapsedTimer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalServer>
#include <QLocalSocket>
#include <QLockFile>
#include <QThread>

#ifdef Q_OS_WIN
#include <windows.h>
#endif

namespace {
constexpr int kMaxPayload = 1024 * 1024;   // 防呆：單則訊息上限
const QByteArray kAck("OK");
}

SingleInstance::SingleInstance(const QString& key, const QString& lockDir, QObject* parent)
    : QObject(parent), m_key(key), m_lockDir(lockDir) {}

SingleInstance::~SingleInstance() {
    if (m_server) m_server->close();           // 移除通道（Unix 會刪除 socket 檔）
    if (m_lock) m_lock->unlock();
}

QString SingleInstance::defaultKey() {
    QString user = qEnvironmentVariable("USERNAME");
    if (user.isEmpty()) user = qEnvironmentVariable("USER");
    const QByteArray seed = (QDir::cleanPath(Portable::dataDir()) + QLatin1Char('|') + user).toUtf8();
    return QStringLiteral("AlexCode-") +
           QString::fromLatin1(QCryptographicHash::hash(seed, QCryptographicHash::Sha1).toHex().left(16));
}

QString SingleInstance::lockPathFor(const QString& key, const QString& lockDir) {
    return QDir(lockDir).filePath(key + QStringLiteral(".lock"));
}

bool SingleInstance::tryBecomePrimary() {
    QDir().mkpath(m_lockDir);
    auto lock = std::make_unique<QLockFile>(lockPathFor(m_key, m_lockDir));
    lock->setStaleLockTime(0);                 // 只以「擁有者行程已不存在」判定過期
    if (!lock->tryLock(0)) return false;
    m_lock = std::move(lock);

    m_server = new QLocalServer(this);
    m_server->setSocketOptions(QLocalServer::UserAccessOption);   // 只允許同一使用者連線
    QLocalServer::removeServer(m_key);         // 持鎖者才清：此時存在的通道必為上次當機的殘留
    if (!m_server->listen(m_key)) {
        qWarning("SingleInstance: listen(%s) failed: %s", qPrintable(m_key),
                 qPrintable(m_server->errorString()));
        return true;                           // 仍是主實例，只是收不到其他實例的檔案
    }
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        while (QLocalSocket* c = m_server->nextPendingConnection()) handleClient(c);
    });
    return true;
}

void SingleInstance::handleClient(QLocalSocket* c) {
    connect(c, &QLocalSocket::disconnected, c, &QObject::deleteLater);
    auto onData = [this, c]() {
        if (c->bytesAvailable() > kMaxPayload + 4) { c->abort(); return; }
        QDataStream in(c);
        in.setVersion(QDataStream::Qt_6_0);
        in.startTransaction();
        QByteArray payload;
        in >> payload;
        if (!in.commitTransaction()) return;   // 尚未收齊，等下一次 readyRead
        const QJsonArray arr = QJsonDocument::fromJson(payload).object().value(QStringLiteral("files")).toArray();
        QStringList files;
        for (const QJsonValue& v : arr) files << v.toString();
        c->write(kAck);                        // 先確認再處理：開檔（可能在網路上）不讓次實例等待
        c->flush();
        emit filesReceived(files);
    };
    connect(c, &QLocalSocket::readyRead, this, onData);
    if (c->bytesAvailable() > 0) QMetaObject::invokeMethod(this, onData, Qt::QueuedConnection);
}

bool SingleInstance::sendToPrimary(const QStringList& files, int timeoutMs) {
#ifdef Q_OS_WIN
    // 本行程由使用者啟動而擁有前景權；授權主實例把視窗帶到前景（否則 Windows 只會閃工作列）
    AllowSetForegroundWindow(ASFW_ANY);
#endif
    QByteArray msg;
    {
        QDataStream out(&msg, QIODevice::WriteOnly);
        out.setVersion(QDataStream::Qt_6_0);
        out << QJsonDocument(QJsonObject{{QStringLiteral("files"), QJsonArray::fromStringList(files)}})
                   .toJson(QJsonDocument::Compact);
    }

    QElapsedTimer t;
    t.start();
    auto remaining = [&] { return int(qMax<qint64>(1, timeoutMs - t.elapsed())); };
    while (t.elapsed() < timeoutMs) {
        QLocalSocket s;
        s.connectToServer(m_key);
        if (!s.waitForConnected(remaining())) {
            QThread::msleep(50);               // 主實例可能剛拿到鎖、尚未開始 listen
            continue;
        }
        s.write(msg);
        if (!s.waitForBytesWritten(remaining())) return false;
        QByteArray reply;
        while (reply.size() < kAck.size() && t.elapsed() < timeoutMs) {
            if (!s.waitForReadyRead(remaining())) break;
            reply += s.readAll();
        }
        return reply.startsWith(kAck);
    }
    return false;
}
