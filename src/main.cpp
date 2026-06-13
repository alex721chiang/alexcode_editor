#include <QApplication>
#include <QIcon>
#include <QFileInfo>
#include "MainWindow.h"
#include "Theme.h"
#include "Portable.h"

int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    QApplication::setOrganizationName("AlexCode");
    QApplication::setApplicationName("AlexCodeEditor");

    // 設定遷移：v4.2 起改存 settings.ini（可攜模式存執行檔旁 ./data）。
    // 首次啟動時把舊登錄檔設定搬過來，避免使用者偏好遺失。
    if (!QFileInfo::exists(Portable::dataDir() + "/settings.ini")) {
        QSettings legacy("AlexCode", "AlexCodeEditor");
        if (!legacy.allKeys().isEmpty()) {
            AppSettings fresh;
            for (const QString& key : legacy.allKeys())
                fresh.setValue(key, legacy.value(key));
        }
    }
    a.setStyle("Fusion");                    // 確保 QSS 在各平台一致
    Theme::setTheme(AppSettings().value("ui/theme", "Neon Grid").toString());
    a.setStyleSheet(Theme::stylesheet());    // 套用使用者選擇的主題（預設 Neon Grid）
    a.setWindowIcon(QIcon(":/icon.png"));
    MainWindow w;
    w.show();
    // 命令列開檔（含「以 AlexCode 開啟」檔案關聯）
    const QStringList args = a.arguments();
    for (int i = 1; i < args.size(); ++i) {
        if (QFileInfo::exists(args[i]))
            QMetaObject::invokeMethod(&w, "openFileByPath", Q_ARG(QString, args[i]));
    }
    return a.exec();
}
