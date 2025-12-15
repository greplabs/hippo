//! Code parsing using tree-sitter for accurate AST analysis

use crate::error::Result;
use crate::models::{CodeInfo, FunctionInfo};
use std::path::Path;

/// Parse code and extract structural information
pub struct CodeParser {
    // Tree-sitter parsers would be initialized here
    // For now, we use simple regex-based parsing
}

impl Default for CodeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a code file and extract detailed information
    pub fn parse(&self, path: &Path, content: &str) -> Result<CodeInfo> {
        let language = detect_language(path);

        let mut info = CodeInfo {
            language: language.clone(),
            lines_of_code: content.lines().filter(|l| !l.trim().is_empty()).count() as u32,
            imports: Vec::new(),
            exports: Vec::new(),
            functions: Vec::new(),
            dependencies: Vec::new(),
        };

        match language.as_str() {
            "rust" => self.parse_rust(content, &mut info),
            "python" => self.parse_python(content, &mut info),
            "javascript" | "typescript" | "javascript-react" | "typescript-react" => {
                self.parse_javascript(content, &mut info)
            }
            "go" => self.parse_go(content, &mut info),
            _ => {}
        }

        Ok(info)
    }

    /// Parse Rust code
    fn parse_rust(&self, content: &str, info: &mut CodeInfo) {
        let mut current_doc: Vec<String> = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Doc comments
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                current_doc.push(trimmed[3..].trim().to_string());
                continue;
            }

            // Use statements (imports)
            if trimmed.starts_with("use ") {
                let import = trimmed
                    .strip_prefix("use ")
                    .and_then(|s| s.strip_suffix(';'))
                    .unwrap_or(trimmed)
                    .to_string();
                info.imports.push(import.clone());

                // Extract crate name for dependencies
                if let Some(crate_name) = import.split("::").next() {
                    if !["std", "core", "alloc", "self", "super", "crate"].contains(&crate_name) {
                        if !info.dependencies.contains(&crate_name.to_string()) {
                            info.dependencies.push(crate_name.to_string());
                        }
                    }
                }
            }

            // Mod statements
            if trimmed.starts_with("mod ") && !trimmed.contains('{') {
                let module = trimmed
                    .strip_prefix("mod ")
                    .and_then(|s| s.strip_suffix(';'))
                    .unwrap_or("")
                    .to_string();
                if !module.is_empty() {
                    info.imports.push(format!("mod::{}", module));
                }
            }

            // Functions
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
            {
                let is_public = trimmed.starts_with("pub ");
                let is_async = trimmed.contains("async fn ");

                if let Some(name) = extract_rust_fn_name(trimmed) {
                    let doc = if current_doc.is_empty() {
                        None
                    } else {
                        Some(current_doc.join(" "))
                    };

                    info.functions.push(FunctionInfo {
                        name: if is_async {
                            format!("async {}", name)
                        } else {
                            name
                        },
                        line_start: line_num as u32 + 1,
                        line_end: 0, // Would need bracket matching
                        is_public,
                        doc_comment: doc,
                    });
                }
            }

            // Pub exports
            if trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("pub type ")
            {
                if let Some(name) = extract_name_after(
                    trimmed,
                    &["pub struct ", "pub enum ", "pub trait ", "pub type "],
                ) {
                    info.exports.push(name);
                }
            }

            // Clear doc comments if we hit a non-doc line
            if !trimmed.starts_with("///") && !trimmed.starts_with("//!") && !trimmed.is_empty() {
                current_doc.clear();
            }
        }
    }

    /// Parse Python code
    fn parse_python(&self, content: &str, info: &mut CodeInfo) {
        let mut current_doc: Option<String> = None;
        let mut in_docstring = false;
        let mut docstring_content = String::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle docstrings
            if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                if in_docstring {
                    in_docstring = false;
                    current_doc = Some(docstring_content.clone());
                    docstring_content.clear();
                } else {
                    in_docstring = true;
                    let rest = &trimmed[3..];
                    if rest.ends_with("\"\"\"") || rest.ends_with("'''") {
                        current_doc = Some(rest[..rest.len() - 3].to_string());
                        in_docstring = false;
                    }
                }
                continue;
            }

            if in_docstring {
                docstring_content.push_str(trimmed);
                docstring_content.push(' ');
                continue;
            }

            // Import statements
            if trimmed.starts_with("import ") {
                let import = trimmed.strip_prefix("import ").unwrap_or("").to_string();
                info.imports.push(import.clone());

                // Extract package name
                if let Some(pkg) = import
                    .split('.')
                    .next()
                    .map(|s| s.split(' ').next())
                    .flatten()
                {
                    if !info.dependencies.contains(&pkg.to_string()) {
                        info.dependencies.push(pkg.to_string());
                    }
                }
            }

            if trimmed.starts_with("from ") && trimmed.contains(" import ") {
                info.imports.push(trimmed.to_string());

                // Extract package name
                if let Some(pkg) = trimmed
                    .strip_prefix("from ")
                    .and_then(|s| s.split(' ').next())
                {
                    let root_pkg = pkg.split('.').next().unwrap_or(pkg);
                    if !info.dependencies.contains(&root_pkg.to_string()) {
                        info.dependencies.push(root_pkg.to_string());
                    }
                }
            }

            // Function definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let is_async = trimmed.starts_with("async ");
                if let Some(name) = extract_python_fn_name(trimmed) {
                    let is_public = !name.starts_with('_');
                    info.functions.push(FunctionInfo {
                        name: if is_async {
                            format!("async {}", name)
                        } else {
                            name
                        },
                        line_start: line_num as u32 + 1,
                        line_end: 0,
                        is_public,
                        doc_comment: current_doc.take(),
                    });
                }
            }

            // Class definitions (as exports)
            if trimmed.starts_with("class ") {
                if let Some(name) = extract_name_after(trimmed, &["class "]) {
                    let name = name.split('(').next().unwrap_or(&name).to_string();
                    if !name.starts_with('_') {
                        info.exports.push(name);
                    }
                }
            }
        }
    }

    /// Parse JavaScript/TypeScript code
    fn parse_javascript(&self, content: &str, info: &mut CodeInfo) {
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Import statements
            if trimmed.starts_with("import ") {
                info.imports.push(trimmed.to_string());

                // Extract package name from 'from "package"' or 'from "package/path"'
                if let Some(from_idx) = trimmed.find(" from ") {
                    let after_from = &trimmed[from_idx + 7..];
                    let pkg = after_from
                        .trim_matches(|c| c == '"' || c == '\'' || c == ';')
                        .split('/')
                        .next()
                        .unwrap_or("");
                    if !pkg.starts_with('.') && !pkg.is_empty() {
                        if !info.dependencies.contains(&pkg.to_string()) {
                            info.dependencies.push(pkg.to_string());
                        }
                    }
                }
            }

            // Require statements
            if trimmed.contains("require(") {
                info.imports.push(trimmed.to_string());
            }

            // Function declarations
            if trimmed.starts_with("function ") || trimmed.starts_with("async function ") {
                let is_async = trimmed.starts_with("async ");
                if let Some(name) = extract_js_fn_name(trimmed) {
                    info.functions.push(FunctionInfo {
                        name: if is_async {
                            format!("async {}", name)
                        } else {
                            name
                        },
                        line_start: line_num as u32 + 1,
                        line_end: 0,
                        is_public: true,
                        doc_comment: None,
                    });
                }
            }

            // Arrow functions assigned to const/let
            if (trimmed.starts_with("const ")
                || trimmed.starts_with("let ")
                || trimmed.starts_with("export const "))
                && trimmed.contains("=>")
            {
                let is_export = trimmed.starts_with("export ");
                if let Some(name) = extract_arrow_fn_name(trimmed) {
                    info.functions.push(FunctionInfo {
                        name,
                        line_start: line_num as u32 + 1,
                        line_end: 0,
                        is_public: is_export,
                        doc_comment: None,
                    });
                }
            }

            // Exports
            if trimmed.starts_with("export ") {
                if let Some(name) = extract_js_export_name(trimmed) {
                    info.exports.push(name);
                }
            }
        }
    }

    /// Parse Go code
    fn parse_go(&self, content: &str, info: &mut CodeInfo) {
        let mut in_import_block = false;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Import block
            if trimmed == "import (" {
                in_import_block = true;
                continue;
            }
            if in_import_block {
                if trimmed == ")" {
                    in_import_block = false;
                    continue;
                }
                let import = trimmed.trim_matches('"').to_string();
                if !import.is_empty() {
                    info.imports.push(import.clone());
                    // Extract package name
                    if let Some(pkg) = import.split('/').last() {
                        if !info.dependencies.contains(&pkg.to_string()) {
                            info.dependencies.push(pkg.to_string());
                        }
                    }
                }
                continue;
            }

            // Single import
            if trimmed.starts_with("import ") && !trimmed.contains('(') {
                let import = trimmed
                    .strip_prefix("import ")
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
                info.imports.push(import);
            }

            // Function declarations
            if trimmed.starts_with("func ") {
                if let Some(name) = extract_go_fn_name(trimmed) {
                    let is_public = name
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false);
                    info.functions.push(FunctionInfo {
                        name,
                        line_start: line_num as u32 + 1,
                        line_end: 0,
                        is_public,
                        doc_comment: None,
                    });
                }
            }
        }
    }
}

