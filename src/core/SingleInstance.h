#pragma once
#include <QObject>
#include <QStringList>
#include <memory>

class QLocalServer;
class QLocalSocket;
class QLockFile;

// 單一實例：雙擊關聯檔案時，作業系統會再啟動一個行程並以 argv 帶入路徑。
// 第一個實例成為「主實例」並監聽本機通道（Windows named pipe / Unix domain socket）；
// 之後的「次實例」把檔案路徑交給主實例（開成新分頁並喚醒視窗）後即結束。
//
// 選主以 QLockFile 決定（而非「連不上就自己 listen」）：
//  - 檔案總管多選後按 Enter 會「同時」啟動多個行程，只有拿到鎖的那一個會 listen，
//    不會出現兩個主實例（Windows 允許同名 pipe 有多個 server，單靠 listen 無法互斥）。
//  - 只有持鎖者才清除殘留的 socket 檔（上次當機留下），不會誤刪正在執行的實例的通道。
//  - 主實例當機後，QLockFile 依擁有者 PID 判定為過期，下次啟動可直接接手。
// 協定：次實例送出長度前綴的 JSON {"files":[...]}（UTF-8，路徑原樣保留），主實例回 "OK"。
class SingleInstance : public QObject {
    Q_OBJECT
public:
    // key：通道名稱（亦為鎖檔名）；lockDir：鎖檔所在目錄
    SingleInstance(const QString& key, const QString& lockDir, QObject* parent = nullptr);
    ~SingleInstance() override;

    // 預設 key：依資料目錄與使用者區分（可攜版與安裝版、不同使用者各自獨立）
    static QString defaultKey();
    static QString lockPathFor(const QString& key, const QString& lockDir);

    // 嘗試成為主實例；成功後開始接收次實例傳來的檔案（filesReceived）
    bool tryBecomePrimary();

    // 次實例：把檔案交給主實例（files 可為空：只喚醒視窗）。阻塞至送達並收到確認或逾時；
    // 主實例仍在啟動中會在逾時內重試。回傳 false 時呼叫端應自行開窗，避免檔案遺失。
    bool sendToPrimary(const QStringList& files, int timeoutMs = 5000);

signals:
    void filesReceived(const QStringList& files);

private:
    void handleClient(QLocalSocket* client);

    QString m_key;
    QString m_lockDir;
    std::unique_ptr<QLockFile> m_lock;
    QLocalServer* m_server = nullptr;
};
