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
    fn expansion(&mut self, stream: proc_macro2::TokenStream) {
        use proc_macro2::{Delimiter, TokenTree};
        let mut tokens = stream.into_iter().peekable();
        let mut dollar = false;
        let mut can_index = false;
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Group(group) => {
                    if group.delimiter() == Delimiter::Bracket && can_index {
                        self.violations
                            .push("unchecked indexing in macro expansion");
                    }
                    self.expansion(group.stream());
                    if dollar && tokens.peek().is_some_and(|next| matches!(next, TokenTree::Punct(p) if matches!(p.as_char(), '*' | '+' | '?'))) {
                        drop(tokens.next());
                    }
                    can_index = group.delimiter() == Delimiter::Parenthesis;
                    dollar = false;
                }
                TokenTree::Punct(p) => {
                    if matches!(p.as_char(), '+' | '-' | '*' | '/' | '%') {
                        self.violations
                            .push("unchecked arithmetic in macro expansion");
                    }
                    dollar = p.as_char() == '$';
                    can_index = false;
                }
                TokenTree::Ident(_) => {
                    can_index = true;
                    dollar = false;
                }
                TokenTree::Literal(_) => {
                    can_index = false;
                    dollar = false;
                }
            }
        }
    }
    fn definition(&mut self, stream: proc_macro2::TokenStream) {
        use proc_macro2::TokenTree;
        let tokens: Vec<_> = stream.into_iter().collect();
        for window in tokens.windows(4) {
            if let [
                TokenTree::Group(_),
                TokenTree::Punct(eq),
                TokenTree::Punct(arrow),
                TokenTree::Group(body),
            ] = window
                && eq.as_char() == '='
                && arrow.as_char() == '>'
            {
                self.expansion(body.stream());
            }
        }
    }
    fn tokens(&mut self, tokens: proc_macro2::TokenStream) {
        let mut definition = false;
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(id) if id == "macro_rules" => definition = true,
                proc_macro2::TokenTree::Ident(id) if forbidden(&id.to_string()) => self
                    .violations
                    .push("forbidden identifier (including macro input)"),
                proc_macro2::TokenTree::Group(group) => {
                    if definition {
                        self.definition(group.stream());
                        definition = false;
                    }
                    self.tokens(group.stream());
                }
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
                "macro_rules"
                    | "compile_error"
                    | "run"
                    | "scope"
                    | "ttl"
                    | "execution_timeout"
                    | "__run_options"
                    | "__scope_before"
                    | "format"
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
    check_documentation(
        source
            .parse()
            .map_err(|e: proc_macro2::LexError| PolicyError::Violation(e.to_string()))?,
    )?;
    if policy.violations.is_empty() {
        Ok(CheckedSource)
    } else {
        Err(PolicyError::Violation(policy.violations.join(", ")))
    }
}
// Adjacent doc attributes form one Markdown document. Recursing through token
// groups covers item, method, field, variant, and inner module documentation.
fn check_documentation(tokens: proc_macro2::TokenStream) -> Result<(), PolicyError> {
    let mut tokens = tokens.into_iter().peekable();
    let mut document = String::new();
    while let Some(token) = tokens.next() {
        if matches!(&token, proc_macro2::TokenTree::Punct(p) if p.as_char() == '#') {
            if matches!(tokens.peek(), Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '!')
                && tokens.next().is_none()
            {
                return Err(PolicyError::Violation(String::from(
                    "incomplete inner attribute",
                )));
            }
            if let Some(proc_macro2::TokenTree::Group(group)) = tokens.next()
                && let Ok(syn::Meta::NameValue(meta)) = syn::parse2(group.stream())
                && meta.path.is_ident("doc")
            {
                match meta.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(text),
                        ..
                    }) => {
                        document.push_str(&text.value());
                        document.push('\n');
                    }
                    // Included files are checked independently by walk().
                    syn::Expr::Macro(m) if m.mac.path.is_ident("include_str") => {}
                    _ => {
                        return Err(PolicyError::Violation(String::from(
                            "unsupported generated documentation",
                        )));
                    }
                }
            }
        } else {
            check_markdown(&document)?;
            document.clear();
            if let proc_macro2::TokenTree::Group(group) = token {
                check_documentation(group.stream())?;
            }
        }
    }
    check_markdown(&document)
}

