//! @efficiency-role: domain-logic
//! Protected paths system — prevents accidental mutation of critical files.
//!
//! Distinguishes read-only access (always allowed) from mutation (blocked by default).
//! Paths are checked in preflight_command before execution reaches permission gate.

use std::path::Path;

/// Protected paths and their access levels.
pub struct ProtectedPaths;

impl ProtectedPaths {
    /// Check whether a mutation operation on the given path should be blocked.
    /// Returns an explanation if blocked, None if the operation is allowed.
    pub fn check_mutation(path: &str) -> Option<String> {
        let path_lower = path.to_lowercase();

        // Always protect .git directory
        if path_lower.contains(".git") || path_lower.starts_with(".git/") || path_lower == ".git" {
            return Some(format!(
                "Path '{}' is protected: .git/ contains version history. Use git commands instead.",
                path
            ));
        }

        // Protect DOTFILE config files at workspace root
        let protected_dotfiles = [
            ".gitignore",
        ];
        for dotfile in &protected_dotfiles {
            if path == *dotfile || path.starts_with(&format!("{}/", dotfile)) {
                return Some(format!(
                    "Path '{}' is protected: {} is a critical workspace configuration file.",
                    path, dotfile
                ));
            }
        }

        None
    }

    /// Check whether any path in a shell command hits a protected path.
    pub fn check_command_paths(command: &str) -> Option<String> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        for part in &parts {
            if part.starts_with('/') || part.starts_with('.') || part.contains("..") {
                continue;
            }
            let path = Path::new(part);
            if path.exists() || path.extension().is_some() {
                if let Some(msg) = Self::check_mutation(part) {
                    return Some(msg);
                }
            }
        }
        None
    }
}
