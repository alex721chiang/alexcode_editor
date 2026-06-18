#pragma once
#include <QString>
#include <QStringList>
#include <QHash>

// ============================================================
//  AlexCode 主題系統（6.3）
//  內建：Neon Grid（預設）/ Paper Light / Matrix
//  - 程式內取色：Theme::EDITOR_BG 等 inline 變數（setTheme 時更新）
//  - QSS：stylesheet() 以目前調色盤產生
// ============================================================
namespace Theme {

struct Palette {
    QString bg, panel, border, accent, accentDim, accent2;
    QString fg, fgDim, fgDim2, fgList, faint;
    QString hover, pressed, currentLine, selBg;
    QString searchBg, searchFg, bracketBg, warn;
    // 語法高亮（keyword / type / comment / string / function / preprocessor / number）
    QString synKw, synType, synComment, synString, synFunc, synPre, synNum;
};

inline const QHash<QString, Palette>& palettes() {
    static const QHash<QString, Palette> p = {
        { QStringLiteral("Neon Grid"), {
            "#0a0e17", "#0d1420", "#1c2940", "#00e5ff", "#00b8d4", "#ff2d95",
            "#d6e4ff", "#9db4d8", "#7d93b8", "#b8cdf0", "#3d5a80",
            "#15233a", "#1a2c47", "#101a2e", "#234a7d",
            "#3a2a55", "#ffd166", "#1a3a4a", "#ffd166",
            "#00e5ff", "#ff79c6", "#5c6f8a", "#c3f73a", "#ffd166", "#ff9e64", "#bd93f9" } },
        { QStringLiteral("Paper Light"), {
            "#fafafa", "#f0f0f0", "#d0d0d0", "#0066cc", "#0052a3", "#d81b60",
            "#1a1a1a", "#444444", "#555555", "#333333", "#999999",
            "#e3eefc", "#d0e2f7", "#eef4fb", "#bcd6f5",
            "#ffe9a8", "#5a4500", "#cfe6ff", "#b8860b",
            "#0000cc", "#267f99", "#008000", "#a31515", "#795e26", "#af00db", "#098658" } },
        { QStringLiteral("Matrix"), {
            "#050905", "#0a120a", "#143814", "#00ff66", "#00cc52", "#ffaa00",
            "#c8efc8", "#8fbf8f", "#7aa97a", "#b0dcb0", "#3f6f3f",
            "#0f2410", "#143514", "#0c1a0c", "#1d4d28",
            "#2c4d10", "#eaff90", "#15402a", "#ffd166",
            "#00ff66", "#7af0a0", "#3f6f3f", "#eaff90", "#b0dcb0", "#ffaa00", "#00cc52" } },
    };
    return p;
}

inline QStringList themeNames() {
    return { QStringLiteral("Neon Grid"), QStringLiteral("Paper Light"), QStringLiteral("Matrix") };
}

// 編輯器配色（程式內使用；setTheme 時更新，預設 Neon Grid）
inline QString EDITOR_BG        = "#0a0e17";
inline QString EDITOR_FG        = "#d6e4ff";
inline QString ACCENT           = "#00e5ff";   // 主強調色
inline QString ACCENT2          = "#ff2d95";   // 次強調色（書籤等）
inline QString BORDER           = "#1c2940";   // 細邊框/分隔線
inline QString CURRENT_LINE     = "#101a2e";
inline QString LINE_NUM_BG      = "#0d1420";
inline QString LINE_NUM_FG      = "#3d5a80";
inline QString LINE_NUM_ACTIVE  = "#00e5ff";
inline QString BRACKET_MATCH_BG = "#1a3a4a";
inline QString BRACKET_MATCH_FG = "#00e5ff";
inline QString SEARCH_MATCH_BG  = "#3a2a55";
inline QString SEARCH_MATCH_FG  = "#ffd166";
inline QString OCCURRENCE_BG    = "#15233a";   // 游標字詞其他出現處的底色（低調）
inline QString LSP_ERROR        = "#ff2d95";   // 診斷波浪底線：錯誤
inline QString LSP_WARNING      = "#ffd166";   // 診斷波浪底線：警告
inline QString GIT_ADDED        = "#00e5ff";   // gutter：新增行
inline QString GIT_MODIFIED     = "#ffd166";   // gutter：修改行
inline QString GIT_DELETED      = "#ff2d95";   // gutter：刪除標記
// 語法高亮色票（setTheme 時更新）
inline QString SYN_KEYWORD      = "#00e5ff";
inline QString SYN_TYPE         = "#ff79c6";
inline QString SYN_COMMENT      = "#5c6f8a";
inline QString SYN_STRING       = "#c3f73a";
inline QString SYN_FUNCTION     = "#ffd166";
inline QString SYN_PREPROC      = "#ff9e64";
inline QString SYN_NUMBER       = "#bd93f9";

inline QString currentThemeName = QStringLiteral("Neon Grid");

inline void setTheme(const QString& name) {
    const Palette pal = palettes().value(palettes().contains(name)
                                             ? name : QStringLiteral("Neon Grid"));
    currentThemeName = palettes().contains(name) ? name : QStringLiteral("Neon Grid");
    EDITOR_BG = pal.bg;             EDITOR_FG = pal.fg;
    ACCENT = pal.accent;            ACCENT2 = pal.accent2;   BORDER = pal.border;
    CURRENT_LINE = pal.currentLine; LINE_NUM_BG = pal.panel;
    LINE_NUM_FG = pal.faint;        LINE_NUM_ACTIVE = pal.accent;
    BRACKET_MATCH_BG = pal.bracketBg; BRACKET_MATCH_FG = pal.accent;
    SEARCH_MATCH_BG = pal.searchBg; SEARCH_MATCH_FG = pal.searchFg;
    OCCURRENCE_BG = pal.bracketBg;   // 比 hover 明顯，與括號配對同調
    LSP_ERROR = pal.accent2;        LSP_WARNING = pal.warn;
    GIT_ADDED = pal.accent;         GIT_MODIFIED = pal.warn;
    GIT_DELETED = pal.accent2;
    SYN_KEYWORD = pal.synKw;   SYN_TYPE = pal.synType;   SYN_COMMENT = pal.synComment;
    SYN_STRING = pal.synString; SYN_FUNCTION = pal.synFunc; SYN_PREPROC = pal.synPre;
    SYN_NUMBER = pal.synNum;
}

inline QString stylesheet() {
    const Palette pal = palettes().value(currentThemeName);
    QString qss = QStringLiteral(R"QSS(
/* ---------- 全域 ---------- */
QMainWindow, QDialog, QWidget {
    background-color: @bg@;
    color: @fg@;
    font-family: "Segoe UI", "Microsoft JhengHei UI", sans-serif;
    font-size: 10pt;
}

/* ---------- 選單列 ---------- */
QMenuBar {
    background-color: @panel@;
    border-bottom: 1px solid @accent@;
    padding: 2px;
}
QMenuBar::item {
    background: transparent;
    padding: 5px 12px;
    border-radius: 4px;
}
QMenuBar::item:selected {
    background-color: @hover@;
    color: @accent@;
}
QMenu {
    background-color: @panel@;
    border: 1px solid @border@;
    padding: 4px;
}
QMenu::item {
    padding: 6px 28px 6px 16px;
    border-radius: 4px;
}
QMenu::item:selected {
    background-color: @hover@;
    color: @accent@;
}
QMenu::separator {
    height: 1px;
    background: @border@;
    margin: 4px 8px;
}

/* ---------- 工具列 ---------- */
QToolBar {
    background-color: @panel@;
    border: none;
    border-bottom: 1px solid @border@;
    padding: 3px;
    spacing: 3px;
}
QToolBar::separator {
    width: 1px;
    background: @border@;
    margin: 4px 6px;
}
QToolButton {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 8px;
    color: @fgDim@;
}
QToolButton:hover {
    background-color: @hover@;
    border: 1px solid @accent@;
    color: @accent@;
}
QToolButton:pressed {
    background-color: @pressed@;
}

/* ---------- 分頁 ---------- */
QTabWidget::pane {
    border: 1px solid @border@;
    border-top: 2px solid @accent@;
}
QTabBar::tab {
    background: @panel@;
    color: @fgDim2@;
    border: 1px solid @border@;
    border-bottom: none;
    padding: 7px 16px;
    margin-right: 2px;
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
}
QTabBar::tab:selected {
    background: @currentLine@;
    color: @accent@;
    border: 1px solid @accent@;
    border-bottom: none;
}
QTabBar::tab:hover:!selected {
    background: @hover@;
    color: @fgList@;
}
QTabBar::close-button {
    image: none;
    subcontrol-position: right;
}

/* ---------- 輸入框 ---------- */
QLineEdit {
    background-color: @panel@;
    border: 1px solid @border@;
    border-radius: 5px;
    padding: 5px 10px;
    color: @fg@;
    selection-background-color: @accent@;
    selection-color: @bg@;
}
QLineEdit:focus {
    border: 1px solid @accent@;
    background-color: @currentLine@;
}
QLineEdit::placeholder { color: @faint@; }

/* ---------- 按鈕 ---------- */
QPushButton {
    background-color: @hover@;
    border: 1px solid @accent@;
    border-radius: 5px;
    padding: 6px 18px;
    color: @accent@;
    font-weight: 600;
}
QPushButton:hover {
    background-color: @accent@;
    color: @bg@;
}
QPushButton:pressed {
    background-color: @accentDim@;
}
QPushButton:disabled {
    border-color: @border@;
    color: @faint@;
    background-color: @panel@;
}

/* ---------- 下拉選單 / 勾選框 ---------- */
QComboBox {
    background-color: @panel@;
    border: 1px solid @border@;
    border-radius: 5px;
    padding: 4px 10px;
    color: @fg@;
    min-width: 70px;
}
QComboBox:hover { border: 1px solid @accent@; }
QComboBox::drop-down { border: none; width: 22px; }
QComboBox QAbstractItemView {
    background-color: @panel@;
    border: 1px solid @accent@;
    selection-background-color: @hover@;
    selection-color: @accent@;
}
QCheckBox { spacing: 6px; color: @fgDim@; }
QCheckBox::indicator {
    width: 15px; height: 15px;
    border: 1px solid @border@;
    border-radius: 3px;
    background: @panel@;
}
QCheckBox::indicator:hover { border-color: @accent@; }
QCheckBox::indicator:checked {
    background-color: @accent@;
    border-color: @accent@;
}

/* ---------- 清單 ---------- */
QListWidget {
    background-color: @bg@;
    border: 1px solid @border@;
    border-radius: 4px;
    color: @fgList@;
    outline: none;
}
QListWidget::item { padding: 4px 8px; border-radius: 3px; }
QListWidget::item:hover { background-color: @currentLine@; }
QListWidget::item:selected {
    background-color: @hover@;
    color: @accent@;
    border-left: 2px solid @accent2@;
}

/* ---------- Dock ---------- */
QDockWidget {
    color: @accent@;
    titlebar-close-icon: none;
    font-weight: 600;
}
QDockWidget::title {
    background: @panel@;
    border-bottom: 1px solid @border@;
    padding: 6px 10px;
    text-align: left;
}

/* ---------- 狀態列 ---------- */
QStatusBar {
    background-color: @panel@;
    border-top: 1px solid @accent@;
    color: @fgDim2@;
}
QStatusBar QLabel {
    color: @fgDim2@;
    padding: 2px 10px;
    border-right: 1px solid @border@;
    background: transparent;
}

/* ---------- 捲軸 ---------- */
QScrollBar:vertical {
    background: @bg@;
    width: 11px;
    margin: 0;
}
QScrollBar::handle:vertical {
    background: @border@;
    border-radius: 5px;
    min-height: 30px;
}
QScrollBar::handle:vertical:hover { background: @accent@; }
QScrollBar:horizontal {
    background: @bg@;
    height: 11px;
    margin: 0;
}
QScrollBar::handle:horizontal {
    background: @border@;
    border-radius: 5px;
    min-width: 30px;
}
QScrollBar::handle:horizontal:hover { background: @accent@; }
QScrollBar::add-line, QScrollBar::sub-line { width: 0; height: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }

/* ---------- 編輯器本體 ---------- */
QPlainTextEdit {
    background-color: @bg@;
    color: @fg@;
    border: none;
    selection-background-color: @selBg@;
    selection-color: @fg@;
}

/* ---------- 訊息框 / 標籤 ---------- */
QLabel { background: transparent; }
QMessageBox QPushButton { min-width: 70px; }
QInputDialog QLineEdit { min-width: 200px; }

/* ---------- 工具提示 ---------- */
QToolTip {
    background-color: @panel@;
    color: @accent@;
    border: 1px solid @accent@;
    padding: 4px 8px;
    border-radius: 4px;
}
)QSS");
    qss.replace("@bg@", pal.bg);
    qss.replace("@panel@", pal.panel);
    qss.replace("@border@", pal.border);
    qss.replace("@accent@", pal.accent);
    qss.replace("@accentDim@", pal.accentDim);
    qss.replace("@accent2@", pal.accent2);
    qss.replace("@fg@", pal.fg);
    qss.replace("@fgDim@", pal.fgDim);
    qss.replace("@fgDim2@", pal.fgDim2);
    qss.replace("@fgList@", pal.fgList);
    qss.replace("@faint@", pal.faint);
    qss.replace("@hover@", pal.hover);
    qss.replace("@pressed@", pal.pressed);
    qss.replace("@currentLine@", pal.currentLine);
    qss.replace("@selBg@", pal.selBg);
    qss.replace("@searchBg@", pal.searchBg);
    qss.replace("@searchFg@", pal.searchFg);
    return qss;
}

} // namespace Theme