fn check_markdown(source: &str) -> Result<(), PolicyError> {
    use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
    let mut rust = false;
    let mut snippet = String::new();
    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                rust = match &kind {
                    CodeBlockKind::Indented => true,
                    CodeBlockKind::Fenced(info) => {
                        info.trim().is_empty()
                            || info.split(',').any(|tag| {
                                matches!(
                                    tag.trim(),
                                    "rust" | "no_run" | "compile_fail" | "should_panic" | "ignore"
                                ) || tag.trim().starts_with("edition")
                            })
                    }
                };
                if let CodeBlockKind::Fenced(info) = &kind
                    && info
                        .split(',')
                        .any(|tag| tag.trim().starts_with("ignore") || tag.trim() == "should_panic")
                {
                    return Err(PolicyError::Violation(String::from(
                        "ignored or panicking documentation example",
                    )));
                }
            }
            Event::Text(text) if rust => snippet.push_str(&text),
            Event::End(TagEnd::CodeBlock) if rust => {
                let mut visible = String::new();
                for line in snippet.lines() {
                    let line = line.trim_start();
                    let line = match line.strip_prefix("##") {
                        Some(rest) => format!("#{rest}"),
                        None => match line.strip_prefix("# ") {
                            Some(rest) => String::from(rest),
                            None if line == "#" => String::new(),
                            None => String::from(line),
                        },
                    };
                    visible.push_str(&line);
                    visible.push('\n');
                }
                // Complete programs and snippets both occur in Rustdoc.
                if syn::parse_file(&visible).is_ok() {
                    check(&visible)?;
                } else {
                    check(&format!("fn documentation_example() {{\n{visible}\n}}"))?;
                }
                snippet.clear();
                rust = false;
            }
            _ => {}
        }
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
            "macro_rules! bad { ($value:expr) => { $value[0] }; }",
            "macro_rules! bad { ($value:expr) => { $value + 1 }; }",
            "fn bad() { daytona::run!(&client, script.clone()); }",
            "fn bad() { daytona::scope!(&client, work: async |_| x.unwrap()); }",
            "macro_rules! __run_options { () => { value.ok() }; }",
            "macro_rules! helper { () => { inner!(x.clone()) }; }",
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
        for source in [
            "/// ```\n/// x.unwrap();\n/// ```\nfn bad() {}",
            "//! ```rust\n//! x.clone();\n//! ```\nfn bad() {}",
            "/// A method.\n///\n///     x.unwrap();\nfn bad() {}",
            "/// A method.\n///\n/// ~~~rust\n/// x.clone();\n/// ~~~\nfn bad() {}",
            "enum Bad { /// ```rust\n/// panic!(\"bad\");\n/// ```\nValue }",
            "/// ```rust\n/// # x.expect(\"bad\");\n/// ```\nfn bad() {}",
        ] {
            if check(source).is_ok() {
                return Err(PolicyError::Fixture(source));
            }
        }
        for markdown in [
            "```\nx.unwrap();\n```",
            "~~~rust\nx.clone();\n~~~",
            "    x.unwrap();",
            "```rust,ignore\nfn main() {}\n```",
            "```rust\n##[derive(Clone)]\nstruct Bad;\n```",
        ] {
            if check_markdown(markdown).is_ok() {
                return Err(PolicyError::Fixture(markdown));
            }
        }
        check(
            "/// ```\n/// # fn main() -> Result<(), std::io::Error> {\n/// # Ok(())\n/// # }\n/// ```\nfn good() {}",
        )?;
        check_markdown("~~~rust\nlet value = 1;\n~~~")?;
        check_markdown("```text\nx.unwrap();\n```")?;
        check("fn good() -> Result<(), std::io::Error> { Ok(()) }")?;
        Ok(())
    }
}
