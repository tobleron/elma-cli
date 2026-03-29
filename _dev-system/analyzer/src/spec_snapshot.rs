use crate::discovery;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use syn::visit::Visit;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FunctionDetail {
    pub name: String,
    pub signature: String,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecSnapshot {
    pub path: String,
    pub functions: Vec<FunctionDetail>,
    pub fingerprint: String,
}

impl SpecSnapshot {
    pub fn from_content(path: &str, content: &str) -> Self {
        let normalized = normalize_path(path);
        let ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let functions = parse_functions(content, &ext);
        let fingerprint = fingerprint_functions(&functions);
        SpecSnapshot {
            path: normalized,
            functions,
            fingerprint,
        }
    }
}

pub fn build_snapshots(
    registry: &HashMap<String, discovery::RegistryEntry>,
) -> HashMap<String, SpecSnapshot> {
    let mut map = HashMap::new();
    for (path, (_, content, _, _, _, _)) in registry {
        map.insert(path.clone(), SpecSnapshot::from_content(path, content));
    }
    map
}

fn parse_functions(content: &str, ext: &str) -> Vec<FunctionDetail> {
    match ext {
        "rs" => parse_rust_functions(content),
        _ => Vec::new(),
    }
}

fn parse_rust_functions(content: &str) -> Vec<FunctionDetail> {
    if let Ok(file) = syn::parse_file(content) {
        struct FnVisitor<'a> {
            source: &'a str,
            functions: Vec<FunctionDetail>,
        }

        impl<'a> FnVisitor<'a> {
            fn push_signature(&mut self, name: String, line: usize) {
                let start = line_start_offset(self.source, line);
                self.functions.push(FunctionDetail {
                    name,
                    signature: extract_line(self.source, start),
                    line,
                });
            }
        }

        impl<'ast, 'a> Visit<'ast> for FnVisitor<'a> {
            fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
                let name = i.sig.ident.to_string();
                let line = i.sig.fn_token.span.start().line;
                self.push_signature(name, line);
                syn::visit::visit_item_fn(self, i);
            }

            fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
                let name = i.sig.ident.to_string();
                let line = i.sig.fn_token.span.start().line;
                self.push_signature(name, line);
                syn::visit::visit_impl_item_fn(self, i);
            }
        }

        let mut visitor = FnVisitor {
            source: content,
            functions: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.functions.sort_by_key(|f| f.line);
        visitor
            .functions
            .dedup_by(|a, b| a.name == b.name && a.signature == b.signature && a.line == b.line);
        return visitor.functions;
    }

    // Fallback to regex if syn fails
    let mut functions = Vec::new();
    let regex = Regex::new(r"\b(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    for capture in regex.captures_iter(content) {
        let name = capture[1].to_string();
        if let Some(mat) = capture.get(0) {
            let start = mat.start();
            let line = line_number_at(content, start);
            let signature = extract_line(content, start);
            functions.push(FunctionDetail {
                name,
                signature,
                line,
            });
        }
    }
    functions.sort_by_key(|f| f.line);
    functions
}

fn fingerprint_functions(functions: &[FunctionDetail]) -> String {
    let mut hasher = Sha256::new();
    for func in functions {
        hasher.update(func.name.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn line_number_at(content: &str, pos: usize) -> usize {
    content[..pos].chars().filter(|&c| c == '\n').count() + 1
}

fn line_start_offset(content: &str, line_1_based: usize) -> usize {
    if line_1_based <= 1 {
        return 0;
    }

    let mut line = 1usize;
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == line_1_based {
                return idx + 1;
            }
        }
    }
    0
}

fn extract_line(content: &str, start: usize) -> String {
    let snippet = &content[start..];
    let end = snippet.find('\n').unwrap_or(snippet.len());
    snippet[..end].trim().to_string()
}

fn normalize_path(path: &str) -> String {
    Path::new(path)
        .strip_prefix("../../")
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}
