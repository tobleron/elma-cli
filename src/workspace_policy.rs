//! @efficiency-role: util-pure
//! Workspace Policy: ignore and protected path handling.
//! Task 691: Default exclusion paths and scope control.
//! Task 696: Scope-bounded search and glob constraints.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Default exclusion paths for workspace-scoped operations (Task 691).
/// These paths are excluded from glob, search, and shell scans by default
/// unless the user explicitly targets them.
pub(crate) static DEFAULT_EXCLUDED_PATHS: &[&str] = &[
    ".git",
    "target",
    ".trash",
    ".kilo",
    ".opencode",
    "project_tmp",
    "sessions",
    "_knowledge_base",
    "node_modules",
    ".crush",
    ".dirac-symbol-index",
];


/// Defines scope boundaries for search and glob operations (Task 696).
#[derive(Debug, Clone, Default)]
pub(crate) struct ScopeConstraint {
    /// Allowed directories (e.g., ["src", "docs", "tests"])
    pub allow_dirs: Vec<String>,
    /// Glob patterns for inclusion (e.g., ["*.rs", "*.md"])
    pub include_globs: Vec<String>,
    /// Glob patterns for exclusion (e.g., ["*_test.rs"])
    pub exclude_globs: Vec<String>,
    /// Whether to apply default exclusions
    pub apply_defaults: bool,
}

impl ScopeConstraint {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_allow_dirs(mut self, dirs: &[&str]) -> Self {
        self.allow_dirs = dirs.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_include_globs(mut self, globs: &[&str]) -> Self {
        self.include_globs = globs.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_exclude_globs(mut self, globs: &[&str]) -> Self {
        self.exclude_globs = globs.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Check if a relative path is within scope.
    pub fn in_scope(&self, rel_path: &str) -> bool {
        if self.allow_dirs.is_empty() {
            return true;
        }
        self.allow_dirs.iter().any(|d| {
            rel_path == d.as_str()
                || rel_path.starts_with(&format!("{}/", d))
                || rel_path.starts_with(&format!("{}\\", d))
        })
    }
}
pub(crate) fn set_scope_constraint(constraint: Option<ScopeConstraint>) {
    let state = crate::session_state::get_session_state();
    let mut lock = match state.scope_constraint.write() {
        Ok(l) => l,
        Err(_) => return,
    };
    *lock = constraint;
}

/// Get the current scope constraint.
pub(crate) fn get_scope_constraint() -> Option<ScopeConstraint> {
    let state = crate::session_state::get_session_state();
    let lock = state.scope_constraint.read().ok()?;
    lock.clone()
}

/// Narrow a search pattern or path to within the allowed scope (Task 696).
/// Returns (narrowed_path, was_narrowed) where narrowed_path is the path
/// constrained to the first allowed directory if scope is set.
pub(crate) fn narrow_to_scope(requested_path: &str) -> (String, bool) {
    let constraint = get_scope_constraint();
    let Some(ref constraint) = constraint else {
        return (requested_path.to_string(), false);
    };

    if constraint.allow_dirs.is_empty() {
        return (requested_path.to_string(), false);
    }

    // If path already starts with an allowed dir, keep it
    for dir in &constraint.allow_dirs {
        if requested_path.starts_with(dir) || requested_path.contains(&format!("/{}", dir)) {
            return (requested_path.to_string(), false);
        }
    }

    // Otherwise narrow to the first allowed directory
    let narrowed = constraint.allow_dirs[0].clone();
    crate::append_trace_log_line(&format!(
        "[SCOPE_NARROW] requested='{}' narrowed to '{}'",
        requested_path, narrowed
    ));
    (narrowed, true)
}

/// Build a scope notice string for transcript display (Task 696).
pub(crate) fn build_scope_notice() -> Option<String> {
    let constraint = get_scope_constraint();
    let Some(ref constraint) = constraint else {
        return None;
    };
    if constraint.allow_dirs.is_empty() {
        return None;
    }
    Some(format!(
        "Scope constrained to: {}",
        constraint.allow_dirs.join(", ")
    ))
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkspacePolicy {
    pub ignore_patterns: HashSet<String>,
    pub protect_patterns: HashSet<String>,
    /// Whether default exclusions are applied (Task 691).
    pub apply_default_exclusions: bool,
}

impl WorkspacePolicy {
    pub fn new(root: &Path) -> Self {
        let mut policy = Self::default();
        policy.load(root);
        policy
    }

    pub fn with_default_exclusions(root: &Path) -> Self {
        let mut policy = Self::new(root);
        policy.apply_default_exclusions = true;
        policy
    }

    fn load(&mut self, root: &Path) {
        let ignore_file = root.join(".elmaignore");
        if ignore_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&ignore_file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        self.ignore_patterns.insert(trimmed.to_string());
                    }
                }
            }
        }

