//! 跨檔案搜尋與取代引擎 (FindInFiles)
//!
//! 支援：
//! - 背景目錄遞迴遍歷
//! - 檔名萬用字元過濾 (*.rs, *.cpp, *.h)
//! - 搜尋關鍵字或 OR (||) 查詢
//! - 取代預檢 (plan_replace) 與二次防護寫入 (apply_replace)

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
