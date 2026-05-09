//! @efficiency-role: storage-index
//!
//! Persistent offline document/code index with citations.
//! Supports Rust source file indexing and search.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A single indexed entry representing a code symbol or declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CodeIndexEntry {
    pub file_path: PathBuf,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub content_snippet: String,
    pub last_modified: u64,
}

/// Persistent offline code index for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CodeIndex {
    pub entries: Vec<CodeIndexEntry>,
    #[serde(skip)]
    pub index_path: PathBuf,
}

// ── Regex patterns for Rust symbol extraction ──────────────────────────────

static FN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?fn\s+(\w+)").unwrap());

static STRUCT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?struct\s+(\w+)").unwrap());

static ENUM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?enum\s+(\w+)").unwrap());

static TRAIT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?trait\s+(\w+)").unwrap());

static IMPL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?impl\s*(?:<[^>]*>\s+)?(\w+)").unwrap()
});

static MOD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*pub\s+mod\s+(\w+)").unwrap());

static USE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*(?:pub\s+)?use\s+(.+)").unwrap());

// ── CodeIndex implementation ──────────────────────────────────────────────

impl Default for CodeIndex {
    fn default() -> Self {
        CodeIndex {
            entries: Vec::new(),
            index_path: PathBuf::new(),
        }
    }
}

impl CodeIndex {
    /// Create a new empty index rooted at `workspace_root/.elma_index/`.
    pub(crate) fn new_at(workspace_root: &Path) -> Self {
        let index_path = workspace_root.join(".elma_index");
        CodeIndex {
            entries: Vec::new(),
            index_path,
        }
    }

    /// Index a single Rust source file, extracting all symbol declarations.
    pub(crate) fn index_file(path: &Path) -> Vec<CodeIndexEntry> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let last_modified = fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        extract_rust_symbols(&content, path, last_modified)
    }

    /// Search the index by symbol name or file path (case-insensitive).
    pub(crate) fn search(&self, query: &str, max_results: usize) -> Vec<&CodeIndexEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                let name_match = e
                    .symbol_name
                    .as_deref()
                    .map(|n| n.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                let path_match = e
                    .file_path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&query_lower);
                name_match || path_match
            })
            .take(max_results)
            .collect()
    }

    /// Persist the index to disk as JSON at `index_path/index.json`.
    pub(crate) fn persist(&self) -> Result<()> {
        fs::create_dir_all(&self.index_path).context("Failed to create index directory")?;
        let index_file = self.index_path.join("index.json");
        let json = serde_json::to_string_pretty(&self.entries)
            .context("Failed to serialize index entries")?;
        fs::write(&index_file, json).context("Failed to write index file")?;
        Ok(())
    }

    /// Load a previously persisted index from `workspace_root/.elma_index/`.
    pub(crate) fn load(workspace_root: &Path) -> Option<Self> {
        let index_path = workspace_root.join(".elma_index");
        let index_file = index_path.join("index.json");
        let json = fs::read_to_string(index_file).ok()?;
        let entries: Vec<CodeIndexEntry> = serde_json::from_str(&json).ok()?;
        Some(CodeIndex {
            entries,
            index_path,
        })
    }
}

