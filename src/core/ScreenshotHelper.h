#pragma once
#include <QObject>

class MainWindow;

// 文件截圖流程（原 MainWindow::*ForShot）：把「自我渲染截圖」用的測試鉤子
// 從主視窗抽離，避免正式 API 被截圖需求污染。
// 以 friend class 存取 MainWindow 內部；正式功能請勿經由此類呼叫。
class ScreenshotHelper : public QObject {
    Q_OBJECT
public:
    explicit ScreenshotHelper(MainWindow* window, QObject* parent = nullptr);

    void openTerminal(const QString& cmd);           // 開終端機並（可選）執行指令
    void openSettings(const QString& outPng);        // 開設定中心並存圖
    void openAbout(const QString& outPng);           // 開關於對話框並存圖
    void openVault(const QString& folder);           // 設資料夾並顯示 backlinks
    void openGraph(const QString& folder);           // 設資料夾並顯示關係圖
    void showMarkdownPreview();                      // 顯示 Markdown 預覽並刷新
    void gotoLine(int line);                         // 跳至指定行（1-based）
    void openCommandPalette(const QString& outPng);  // 開命令面板並截圖
    void openProjectSymbol(const QString& outPng);   // 開專案符號搜尋並截圖
    void findRefs(const QString& name);              // 同步建索引並找引用
    void openCallGraph(const QString& outPng);        // 開目前游標所在函式的 Call Graph
    void openMenu(int index, const QString& outPng);  // 彈出選單列選單以驗證 i18n
    void openGitDiff(const QString& outPng);          // 目前檔 vs HEAD 並排 diff 後截圖

private:
    MainWindow* m_window = nullptr;
};
