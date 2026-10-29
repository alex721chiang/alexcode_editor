//! 網路路徑判斷邏輯 (PathKind)
//!
//! 判斷路徑是否位於網路共享（SMB / NFS 等）。只查詢本機資訊（字串前綴、掛載表、磁碟機類型），
//! 不對遠端目標路徑發起任何阻塞式網路 I/O，以避免在遠端伺服器離線時造成主執行緒凍結或未回應。

pub fn is_unc_path(path: &str) -> bool {
    path.starts_with(r"\\") || path.starts_with("//")
}

pub fn is_network_fs_type(fs_type: &str) -> bool {
    let lower = fs_type.to_lowercase();
    matches!(
        lower.as_str(),
        "cifs" | "smbfs" | "nfs" | "nfs4" | "sshfs" | "davfs" | "fuse.sshfs" | "afpfs"
    )
}

/// 在掛載表 (mount_point, fs_type) 中，取最長前綴相符者的檔案系統類型
pub fn fs_type_for_path(abs_path: &str, mounts: &[(String, String)]) -> String {
    let mut best_prefix_len = 0;
    let mut best_type = String::new();

    for (mount_point, fs_type) in mounts {
        if abs_path.starts_with(mount_point) && mount_point.len() > best_prefix_len {
            best_prefix_len = mount_point.len();
            best_type = fs_type.clone();
        }
    }

    best_type
}

/// 判斷路徑是否為網路共享路徑
pub fn is_network_path(path: &str) -> bool {
    if is_unc_path(path) {
        return true;
    }

    // 檢查環境變數自訂前綴 ALEXCODE_NETWORK_PREFIXES
    if let Ok(prefixes) = std::env::var("ALEXCODE_NETWORK_PREFIXES") {
        for prefix in prefixes.split(';') {
            let p = prefix.trim();
            if !p.is_empty() && path.starts_with(p) {
                return true;
            }
        }
    }

    false
}
