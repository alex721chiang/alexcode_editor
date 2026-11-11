use alexcode::core::shell_open::{available_shells, cmd_needs_pushd, explorer_select_arg};
use std::path::Path;

#[test]
fn explorer_select_quotes_paths_with_spaces() {
    assert_eq!(explorer_select_arg(Path::new(r"C:\My Docs\a b.txt")), r#"/select,"C:\My Docs\a b.txt""#);
}

#[test]
fn unc_paths_need_pushd_for_cmd() {
    assert!(cmd_needs_pushd(Path::new(r"\\nas\share\logs")));
    assert!(!cmd_needs_pushd(Path::new(r"C:\work")));
}

#[test]
fn every_platform_offers_a_terminal() {
    assert!(!available_shells().is_empty());
}
