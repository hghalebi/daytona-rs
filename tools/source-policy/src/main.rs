use std::{io::Write, path::Path};
use syn::visit::Visit;

#[derive(Debug, thiserror::Error)]
enum PolicyError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] syn::Error),
    #[error("source violates SDK policy: {0}")]
    Violation(String),
    #[cfg(test)]
    #[error("negative policy fixture was accepted: {0}")]
    Fixture(&'static str),
}
struct SourceFile<'a>(&'a Path);
struct CheckedSource;
struct Policy {
    violations: Vec<&'static str>,
}
fn forbidden(name: &str) -> bool {
    name.starts_with("unwrap")
        || name.starts_with("expect")
        || name.starts_with("assert")
        || name.starts_with("debug_assert")
        || matches!(
            name,
            "clone"
                | "clone_from"
                | "cloned"
                | "Clone"
                | "Copy"
                | "to_owned"
                | "to_vec"
                | "repeat"
                | "repeat_n"
                | "panic"
                | "todo"
                | "unimplemented"
                | "unreachable"
                | "unsafe"
                | "ok"
        )
}
impl Policy {
    fn error_derive(&mut self, name: &proc_macro2::Ident, attributes: &[syn::Attribute]) {
        if name.to_string().ends_with("Error")
            && !attributes.iter().any(|attribute| {
                attribute.path().is_ident("derive")
                    && match &attribute.meta {
                        syn::Meta::List(list) => list.tokens.to_string().split(',').any(|item| {
                            matches!(item.replace(' ', "").as_str(), "Error" | "thiserror::Error")
                        }),
                        _ => false,
                    }
            })
        {
            self.violations
                .push("error definition missing thiserror derive");
        }
    }
    fn tokens(&mut self, tokens: proc_macro2::TokenStream) {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(id) if forbidden(&id.to_string()) => self
                    .violations
                    .push("forbidden identifier (including macro input)"),
                proc_macro2::TokenTree::Group(group) => self.tokens(group.stream()),
                _ => {}
            }
        }
    }
}
impl<'ast> Visit<'ast> for Policy {
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if node.path().is_ident("allow") || node.path().is_ident("expect") {
            self.violations.push("source-level lint suppression");
        }
        if node.path().is_ident("derive") {
            match &node.meta {
                syn::Meta::List(list) => {
                    for token in list.tokens.to_string().split(',') {
                        let path = token.replace(' ', "");
                        if !matches!(
                            path.as_str(),
                            "Debug"
                                | "PartialEq"
                                | "Eq"
                                | "Serialize"
                                | "Deserialize"
                                | "Error"
                                | "thiserror::Error"
                                | "serde::Serialize"
                                | "serde::Deserialize"
                        ) {
                            self.violations.push("unreviewed derive macro");
                        }
                    }
                }
                _ => self.violations.push("malformed derive attribute"),
            }
        }
        syn::visit::visit_attribute(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.trait_.as_ref().is_some_and(|(_, path, _)| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "Error")
        }) {
            self.violations
                .push("owned errors must derive thiserror::Error");
        }
        syn::visit::visit_item_impl(self, node);
    }
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.error_derive(&node.ident, &node.attrs);
        syn::visit::visit_item_enum(self, node);
    }
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.error_derive(&node.ident, &node.attrs);
        syn::visit::visit_item_struct(self, node);
    }
    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        self.violations.push("unchecked indexing/slicing");
        syn::visit::visit_expr_index(self, node);
    }
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(
            node.op,
            syn::BinOp::Add(_)
                | syn::BinOp::Sub(_)
                | syn::BinOp::Mul(_)
                | syn::BinOp::Div(_)
                | syn::BinOp::Rem(_)
                | syn::BinOp::Shl(_)
                | syn::BinOp::Shr(_)
                | syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) {
            self.violations.push("unchecked arithmetic");
        }
        syn::visit::visit_expr_binary(self, node);
    }
    fn visit_local(&mut self, node: &'ast syn::Local) {
        if matches!(node.pat, syn::Pat::Wild(_)) {
            self.violations.push("explicitly discarded result");
        }
        syn::visit::visit_local(self, node);
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = node.path.segments.last().map(|s| s.ident.to_string());
        if name.as_deref() == Some("vec") && node.tokens.to_string().contains(';') {
            self.violations.push("vec repetition generates cloning");
        }
        // Closed set of reviewed macros. New code-generating macros require a
        // policy change; macro_rules bodies still undergo token inspection.
        if !name.as_deref().is_some_and(|s| {
            matches!(
                s,
                "format"
                    | "write"
                    | "writeln"
                    | "vec"
                    | "matches"
                    | "include_str"
                    | "env"
                    | "join"
                    | "pin"
                    | "select"
            )
        }) {
            self.violations.push("unreviewed macro");
        }
        syn::visit::visit_macro(self, node);
    }
}
fn check(source: &str) -> Result<CheckedSource, PolicyError> {
    let ast = syn::parse_file(source)?;
    let mut policy = Policy {
        violations: Vec::new(),
    };
    policy.tokens(
        source
            .parse::<proc_macro2::TokenStream>()
            .map_err(|e| PolicyError::Violation(e.to_string()))?,
    );
    policy.visit_file(&ast);
    if policy.violations.is_empty() {
        Ok(CheckedSource)
    } else {
        Err(PolicyError::Violation(policy.violations.join(", ")))
    }
}
fn check_markdown(source: &str) -> Result<(), PolicyError> {
    let mut fenced = false;
    let mut rust = false;
    let mut snippet = String::new();
    for line in source.lines() {
        if let Some(language) = line.trim().strip_prefix("```") {
            if fenced {
                if rust {
                    check(&format!("fn documentation_example() {{\n{snippet}\n}}"))?;
                }
                fenced = false;
                rust = false;
                snippet.clear();
            } else {
                fenced = true;
                rust = language
                    .split(',')
                    .any(|tag| matches!(tag.trim(), "rust" | "no_run" | "compile_fail"));
            }
        } else if fenced && rust {
            let line = match line.strip_prefix("# ") {
                Some(hidden) => hidden,
                None => line,
            };
            snippet.push_str(line);
            snippet.push('\n');
        }
    }
    if fenced && rust {
        return Err(PolicyError::Violation(String::from(
            "unterminated Rust documentation fence",
        )));
    }
    Ok(())
}
fn inspect(file: SourceFile<'_>) -> Result<(), PolicyError> {
    let source = std::fs::read_to_string(file.0)?;
    check(&source).map_err(|e| PolicyError::Violation(format!("{}: {e}", file.0.display())))?;
    Ok(())
}
fn walk(path: &Path) -> Result<(), PolicyError> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_symlink() {
            return Err(PolicyError::Violation(format!(
                "symlink in source tree: {}",
                path.display()
            )));
        }
        if kind.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != ".git" {
                walk(&path)?;
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            inspect(SourceFile(&path))?;
        } else if path.extension().is_some_and(|e| e == "md") {
            check_markdown(&std::fs::read_to_string(&path)?)
                .map_err(|e| PolicyError::Violation(format!("{}: {e}", path.display())))?;
        }
    }
    Ok(())
}
fn main() -> Result<(), PolicyError> {
    let root = match std::env::args_os().nth(1) {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()?,
    };
    walk(&root)?;
    writeln!(std::io::stdout().lock(), "SDK source policy passed")?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_rejects_negative_fixtures() -> Result<(), PolicyError> {
        for source in [
            "fn bad() { panic!(\"bad\"); }",
            "fn bad() { x.unwrap_or_default(); }",
            "fn bad() { x.expect(\"bad\"); }",
            "#[derive(Clone)] struct Bad;",
            "#[derive(Copy)] struct Bad;",
            "impl Clone for Bad {}",
            "fn bad() { Arc::clone(&x); }",
            "fn bad() { Clone::clone(&x); }",
            "fn bad() { x.cloned(); }",
            "fn bad() { x.to_owned(); }",
            "fn bad() { x.ok(); }",
            "fn bad() { let _ = x; }",
            "fn bad() { x[0]; }",
            "fn bad() { x + 1; }",
            "fn bad() { unsafe {} }",
            "fn bad() { assert_eq!(1, 1); }",
            "fn bad() { todo!(); }",
            "fn bad() { unimplemented!(); }",
            "fn bad() { unreachable!(); }",
            "macro_rules! bad { () => { x.clone() }; }",
            "fn bad() { mystery!(); }",
            "fn bad() { vec![String::new(); 5]; }",
            "#[derive(Mystery)] struct Bad;",
            "#[allow(unused_must_use)] fn bad() {}",
            "struct BadError;",
            "impl std::error::Error for Failure {}",
        ] {
            if check(source).is_ok() {
                return Err(PolicyError::Fixture(source));
            }
        }
        if check_markdown("```rust\nx.unwrap();\n```").is_ok() {
            return Err(PolicyError::Fixture("markdown unwrap"));
        }
        check("fn good() -> Result<(), std::io::Error> { Ok(()) }")?;
        Ok(())
    }
}
