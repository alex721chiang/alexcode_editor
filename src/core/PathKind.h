#pragma once
#include <QList>
#include <QPair>
#include <QString>

// 判斷路徑是否位於網路分享（SMB/NFS…）。只查本機資訊（字串、掛載表、磁碟機類型），
// 不對目標路徑做任何 I/O——伺服器離線時也不會卡住。
namespace PathKind {

bool isUncPath(const QString& path);             // \\server\share 或 //server/share
bool isNetworkFsType(const QString& fsType);     // cifs / smbfs / nfs / sshfs …
// 掛載表（掛載點, 檔案系統類型）中，取最長前綴符合者的類型；找不到回空字串
QString fsTypeForPath(const QString& absPath, const QList<QPair<QString, QString>>& mounts);

bool isNetworkPath(const QString& path);         // 平台實作（Windows / Linux / macOS）；
                                                 // 另可用環境變數 ALEXCODE_NETWORK_PREFIXES 指定前綴

} // namespace PathKind
