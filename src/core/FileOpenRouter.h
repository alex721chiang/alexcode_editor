#pragma once
#include <QObject>
#include <QStringList>

// macOS 開檔路由：Finder 雙擊、「打開方式」、拖到 Dock 圖示時，系統以 QFileOpenEvent 送給
// QApplication（不經 argv；程式已在執行時也不會再啟動新行程）。冷啟動時事件可能在主視窗建立前
// 到達，因此先暫存，setReady() 後一次交付，之後的事件立即交付。其他平台不會收到此事件，無副作用。
class FileOpenRouter : public QObject {
    Q_OBJECT
public:
    explicit FileOpenRouter(QObject* parent = nullptr);   // 於 QCoreApplication 安裝事件過濾器
    ~FileOpenRouter() override;
    void setReady();

signals:
    void filesRequested(const QStringList& files);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    bool m_ready = false;
    QStringList m_pending;
};
