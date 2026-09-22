#include "PathKind.h"
#include <QDir>
#include <QSet>

#if defined(Q_OS_WIN)
#  include <windows.h>
#elif defined(Q_OS_MACOS)
#  include <sys/param.h>
#  include <sys/ucred.h>
#  include <sys/mount.h>
#elif defined(Q_OS_LINUX)
#  include <QFile>
#endif

namespace PathKind {

bool isUncPath(const QString& path) {
    return path.startsWith(QLatin1String("\\\\")) || path.startsWith(QLatin1String("//"));
}

bool isNetworkFsType(const QString& fsType) {
    static const QSet<QString> kNet = {
        "cifs", "smb", "smb2", "smb3", "smbfs", "nfs", "nfs4", "afpfs", "webdav", "davfs",
        "ncpfs", "9p", "sshfs", "fuse.sshfs", "fuse.rclone", "fuse.smbnetfs",
    };
    return kNet.contains(fsType.toLower());
}

QString fsTypeForPath(const QString& absPath, const QList<QPair<QString, QString>>& mounts) {
    const QString p = QDir::cleanPath(absPath);
    int bestLen = -1;
    QString best;
    for (const auto& m : mounts) {
        const QString mp = QDir::cleanPath(m.first);
        const bool hit = mp == QLatin1String("/")
                      || p == mp
                      || p.startsWith(mp + QLatin1Char('/'));
        if (hit && mp.size() > bestLen) { bestLen = mp.size(); best = m.second; }
    }
    return best;
}

#if defined(Q_OS_LINUX)
// /proc/self/mounts 以 \040 等八進位跳脫空白/Tab/換行/反斜線
static QString unescapeMountField(const QByteArray& f) {
    QByteArray out;
    for (int i = 0; i < f.size(); ++i) {
        if (f[i] == '\\' && i + 3 < f.size()) {
            bool ok = false;
            const int v = f.mid(i + 1, 3).toInt(&ok, 8);
            if (ok) { out.append(char(v)); i += 3; continue; }
        }
        out.append(f[i]);
    }
    return QString::fromUtf8(out);
}
#endif

bool isNetworkPath(const QString& path) {
    if (path.isEmpty()) return false;
    if (isUncPath(path)) return true;
    // 自動偵測不到的網路路徑（DFS、特殊 FUSE…）可用環境變數指定前綴，以 ';' 分隔
    const QString extra = qEnvironmentVariable("ALEXCODE_NETWORK_PREFIXES");
    const QString clean = QDir::cleanPath(path);
    for (const QString& pre : extra.split(QLatin1Char(';'), Qt::SkipEmptyParts)) {
        const QString root = QDir::cleanPath(pre.trimmed());
        if (clean.compare(root, Qt::CaseInsensitive) == 0
            || clean.startsWith(root + QLatin1Char('/'), Qt::CaseInsensitive)) return true;
    }
#if defined(Q_OS_WIN)
    // 對應磁碟機（Z: → \\server\share）：GetDriveTypeW 只查本機對應表，不連線
    if (path.size() >= 2 && path[1] == QLatin1Char(':')) {
        const QString root = path.left(2) + QLatin1Char('\\');
        return GetDriveTypeW(reinterpret_cast<LPCWSTR>(root.utf16())) == DRIVE_REMOTE;
    }
    return false;
#elif defined(Q_OS_MACOS)
    struct statfs* mnts = nullptr;
    const int n = getmntinfo(&mnts, MNT_NOWAIT);     // NOWAIT：不向檔案系統刷新統計，不會卡
    QList<QPair<QString, QString>> mounts;
    for (int i = 0; i < n; ++i)
        mounts.append({QString::fromUtf8(mnts[i].f_mntonname), QString::fromUtf8(mnts[i].f_fstypename)});
    return isNetworkFsType(fsTypeForPath(QDir::cleanPath(path), mounts));
#elif defined(Q_OS_LINUX)
    QFile f(QStringLiteral("/proc/self/mounts"));
    if (!f.open(QIODevice::ReadOnly)) return false;
    QList<QPair<QString, QString>> mounts;
    for (const QByteArray& line : f.readAll().split('\n')) {
        const QList<QByteArray> cols = line.split(' ');
        if (cols.size() < 3) continue;
        mounts.append({unescapeMountField(cols[1]), QString::fromUtf8(cols[2])});
    }
    return isNetworkFsType(fsTypeForPath(QDir::cleanPath(path), mounts));
#else
    return false;
#endif
}

} // namespace PathKind
