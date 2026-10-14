#include "FileOpenRouter.h"
#include <QCoreApplication>
#include <QFileOpenEvent>
#include <utility>

FileOpenRouter::FileOpenRouter(QObject* parent) : QObject(parent) {
    if (auto* app = QCoreApplication::instance()) app->installEventFilter(this);
}

FileOpenRouter::~FileOpenRouter() {
    if (auto* app = QCoreApplication::instance()) app->removeEventFilter(this);
}

void FileOpenRouter::setReady() {
    m_ready = true;
    if (!m_pending.isEmpty()) emit filesRequested(std::exchange(m_pending, {}));
}

bool FileOpenRouter::eventFilter(QObject* watched, QEvent* event) {
    if (event->type() != QEvent::FileOpen || watched != QCoreApplication::instance())
        return QObject::eventFilter(watched, event);
    const QString path = static_cast<QFileOpenEvent*>(event)->file();
    if (path.isEmpty()) return false;                  // 非本機檔案的 URL：不處理
    if (m_ready) emit filesRequested({path});
    else m_pending << path;
    return true;
}
