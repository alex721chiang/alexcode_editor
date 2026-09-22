#pragma once
#include <QObject>
#include <QVector>
#include <QSet>
#include <QList>
#include <QPair>
#include <QString>
#include <QAction>
#include <QKeySequence>
#include <algorithm>
#include "CommandPalette.h"
#include "Theme.h"
#include "Version.h"   // aboutHtml() 使用 ALEXCODE_VERSION；自足標頭，不依賴引入順序

// 主視窗共用的純函式（原 MainWindow.cpp 的 file-static）：
// 命令面板蒐集、快捷鍵蒐集、關於對話框 HTML。以 inline 集中於此，
// 讓 MainWindow 與 ScreenshotHelper 共用同一份實作。
namespace UiHelpers {

inline QVector<CommandPalette::Command> gatherCommands(const QObject* w) {
    QVector<CommandPalette::Command> cmds;
    QSet<QString> seen;
    const QList<QAction*> actions = w->findChildren<QAction*>();
    for (QAction* a : actions) {
        if (a->isSeparator() || a->menu()) continue;         // 跳過分隔線與子選單
        QString name = a->text();
        name.remove(QLatin1Char('&'));                        // 去掉助記符
        name = name.trimmed();
        if (name.isEmpty() || seen.contains(name)) continue;
        seen.insert(name);
        cmds.append({ name, a->shortcut().toString(QKeySequence::NativeText), a });
    }
    std::sort(cmds.begin(), cmds.end(),
              [](const CommandPalette::Command& a, const CommandPalette::Command& b) {
                  return a.name.localeAwareCompare(b.name) < 0;
              });
    return cmds;
}

// 蒐集有快捷鍵的動作（名稱去重，與 applyKeymap 同邏輯）
inline QList<QPair<QString, QString>> gatherShortcutActions(const QObject* w) {
    QList<QPair<QString, QString>> actions;
    QSet<QString> seen;
    for (QAction* a : w->findChildren<QAction*>()) {
        if (a->text().isEmpty() || a->shortcut().isEmpty()) continue;
        const QString name = QString(a->text()).remove(QLatin1Char('&'));
        if (seen.contains(name)) continue;
        seen.insert(name);
        actions.append({name, a->shortcut().toString()});
    }
    return actions;
}

inline QString aboutHtml() {
    // 字標以內嵌的 Orbitron 呈現（未來科技感）；未註冊成功時退回一般字型。
    // 刻意用 QObject::tr（沿用原 context），搬移至此不影響既有翻譯。
    return QObject::tr(
        "<div style=\"font-family:'Orbitron','Segoe UI',sans-serif; font-size:20px;"
        " font-weight:800; letter-spacing:3px; color:%3;\">ALEXCODE</div>"
        "<p style=\"color:%4; letter-spacing:2px;\">v%1 &nbsp;·&nbsp; NEON EDITION</p>"
        "<p>輕量級 Qt 程式碼編輯器：LSP、Git、互動式終端機、多語言語法高亮、"
        "log 分析、多套主題（含 Neon HUD）、繁中/英文介面。</p>"
        "<p>以 Qt %2 建置。</p>"
        "<p><a href=\"https://github.com/alex721chiang/alexcode_editor\">GitHub 專案</a></p>")
        .arg(QStringLiteral(ALEXCODE_VERSION), QStringLiteral(QT_VERSION_STR),
             Theme::ACCENT, Theme::ACCENT2);
}

} // namespace UiHelpers
