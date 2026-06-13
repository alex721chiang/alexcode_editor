#pragma once
#include <QSettings>
#include <QString>
#include <QCoreApplication>
#include <QStandardPaths>
#include <QDir>
#include <QFileInfo>

// 6.5 可攜模式：執行檔旁存在 portable.ini 時，所有設定與資料改存 ./data
// （解壓即用、不碰使用者目錄與登錄檔）。否則沿用系統 AppData。
namespace Portable {

inline bool isPortable() {
    static const bool p = QFileInfo::exists(
        QCoreApplication::applicationDirPath() + QStringLiteral("/portable.ini"));
    return p;
}

inline QString dataDir() {
    const QString d = isPortable()
        ? QCoreApplication::applicationDirPath() + QStringLiteral("/data")
        : QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    QDir().mkpath(d);
    return d;
}

} // namespace Portable

// 統一設定入口（取代 QSettings("AlexCode", "AlexCodeEditor")）：
// 一律存 ini 於 dataDir，可攜/一般模式僅路徑不同。首次啟動由 main() 將舊登錄檔設定遷移過來。
class AppSettings : public QSettings {
public:
    AppSettings() : QSettings(Portable::dataDir() + QStringLiteral("/settings.ini"),
                              QSettings::IniFormat) {}
};
