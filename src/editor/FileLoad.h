#pragma once
#include <QString>
#include <QByteArray>

// 讀檔 + 編碼/換行偵測（原 MainWindow::openFileByPath 內嵌邏輯抽出）。
// 純函式、不碰 GUI，可安全地在背景執行緒（QtConcurrent）呼叫——
// 網路分享上的檔案讀取因此不必卡住主執行緒。
namespace FileLoad {

struct Result {
    bool ok = false;
    QString error;          // ok == false 時的錯誤訊息
    qint64 bytes = 0;       // 原始位元組數（大檔分級用）
    QString text;           // 已解碼、換行統一為 \n
    QString encoding;       // "UTF-8" / "Big5"
    QString eol;            // "CRLF" / "LF"
};

Result decode(const QByteArray& raw);   // UTF-8 驗證失敗 → 嘗試 Big5
Result load(const QString& path);

} // namespace FileLoad
