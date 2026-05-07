//! @efficiency-role: domain-logic
//!
//! Workspace Path Resolution And Failed Path Recovery — Task 765.
//!
//! When the model requests a path that doesn't exist, this module
//! generates ranked candidates based on workspace tree, suffix similarity,
//! and basename matching. The strongest candidate can be auto-suggested
//! or presented to the model for selection.

use crate::*;
use std::path::Path;

/// A candidate path resolution.
#[derive(Debug, Clone)]
pub(crate) struct PathCandidate {
    /// Full resolved path relative to workspace root.
    pub resolved: String,
    /// Similarity score 0.0–1.0.
    pub score: f64,
    /// How this was found: "suffix_match", "basename_match", "prefix_match"
    pub match_kind: String,
}

/// Rank candidate paths for a given query within the workspace tree.
pub(crate) fn resolve_missing_path(query: &str, workspace_root: &Path) -> Vec<PathCandidate> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let query_lower = query.to_lowercase();
    let query_basename = Path::new(&query_lower)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&query_lower)
        .to_string();
    let query_suffix = extract_suffix(&query_lower);

    let mut candidates: Vec<PathCandidate> = Vec::new();

    // Walk workspace tree (shallow: first 3 levels)
    if let Ok(entries) = walk_workspace_shallow(workspace_root, 3) {
        for entry in entries {
            let entry_lower = entry.to_lowercase();
            let entry_basename = Path::new(&entry_lower)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Score this entry
            let mut score = 0.0_f64;
            let mut match_kind = "none";

            // Exact suffix match (e.g., "tasks/completed" -> "_tasks/completed")
            if let Some(ref suffix) = query_suffix {
                if entry_lower.ends_with(suffix) && entry_lower != query_lower {
                    score = 0.9;
                    match_kind = "suffix_match";
                }
            }

            // Basename match
            if entry_basename == query_basename && entry_lower != query_lower {
                if score == 0.0 {
                    score = 0.7;
                    match_kind = "basename_match";
                } else {
                    score = score.max(0.85);
                    match_kind = "suffix+basename_match";
                }
            }

            // Prefix match (query is prefix of entry, and query is non-empty)
            if !query_lower.is_empty()
                && entry_lower.starts_with(&query_lower)
                && entry_lower != query_lower
            {
                score = score.max(0.5);
                match_kind = "prefix_match";
            }

            if score > 0.0 {
                candidates.push(PathCandidate {
                    resolved: entry.clone(),
                    score,
                    match_kind: match_kind.to_string(),
                });
            }
        }
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(5);
    candidates
}

/// Get the best candidate, if any (score >= threshold).
pub(crate) fn best_candidate(query: &str, workspace_root: &Path, threshold: f64) -> Option<PathCandidate> {
    let candidates = resolve_missing_path(query, workspace_root);
    candidates.into_iter().find(|c| c.score >= threshold)
}

/// Shallow workspace walk (up to `max_depth` levels).
fn walk_workspace_shallow(root: &Path, max_depth: usize) -> Result<Vec<String>> {
    let mut results = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let excluded = ["target", ".git", ".trash", "node_modules", "vendor"];

    while let Some((path, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                let rel = entry_path
                    .strip_prefix(root)
                    .unwrap_or(&entry_path)
                    .display()
                    .to_string();
                let fname = entry_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if excluded.contains(&fname) {
                    continue;
                }
                if entry_path.is_dir() {
                    if depth < max_depth {
                        stack.push((entry_path, depth + 1));
                    }
                    results.push(format!("{}/", rel));
                } else {
                    results.push(rel);
                }
            }
        }
    }
    Ok(results)
}

/// Extract the longest suffix after the first `/` that the query contains.
fn extract_suffix(query: &str) -> Option<String> {
    if let Some(pos) = query.find('/') {
        Some(query[pos..].to_string())
    } else {
        None
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_workspace() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        // Create _tasks/completed
        fs::create_dir_all(root.join("_tasks").join("completed")).unwrap();
        fs::write(root.join("_tasks").join("completed").join("001_done.md"), "done").unwrap();
        fs::write(root.join("AGENTS.md"), "agents").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        (dir, root)
    }

    #[test]
    fn test_resolve_suffix_match() {
        let (_dir, root) = setup_test_workspace();
        let candidates = resolve_missing_path("tasks/completed", &root);
        let has_match = candidates.iter().any(|c| c.resolved.contains("_tasks/completed"));
        assert!(has_match, "Should find _tasks/completed from tasks/completed query");
    }

    #[test]
    fn test_resolve_basename_match() {
        let (_dir, root) = setup_test_workspace();
        let candidates = resolve_missing_path("AGENTS", &root);
        let has_agents = candidates.iter().any(|c| c.resolved.contains("AGENTS.md"));
        assert!(has_agents, "Should find AGENTS.md from AGENTS query");
    }

    #[test]
    fn test_best_candidate_found() {
        let (_dir, root) = setup_test_workspace();
        let candidate = best_candidate("tasks/completed", &root, 0.5);
        assert!(candidate.is_some(), "Should find a candidate for tasks/completed");
        if let Some(c) = candidate {
            assert!(c.resolved.contains("_tasks/completed"));
            assert!(c.score >= 0.5);
        }
    }

    #[test]
    fn test_best_candidate_not_found() {
        let (_dir, root) = setup_test_workspace();
        let candidate = best_candidate("nonexistent_path_xyz", &root, 0.9);
        assert!(candidate.is_none());
    }

    #[test]
    fn test_extract_suffix() {
        assert_eq!(extract_suffix("tasks/completed"), Some("/completed".to_string()));
        assert_eq!(extract_suffix("no_slash"), None);
    }

    #[test]
    fn test_empty_query() {
        let (_dir, root) = setup_test_workspace();
        let candidates = resolve_missing_path("", &root);
        assert!(candidates.is_empty());
    }
}
