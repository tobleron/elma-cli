//! @efficiency-role: service-orchestrator
//!
//! Read-Only Whole-System File Scout Skill.
//! Searches beyond the workspace, discloses roots, and hands candidates
//! to later formula stages.

use crate::*;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ScoutResult {
    pub searched_roots: Vec<String>,
    pub skipped_roots: Vec<String>,
    pub candidate_files: Vec<ScoutCandidate>,
    pub inspected_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScoutCandidate {
    pub path: String,
    pub reason: String,
}

/// Default exclusions for pseudo-filesystems and irrelevant directories.
pub(crate) fn default_scout_exclusions() -> HashSet<String> {
    [
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/tmp",
        "/var/tmp",
        "/boot",
        "/lost+found",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Scout files matching a query across the given roots.
/// Stays read-only. Discloses searched and skipped roots.
pub(crate) fn scout_files(query: &str, roots: &[PathBuf], explicit_roots: bool) -> ScoutResult {
    let exclusions = default_scout_exclusions();
    let query_lower = query.to_ascii_lowercase();

    let effective_roots: Vec<PathBuf> = if roots.is_empty() {
        vec![PathBuf::from("/")]
    } else {
        roots.to_vec()
    };

    // Separate skipped and searched roots
    let (searched_roots, skipped_roots): (Vec<PathBuf>, Vec<String>) =
        effective_roots.into_iter().partition_map(|root| {
            let root_str = root.display().to_string();
            if !explicit_roots && exclusions.contains(&root_str) {
                itertools::Either::Right(root_str)
            } else {
                itertools::Either::Left(root)
            }
        });

    // Process searched roots in parallel
    let all_candidates: Mutex<Vec<ScoutCandidate>> = Mutex::new(Vec::new());
    let seen: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());

    searched_roots.par_iter().for_each(|root| {
        if let Ok(entries) = walk_dir_bounded(root, 3) {
            for path in entries {
                let mut seen_guard = seen.lock().unwrap();
                if seen_guard.contains(&path) {
                    continue;
                }
                let fname = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if fname.to_ascii_lowercase().contains(&query_lower) {
                    all_candidates.lock().unwrap().push(ScoutCandidate {
                        path: path.display().to_string(),
                        reason: "name matches query".to_string(),
                    });
                    seen_guard.insert(path);
                }
            }
        }
    });

    let mut candidate_files = all_candidates.into_inner().unwrap_or_default();
    // Limit candidates
    candidate_files.truncate(20);

    let searched_roots_str: Vec<String> = searched_roots
        .iter()
        .map(|r| r.display().to_string())
        .collect();

    let inspected_files: Vec<String> = candidate_files.iter().map(|c| c.path.clone()).collect();

    ScoutResult {
        searched_roots: searched_roots_str,
        skipped_roots,
        candidate_files,
        inspected_files,
    }
}

fn walk_dir_bounded(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let exclusions = default_scout_exclusions();

    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let canonical = path.display().to_string();
                    if !exclusions.contains(&canonical) {
                        stack.push((path, depth + 1));
                    }
                } else {
                    results.push(path);
                }
            }
        }
    }
    Ok(results)
}

pub(crate) fn render_scout_result(result: &ScoutResult) -> String {
    let mut lines = Vec::new();
    lines.push("FILE SCOUT RESULT".to_string());
    lines.push(format!(
        "Searched roots ({}): {}",
        result.searched_roots.len(),
        result.searched_roots.join(", ")
    ));
    if !result.skipped_roots.is_empty() {
        lines.push(format!(
            "Skipped roots ({}): {}",
            result.skipped_roots.len(),
            result.skipped_roots.join(", ")
        ));
    }
    lines.push(format!(
        "Candidate files ({}):",
        result.candidate_files.len()
    ));
    for c in &result.candidate_files {
        lines.push(format!("  {} — {}", c.path, c.reason));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_exclusions_contains_proc() {
        let ex = default_scout_exclusions();
        assert!(ex.contains("/proc"));
        assert!(ex.contains("/sys"));
        assert!(ex.contains("/dev"));
    }

    #[test]
    fn scout_current_dir_finds_cargo_toml() {
        let result = scout_files("Cargo", &[PathBuf::from(".")], true);
        assert!(result
            .candidate_files
            .iter()
            .any(|c| c.path.contains("Cargo.toml")));
        assert!(result.searched_roots.contains(&".".to_string()));
    }

    #[test]
    fn excluded_roots_are_skipped() {
        let result = scout_files("test", &[PathBuf::from("/proc")], false);
        assert!(result.skipped_roots.contains(&"/proc".to_string()));
        assert!(!result.searched_roots.contains(&"/proc".to_string()));
    }

    #[test]
    fn explicit_root_can_override_exclusion() {
        let result = scout_files("test", &[PathBuf::from("/proc")], true);
        assert!(result.searched_roots.contains(&"/proc".to_string()));
    }
}
