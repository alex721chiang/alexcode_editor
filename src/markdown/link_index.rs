//! Markdown 筆記連結索引 (MarkdownLinkIndex)
//!
//! 掃描整個 vault（資料夾）下所有 .md 檔案，建立雙向連結關係圖：
//! - 每個檔案的對外連結 (out_links)
//! - 反向連結 (backlinks)
//! - 全域關聯邊集 (edges)

use super::md_link::{extract_refs, normalize_key};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MarkdownLinkIndex {
    files: Vec<String>,
    key_to_file: HashMap<String, String>,
    raw_refs: HashMap<String, Vec<String>>,
    out_links: HashMap<String, Vec<String>>,
    back_links: HashMap<String, Vec<String>>,
}

impl MarkdownLinkIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 從記憶體中的檔案對應表建立索引（純邏輯，便於單元測試）
    pub fn build_from_contents(&mut self, file_to_content: &HashMap<String, String>) {
        self.files.clear();
        self.key_to_file.clear();
        self.raw_refs.clear();
        self.out_links.clear();
        self.back_links.clear();

        for (file, content) in file_to_content {
            self.files.push(file.clone());
            let key = normalize_key(file);
            self.key_to_file.insert(key, file.clone());

            let refs = extract_refs(content);
            let targets: Vec<String> = refs.into_iter().map(|r| r.target).collect();
            self.raw_refs.insert(file.clone(), targets);
        }

        self.files.sort();
        self.finalize();
    }

    fn finalize(&mut self) {
        for (from_file, targets) in &self.raw_refs {
            for target in targets {
                if let Some(to_file) = self.resolve(target) {
                    if to_file != *from_file {
                        let outs = self.out_links.entry(from_file.clone()).or_default();
                        if !outs.contains(&to_file) {
                            outs.push(to_file.clone());
                        }

                        let backs = self.back_links.entry(to_file.clone()).or_default();
                        if !backs.contains(from_file) {
                            backs.push(from_file.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn out_links(&self, file: &str) -> Vec<String> {
        self.out_links.get(file).cloned().unwrap_or_default()
    }

    pub fn backlinks(&self, file: &str) -> Vec<String> {
        self.back_links.get(file).cloned().unwrap_or_default()
    }

    /// 解析單一目標（wikilink 或相對檔名）為絕對檔案識別路徑
    pub fn resolve(&self, target: &str) -> Option<String> {
        let key = normalize_key(target);
        self.key_to_file.get(&key).cloned()
    }

    /// 取得所有已解析且去重的有向邊 (from, to)
    pub fn edges(&self) -> Vec<(String, String)> {
        let mut edges = Vec::new();
        for (from, outs) in &self.out_links {
            for to in outs {
                edges.push((from.clone(), to.clone()));
            }
        }
        edges.sort();
        edges
    }
}
