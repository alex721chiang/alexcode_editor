use alexcode::editor::sticky_headers;
use alexcode::index::{extract_symbols, Symbol, SymbolLang};

fn lines(src: &str) -> Vec<String> {
    src.lines().map(str::to_string).collect()
}

/// (kind, name, line, end_line, depth)
fn brief(syms: &[Symbol]) -> Vec<(&str, &str, usize, usize, usize)> {
    syms.iter().map(|s| (s.kind.as_str(), s.name.as_str(), s.line, s.end_line, s.depth)).collect()
}

#[test]
fn rust_items_and_nesting() {
    let src = "\
use std::fmt;

#[derive(Debug)]
pub struct Point {
    x: i32,
}

impl fmt::Display for Point {
    pub(crate) async fn show(&self) -> String {
        if self.x > 0 {
            let p = Point { x: 1 };
            match p.x {
                1 => {}
                _ => {}
            }
        }
        String::new()
    }

    fn one_line(&self) {}
}

mod inner {
    pub enum Kind {
        A,
    }
}
";
    let syms = extract_symbols(&lines(src), SymbolLang::Rust);
    assert_eq!(
        brief(&syms),
        vec![
            ("struct", "Point", 3, 5, 0),
            ("impl", "fmt::Display for Point", 7, 20, 0),
            ("method", "show", 8, 17, 1),
            ("module", "inner", 22, 26, 0),
            ("enum", "Kind", 23, 25, 1),
        ],
        "control flow, struct literals and single-line items are not sticky scopes"
    );
}

#[test]
fn rust_braces_in_strings_comments_and_chars_are_ignored() {
    let src = "\
fn tricky<'a>(s: &'a str) -> char {
    let a = \"{{{\";
    let b = r#\"} \" }\"#;
    let c = '{';
    let d = '\\'';
    // }
    /* }
       } */
    c
}

fn multi_line_signature(
    a: [u8; 4],
) -> u8
where
    u8: Copy,
{
    a[0]
}
";
    let syms = extract_symbols(&lines(src), SymbolLang::Rust);
    assert_eq!(
        brief(&syms),
        vec![("function", "tricky", 0, 9, 0), ("function", "multi_line_signature", 11, 18, 0)]
    );
}

#[test]
fn rust_long_parameter_list_is_still_one_header() {
    let mut src = String::from("#[allow(clippy::too_many_arguments)]\nfn render(\n");
    for i in 0..20 {
        src += &format!("    arg{i}: usize,\n");
    }
    src += ") -> usize {\n    arg0\n}\n";
    let syms = extract_symbols(&lines(&src), SymbolLang::Rust);
    assert_eq!(brief(&syms), vec![("function", "render", 1, 24, 0)]);
}

#[test]
fn c_family_functions_classes_and_control_flow() {
    let src = "\
#include <stdio.h>
#define X {

namespace app {

template <typename T>
class Widget : public Base<T> {
public:
    Widget() : value_(0) {
        for (int i = 0; i < 3; ++i) {
            run([&](int x) {
                value_ += x;
            });
        }
    }

    int get() const
    {
        return value_;
    }
};

static int
helper(int a, int b)
{
    if (a) {
        return b;
    } else {
        return a;
    }
}

struct Config cfg = {
    1,
};

} // namespace app
";
    let syms = extract_symbols(&lines(src), SymbolLang::CFamily);
    assert_eq!(
        brief(&syms),
        vec![
            ("namespace", "app", 3, 36, 0),
            ("class", "Widget", 6, 20, 1),
            ("method", "Widget", 8, 14, 2),
            ("method", "get", 16, 19, 2),
            ("function", "helper", 23, 30, 1),
        ]
    );
}

#[test]
fn javascript_functions_arrows_and_methods() {
    let src = "\
export async function load(url) {
  const r = await fetch(url);
  return r;
}

const add = (a, b) => {
  return a + b;
};

class Store {
  get(key) {
    return this.map[key];
  }
}

items.forEach((it) => {
  console.log(`}${it}`);
});

const config = {
  a: 1,
};
";
    let syms = extract_symbols(&lines(src), SymbolLang::CFamily);
    assert_eq!(
        brief(&syms),
        vec![
            ("function", "load", 0, 3, 0),
            ("function", "add", 5, 7, 0),
            ("class", "Store", 9, 13, 0),
            ("method", "get", 10, 12, 1),
        ]
    );
}

#[test]
fn python_blocks_follow_indentation() {
    let src = "\
import os

class Repo:
    \"\"\"doc\"\"\"

    def __init__(
        self,
        path,
    ):
        self.path = path

    async def fetch(self):
        if self.path:
            return 1

# trailing comment at column 0 does not end the class
        return 2


def main():
    pass
x = 1
";
    let syms = extract_symbols(&lines(src), SymbolLang::Python);
    assert_eq!(
        brief(&syms),
        vec![
            ("class", "Repo", 2, 16, 0),
            ("method", "__init__", 5, 9, 1),
            ("method", "fetch", 11, 16, 1),
            ("function", "main", 19, 20, 0),
        ]
    );
}

#[test]
fn headers_pick_the_enclosing_chain() {
    let src = "\
impl A {
    fn f(&self) {
        let x = 1;
        let y = 2;
    }
    fn g(&self) {
        let z = 3;
    }
}
";
    let syms = extract_symbols(&lines(src), SymbolLang::Rust);
    let at = |line: usize| sticky_headers(&syms, line, 5).iter().map(|s| s.line).collect::<Vec<_>>();
    assert_eq!(at(0), Vec::<usize>::new(), "nothing is pinned while the header itself is visible");
    assert_eq!(at(1), vec![0]);
    assert_eq!(at(3), vec![0, 1]);
    assert_eq!(at(5), vec![0], "f ended on line 4");
    assert_eq!(at(7), vec![0, 5]);
    assert_eq!(at(9), Vec::<usize>::new());
    assert_eq!(sticky_headers(&syms, 3, 1).iter().map(|s| s.line).collect::<Vec<_>>(), vec![1], "max depth keeps the innermost");
}
