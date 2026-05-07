//! @efficiency-role: domain-logic
//!
//! Safe File Operation Planning And Verification — Task 692.
//!
//! Provides deterministic source-tree backup/copy operations that
//! preserve directory hierarchy, exclude generated paths, record
//! manifests, and verify file counts against source queries.

use crate::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of a safe backup operation.
#[derive(Debug, Clone)]
pub(crate) struct BackupResult {
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub files_copied: usize,
    pub source_match_count: usize,
    pub manifest_path: PathBuf,
    pub errors: Vec<String>,
    pub completed: bool,
}

    /// Configuration for a safe backup/copy operation.
    #[derive(Debug, Clone)]
    pub(crate) struct BackupConfig {
        pub source_dir: PathBuf,
        pub dest_dir: PathBuf,
        pub include_patterns: Vec<String>,
        pub exclude_patterns: Vec<String>,
        pub verify_count: bool,
        pub preserve_hierarchy: bool,
    }

    impl Default for BackupConfig {
        fn default() -> Self {
            Self {
                source_dir: PathBuf::from("."),
                dest_dir: PathBuf::from("project_tmp/backup"),
                include_patterns: vec!["**/*.rs".to_string()],
                exclude_patterns: vec![
                    ".git".to_string(),
                    "target".to_string(),
                    "node_modules".to_string(),
                    ".trash".to_string(),
                    "sessions".to_string(),
                    "project_tmp".to_string(),
                    "_knowledge_base".to_string(),
                    ".crush".to_string(),
                    ".dirac-symbol-index".to_string(),
                    ".kilo".to_string(),
                    "_dev-system".to_string(),
                    "_elma-tasks".to_string(),
                    ".opencode".to_string(),
                ],
                verify_count: true,
                preserve_hierarchy: true,
            }
        }
    }

/// Run a safe backup operation. Returns a BackupResult with details.
///
/// Steps:
/// 1. Create destination directory
/// 2. Find all matching source files (respecting exclusions)
/// 3. Copy preserving relative path hierarchy
/// 4. Write manifest
/// 5. Verify count if configured
pub(crate) fn run_safe_backup(config: &BackupConfig) -> Result<BackupResult> {
    let source_dir = std::fs::canonicalize(&config.source_dir)
        .with_context(|| format!("source directory not found: {}", config.source_dir.display()))?;
    let dest_dir = config.dest_dir.clone();
    let manifest_path = dest_dir.join("backup_manifest.txt");

    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("create destination: {}", dest_dir.display()))?;

    // Build exclude set
    let exclude_set: HashSet<&str> = config.exclude_patterns.iter().map(|s| s.as_str()).collect();

    // Walk source directory and collect matching files
    let mut files_to_copy: Vec<PathBuf> = Vec::new();
    collect_files_recursive(&source_dir, &source_dir, &exclude_set, &config.include_patterns, &mut files_to_copy);

    let source_match_count = files_to_copy.len();
    let mut errors: Vec<String> = Vec::new();
    let mut files_copied = 0usize;
    let mut manifest_lines: Vec<String> = Vec::new();

    // Create unique run identifier
    let run_id = format!("backup_{:x}", {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        nanos
    });

    // Copy files preserving hierarchy
    for src_path in &files_to_copy {
        let relative = src_path
            .strip_prefix(&source_dir)
            .unwrap_or(src_path);
        let dest_path = if config.preserve_hierarchy {
            dest_dir.join(relative)
        } else {
            let filename = src_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            dest_dir.join(format!("{}_{}", run_id, filename))
        };

        // Create parent directory if preserving hierarchy
        if config.preserve_hierarchy {
            if let Some(parent) = dest_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    errors.push(format!("mkdir {}: {}", parent.display(), e));
                    continue;
                }
            }
        }

        // Copy file
        match std::fs::copy(src_path, &dest_path) {
            Ok(_) => {
                files_copied += 1;
                manifest_lines.push(format!("{} -> {}", src_path.display(), dest_path.display()));
            }
            Err(e) => {
                errors.push(format!("copy {}: {}", src_path.display(), e));
            }
        }
    }

    // Write manifest
    let manifest_content = format!(
        "Backup Manifest\n\
         Source: {}\n\
         Destination: {}\n\
         Files Matched: {}\n\
         Files Copied: {}\n\
         Errors: {}\n\
         Preserve Hierarchy: {}\n\
         ---\n{}\n",
        source_dir.display(),
        dest_dir.display(),
        source_match_count,
        files_copied,
        errors.len(),
        config.preserve_hierarchy,
        manifest_lines.join("\n"),
    );
    if let Err(e) = std::fs::write(&manifest_path, &manifest_content) {
        errors.push(format!("write manifest: {}", e));
    }

    let completed = errors.is_empty() || files_copied > 0;

    Ok(BackupResult {
        source_dir,
        dest_dir,
        files_copied,
        source_match_count,
        manifest_path,
        errors,
        completed,
    })
}

