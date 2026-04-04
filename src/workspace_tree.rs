//! @efficiency-role: util-pure
//! Workspace Tree Generation

use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct WorkspaceTree {
    root: PathBuf,
    max_depth: usize,
    max_entries: usize,
}

impl WorkspaceTree {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            max_depth: 2,
            max_entries: 160,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    pub fn build(&self) -> Result<String> {
        let mut tree = BTreeMap::new();

        let walker = WalkBuilder::new(&self.root)
            .max_depth(Some(self.max_depth))
            .hidden(true)
            .filter_entry(|entry| {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip ignored directories
                if path.is_dir() {
                    return !is_ignored_dir(name);
                }

                // Skip ignored files
                !is_ignored_file(name)
            })
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if let Ok(rel_path) = path.strip_prefix(&self.root) {
                if rel_path.components().count() > 0 {
                    let depth = rel_path.components().count();
                    let is_dir = path.is_dir();
                    tree.insert(rel_path.to_path_buf(), (depth, is_dir));
                }
            }
        }

        Ok(format_tree(&tree, self.max_entries))
    }
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | ".git"
            | ".cargo"
            | "dist"
            | "build"
            | "__pycache__"
            | ".venv"
            | ".qwen"
            | "sessions"
            | "trace"
    )
}

fn is_ignored_file(name: &str) -> bool {
    matches!(
        name,
        ".DS_Store" | "Thumbs.db" | "*.lock" | "*.pyc" | "*.pyo"
    )
}

fn format_tree(entries: &BTreeMap<PathBuf, (usize, bool)>, max_entries: usize) -> String {
    let mut output = String::new();
    let mut emitted = 0usize;

    for (path, (depth, is_dir)) in entries {
        if emitted >= max_entries {
            let remaining = entries.len().saturating_sub(emitted);
            output.push_str(&format!("... ({remaining} more entries omitted)\n"));
            break;
        }

        // Add indentation
        for _ in 0..(depth - 1) {
            output.push_str("│   ");
        }

        // Add tree character
        if *depth > 1 {
            output.push_str("├── ");
        }

        // Add file/dir name
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            output.push_str(name);
            if *is_dir {
                output.push('/');
            }
            output.push('\n');
            emitted += 1;
        }
    }

    if output.is_empty() {
        output.push_str("(empty)\n");
    }

    output
}

pub fn generate_workspace_brief(repo: &Path) -> String {
    match WorkspaceTree::new(repo).with_max_depth(3).build() {
        Ok(tree) => tree,
        Err(_) => generate_workspace_brief_fallback(repo),
    }
}

fn generate_workspace_brief_fallback(repo: &Path) -> String {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(repo) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if !is_ignored_dir(&name) && !is_ignored_file(&name) {
                    files.push(name);
                }
            }
        }
    }
    files.join(", ")
}