fn detect_language(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "javascript-react",
            "tsx" => "typescript-react",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "cc" | "hpp" => "cpp",
            _ => "unknown",
        })
        .unwrap_or("unknown")
        .to_string()
}

fn extract_rust_fn_name(line: &str) -> Option<String> {
    let start = line.find("fn ")? + 3;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    Some(rest[..end].to_string())
}

fn extract_python_fn_name(line: &str) -> Option<String> {
    let start = if line.contains("async def ") {
        line.find("async def ")? + 10
    } else {
        line.find("def ")? + 4
    };
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(rest[..end].to_string())
}

fn extract_js_fn_name(line: &str) -> Option<String> {
    let start = if line.contains("async function ") {
        line.find("async function ")? + 15
    } else {
        line.find("function ")? + 9
    };
    let rest = &line[start..];
    let end = rest.find(|c: char| c == '(' || c.is_whitespace())?;
    Some(rest[..end].to_string())
}

fn extract_arrow_fn_name(line: &str) -> Option<String> {
    // "const foo = () =>" or "export const foo = async () =>"
    let start = line.find("const ")? + 6;
    let rest = &line[start..];
    let end = rest.find(|c: char| c == ' ' || c == '=')?;
    Some(rest[..end].to_string())
}

fn extract_go_fn_name(line: &str) -> Option<String> {
    let start = line.find("func ")? + 5;
    let rest = &line[start..];

    // Skip receiver if present: func (r Receiver) Name()
    let rest = if rest.starts_with('(') {
        let paren_end = rest.find(')')?;
        rest[paren_end + 1..].trim()
    } else {
        rest
    };

    let end = rest.find('(')?;
    Some(rest[..end].trim().to_string())
}