/// Check if a path matches any of the glob-like include patterns.
fn matches_pattern(path: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }
    let path_str = path.to_string_lossy().replace('\\', "/");
    for pattern in patterns {
        if simple_glob_match(pattern, &path_str) {
            return true;
        }
    }
    false
}

/// Recursively collect files matching include patterns, excluding known paths.
fn collect_files_recursive(
    root: &Path,
    dir: &Path,
    exclude_set: &HashSet<&str>,
    include_patterns: &[String],
    files: &mut Vec<PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let file_name = entry.file_name().to_string_lossy().to_string();
        if exclude_set.contains(file_name.as_str()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(root, &path, exclude_set, include_patterns, files);
        } else if path.is_file() && matches_pattern(&path, include_patterns) {
            files.push(path);
        }
    }
}

/// Simple glob matching (supports `**/*.rs`, `*.rs`, `src/**/*.rs` patterns).
fn simple_glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "**/*" || pattern == "*" {
        return true;
    }

    // Handle `**/*.ext` patterns
    if let Some(ext) = pattern.strip_prefix("**/*") {
        return path.ends_with(ext);
    }

    // Handle `*.ext` patterns — match filename only, not paths with separators
    if let Some(ext) = pattern.strip_prefix('*') {
        if !ext.contains('*') && !path.contains('/') {
            return path.ends_with(ext);
        }
    }

    // Handle `dirname/**/*.ext` patterns
    if let Some((prefix, suffix)) = pattern.split_once("/**/*") {
        return path.starts_with(prefix) && path.ends_with(suffix);
    }

    pattern == path
}

/// Convenience entry point for running a backup operation from the tool pipeline (Task 705).
/// Accepts source directory, destination directory, include globs, and optional extra exclude globs.
pub(crate) fn run_backup_operation(
    source_dir: &Path,
    dest_dir: &Path,
    include_patterns: &[String],
    extra_excludes: &[String],
) -> Result<BackupResult> {
    let mut excludes = BackupConfig::default().exclude_patterns;
    for ex in extra_excludes {
        if !excludes.contains(ex) {
            excludes.push(ex.clone());
        }
    }
    let config = BackupConfig {
        source_dir: source_dir.to_path_buf(),
        dest_dir: dest_dir.to_path_buf(),
        include_patterns: include_patterns.to_vec(),
        exclude_patterns: excludes,
        ..Default::default()
    };
    run_safe_backup(&config)
}

/// Count files matching the backup config without actually copying (Task 705).
/// Useful for preflight verification.
pub(crate) fn backup_file_count(config: &BackupConfig) -> usize {
    let source_dir = match std::fs::canonicalize(&config.source_dir) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    let exclude_set: std::collections::HashSet<&str> =
        config.exclude_patterns.iter().map(|s| s.as_str()).collect();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    collect_files_recursive(&source_dir, &source_dir, &exclude_set, &config.include_patterns, &mut files);
    files.len()
}

