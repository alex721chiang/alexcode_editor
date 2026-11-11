pub mod config;
pub mod path_kind;
pub mod shell_open;

pub use config::EditorConfig;
pub use path_kind::{fs_type_for_path, is_network_fs_type, is_network_path, is_unc_path};
pub use shell_open::{available_shells, open_terminal, reveal_in_file_manager, ShellKind};
