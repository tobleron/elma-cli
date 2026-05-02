//! @efficiency-role: domain-logic
//!
//! Repo Map - Symbol-Aware Repository Map with Tag Cache
//!
//! Provides a token-budgeted repo map that indexes symbols, definitions,
//! and file relationships. Cache entries are invalidated when files change.

use crate::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Symbol information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SymbolEntry {
    pub(crate) name: String,
    pub(crate) kind: String, // "function", "struct", "enum", "trait", etc.
    pub(crate) file: String,
    pub(crate) line: u32,
    pub(crate) signature: Option<String>,
}

/// File entry in the repo map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FileEntry {
    pub(crate) path: String,
    pub(crate) mtime: u64, // last modified time (unix seconds)
    pub(crate) hash: String, // file content hash for cache invalidation
    pub(crate) symbols: Vec<SymbolEntry>,
}

/// Repo map cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepoMapCache {
    pub(crate) version: u32,
    pub(crate) generated_unix_s: u64,
    pub(crate) repo_root: String,
    pub(crate) files: HashMap<String, FileEntry>,
}

impl RepoMapCache {
    pub(crate) fn new(repo_root: &Path) -> Self {
        Self {
            version: 1,
            generated_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            repo_root: repo_root.display().to_string(),
            files: HashMap::new(),
        }
    }

    pub(crate) fn get_cache_path(repo_root: &Path) -> PathBuf {
        // Store cache in the project directory or session directory
        repo_root.join(".elma-repo-map-cache.json")
    }

    pub(crate) fn load(repo_root: &Path) -> Option<Self> {
        let path = Self::get_cache_path(repo_root);
        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub(crate) fn save(&self, repo_root: &Path) -> Result<()> {
        let path = Self::get_cache_path(repo_root);
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize repo map cache")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    pub(crate) fn is_file_changed(&self, file: &str, current_mtime: u64, current_hash: &str) -> bool {
        match self.files.get(file) {
            Some(entry) => entry.mtime != current_mtime || entry.hash != current_hash,
            None => true, // New file
        }
    }

    pub(crate) fn update_file(&mut self, entry: FileEntry) {
        self.files.insert(entry.path.clone(), entry);
    }

    pub(crate) fn remove_file(&mut self, file: &str) {
        self.files.remove(file);
    }
}

/// Generate a simple hash for a file's content (using mtime-based hash for now).
pub(crate) fn hash_file(path: &Path) -> String {
    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                return format!("{:x}", duration.as_nanos());
            }
        }
    }
    "0".to_string()
}

/// Extract symbols from a file using a fallback parser (regex-based).
pub(crate) fn extract_symbols(path: &Path) -> Vec<SymbolEntry> {
    let mut symbols = Vec::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return symbols;
    };

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "rs" => extract_rust_symbols(&content, path),
        "py" => extract_python_symbols(&content, path),
        "js" | "ts" => extract_js_symbols(&content, path),
        _ => Vec::new(), // Unsupported language
    }
}

/// Extract Rust symbols (functions, structs, enums, traits, impls).
fn extract_rust_symbols(content: &str, path: &Path) -> Vec<SymbolEntry> {
    let mut symbols = Vec::new();
    let path_str = path.display().to_string();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        let (kind, name) = if line.starts_with("fn ") {
            ("function", line.trim_start_matches("fn ").split('(').next().map(|s| s.trim()))
        } else if line.starts_with("struct ") {
            ("struct", line.trim_start_matches("struct ").split('{').next().map(|s| s.trim()))
        } else if line.starts_with("enum ") {
            ("enum", line.trim_start_matches("enum ").split('{').next().map(|s| s.trim()))
        } else if line.starts_with("trait ") {
            ("trait", line.trim_start_matches("trait ").split('{').next().map(|s| s.trim()))
        } else if line.starts_with("impl ") {
            ("impl", line.trim_start_matches("impl ").split('{').next().map(|s| s.trim()))
        } else {
            continue;
        };

        if let Some(name) = name {
            symbols.push(SymbolEntry {
                name: name.to_string(),
                kind: kind.to_string(),
                file: path_str.clone(),
                line: line_num as u32 + 1,
                signature: None,
            });
        }
    }

    symbols
}

