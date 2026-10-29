//! AI 補全與輔助用戶端 (AICompletionProvider)
//!
//! 與 OpenAI 相容端點通訊（如本地 LM Studio、Ollama、或雲端端點），
//! 支援：
//! - 程式碼自動續寫補全（Ghost Text 建議）
//! - 程式碼解釋 (Explain)
//! - 程式碼重構 (Refactor Diff)

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub auto_trigger: bool,
    pub auto_delay_ms: u32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
            api_key: String::new(),
            model: "google/gemma-4-e4b".to_string(),
            max_tokens: 256,
            temperature: 0.2,
            auto_trigger: false,
            auto_delay_ms: 1200,
        }
    }
}

impl AIConfig {
    pub fn from_json(json_str: &str) -> Self {
        serde_json::from_str(json_str).unwrap_or_default()
    }
}

/// 解析 OpenAI 相容回應的第一個 choice 訊息內容
pub fn parse_content(response_bytes: &[u8]) -> Option<String> {
    let val: Value = serde_json::from_slice(response_bytes).ok()?;
    let content = val["choices"]
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()?;
    Some(content.to_string())
}
