//! 語法樹符號分類 (TsSymbols)
//!
//! 供 Go to Symbol (Ctrl+Shift+O)、專案符號索引 (Ctrl+T) 與麵包屑 (Breadcrumb) 使用。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub end_line: usize,
    pub depth: usize,
}

pub fn symbol_kind(type_str: &str) -> Option<&'static str> {
    match type_str {
        "function_definition" | "function_declaration" | "generator_function_declaration" => {
            Some("function")
        }
        "method_definition" => Some("method"),
        "class_specifier" | "class_definition" | "class_declaration" => Some("class"),
        "struct_specifier" => Some("struct"),
        "union_specifier" => Some("union"),
        "enum_specifier" => Some("enum"),
        "namespace_definition" => Some("namespace"),
        _ => None,
    }
}

pub fn is_symbol_node(type_str: &str) -> bool {
    symbol_kind(type_str).is_some()
}

pub fn is_call_node(type_str: &str) -> bool {
    type_str == "call_expression" || type_str == "call"
}