/// Extract Python symbols (functions, classes).
fn extract_python_symbols(content: &str, path: &Path) -> Vec<SymbolEntry> {
    let mut symbols = Vec::new();
    let path_str = path.display().to_string();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        let (kind, name) = if line.starts_with("def ") {
            ("function", line.trim_start_matches("def ").split('(').next().map(|s| s.trim()))
        } else if line.starts_with("class ") {
            ("class", line.trim_start_matches("class ").split('(').next().map(|s| s.trim()))
        } else {
            continue;
        };

        if let Some(name) = name {
            symbols.push(SymbolEntry {
                name: name.to_string(),
                kind: kind.to_string(),
                file: path_str.clone(),
                line: line_num as u32 + 1,
                signature: None,
            });
        }
    }

    symbols
}

/// Extract JavaScript/TypeScript symbols (functions, classes).
fn extract_js_symbols(content: &str, path: &Path) -> Vec<SymbolEntry> {
    let mut symbols = Vec::new();
    let path_str = path.display().to_string();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        let (kind, name) = if line.starts_with("function ") {
            ("function", line.trim_start_matches("function ").split('(').next().map(|s| s.trim()))
        } else if line.starts_with("class ") {
            ("class", line.trim_start_matches("class ").split('{').next().map(|s| s.trim()))
        } else if line.starts_with("const ") && line.contains('=') && line.contains("=>") {
            ("arrow_function", line.trim_start_matches("const ").split('=').next().map(|s| s.trim()))
        } else {
            continue;
        };

        if let Some(name) = name {
            symbols.push(SymbolEntry {
                name: name.to_string(),
                kind: kind.to_string(),
                file: path_str.clone(),
                line: line_num as u32 + 1,
                signature: None,
            });
        }
    }

    symbols
}

/// Build a repo map within a token budget.
pub(crate) fn build_repo_map(
    repo_root: &Path,
    token_budget: usize,
    max_files: usize,
) -> (String, usize) {
    let mut cache = RepoMapCache::load(repo_root).unwrap_or_else(|| RepoMapCache::new(repo_root));

    let mut output = String::new();
    let mut token_count: usize = 0;
    let mut files_processed = 0;

    output.push_str("# Repo Map\n\n");

    // Walk the directory (non-recursive for simplicity)
    let Ok(entries) = std::fs::read_dir(repo_root) else {
        return ("Error: Failed to read directory".to_string(), 0);
    };

    for entry in entries.filter_map(|e| e.ok()).take(max_files) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        let supported = ["rs", "py", "js", "ts"];
        if !supported.contains(&extension) {
            continue;
        }

        let file_str = path.display().to_string();
        let mtime = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let hash = hash_file(&path);

        if !cache.is_file_changed(&file_str, mtime, &hash) {
            // Use cached symbols
            if let Some(entry) = cache.files.get(&file_str) {
                let tokens = format_file_entry(entry, token_budget - token_count);
                if token_count + tokens.len() <= token_budget {
                    output.push_str(&tokens);
                    token_count += tokens.len();
                }
                files_processed += 1;
                continue;
            }
        }

        // Extract symbols
        let symbols = extract_symbols(&path);
        let entry = FileEntry {
            path: file_str.clone(),
            mtime,
            hash,
            symbols,
        };

        cache.update_file(entry);

        let tokens = format_file_entry(cache.files.get(&file_str).unwrap(), token_budget - token_count);
        if token_count + tokens.len() <= token_budget {
            output.push_str(&tokens);
            token_count += tokens.len();
        }

        files_processed += 1;

        if token_count >= token_budget {
            output.push_str("\n... (token budget exceeded)");
            break;
        }
    }

    // Save cache
    let _ = cache.save(repo_root);

    output.push_str(&format!("\n\n{} files processed, {} tokens used\n", files_processed, token_count));
    (output, token_count)
}

/// Format a file entry for output.
fn format_file_entry(entry: &FileEntry, token_budget: usize) -> String {
    let mut output = String::new();
    output.push_str(&format!("## {}\n", entry.path));

    for symbol in &entry.symbols {
        let line = format!("  - {} {} (line {})\n", symbol.kind, symbol.name, symbol.line);
        if output.len() + line.len() > token_budget {
            output.push_str("  ...\n");
            break;
        }
        output.push_str(&line);
    }

    output
}
