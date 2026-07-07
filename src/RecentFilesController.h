#pragma once
#include <QObject>
#include <QStringList>

class QMenu;
class QAction;

// 從 MainWindow 拆出的「最近開啟檔案」子系統：選單項目維護 + AppSettings 存取。
// 建構子直接在傳入的 fileMenu 底下建立「Open Recent」子選單與 15 個動作，
// 並立刻從設定檔載入既有清單、套用到選單上——外部不用再多呼叫一次初始化。
class RecentFilesController : public QObject {
    Q_OBJECT
public:
    explicit RecentFilesController(QMenu* fileMenu, QObject* parent = nullptr);

    void addFile(const QString& filePath);   // 開檔/存檔成功後呼叫：搬到最前、裁到 15 筆、存檔、更新選單

signals:
    void fileOpenRequested(const QString& path);   // 使用者點了某個最近檔案項目

private:
    void updateActions();
    void save() const;
    void load();

    static constexpr int kMaxRecentFiles = 15;
    QMenu* m_menu = nullptr;
    QAction* m_actions[kMaxRecentFiles] = {};
    QStringList m_files;
};