/// Extract Rust symbol declarations from source content using line-based regex.
///
/// Recognised forms: `fn`, `struct`, `enum`, `trait`, `impl`, `pub mod`, `use`.
pub(crate) fn extract_rust_symbols(
    content: &str,
    file_path: &Path,
    last_modified: u64,
) -> Vec<CodeIndexEntry> {
    let mut entries = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1; // 1-indexed

        if let Some(caps) = FN_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("function".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = STRUCT_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("struct".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = ENUM_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("enum".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = TRAIT_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("trait".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = IMPL_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("impl".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = MOD_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("module".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        } else if let Some(caps) = USE_RE.captures(line) {
            entries.push(CodeIndexEntry {
                file_path: file_path.to_path_buf(),
                symbol_name: caps.get(1).map(|m| m.as_str().to_string()),
                symbol_kind: Some("use".to_string()),
                line_start: line_num,
                line_end: line_num,
                content_snippet: line.to_string(),
                last_modified,
            });
        }
    }

    entries
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── extract_rust_symbols ───────────────────────────────────────────────

    #[test]
    fn extracts_function_declarations() {
        let source = r#"
fn plain() {}
pub fn public_fn() -> i32 { 42 }
pub(crate) fn crate_visible() {}
pub(super) fn super_visible() {}
fn generic<T: Debug>(t: T) {}
"#;
        let entries = extract_rust_symbols(source, Path::new("lib.rs"), 0);
        let fns: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("function"))
            .collect();
        assert_eq!(fns.len(), 5);
        assert_eq!(fns[0].symbol_name.as_deref(), Some("plain"));
        assert_eq!(fns[1].symbol_name.as_deref(), Some("public_fn"));
        assert_eq!(fns[2].symbol_name.as_deref(), Some("crate_visible"));
        assert_eq!(fns[3].symbol_name.as_deref(), Some("super_visible"));
        assert_eq!(fns[4].symbol_name.as_deref(), Some("generic"));
    }

    #[test]
    fn extracts_struct_definitions() {
        let source = r#"
struct Empty;
pub struct Point { x: i32, y: i32 }
pub(crate) struct Internal;
struct Generic<T>(T);
"#;
        let entries = extract_rust_symbols(source, Path::new("types.rs"), 0);
        let structs: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("struct"))
            .collect();
        assert_eq!(structs.len(), 4);
        assert_eq!(structs[1].symbol_name.as_deref(), Some("Point"));
    }

    #[test]
    fn extracts_enum_definitions() {
        let source = r#"
enum Color { Red, Green, Blue }
pub enum Status { Active, Inactive }
pub(crate) enum Internal { A, B }
"#;
        let entries = extract_rust_symbols(source, Path::new("enums.rs"), 0);
        let enums: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("enum"))
            .collect();
        assert_eq!(enums.len(), 3);
        assert_eq!(enums[0].symbol_name.as_deref(), Some("Color"));
        assert_eq!(enums[1].symbol_name.as_deref(), Some("Status"));
    }

    #[test]
    fn extracts_trait_definitions() {
        let source = r#"
trait Default { fn default() -> Self; }
pub trait Display { fn fmt(&self); }
pub(crate) trait Internal {}
"#;
        let entries = extract_rust_symbols(source, Path::new("traits.rs"), 0);
        let traits: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("trait"))
            .collect();
        assert_eq!(traits.len(), 3);
        assert_eq!(traits[0].symbol_name.as_deref(), Some("Default"));
        assert_eq!(traits[1].symbol_name.as_deref(), Some("Display"));
    }

    #[test]
    fn extracts_impl_blocks() {
        let source = r#"
impl MyStruct {
    fn method(&self) {}
}
impl<T> GenericStruct<T> {
    fn new() -> Self { Default::default() }
}
impl MyTrait for MyStruct {
    fn trait_method(&self) {}
}
"#;
        let entries = extract_rust_symbols(source, Path::new("impls.rs"), 0);
        let impls: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("impl"))
            .collect();
        assert_eq!(impls.len(), 3);
        assert_eq!(impls[0].symbol_name.as_deref(), Some("MyStruct"));
        assert_eq!(impls[1].symbol_name.as_deref(), Some("GenericStruct"));
        assert_eq!(impls[2].symbol_name.as_deref(), Some("MyTrait"));
    }

    #[test]
    fn extracts_module_declarations() {
        let source = r#"
pub mod foo;
pub mod bar;
pub mod nested;
"#;
        let entries = extract_rust_symbols(source, Path::new("mods.rs"), 0);
        let mods: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("module"))
            .collect();
        assert_eq!(mods.len(), 3);
        assert_eq!(mods[0].symbol_name.as_deref(), Some("foo"));
    }

    #[test]
    fn extracts_use_statements() {
        let source = r#"
use std::collections::HashMap;
use crate::util;
pub use serde::{Serialize, Deserialize};
"#;
        let entries = extract_rust_symbols(source, Path::new("uses.rs"), 0);
        let uses: Vec<_> = entries
            .iter()
            .filter(|e| e.symbol_kind.as_deref() == Some("use"))
            .collect();
        assert_eq!(uses.len(), 3);
        assert!(uses[0]
            .symbol_name
            .as_deref()
            .unwrap()
            .contains("std::collections::HashMap"));
    }

    #[test]
    fn extracts_all_symbol_types_from_mixed_source() {
        let source = r#"
pub mod my_module;

use std::collections::HashMap;

pub struct MyStruct {
    field: i32,
}

pub enum MyEnum {
    A,
    B(i32),
}

pub trait MyTrait {
    fn do_thing(&self);
}

impl MyStruct {
    pub fn new() -> Self {
        MyStruct { field: 0 }
    }

    fn private_method(&self) -> i32 {
        self.field
    }
}

impl MyTrait for MyStruct {
    fn do_thing(&self) {
        println!("doing thing");
    }
}

pub(crate) fn helper_function() -> bool {
    true
}
"#;
        let entries = extract_rust_symbols(source, Path::new("full.rs"), 0);
        let kinds: Vec<Option<String>> = entries.iter().map(|e| e.symbol_kind.clone()).collect();
        assert!(kinds.contains(&Some("module".to_string())));
        assert!(kinds.contains(&Some("use".to_string())));
        assert!(kinds.contains(&Some("struct".to_string())));
        assert!(kinds.contains(&Some("enum".to_string())));
        assert!(kinds.contains(&Some("trait".to_string())));
        assert!(kinds.contains(&Some("impl".to_string())));
        assert!(kinds.contains(&Some("function".to_string())));
    }

    #[test]
    fn returns_empty_for_non_rust_content() {
        let entries = extract_rust_symbols("hello world\nsome text\n", Path::new("foo.txt"), 0);
        assert!(entries.is_empty());
    }

    #[test]
    fn sets_correct_line_numbers() {
        let source = "// line 1\nfn foo() {}\n// line 3\nfn bar() {}\n";
        let entries = extract_rust_symbols(source, Path::new("lines.rs"), 0);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].line_start, 2);
        assert_eq!(entries[0].symbol_name.as_deref(), Some("foo"));
        assert_eq!(entries[1].line_start, 4);
        assert_eq!(entries[1].symbol_name.as_deref(), Some("bar"));
    }

    // ── CodeIndex ──────────────────────────────────────────────────────────

    #[test]
    fn new_creates_empty_index() {
        let index = CodeIndex::new_at(Path::new("/tmp/workspace"));
        assert!(index.entries.is_empty());
        assert_eq!(index.index_path, Path::new("/tmp/workspace/.elma_index"));
    }

    #[test]
    fn search_finds_by_symbol_name() {
        let mut index = CodeIndex::new_at(Path::new("/tmp"));
        index.entries = vec![
            CodeIndexEntry {
                file_path: PathBuf::from("src/main.rs"),
                symbol_name: Some("run".to_string()),
                symbol_kind: Some("function".to_string()),
                line_start: 10,
                line_end: 10,
                content_snippet: "fn run() {}".to_string(),
                last_modified: 1000,
            },
            CodeIndexEntry {
                file_path: PathBuf::from("src/lib.rs"),
                symbol_name: Some("Helper".to_string()),
                symbol_kind: Some("struct".to_string()),
                line_start: 5,
                line_end: 5,
                content_snippet: "struct Helper;".to_string(),
                last_modified: 1000,
            },
        ];
        let results = index.search("run", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_name.as_deref(), Some("run"));
    }

    #[test]
    fn search_finds_by_file_path() {
        let mut index = CodeIndex::new_at(Path::new("/tmp"));
        index.entries = vec![CodeIndexEntry {
            file_path: PathBuf::from("src/main.rs"),
            symbol_name: Some("run".to_string()),
            symbol_kind: Some("function".to_string()),
            line_start: 10,
            line_end: 10,
            content_snippet: "fn run() {}".to_string(),
            last_modified: 1000,
        }];
        let results = index.search("main.rs", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut index = CodeIndex::new_at(Path::new("/tmp"));
        index.entries = vec![CodeIndexEntry {
            file_path: PathBuf::from("src/main.rs"),
            symbol_name: Some("RunCommand".to_string()),
            symbol_kind: Some("function".to_string()),
            line_start: 10,
            line_end: 10,
            content_snippet: "fn RunCommand() {}".to_string(),
            last_modified: 1000,
        }];
        assert_eq!(index.search("runcommand", 10).len(), 1);
        assert_eq!(index.search("RUNCOMMAND", 10).len(), 1);
        assert_eq!(index.search("RunCommand", 10).len(), 1);
    }

    #[test]
    fn search_respects_max_results() {
        let mut index = CodeIndex::new_at(Path::new("/tmp"));
        index.entries = vec![
            CodeIndexEntry {
                file_path: PathBuf::from("a.rs"),
                symbol_name: Some("foo".to_string()),
                symbol_kind: Some("fn".to_string()),
                line_start: 1,
                line_end: 1,
                content_snippet: "fn foo() {}".to_string(),
                last_modified: 0,
            },
            CodeIndexEntry {
                file_path: PathBuf::from("b.rs"),
                symbol_name: Some("foobar".to_string()),
                symbol_kind: Some("fn".to_string()),
                line_start: 1,
                line_end: 1,
                content_snippet: "fn foobar() {}".to_string(),
                last_modified: 0,
            },
        ];
        assert_eq!(index.search("foo", 1).len(), 1);
        assert_eq!(index.search("foo", 10).len(), 2);
    }

    #[test]
    fn persist_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path();

        let mut index = CodeIndex::new_at(workspace_root);
        index.entries = vec![CodeIndexEntry {
            file_path: PathBuf::from("src/main.rs"),
            symbol_name: Some("main".to_string()),
            symbol_kind: Some("function".to_string()),
            line_start: 1,
            line_end: 1,
            content_snippet: "fn main() {}".to_string(),
            last_modified: 42,
        }];
        index.persist().unwrap();

        let loaded = CodeIndex::load(workspace_root).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].symbol_name.as_deref(), Some("main"));
        assert_eq!(loaded.entries[0].last_modified, 42);
        assert_eq!(loaded.index_path, workspace_root.join(".elma_index"));
    }

    #[test]
    fn load_returns_none_when_no_index_exists() {
        let dir = tempfile::tempdir().unwrap();
        assert!(CodeIndex::load(dir.path()).is_none());
    }

    #[test]
    fn index_file_reads_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("example.rs");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "pub fn hello() {{}}").unwrap();
        writeln!(f, "struct World;").unwrap();
        drop(f);

        let entries = CodeIndex::index_file(&file_path);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].symbol_name.as_deref(), Some("hello"));
        assert_eq!(entries[0].symbol_kind.as_deref(), Some("function"));
        assert_eq!(entries[1].symbol_name.as_deref(), Some("World"));
        assert_eq!(entries[1].symbol_kind.as_deref(), Some("struct"));
    }

    #[test]
    fn index_file_returns_empty_for_missing_file() {
        let entries = CodeIndex::index_file(Path::new("/nonexistent/file.rs"));
        assert!(entries.is_empty());
    }
}
