pub mod command_match;
pub mod find_in_files;
pub mod in_file;
pub mod text_refs;

pub use command_match::score as command_match_score;
pub use find_in_files::{search as search_in_files, Match as FileMatch, Stats as SearchStats};
pub use find_in_files::{apply_replace as apply_replace_in_files, plan_replace as plan_replace_in_files, ReplaceHit, ReplaceResult};
pub use text_refs::{find_whole_word, Hit as TextRefHit};
pub use in_file::{find_all, replace_all, replace_range, FindOptions};