        let protect_file = root.join(".elmaprotect");
        if protect_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&protect_file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        self.protect_patterns.insert(trimmed.to_string());
                    }
                }
            }
        }

        let protect_toml = root.join(".elmaprotect.toml");
        if protect_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&protect_toml) {
                if let Ok(config) = toml::from_str::<toml::Value>(&content) {
                    if let Some(protected) = config.get("protected").and_then(|v| v.as_array()) {
                        for pattern in protected {
                            if let Some(p) = pattern.as_str() {
                                self.protect_patterns.insert(p.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if a path component matches any default exclusion pattern (Task 691).
    pub fn is_default_excluded(path_component: &str) -> bool {
        let normalized = path_component.replace('\\', "/");
        let component = normalized.trim_end_matches('/');
        DEFAULT_EXCLUDED_PATHS
            .iter()
            .any(|p| *p == component || component.starts_with(&format!("{}/", p)))
    }

    /// Check if a full path contains any excluded component (Task 691).
    pub fn path_is_default_excluded(path: &Path) -> bool {
        path.components().any(|c| {
            if let std::path::Component::Normal(os_str) = c {
                if let Some(s) = os_str.to_str() {
                    return Self::is_default_excluded(s);
                }
            }
            false
        })
    }

    /// Get a curated list of default exclusion notes for transcript display (Task 691).
    pub fn default_exclusion_notice() -> String {
        format!(
            "Scope narrowed: excluding default paths ({})",
            DEFAULT_EXCLUDED_PATHS.join(", ")
        )
    }

    /// Check if a path should be excluded from workspace operations.
    /// Considers both custom ignore patterns and default exclusions.
    pub fn is_excluded(&self, path: &Path) -> bool {
        if self.is_ignored(path) {
            return true;
        }
        if self.apply_default_exclusions && Self::path_is_default_excluded(path) {
            return true;
        }
        false
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let rel_path = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let rel_str = rel_path.replace('\\', "/");

        for pattern in &self.ignore_patterns {
            if glob_match(pattern, &rel_str) {
                return true;
            }
        }

        false
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        let rel_path = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let rel_str = rel_path.replace('\\', "/");

        for pattern in &self.protect_patterns {
            if glob_match(pattern, &rel_str) {
                return true;
            }
        }

        false
    }

    pub fn blocked_message(&self, path: &Path, operation: &str) -> Option<String> {
        if crate::ui_state::is_full_access() {
            return None;
        }
        if self.is_protected(path) {
            return Some(format!(
                "protected_path_blocked: {} is protected from {} by .elmaprotect policy",
                path.display(),
                operation
            ));
        }
        None
    }
}


fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }

    if let Some(stripped) = pattern.strip_prefix("**/") {
        return path.ends_with(stripped) || path.contains(&format!("/{stripped}"));
    }

    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return (prefix.is_empty() || path.starts_with(prefix))
            && (suffix.is_empty() || path.ends_with(suffix));
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("foo.txt", "foo.txt"));
        assert!(!glob_match("foo.txt", "bar.txt"));
    }

    #[test]
    fn test_glob_match_prefix() {
        assert!(glob_match("*.txt", "foo.txt"));
        assert!(glob_match("*.txt", "bar.txt"));
        assert!(!glob_match("*.txt", "foo.rs"));
    }

    #[test]
    fn test_glob_match_suffix() {
        assert!(glob_match("**/config", "something/config"));
        assert!(glob_match("config", "config"));
    }

    // ── Task 691: Default exclusion paths ──

    #[test]
    fn test_default_excluded_matches_dotgit() {
        assert!(WorkspacePolicy::is_default_excluded(".git"));
    }

    #[test]
    fn test_default_excluded_matches_target() {
        assert!(WorkspacePolicy::is_default_excluded("target"));
    }

    #[test]
    fn test_default_excluded_matches_node_modules() {
        assert!(WorkspacePolicy::is_default_excluded("node_modules"));
    }

    #[test]
    fn test_default_excluded_matches_project_tmp() {
        assert!(WorkspacePolicy::is_default_excluded("project_tmp"));
    }

    #[test]
    fn test_default_excluded_does_not_match_src() {
        assert!(!WorkspacePolicy::is_default_excluded("src"));
    }

    #[test]
    fn test_path_is_default_excluded() {
        let p = Path::new("project_tmp/report.md");
        assert!(WorkspacePolicy::path_is_default_excluded(p));
    }

    #[test]
    fn test_path_not_default_excluded() {
        let p = Path::new("src/main.rs");
        assert!(!WorkspacePolicy::path_is_default_excluded(p));
    }

    #[test]
    fn test_is_excluded_with_defaults() {
        let policy = WorkspacePolicy::with_default_exclusions(Path::new("."));
        let p = Path::new("project_tmp/file.txt");
        assert!(policy.is_excluded(p));
    }

    #[test]
    fn test_is_excluded_without_defaults() {
        let mut policy = WorkspacePolicy::new(Path::new("."));
        policy.apply_default_exclusions = false;
        let p = Path::new("project_tmp/file.txt");
        assert!(!policy.is_excluded(p));
    }

    #[test]
    fn test_default_exclusion_notice_format() {
        let notice = WorkspacePolicy::default_exclusion_notice();
        assert!(notice.contains(".git"));
        assert!(notice.contains("target"));
    }

    #[test]
    fn test_default_excluded_component_in_path() {
        let p = Path::new("some/dir/.kilo/node_modules/file.js");
        assert!(WorkspacePolicy::path_is_default_excluded(p));
    }

    #[test]
    fn test_sessions_excluded() {
        assert!(WorkspacePolicy::is_default_excluded("sessions"));
    }

    #[test]
    fn test_knowledge_base_excluded() {
        assert!(WorkspacePolicy::is_default_excluded("_knowledge_base"));
    }

    #[test]
    fn test_trash_excluded() {
        assert!(WorkspacePolicy::is_default_excluded(".trash"));
    }
}
