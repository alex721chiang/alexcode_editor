#include <QApplication>
#include <QIcon>
#include <QFileInfo>
#include <QTranslator>
#include <QLocale>
#include <QTimer>
#include <QPixmap>
#include "MainWindow.h"
#include "Theme.h"
#include "Portable.h"

int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    QApplication::setOrganizationName("AlexCode");
    QApplication::setApplicationName("AlexCodeEditor");

    // UI 語言：ui/language = "system"（預設，跟隨系統地區）/ "en" / "zh_TW"。
    // 來源字串為英文與中文混合，因此一律安裝一個翻譯器以確保介面語言一致。
    {
        QString lang = AppSettings().value("ui/language", "system").toString();
        if (lang == "system")
            lang = QLocale::system().name().startsWith("zh") ? "zh_TW" : "en";
        static QTranslator translator;
        if (translator.load(QStringLiteral(":/i18n/app_%1.qm").arg(lang)))
            a.installTranslator(&translator);
    }

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

    // 命令列解析：一般開檔；--screenshot <out.png> 讓程式自我渲染存圖（不需螢幕、桌面鎖定也可用）；
    // --termcmd "<cmd>" 搭配 --screenshot 時，開終端機並執行該指令後再截圖。
    const QStringList args = a.arguments();
    QString shotPath, termCmd;
    for (int i = 1; i < args.size(); ++i) {
        if (args[i] == "--screenshot" && i + 1 < args.size()) shotPath = args[++i];
        else if (args[i] == "--termcmd" && i + 1 < args.size()) termCmd = args[++i];
        else if (QFileInfo::exists(args[i]))
            QMetaObject::invokeMethod(&w, "openFileByPath", Q_ARG(QString, args[i]));
    }

    if (!shotPath.isEmpty()) {
        w.resize(1200, 800);
        if (!termCmd.isEmpty()) w.openTerminalForShot(termCmd);
        const int delayMs = termCmd.isEmpty() ? 1500 : 3000;   // 終端機需等子行程輸出
        QTimer::singleShot(delayMs, &w, [&w, shotPath, &a]() {
            w.grab().save(shotPath);                            // Qt 自我渲染，與螢幕/鎖定無關
            a.quit();
        });
    }
    return a.exec();
}