/// Verify backup result: compare actual files copied against count from source query.
pub(crate) fn verify_backup(result: &BackupResult) -> Vec<String> {
    let mut issues = Vec::new();

    if result.source_match_count > 0 && result.files_copied == 0 {
        issues.push(format!(
            "CRITICAL: {} files matched but 0 were copied",
            result.source_match_count
        ));
    }

    if result.source_match_count != result.files_copied {
        issues.push(format!(
            "Count mismatch: {} files matched source but {} were copied ({} missing)",
            result.source_match_count,
            result.files_copied,
            result.source_match_count.saturating_sub(result.files_copied)
        ));
    }

    if !result.errors.is_empty() {
        issues.push(format!("Errors during backup ({}): {:?}", result.errors.len(), result.errors));
    }

    // Check for potential basename collisions when not preserving hierarchy
    if !result.completed {
        issues.push("Backup did not complete successfully".to_string());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_glob_match_full() {
        assert!(simple_glob_match("**/*.rs", "src/main.rs"));
        assert!(simple_glob_match("**/*.rs", "src/util/helper.rs"));
        assert!(!simple_glob_match("**/*.rs", "src/main.py"));
    }

    #[test]
    fn test_simple_glob_match_extension() {
        assert!(simple_glob_match("*.rs", "main.rs"));
        assert!(!simple_glob_match("*.rs", "src/main.rs"));
        assert!(!simple_glob_match("*.rs", "main.py"));
    }

    #[test]
    fn test_simple_glob_match_dir_pattern() {
        assert!(simple_glob_match("src/**/*.rs", "src/main.rs"));
        assert!(simple_glob_match("src/**/*.rs", "src/util/helper.rs"));
        assert!(!simple_glob_match("src/**/*.rs", "tests/test.rs"));
    }

    #[test]
    fn test_simple_glob_wildcard_all() {
        assert!(simple_glob_match("**/*", "anything/file.txt"));
    }

    #[test]
    fn test_matches_pattern_empty() {
        let path = Path::new("src/main.rs");
        assert!(matches_pattern(path, &[]));
    }

    #[test]
    fn test_matches_pattern_with_patterns() {
        let path = Path::new("src/main.rs");
        let patterns = vec!["**/*.rs".to_string()];
        assert!(matches_pattern(path, &patterns));

        let path2 = Path::new("src/main.py");
        assert!(!matches_pattern(path2, &patterns));
    }

    #[test]
    fn test_backup_config_defaults() {
        let config = BackupConfig::default();
        assert!(config.include_patterns.contains(&"**/*.rs".to_string()));
        assert!(config.exclude_patterns.contains(&".git".to_string()));
        assert!(config.verify_count);
        assert!(config.preserve_hierarchy);
    }

    #[test]
    fn test_verify_backup_perfect() {
        let result = BackupResult {
            source_dir: PathBuf::from("src"),
            dest_dir: PathBuf::from("backup"),
            files_copied: 10,
            source_match_count: 10,
            manifest_path: PathBuf::from("backup/manifest.txt"),
            errors: vec![],
            completed: true,
        };
        let issues = verify_backup(&result);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_verify_backup_mismatch() {
        let result = BackupResult {
            source_dir: PathBuf::from("src"),
            dest_dir: PathBuf::from("backup"),
            files_copied: 5,
            source_match_count: 10,
            manifest_path: PathBuf::from("backup/manifest.txt"),
            errors: vec![],
            completed: true,
        };
        let issues = verify_backup(&result);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("Count mismatch"));
    }

    #[test]
    fn test_verify_backup_zero_copied() {
        let result = BackupResult {
            source_dir: PathBuf::from("src"),
            dest_dir: PathBuf::from("backup"),
            files_copied: 0,
            source_match_count: 10,
            manifest_path: PathBuf::from("backup/manifest.txt"),
            errors: vec![],
            completed: true,
        };
        let issues = verify_backup(&result);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("CRITICAL"));
    }

    #[test]
    fn test_verify_backup_not_completed() {
        let result = BackupResult {
            source_dir: PathBuf::from("src"),
            dest_dir: PathBuf::from("backup"),
            files_copied: 3,
            source_match_count: 5,
            manifest_path: PathBuf::from("backup/manifest.txt"),
            errors: vec!["permission denied".to_string()],
            completed: false,
        };
        let issues = verify_backup(&result);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.contains("did not complete")));
    }

    #[test]
    fn test_backup_creates_manifest() {
        let tmp = std::env::temp_dir().join(format!("test_backup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("source");
        let dst = tmp.join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("test.rs"), "fn main() {}").unwrap();
        std::fs::write(src.join("test.py"), "print('hello')").unwrap();

        let config = BackupConfig {
            source_dir: src.clone(),
            dest_dir: dst.clone(),
            include_patterns: vec!["**/*.rs".to_string()],
            ..Default::default()
        };

        let result = run_safe_backup(&config).unwrap();
        assert_eq!(result.source_match_count, 1);
        assert_eq!(result.files_copied, 1);
        assert!(result.manifest_path.exists());
        assert!(dst.join("test.rs").exists());
        assert!(!dst.join("test.py").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_backup_preserves_hierarchy() {
        let tmp = std::env::temp_dir().join(format!("test_backup_hier_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("source");
        let dst = tmp.join("dest");
        std::fs::create_dir_all(src.join("sub/dir")).unwrap();
        std::fs::write(src.join("sub/dir/file.rs"), "fn f() {}").unwrap();

        let config = BackupConfig {
            source_dir: src.clone(),
            dest_dir: dst.clone(),
            include_patterns: vec!["**/*.rs".to_string()],
            ..Default::default()
        };

        let result = run_safe_backup(&config).unwrap();
        assert_eq!(result.files_copied, 1);
        assert!(dst.join("sub/dir/file.rs").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_backup_excludes_default_paths() {
        let tmp = std::env::temp_dir().join(format!("test_backup_excl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("source");
        let dst = tmp.join("dest");
        std::fs::create_dir_all(src.join(".git")).unwrap();
        std::fs::create_dir_all(src.join("src")).unwrap();
        std::fs::write(src.join(".git/config"), "[core]").unwrap();
        std::fs::write(src.join("src/main.rs"), "fn main() {}").unwrap();

        let config = BackupConfig {
            source_dir: src.clone(),
            dest_dir: dst.clone(),
            ..Default::default()
        };

        let result = run_safe_backup(&config).unwrap();
        assert_eq!(result.files_copied, 1);
        assert!(dst.join("src/main.rs").exists());
        assert!(!dst.join(".git/config").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
