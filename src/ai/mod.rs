pub mod local_completion;
pub mod provider;

pub use local_completion::suggest as suggest_local_completions;
pub use provider::{parse_content as parse_ai_response, AIConfig};
