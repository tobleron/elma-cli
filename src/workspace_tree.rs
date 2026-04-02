//! Workspace Tree Generation

use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct WorkspaceTree {
    root: PathBuf,
    max_depth: usize,
}

impl WorkspaceTree {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            max_depth: 3,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
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

        Ok(format_tree(&tree))
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

fn format_tree(entries: &BTreeMap<PathBuf, (usize, bool)>) -> String {
    let mut output = String::new();

    for (path, (depth, is_dir)) in entries {
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
