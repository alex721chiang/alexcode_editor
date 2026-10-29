use alexcode::lsp::*;
use serde_json::json;

#[test]
fn test_encodes_content_length_header() {
    let out = frame(&json!({"jsonrpc": "2.0", "method": "x"}));
    let s = String::from_utf8_lossy(&out);
    assert!(s.starts_with("Content-Length: "));
    let header_end = s.find("\r\n\r\n").unwrap();
    let len: usize = s[16..header_end].parse().unwrap();
    assert_eq!(len, out.len() - header_end - 4);
}

#[test]
fn test_parses_whole_message() {
    let mut r = Reader::new();
    r.feed(&frame(&json!({"id": 1, "method": "test"})));
    let msgs = r.take_messages();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["method"].as_str().unwrap(), "test");
}

#[test]
fn test_handles_chunk_split_mid_header_and_mid_body() {
    let mut r = Reader::new();
    let full = frame(&json!({"id": 7, "method": "split"}));
    r.feed(&full[0..5]);
    assert!(r.take_messages().is_empty());
    r.feed(&full[5..full.len() - 5]);
    assert!(r.take_messages().is_empty());
    r.feed(&full[full.len() - 5..]);
    let msgs = r.take_messages();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["id"].as_i64().unwrap(), 7);
}

#[test]
fn test_parses_two_messages_in_one_chunk() {
    let mut r = Reader::new();
    let mut chunk = frame(&json!({"id": 1}));
    chunk.extend_from_slice(&frame(&json!({"id": 2})));
    r.feed(&chunk);
    let msgs = r.take_messages();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[1]["id"].as_i64().unwrap(), 2);
}

#[test]
fn test_ignores_extra_headers_and_case_insensitive_content_length() {
    let mut r = Reader::new();
    let body = r#"{"id":3}"#;
    let raw = format!(
        "content-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    r.feed(raw.as_bytes());
    let msgs = r.take_messages();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["id"].as_i64().unwrap(), 3);
}

#[test]
fn test_uri_round_trip() {
    let path = "/tmp/my project/main.cpp";
    let uri = uri_from_path(path);
    assert!(uri.starts_with("file://"));
    assert_eq!(path_from_uri(&uri).unwrap(), path);
}

#[test]
fn test_non_file_uri_returns_none() {
    assert_eq!(path_from_uri("https://example.com/x"), None);
}

#[test]
fn test_parses_diagnostics() {
    let arr = json!([
      {"range": {"start": {"line": 2, "character": 4}, "end": {"line": 2, "character": 9}},
       "severity": 2, "message": "unused variable", "source": "clangd"}
    ]);
    let diags = parse_diagnostics(arr.as_array().unwrap());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].start_line, 2);
    assert_eq!(diags[0].start_char, 4);
    assert_eq!(diags[0].end_char, 9);
    assert_eq!(diags[0].severity, 2);
    assert_eq!(diags[0].message, "unused variable");
    assert_eq!(diags[0].source, "clangd");
}

#[test]
fn test_accepts_item_array_and_completion_list() {
    let as_array = json!([{"label": "foo"}, {"label": "bar", "insertText": "bar()"}]);
    assert_eq!(extract_completions(&as_array, 50), vec!["foo", "bar()"]);

    let as_list = json!({"isIncomplete": false, "items": [{"label": "baz"}]});
    assert_eq!(extract_completions(&as_list, 50), vec!["baz"]);
}

#[test]
fn test_strips_snippet_placeholders() {
    assert_eq!(strip_snippet_placeholders("foo(${1:int x}, $0)"), "foo(int x, )");
    assert_eq!(strip_snippet_placeholders("name${2}"), "name");
}

#[test]
fn test_accepts_definition_location() {
    let val = json!({
        "uri": "file:///tmp/a.h",
        "range": {"start": {"line": 5, "character": 2}, "end": {"line": 5, "character": 8}}
    });
    let (p, l, c) = extract_definition(&val).unwrap();
    assert_eq!(p, "/tmp/a.h");
    assert_eq!(l, 5);
    assert_eq!(c, 2);
}

#[test]
fn test_extracts_hover_text() {
    let val = json!({"contents": "plain text"});
    assert_eq!(extract_hover_text(&val, 1200), "plain text");
}

#[test]
fn test_applies_multiple_edits_against_original_positions() {
    let text = "foo bar\nfoo baz\n";
    let edits = vec![
        TextEdit {
            start_line: 0,
            start_char: 0,
            end_line: 0,
            end_char: 3,
            new_text: "renamed".to_string(),
        },
        TextEdit {
            start_line: 1,
            start_char: 0,
            end_line: 1,
            end_char: 3,
            new_text: "renamed".to_string(),
        },
    ];
    assert_eq!(apply_text_edits(text, &edits), "renamed bar\nrenamed baz\n");
}

#[test]
fn test_incremental_edit_computation() {
    let old_text = "int foo = 1;\nint bar;\n";
    let new_text = "int foo = 42;\nint bar;\n";
    let edit = compute_incremental_edit(old_text, new_text).unwrap();
    assert_eq!(edit.start_line, 0);
    assert_eq!(apply_text_edits(old_text, &[edit]), new_text);
}
