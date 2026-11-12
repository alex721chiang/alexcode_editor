//! 跨檔案搜尋與取代引擎 (FindInFiles)
//!
//! 支援：
//! - 背景目錄遞迴遍歷
//! - 檔名萬用字元過濾 (*.rs, *.cpp, *.h)
//! - 搜尋關鍵字或 OR (||) 查詢
//! - 取代預檢 (plan_replace) 與二次防護寫入 (apply_replace)

use super::in_file::{find_all, replace_all, FindOptions};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Match {
    pub file_path: String,
    pub line: usize, // 1-based
    pub snippet: String,
}

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub files_scanned: usize,
    pub matches: usize,
    pub canceled: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct ReplaceHit {
    pub path: String,
    pub count: usize,
}

#[derive(Debug, Default, Clone)]
pub struct ReplaceResult {
    pub files_written: usize,
    pub replacements: usize,
    pub canceled: bool,
    /// 沒有寫入的檔案與原因（預覽之後內容有變動、已被刪除、寫入失敗）
    pub skipped: Vec<(String, String)>,
}

/// 全專案取代的比對方式與 [`search`] 相同：不分大小寫的純文字（不是正規表示式）
const REPLACE_OPTS: FindOptions = FindOptions { case_sensitive: false, whole_word: false, regex: false };

/// 走訪 `root` 底下符合檔名過濾的檔案（與 [`search`] 相同的排除規則）；回傳是否被取消
fn visit_files(root: &Path, name_filters: &[String], cancel: &Arc<AtomicBool>, mut visit: impl FnMut(&Path)) -> bool {
    let mut dirs_to_visit = vec![root.to_path_buf()];
    while let Some(dir) = dirs_to_visit.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                return true;
            }
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if path.is_dir() {
                if name != ".git" && name != "target" && name != "node_modules" && name != "build" {
                    dirs_to_visit.push(path);
                }
            } else if path.is_file()
                && (name_filters.is_empty() || name_filters.iter().any(|f| name.ends_with(f.trim_start_matches('*'))))
            {
                visit(&path);
            }
        }
    }
    cancel.load(Ordering::Relaxed)
}

/// 讀取可以安全改寫的文字檔：必須是合法的 UTF-8 且不含 NUL。
/// 其他編碼（Big5、UTF-16…）與二進位檔回傳 None：改寫它們會破壞內容。
fn read_replaceable(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().filter(|text| !text.contains('\0'))
}

/// 取代預檢：只計算每個檔案會被取代幾處，不寫入任何東西。結果依路徑排序。
pub fn plan_replace(root: &Path, name_filters: &[String], query: &str, cancel: &Arc<AtomicBool>) -> (Vec<ReplaceHit>, Stats) {
    let mut hits = Vec::new();
    let mut stats = Stats::default();
    if query.is_empty() {
        return (hits, stats);
    }
    stats.canceled = visit_files(root, name_filters, cancel, |path| {
        stats.files_scanned += 1;
        let count = read_replaceable(path).and_then(|text| find_all(&text, query, &REPLACE_OPTS).ok()).map_or(0, |m| m.len());
        if count > 0 {
            stats.matches += count;
            hits.push(ReplaceHit { path: path.to_string_lossy().to_string(), count });
        }
    });
    if stats.canceled {
        hits.clear(); // 不完整的預覽不能拿來執行
    }
    hits.sort_by(|a, b| a.path.cmp(&b.path));
    (hits, stats)
}

/// 依預檢結果寫入。每個檔案寫入前重新讀取並比對命中數：與預檢不同（預覽之後被修改）就略過，
/// 使用者確認過的數量才會真的寫進磁碟。取消時已寫入的檔案不會復原。
pub fn apply_replace(plan: &[ReplaceHit], query: &str, replacement: &str, cancel: &Arc<AtomicBool>) -> ReplaceResult {
    let mut res = ReplaceResult::default();
    for hit in plan {
        if cancel.load(Ordering::Relaxed) {
            res.canceled = true;
            break;
        }
        let path = Path::new(&hit.path);
        let replaced = read_replaceable(path).and_then(|text| replace_all(&text, query, replacement, &REPLACE_OPTS).ok());
        match replaced {
            Some((text, count)) if count == hit.count && count > 0 => match fs::write(path, text) {
                Ok(()) => {
                    res.files_written += 1;
                    res.replacements += count;
                }
                Err(e) => res.skipped.push((hit.path.clone(), format!("寫入失敗：{e}"))),
            },
            Some(_) => res.skipped.push((hit.path.clone(), "預覽之後內容有變動".to_string())),
            None => res.skipped.push((hit.path.clone(), "檔案已無法讀取".to_string())),
        }
    }
    res
}

pub fn search(
    root: &Path,
    name_filters: &[String],
    query: &str,
    cancel: &Arc<AtomicBool>,
    max_results: usize,
) -> (Vec<Match>, Stats) {
    let mut results = Vec::new();
    let mut stats = Stats::default();
    let query_lower = query.to_lowercase();

    let mut dirs_to_visit = vec![root.to_path_buf()];

    while let Some(dir) = dirs_to_visit.pop() {
        if cancel.load(Ordering::Relaxed) {
            stats.canceled = true;
            break;
        }

        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                stats.canceled = true;
                break;
            }

            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                // 排除常見巨大目錄
                if name != ".git" && name != "target" && name != "node_modules" && name != "build" {
                    dirs_to_visit.push(path);
                }
            } else if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();

                // 檔名過濾
                let matches_filter = if name_filters.is_empty() {
                    true
                } else {
                    name_filters.iter().any(|filter| {
                        let pat = filter.trim_start_matches('*');
                        name.ends_with(pat)
                    })
                };

                if !matches_filter {
                    continue;
                }

                stats.files_scanned += 1;

                if let Ok(content) = fs::read_to_string(&path) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&query_lower) {
                            stats.matches += 1;
                            let trimmed_snippet = if line.trim().chars().count() > 100 {
                                let mut s: String = line.trim().chars().take(100).collect();
                                s.push('…');
                                s
                            } else {
                                line.trim().to_string()
                            };

                            results.push(Match {
                                file_path: path.to_string_lossy().to_string(),
                                line: line_idx + 1,
                                snippet: trimmed_snippet,
                            });

                            if results.len() >= max_results {
                                stats.truncated = true;
                                return (results, stats);
                            }
                        }
                    }
                }
            }
        }
    }

    (results, stats)
}
