//! @efficiency-role: util-pure
//! Workspace Policy: ignore and protected path handling.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkspacePolicy {
    pub ignore_patterns: HashSet<String>,
    pub protect_patterns: HashSet<String>,
}

impl WorkspacePolicy {
    pub fn new(root: &Path) -> Self {
        let mut policy = Self::default();
        policy.load(root);
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

    pub fn is_ignored(&self, path: &Path) -> bool {
        let rel_path = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let rel_str = rel_path.replace('\\', "/");

        for pattern in &self.ignore_patterns {
            if glob_match(pattern, &rel_str) {
                return true;
            }
        }

        false
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        let rel_path = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

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
        if !prefix.is_empty() && path.starts_with(prefix) {
            return suffix.is_empty() || path.contains(suffix);
        }
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
}