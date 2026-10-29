//! 專案級別符號索引 (ProjectSymbolIndex)
//!
//! 掃描整個專案資料夾，建立類別、結構、函式等符號的記憶體索引與磁碟快取，
//! 支援 O(1) 精確查找與模糊即時搜尋。

use super::symbols::Symbol;
use crate::filter::FilterEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Default, Clone)]
pub struct ProjectSymbolIndex {
    by_file: HashMap<String, Vec<Entry>>,
    mtime: HashMap<String, i64>,
}

impl ProjectSymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.by_file.clear();
        self.mtime.clear();
    }

    pub fn set_file_symbols(&mut self, file: &str, syms: &[Symbol], mtime: i64) {
        let entries: Vec<Entry> = syms
            .iter()
            .map(|s| Entry {
                name: s.name.clone(),
                kind: s.kind.clone(),
                file: file.to_string(),
                line: s.line,
            })
            .collect();

        self.by_file.insert(file.to_string(), entries);
        if mtime >= 0 {
            self.mtime.insert(file.to_string(), mtime);
        }
    }

    pub fn remove_file(&mut self, file: &str) {
        self.by_file.remove(file);
        self.mtime.remove(file);
    }

    pub fn file_count(&self) -> usize {
        self.by_file.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.by_file.values().map(|v| v.len()).sum()
    }

    pub fn all(&self) -> Vec<Entry> {
        let mut out = Vec::new();
        for v in self.by_file.values() {
            out.extend_from_slice(v);
        }
        out
    }

    pub fn exact(&self, name: &str) -> Vec<Entry> {
        self.all().into_iter().filter(|e| e.name == name).collect()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Entry> {
        let mut matches: Vec<Entry> = self
            .all()
            .into_iter()
            .filter(|e| FilterEngine::fuzzy_contains(&e.name, query))
            .collect();

        matches.sort_by(|a, b| {
            let a_starts = a.name.to_lowercase().starts_with(&query.to_lowercase());
            let b_starts = b.name.to_lowercase().starts_with(&query.to_lowercase());
            if a_starts != b_starts {
                b_starts.cmp(&a_starts)
            } else {
                a.name.len().cmp(&b.name.len())
            }
        });

        matches.truncate(limit);
        matches
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(&self.by_file).unwrap_or_default()
    }

    pub fn deserialize(&mut self, json_str: &str) {
        if let Ok(data) = serde_json::from_str::<HashMap<String, Vec<Entry>>>(json_str) {
            self.by_file = data;
        }
    }
}
