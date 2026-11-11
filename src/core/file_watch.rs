//! 偵測已開啟的檔案是否被其他程式修改
//!
//! 以背景執行緒定期檢查修改時間與大小（不在 UI 執行緒做磁碟 I/O：網路磁碟的 stat 可能卡住數秒）。
//! 不使用作業系統的檔案通知：網路磁碟（SMB / NFS）上通知並不可靠，輪詢在各種檔案系統行為一致。

use crossbeam_channel::{unbounded, Receiver};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// 檔案在磁碟上的版本：(修改時間, 大小)
pub type Stamp = (SystemTime, u64);

pub fn stamp_of(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

pub struct FileWatcher {
    paths: Arc<Mutex<Vec<PathBuf>>>,
    rx: Receiver<(PathBuf, Option<Stamp>)>,
    stop: Arc<AtomicBool>,
}

impl FileWatcher {
    /// `wake` 在偵測到變更時由背景執行緒呼叫（用來喚醒 UI 重繪）
    pub fn new(interval: Duration, wake: impl Fn() + Send + 'static) -> Self {
        let paths: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded();
        let (p, s) = (paths.clone(), stop.clone());
        std::thread::Builder::new()
            .name("file-watch".into())
            .spawn(move || {
                let mut seen: HashMap<PathBuf, Option<Stamp>> = HashMap::new();
                while !s.load(Ordering::Relaxed) {
                    let list = p.lock().map(|g| g.clone()).unwrap_or_default();
                    seen.retain(|k, _| list.contains(k));
                    let mut changed = false;
                    for path in list {
                        let now = stamp_of(&path);
                        match seen.insert(path.clone(), now) {
                            // 第一次看到：只記錄，不算變更
                            None => {}
                            Some(prev) if prev == now => {}
                            Some(_) => {
                                if tx.send((path, now)).is_err() {
                                    return;
                                }
                                changed = true;
                            }
                        }
                    }
                    if changed {
                        wake();
                    }
                    std::thread::sleep(interval);
                }
            })
            .ok();
        Self { paths, rx, stop }
    }

    /// 設定要監看的檔案清單（取代先前的清單）
    pub fn set_paths(&self, list: Vec<PathBuf>) {
        if let Ok(mut g) = self.paths.lock() {
            if *g != list {
                *g = list;
            }
        }
    }

    /// 取出上次呼叫以來偵測到的變更；`None` 表示檔案已不存在
    pub fn poll(&self) -> Vec<(PathBuf, Option<Stamp>)> {
        self.rx.try_iter().collect()
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
