#include <gtest/gtest.h>
#include <QDir>
#include "PathKind.h"

TEST(PathKindTest, UncPathsAreNetwork) {
    EXPECT_TRUE(PathKind::isUncPath("\\\\server\\share\\a.txt"));
    EXPECT_TRUE(PathKind::isUncPath("//server/share/a.txt"));
    EXPECT_FALSE(PathKind::isUncPath("C:/work/a.txt"));
    EXPECT_FALSE(PathKind::isUncPath("/home/u/a.txt"));
    EXPECT_TRUE(PathKind::isNetworkPath("//server/share/a.txt"));   // 各平台皆成立，且不做 I/O
}

TEST(PathKindTest, NetworkFsTypes) {
    EXPECT_TRUE(PathKind::isNetworkFsType("cifs"));
    EXPECT_TRUE(PathKind::isNetworkFsType("NFS4"));
    EXPECT_TRUE(PathKind::isNetworkFsType("smbfs"));
    EXPECT_TRUE(PathKind::isNetworkFsType("fuse.sshfs"));
    EXPECT_FALSE(PathKind::isNetworkFsType("ext4"));
    EXPECT_FALSE(PathKind::isNetworkFsType("apfs"));
    EXPECT_FALSE(PathKind::isNetworkFsType(""));
}

TEST(PathKindTest, LongestMountPrefixWins) {
    const QList<QPair<QString, QString>> mounts = {
        {"/", "ext4"}, {"/mnt/share", "cifs"}, {"/mnt/share/local", "ext4"}, {"/mnt/sharex", "nfs"},
    };
    EXPECT_EQ(PathKind::fsTypeForPath("/home/u/a.txt", mounts), "ext4");
    EXPECT_EQ(PathKind::fsTypeForPath("/mnt/share/a.txt", mounts), "cifs");
    EXPECT_EQ(PathKind::fsTypeForPath("/mnt/share", mounts), "cifs");
    EXPECT_EQ(PathKind::fsTypeForPath("/mnt/share/local/b", mounts), "ext4");
    EXPECT_EQ(PathKind::fsTypeForPath("/mnt/sharex/c", mounts), "nfs");   // 不可把 /mnt/share 當 /mnt/sharex 前綴
}

TEST(PathKindTest, LocalTempDirIsNotNetwork) {
    EXPECT_FALSE(PathKind::isNetworkPath(QDir::tempPath()));
    EXPECT_FALSE(PathKind::isNetworkPath(""));
}
