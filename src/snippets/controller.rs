//! 程式碼樣板 (Snippets) 管理控制器
//!
//! 支援 trigger + Tab 展開，跟隨目前游標縮排，
//! 並讀取/寫入 `alexcode-snippets.json` 設定檔。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetDef {
    pub trigger: String,
    pub language: String,
    pub body: String,
}

#[derive(Debug, Default, Clone)]
pub struct SnippetsController {
    snippets: Vec<SnippetDef>,
}

impl SnippetsController {
    pub fn new() -> Self {
        let mut ctrl = Self {
            snippets: Vec::new(),
        };
        ctrl.load_defaults();
        ctrl
    }

    pub fn load_defaults(&mut self) {
        self.snippets = vec![
            SnippetDef {
                trigger: "fn".to_string(),
                language: "rust".to_string(),
                body: "fn ${1:name}(${2}) -> ${3:()} {\n    ${0}\n}".to_string(),
            },
            SnippetDef {
                trigger: "struct".to_string(),
                language: "rust".to_string(),
                body: "#[derive(Debug, Clone)]\npub struct ${1:Name} {\n    ${0}\n}".to_string(),
            },
            SnippetDef {
                trigger: "for".to_string(),
                language: "rust".to_string(),
                body: "for ${1:item} in ${2:iter} {\n    ${0}\n}".to_string(),
            },
            SnippetDef {
                trigger: "main".to_string(),
                language: "cpp".to_string(),
                body: "int main(int argc, char* argv[]) {\n    ${0}\n    return 0;\n}".to_string(),
            },
        ];
    }

    pub fn find_snippet(&self, trigger: &str, language: &str) -> Option<&SnippetDef> {
        self.snippets.iter().find(|s| {
            s.trigger == trigger && (s.language == language || s.language == "*" || s.language.is_empty())
        })
    }

    pub fn all_snippets(&self) -> &[SnippetDef] {
        &self.snippets
    }
}
