#pragma once
#include <QIcon>
#include <QString>

// 自繪單色線條圖示（取代 Qt 內建的 SP_* 舊式彩色圖示，與霓虹主題一致）。
// 線色取 Theme::EDITOR_FG（降透明度）、點綴色取 Theme::ACCENT——呼叫當下取色，
// 主題切換後重呼叫即可換色（MainWindow::applyActionIcons）。
namespace NeonIcons {

// kind: "new" | "open" | "save" | "undo" | "redo" | "find" | "find-in-files"
QIcon icon(const QString& kind);

} // namespace NeonIcons
