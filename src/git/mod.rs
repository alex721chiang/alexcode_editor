pub mod diff_calc;
pub mod git_blame;
pub mod git_gutter;
pub mod head_content;

pub use diff_calc::{align, change_count, Row, RowType};
pub use git_blame::{parse_porcelain, status_text, LineInfo};
pub use git_gutter::{diff_line_states, ADDED, DELETED_ABOVE, MODIFIED};
pub use head_content::head_lines;
