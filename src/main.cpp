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
    QString shotPath, termCmd, settingsShot, aboutShot, vaultDir, graphDir, paletteShot, projsymShot, callgraphShot, menuShot;
    bool wantPreview = false;
    bool wantWhitespace = false;
    QString refsName;
    int gotoLineArg = 0;
    for (int i = 1; i < args.size(); ++i) {
        if (args[i] == "--screenshot" && i + 1 < args.size()) shotPath = args[++i];
        else if (args[i] == "--termcmd" && i + 1 < args.size()) termCmd = args[++i];
        else if (args[i] == "--settings-shot" && i + 1 < args.size()) settingsShot = args[++i];
        else if (args[i] == "--about-shot" && i + 1 < args.size()) aboutShot = args[++i];
        else if (args[i] == "--palette-shot" && i + 1 < args.size()) paletteShot = args[++i];
        else if (args[i] == "--projsym-shot" && i + 1 < args.size()) projsymShot = args[++i];
        else if (args[i] == "--callgraph-shot" && i + 1 < args.size()) callgraphShot = args[++i];
        else if (args[i] == "--menu-shot" && i + 1 < args.size()) menuShot = args[++i];
        else if ((args[i] == "--folder" || args[i] == "--vault") && i + 1 < args.size()) vaultDir = args[++i];
        else if (args[i] == "--graph" && i + 1 < args.size()) graphDir = args[++i];
        else if (args[i] == "--goto" && i + 1 < args.size()) gotoLineArg = args[++i].toInt();
        else if (args[i] == "--refs" && i + 1 < args.size()) refsName = args[++i];
        else if (args[i] == "--whitespace") wantWhitespace = true;
        else if (args[i] == "--preview") wantPreview = true;
        else if (QFileInfo(args[i]).isFile())
            QMetaObject::invokeMethod(&w, "openFileByPath", Q_ARG(QString, args[i]));
    }
    if (gotoLineArg > 0) w.gotoLineForShot(gotoLineArg);
    if (wantWhitespace) w.setShowWhitespaceAll(true);
    if (wantPreview) w.showMarkdownPreviewForShot();
    if (!graphDir.isEmpty() && QFileInfo(graphDir).isDir())
        w.openGraphForShot(graphDir);
    else if (!vaultDir.isEmpty() && QFileInfo(vaultDir).isDir())
        w.openVaultForShot(vaultDir);
    if (!refsName.isEmpty()) w.findRefsForShot(refsName);

    if (!aboutShot.isEmpty()) {               // 截關於對話框
        w.openAboutForShot(aboutShot);
        QTimer::singleShot(1400, &a, [&a]() { a.quit(); });
    } else if (!paletteShot.isEmpty()) {      // 截命令面板
        w.resize(1200, 800);
        w.openCommandPaletteForShot(paletteShot);
        QTimer::singleShot(1400, &a, [&a]() { a.quit(); });
    } else if (!projsymShot.isEmpty()) {      // 截專案符號搜尋
        w.resize(1200, 800);
        w.openProjectSymbolForShot(projsymShot);
        QTimer::singleShot(1400, &a, [&a]() { a.quit(); });
    } else if (!callgraphShot.isEmpty()) {    // 截 Call Graph（游標位置由 --goto 設定）
        w.resize(1200, 800);
        w.openCallGraphForShot(callgraphShot);
        QTimer::singleShot(1400, &a, [&a]() { a.quit(); });
    } else if (!menuShot.isEmpty()) {         // 截「檢視」選單（index 4）以驗證 i18n
        w.resize(1200, 800);
        w.openMenuForShot(4, menuShot);
        QTimer::singleShot(1400, &a, [&a]() { a.quit(); });
    } else if (!settingsShot.isEmpty()) {     // 截設定中心對話框
        w.resize(1200, 800);
        w.openSettingsForShot(settingsShot);
        QTimer::singleShot(1600, &a, [&a]() { a.quit(); });
    } else if (!shotPath.isEmpty()) {
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