fn extract_name_after(line: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

fn extract_js_export_name(line: &str) -> Option<String> {
    // "export default Foo" -> "Foo"
    // "export const foo" -> "foo"
    // "export function bar" -> "bar"
    let rest = line.strip_prefix("export ")?.trim();

    if rest.starts_with("default ") {
        let name_part = rest.strip_prefix("default ")?;
        return Some(name_part.split_whitespace().next()?.to_string());
    }

    if rest.starts_with("const ") || rest.starts_with("let ") || rest.starts_with("var ") {
        let name_part = rest.split_whitespace().nth(1)?;
        return Some(name_part.split('=').next()?.trim().to_string());
    }

    if rest.starts_with("function ") {
        return extract_js_fn_name(&format!("function {}", &rest[9..]));
    }

    if rest.starts_with("class ") {
        return extract_name_after(rest, &["class "]);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_fn_extraction() {
        assert_eq!(extract_rust_fn_name("fn hello() {"), Some("hello".into()));
        assert_eq!(
            extract_rust_fn_name("pub fn world(x: i32) -> bool {"),
            Some("world".into())
        );
        assert_eq!(
            extract_rust_fn_name("async fn fetch<T>() {"),
            Some("fetch".into())
        );
    }

    #[test]
    fn test_python_fn_extraction() {
        assert_eq!(extract_python_fn_name("def hello():"), Some("hello".into()));
        assert_eq!(
            extract_python_fn_name("async def fetch(url):"),
            Some("fetch".into())
        );
    }

    #[test]
    fn test_js_fn_extraction() {
        assert_eq!(
            extract_js_fn_name("function hello() {"),
            Some("hello".into())
        );
        assert_eq!(
            extract_js_fn_name("async function fetch() {"),
            Some("fetch".into())
        );
    }
}
