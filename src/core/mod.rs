pub mod app_state;
pub mod config;
pub mod dir_listing;
pub mod file_watch;
pub mod hot_exit;
pub mod single_instance;
pub mod path_kind;
pub mod shell_open;

pub use config::EditorConfig;
pub use path_kind::{fs_type_for_path, is_network_fs_type, is_network_path, is_unc_path};
pub use shell_open::{available_shells, open_terminal, reveal_in_file_manager, ShellKind};
pub use app_state::{config_dir, AppState};
pub use dir_listing::{list_dir, DirEntryInfo};
pub use file_watch::{stamp_of, FileWatcher, Stamp};
pub use hot_exit::{disk_signature, HotExit, UnsavedTab};
