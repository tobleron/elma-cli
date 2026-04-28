//! @efficiency-role: infrastructure
//!
//! Environment Utilities (Task 290)
//!
//! Provides clean environment injection for persistent shell processes.
//! Filters out noisy/sensitive environment variables to create a "clean room"
//! environment that prevents profile output and noise from shell initialization.

use std::collections::HashMap;
use std::env;

/// Environment variables that typically introduce noise or are sensitive
const NOISE_ENV_PATTERNS: &[&str] = &[
    // Shell initialization/profile noise
    "PS1",
    "PS2",
    "PS3",
    "PS4",
    "PROMPT",
    // Color and terminal noise
    "TERM",
    "COLORTERM",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    // Developer/environment-specific noise
    "HISTFILE",
    "HISTSIZE",
    "HISTCONTROL",
    "HISTIGNORE",
    // Python and other interpreter caches
    "PYTHONHASHSEED",
    "PYTHONDONTWRITEBYTECODE",
    // LSP, editor, and tool cache paths
    "EDITOR",
    "VISUAL",
    "PAGER",
    // User-specific customizations that may introduce output
    "LESS",
    "LESSHISTFILE",
    // Language/locale settings that might affect output
    "LC_ALL",
    "LC_NUMERIC",
    "LC_TIME",
    "LC_COLLATE",
    "LC_MONETARY",
    "LC_MESSAGES",
    "LC_PAPER",
    "LC_NAME",
    "LC_ADDRESS",
    "LC_TELEPHONE",
    "LC_MEASUREMENT",
    "LC_IDENTIFICATION",
    // Rust and other toolchain caches
    "RUSTFLAGS",
    "RUST_LOG",
    "RUST_BACKTRACE",
    // Node.js and npm settings
    "NODE_OPTIONS",
    // CI/CD environment variables that can introduce noise
    "CI",
    "CONTINUOUS_INTEGRATION",
    // Git settings that may introduce output
    "GIT_TRACE",
    "GIT_TRACE_PERFORMANCE",
    // Custom shell scripts in common directories
    "BASH_ENV",
    "ENV",
];

/// Sensitive environment variables to filter out
const SENSITIVE_ENV_PATTERNS: &[&str] = &[
    // Credentials
    "TOKEN",
    "PASSWORD",
    "SECRET",
    "KEY",
    "API_KEY",
    "ACCESS_TOKEN",
    "REFRESH_TOKEN",
    // AWS and cloud credentials
    "AWS_ACCESS_KEY",
    "AWS_SECRET_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_",
    "GCP_",
    // SSH and crypto keys
    "SSH_KEY",
    "PRIVATE_KEY",
    // User-specific paths that may reveal identity
    "USER",
    "USERNAME",
    "HOME",
    // GitHub and Git credentials
    "GITHUB_TOKEN",
    "GITLAB_TOKEN",
    "GITEA_TOKEN",
];

/// Get a baseline environment with noise and sensitive variables filtered out.
/// This creates a "clean room" environment suitable for deterministic shell operations.
pub fn get_baseline_environment() -> HashMap<String, String> {
    let mut clean_env = HashMap::new();

    for (key, value) in env::vars() {
        let key_upper = key.to_uppercase();

        // Skip noise patterns
        if NOISE_ENV_PATTERNS.iter().any(|p| key_upper.contains(p)) {
            continue;
        }

        // Skip sensitive patterns
        if SENSITIVE_ENV_PATTERNS.iter().any(|p| key_upper.contains(p)) {
            continue;
        }

        // Keep PATH and other essential variables
        if key == "PATH" || key == "HOME" || key == "PWD" || key == "SHELL" {
            clean_env.insert(key, value);
        }
        // Keep language-related settings for UTF-8 support
        else if key == "LANG" || key == "LC_ALL" {
            clean_env.insert(key, value);
        }
        // Keep workspace and project-related variables
        else if key.starts_with("CARGO_") || key.starts_with("RUST_") {
            continue; // Skip Rust-specific settings
        }
        // Keep reasonable custom vars that don't look noisy
        else if !key.contains("HISTORY") && !key.contains("CACHE") && !key.contains("LOG") {
            clean_env.insert(key, value);
        }
    }

    // Ensure critical variables are present
    if !clean_env.contains_key("PATH") {
        if let Ok(path) = env::var("PATH") {
            clean_env.insert("PATH".to_string(), path);
        }
    }

    if !clean_env.contains_key("HOME") {
        if let Ok(home) = env::var("HOME") {
            clean_env.insert("HOME".to_string(), home);
        }
    }

    clean_env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_environment_has_essential_vars() {
        let env = get_baseline_environment();
        // At minimum, PATH and HOME should be present
        assert!(
            env.contains_key("PATH"),
            "PATH should be in baseline environment"
        );
        assert!(
            env.contains_key("HOME"),
            "HOME should be in baseline environment"
        );
    }

    #[test]
    fn test_baseline_environment_filters_noise() {
        let env = get_baseline_environment();
        // Noisy variables should be filtered
        for pattern in NOISE_ENV_PATTERNS {
            for (key, _) in env.iter() {
                assert!(
                    !key.to_uppercase().contains(pattern),
                    "Variable {} should be filtered out due to noise pattern {}",
                    key,
                    pattern
                );
            }
        }
    }

    #[test]
    fn test_baseline_environment_filters_sensitive() {
        let env = get_baseline_environment();
        // Sensitive variables should be filtered, except essential ones (HOME, PATH, PWD, SHELL, LANG)
        let essential: &[&str] = &["PATH", "HOME", "PWD", "SHELL", "LANG", "LC_ALL"];
        for pattern in SENSITIVE_ENV_PATTERNS {
            for (key, _) in env.iter() {
                if essential.contains(&key.as_str()) {
                    continue;
                }
                assert!(
                    !key.to_uppercase().contains(pattern),
                    "Variable {} should be filtered out due to sensitive pattern {}",
                    key,
                    pattern
                );
            }
        }
    }
}
