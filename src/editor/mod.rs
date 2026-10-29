pub mod auto_pair;
pub mod box_select;
pub mod buffer;
pub mod file_tier;
pub mod sticky_scroll;
pub mod text_tools;

pub use auto_pair::{closing_for, decide, is_closer, is_opener, is_quote, is_relevant, should_delete_pair, Action, Decision};
pub use box_select::{compute as compute_box_select, Range as BoxRange};
pub use buffer::{EditorBuffer, Position, Selection};
pub use file_tier::{for_size_mb, Level as FileTierLevel};
pub use sticky_scroll::{headers as sticky_headers, Symbol as StickySymbol};
pub use text_tools::*;
