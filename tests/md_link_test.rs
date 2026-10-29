use alexcode::markdown::{extract_refs, normalize_key};

fn targets(text: &str, wiki_only: bool) -> Vec<String> {
    extract_refs(text)
        .into_iter()
        .filter(|r| !wiki_only || r.wiki)
        .map(|r| r.target)
        .collect()
}

#[test]
fn test_plain_wikilink() {
    assert_eq!(targets("see [[Other Note]] here", false), vec!["Other Note"]);
}

#[test]
fn test_wikilink_alias_stripped() {
    assert_eq!(targets("[[Target|display text]]", false), vec!["Target"]);
}

#[test]
fn test_wikilink_heading_stripped() {
    assert_eq!(targets("[[Target#Section]]", false), vec!["Target"]);
    assert_eq!(targets("[[Target#Section|alias]]", false), vec!["Target"]);
}

#[test]
fn test_multiple_wikilinks() {
    assert_eq!(
        targets("[[A]] and [[B]] and [[C]]", false),
        vec!["A", "B", "C"]
    );
}

#[test]
fn test_markdown_local_link() {
    assert_eq!(
        targets("[label](notes/foo.md)", false),
        vec!["notes/foo.md"]
    );
}

#[test]
fn test_ignores_http_links() {
    assert!(targets("[site](https://example.com)", false).is_empty());
    assert!(targets("[mail](mailto:a@b.com)", false).is_empty());
}

#[test]
fn test_ignores_pure_anchor() {
    assert!(targets("[jump](#heading)", false).is_empty());
}

#[test]
fn test_ignores_links_in_fenced_code() {
    let text = "real [[Yes]]\n```\ncode [[No]] block\n```\nafter [[Also]]";
    assert_eq!(targets(text, true), vec!["Yes", "Also"]);
}

#[test]
fn test_ignores_links_in_inline_code() {
    assert_eq!(
        targets("text `[[NotALink]]` and [[Real]]", true),
        vec!["Real"]
    );
}

#[test]
fn test_normalize_key_basename_lower_no_ext() {
    assert_eq!(normalize_key("Notes/My File.md"), "my file");
    assert_eq!(normalize_key("My File"), "my file");
    assert_eq!(normalize_key(r"a\b\C.MD"), "c");
}

#[test]
fn test_empty_and_no_links() {
    assert!(targets("", false).is_empty());
    assert!(targets("just plain text with [not a link]", false).is_empty());
}
