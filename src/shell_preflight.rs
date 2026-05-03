//! @efficiency-role: domain-logic
//!
//! Shell Preflight (Tasks 116, 118, 119, 120)
//!
//! Validates shell commands before execution:
//! - Classifies commands by risk level (safe/caution/dangerous)
//! - Validates source/destination paths for mv/cp/rm
//! - Detects unscoped patterns (find . without -maxdepth, rm *)
//! - Dry-run preview for destructive commands (Task 119)
//! - Estimates match count for bulk operations
//! - Protects critical directories from mutation
//! - Returns specific error guidance to model on failure

use crate::*;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

/// Confirmation cache: commands that have been confirmed after dry-run preview.
static CONFIRMED_COMMANDS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Threshold: warn about unscoped operations affecting this many files.
const UNSCOPED_WARN_THRESHOLD: usize = 20;
/// Threshold: block unscoped operations affecting this many files.
const UNSCOPED_BLOCK_THRESHOLD: usize = 100;

/// Clear the confirmation cache (called on session reset).
pub(crate) fn clear_confirmation_cache() {
    if let Ok(mut cache) = CONFIRMED_COMMANDS.lock() {
        cache.clear();
    }
}

/// Confirm a command for one-time execution (after dry-run preview).
pub(crate) fn confirm_command(command: &str) {
    if let Ok(mut cache) = CONFIRMED_COMMANDS.lock() {
        cache.insert(command.to_string());
    }
}

/// Check if a command has been confirmed.
fn is_confirmed(command: &str) -> bool {
    if let Ok(cache) = CONFIRMED_COMMANDS.lock() {
        cache.contains(command)
    } else {
        false
    }
}

/// Risk level of a shell command.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RiskLevel {
    Safe,
    Caution,
    Dangerous(String),
}

/// Preflight validation result.
#[derive(Debug, Clone)]
pub(crate) struct PreflightResult {
    pub(crate) risk: RiskLevel,
    pub(crate) error_guidance: Option<String>,
    /// Task 119: Dry-run preview shown before destructive execution.
    /// If present, command requires confirmation.
    pub(crate) dry_run_preview: Option<String>,
}

impl PreflightResult {
    pub(crate) fn can_execute(&self) -> bool {
        self.error_guidance.is_none() && self.dry_run_preview.is_none()
    }
}

/// Unscoped detection result.
#[derive(Debug, Clone)]
pub(crate) struct UnscopedResult {
    pub(crate) is_unscoped: bool,
    pub(crate) estimated_count: usize,
    pub(crate) suggestion: Option<String>,
}

const DESTRUCTIVE_PATTERNS: &[(&str, &str)] = &[
    ("rm ", "rm: removes files permanently"),
    ("rmdir ", "rmdir: removes directories permanently"),
    (
        "git reset --hard",
        "git reset --hard: discards all uncommitted changes",
    ),
    (
        "git clean -f",
        "git clean: removes untracked files permanently",
    ),
    ("dd if=", "dd: low-level disk write, extremely dangerous"),
    ("> ", "redirect truncates file"),
    (">> ", "redirect appends to file"),
    (
        "chmod -R 777",
        "chmod -R 777: opens all permissions recursively",
    ),
];

const PIPE_DESTRUCTIVE_PATTERNS: &[(&str, &str)] = &[
    ("| xargs rm", "pipe-to-xargs-rm: bulk deletion"),
    ("| xargs mv", "pipe-to-xargs-mv: bulk move"),
    ("| xargs cp ", "pipe-to-xargs-cp: bulk copy overwrite"),
    (
        "| xargs chmod",
        "pipe-to-xargs-chmod: bulk permission change",
    ),
    (
        "| xargs chown",
        "pipe-to-xargs-chown: bulk ownership change",
    ),
    (
        "| xargs truncate",
        "pipe-to-xargs-truncate: bulk file truncation",
    ),
    ("| xargs shred", "pipe-to-xargs-shred: bulk secure deletion"),
];

/// Commands that are safe to run in bulk via pipe (read-only inspection).
const PIPE_SAFE_READ_ONLY: &[&str] = &[
    "| xargs stat",
    "| xargs ls",
    "| xargs file",
    "| xargs wc",
    "| xargs head",
    "| xargs tail",
    "| xargs md5",
    "| xargs shasum",
    "| xargs sha256",
    "| xargs cat",
    "| xargs grep",
    "| xargs rg",
];

/// Destructive keywords that make a while-read loop dangerous.
const WHILE_LOOP_DESTRUCTIVE_KEYWORDS: &[&str] = &[
    "rm ",
    "rm;",
    "rm\n",
    "mv ",
    "mv;",
    "mv\n",
    "cp ",
    "cp;",
    "cp\n",
    "chmod ",
    "chmod;",
    "chmod\n",
    "chown ",
    "chown;",
    "chown\n",
    "truncate ",
    "truncate;",
    "shred ",
    "shred;",
    "> ",
    ">> ",
];

pub(crate) fn classify_command(command: &str) -> RiskLevel {
    let cmd = command.trim();
    if cmd.is_empty() {
        return RiskLevel::Safe;
    }

    // DESTRUCTIVE must be evaluated before safe overrides
    for (pattern, reason) in PIPE_DESTRUCTIVE_PATTERNS {
        if cmd.contains(pattern) {
            return RiskLevel::Dangerous(format!("BULK DESTRUCTIVE: {} pattern detected.", reason));
        }
    }

    // While-read loops
    if cmd.contains("| while read") || cmd.contains("|while read") {
        for keyword in WHILE_LOOP_DESTRUCTIVE_KEYWORDS {
            if cmd.contains(keyword) {
                return RiskLevel::Dangerous(
                    "BULK DESTRUCTIVE: while-read loop contains destructive operation.".to_string(),
                );
            }
        }
        return RiskLevel::Safe;
    }

    // Safe read-only pipes (only relevant when no destructive pipe found above)
    for pattern in PIPE_SAFE_READ_ONLY {
        if cmd.contains(pattern) {
            return RiskLevel::Safe;
        }
    }

    for (pattern, reason) in DESTRUCTIVE_PATTERNS {
        if cmd.starts_with(pattern)
            || cmd.contains(&format!("; {}", pattern))
            || cmd.contains(&format!("&& {}", pattern))
        {
            return RiskLevel::Dangerous(format!("DANGEROUS: {}", reason));
        }
    }

    if cmd.starts_with("mv ") {
        return RiskLevel::Caution;
    }
    if cmd.starts_with("cp ") {
        return RiskLevel::Caution;
    }
    if cmd.contains("rm *") || (cmd.starts_with("rm ") && cmd.contains('*')) {
        return RiskLevel::Dangerous("rm with glob: may delete many files".to_string());
    }

    let safe_prefixes = [
        "ls ",
        "ls\n",
        "ls",
        "cat ",
        "head ",
        "tail ",
        "wc ",
        "echo ",
        "rg ",
        "grep ",
        "pwd",
        "pwd ",
        "whoami",
        "whoami ",
        "date",
        "tree ",
        "find ",
        "stat ",
        "file ",
        "du ",
        "df ",
        "du -sh",
        "find . -type f",
        "git status",
        "git log",
        "git diff",
        "git branch",
        "cargo build",
        "cargo test",
        "cargo check",
    ];
    for prefix in &safe_prefixes {
        if cmd.starts_with(prefix) {
            return RiskLevel::Safe;
        }
    }

    // Detect analytical find + du + wc patterns (read-only aggregation queries)
    if cmd.contains("find . -type f") && cmd.contains("| wc -l") {
        return RiskLevel::Safe;
    }
    if cmd.contains("find . -type f") && cmd.contains("du -ch") && cmd.contains("tail -1") {
        return RiskLevel::Safe;
    }
    if cmd.contains("du -sh") && cmd.contains("find . -type f") {
        return RiskLevel::Safe;
    }

    RiskLevel::Caution
}

pub(crate) fn detect_unscoped(command: &str, workdir: &PathBuf) -> UnscopedResult {
    let cmd = command.trim();
    if let Some(r) = check_find_unscoped(cmd, workdir) {
        return r;
    }
    if let Some(r) = check_glob_unscoped(cmd, workdir) {
        return r;
    }
    UnscopedResult {
        is_unscoped: false,
        estimated_count: 0,
        suggestion: None,
    }
}

fn check_find_unscoped(cmd: &str, workdir: &PathBuf) -> Option<UnscopedResult> {
    if !cmd.starts_with("find ") || cmd.contains("-maxdepth") {
        return None;
    }
    if !cmd.contains(". ") && !cmd.contains("./") {
        return None;
    }

    let count = estimate_find_count(cmd, workdir);
    if count < UNSCOPED_WARN_THRESHOLD {
        return None;
    }

    let suggestion = if cmd.contains("-name ") || cmd.contains("-iname ") {
        format!("This find command may match {} files. Consider adding `-maxdepth 1` to limit to the current directory.", count)
    } else {
        format!("This find command may match {} files. Consider: (1) add `-maxdepth 1`, (2) use `-name '*.ext'`, or (3) specify a narrower path.", count)
    };

    Some(UnscopedResult {
        is_unscoped: true,
        estimated_count: count,
        suggestion: Some(suggestion),
    })
}

fn check_glob_unscoped(cmd: &str, workdir: &PathBuf) -> Option<UnscopedResult> {
    if !cmd.contains("*") {
        return None;
    }
    let count = match std::fs::read_dir(workdir) {
        Ok(entries) => entries.count(),
        Err(_) => return None,
    };
    if count < UNSCOPED_WARN_THRESHOLD {
        return None;
    }

    if cmd.starts_with("mv ") || cmd.starts_with("cp ") {
        return Some(UnscopedResult {
            is_unscoped: true, estimated_count: count,
            suggestion: Some(format!("Glob pattern (*) may expand to {} files. Use specific file names or `find` with filters.", count)),
        });
    }
    if cmd.starts_with("rm ") {
        return Some(UnscopedResult {
            is_unscoped: true, estimated_count: count,
            suggestion: Some(format!("Glob pattern (*) may match {} files for deletion. Use `find` with `-name` filters.", count)),
        });
    }
    None
}

fn estimate_find_count(cmd: &str, workdir: &PathBuf) -> usize {
    let dry_cmd = format!("{} -print 2>/dev/null | wc -l", cmd);
    match crate::program_utils::run_shell_persistent_sync(&dry_cmd, workdir) {
        Ok(r) => r.inline_text.trim().parse::<usize>().unwrap_or(0),
        Err(_) => 0,
    }
}

/// Task 119: Generate dry-run preview for destructive commands.
/// Returns a preview of what the command would do, without executing it.
fn generate_dry_run_preview(command: &str, workdir: &PathBuf) -> Option<String> {
    let cmd = command.trim();

    if let Some(args) = cmd.strip_prefix("rm ") {
        return dry_run_rm(args, workdir);
    }
    if let Some(args) = cmd.strip_prefix("mv ") {
        return dry_run_mv(args, workdir);
    }
    if let Some(args) = cmd.strip_prefix("cp ") {
        return dry_run_cp(args, workdir);
    }

    None
}

fn dry_run_rm(args: &str, workdir: &PathBuf) -> Option<String> {
    // Parse args to find files/globs
    let parts: Vec<String> = try_parse_shlex(args)?;
    let parts: Vec<&str> = parts
        .iter()
        .map(|s| s.as_str())
        .filter(|a| !a.starts_with('-'))
        .collect();
    if parts.is_empty() {
        return None;
    }

    let mut files: Vec<String> = Vec::new();
    for pat in &parts {
        let full = resolve_path(pat, workdir);
        if pat.contains('*') || pat.contains('?') {
            // Glob pattern
            if let Ok(entries) = std::fs::read_dir(full.parent().unwrap_or(workdir)) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if glob_match(pat, &name) {
                        files.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        } else if full.exists() {
            files.push(full.to_string_lossy().to_string());
        }
    }

    if files.is_empty() {
        return Some("Dry-run: No files match the given pattern.".to_string());
    }

    let preview = if files.len() > 20 {
        let first: String = files
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ");
        format!(
            "Dry-run: This would delete {} files:\n  {}\n  ... and {} more",
            files.len(),
            first,
            files.len() - 10
        )
    } else {
        let list = files.join("\n  ");
        format!(
            "Dry-run: This would delete {} file(s):\n  {}",
            files.len(),
            list
        )
    };
    Some(preview)
}

fn dry_run_mv(args: &str, workdir: &PathBuf) -> Option<String> {
    let parts = try_parse_shlex(args)?;
    if parts.len() < 2 {
        return None;
    }

    let (sources, dest) = (&parts[0], &parts[1]);
    let dest_path = resolve_path(dest, workdir);

    if sources.contains('*') || sources.contains('?') {
        // Glob pattern
        let parent = workdir.clone();
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if glob_match(sources, &name) {
                    files.push(name);
                }
            }
        }
        if files.is_empty() {
            return Some(format!(
                "Dry-run: No files match '{}' in current directory.",
                sources
            ));
        }
        return Some(format!(
            "Dry-run: This would move {} file(s) to {}:\n  {}",
            files.len(),
            dest_path.display(),
            files.join("\n  ")
        ));
    }

    let src = resolve_path(sources, workdir);
    if !src.exists() {
        return Some(format!("Dry-run: Source '{}' does not exist.", sources));
    }

    if dest.ends_with('/') || dest_path.is_dir() {
        Some(format!(
            "Dry-run: This would move '{}' → '{}'",
            src.display(),
            dest_path.join(src.file_name()?).display()
        ))
    } else {
        Some(format!(
            "Dry-run: This would move '{}' → '{}'",
            src.display(),
            dest_path.display()
        ))
    }
}

fn dry_run_cp(args: &str, workdir: &PathBuf) -> Option<String> {
    let parts = try_parse_shlex(args)?;
    if parts.len() < 2 {
        return None;
    }
    let src = resolve_path(&parts[0], workdir);
    let dest = resolve_path(&parts[1], workdir);
    if !src.exists() {
        return Some(format!("Dry-run: Source '{}' does not exist.", parts[0]));
    }
    Some(format!(
        "Dry-run: This would copy '{}' → '{}'",
        src.display(),
        dest.display()
    ))
}

fn try_parse_shlex(args: &str) -> Option<Vec<String>> {
    shlex::split(args)
}

fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return false;
    }

    let Some(remaining) = name.strip_prefix(parts[0]) else {
        return false;
    };
    let mut pos = remaining;

    for segment in &parts[1..parts.len() - 1] {
        if segment.is_empty() {
            continue;
        }
        let Some(idx) = pos.find(segment) else {
            return false;
        };
        pos = &pos[idx + segment.len()..];
    }

    if let Some(last) = parts.last() {
        if !last.is_empty() && !pos.ends_with(last) {
            return false;
        }
    }
    true
}

fn resolve_path(path: &str, workdir: &PathBuf) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workdir.join(p)
    }
}

/// Preflight validation: validate paths before executing destructive commands.
pub(crate) fn preflight_command(command: &str, workdir: &PathBuf) -> PreflightResult {
    let risk = classify_command(command);

    if let RiskLevel::Dangerous(reason) = &risk {
        return PreflightResult {
            risk: risk.clone(),
            error_guidance: Some(format!(
                "Command blocked: {}. Use a safer approach or be more specific.",
                reason
            )),
            dry_run_preview: None,
        };
    }

    if let Some(msg) = check_protected_paths(command) {
        return PreflightResult {
            risk: RiskLevel::Dangerous("protected_path".to_string()),
            error_guidance: Some(msg),
            dry_run_preview: None,
        };
    }

    let unscoped = detect_unscoped(command, workdir);
    if unscoped.is_unscoped && unscoped.estimated_count >= UNSCOPED_BLOCK_THRESHOLD {
        return PreflightResult {
            risk: RiskLevel::Dangerous("unscoped_bulk".to_string()),
            error_guidance: Some(format!(
                "Command blocked: unscoped bulk operation estimated to affect {} files. {}\nSuggested fix: {}",
                unscoped.estimated_count,
                unscoped.suggestion.as_deref().unwrap_or("Use more specific patterns."),
                suggest_scoped_alternative(command)
            )),
            dry_run_preview: None,
        };
    }

    if let Some(mv_args) = command.strip_prefix("mv ") {
        return preflight_mv(mv_args, workdir);
    }
    if let Some(cp_args) = command.strip_prefix("cp ") {
        return preflight_cp(cp_args, workdir);
    }
    if let Some(rm_args) = command.strip_prefix("rm ") {
        return preflight_rm(rm_args, workdir);
    }

    // Warn about unscoped operations that are under the block threshold
    // (suggestion goes to trace only, not blocking)
    if unscoped.is_unscoped {
        return PreflightResult {
            risk: RiskLevel::Safe,
            error_guidance: None,
            dry_run_preview: None,
        };
    }

    // Task 119: For destructive commands that pass basic checks, show dry-run preview
    // (skip if already confirmed — model saw preview and re-issued the command)
    if matches!(risk, RiskLevel::Caution) && !is_confirmed(command) {
        if let Some(preview) = generate_dry_run_preview(command, workdir) {
            return PreflightResult {
                risk: risk.clone(),
                error_guidance: None,
                dry_run_preview: Some(preview),
            };
        }
    }

    PreflightResult {
        risk,
        error_guidance: None,
        dry_run_preview: None,
    }
}

fn check_protected_paths(command: &str) -> Option<String> {
    let cmd = command.trim();

    let path_args: Option<Vec<&str>> = if let Some(args) = cmd.strip_prefix("rm ") {
        Some(args.split_whitespace().filter(|a| !a.starts_with('-')).collect())
    } else if let Some(args) = cmd.strip_prefix("mv ") {
        Some(args.split_whitespace().filter(|a| !a.starts_with('-')).collect())
    } else if let Some(args) = cmd.strip_prefix("cp ") {
        Some(args.split_whitespace().filter(|a| !a.starts_with('-')).collect())
    } else if let Some(args) = cmd.strip_prefix("chmod ") {
        let parts: Vec<&str> = args.split_whitespace().filter(|a| !a.starts_with('-')).collect();
        Some(parts.into_iter().skip(1).collect())
    } else if let Some(args) = cmd.strip_prefix("chown ") {
        let parts: Vec<&str> = args.split_whitespace().filter(|a| !a.starts_with('-')).collect();
        Some(parts.into_iter().skip(1).collect())
    } else {
        None
    };

    if let Some(targets) = path_args {
        for target in targets {
            if let Some(msg) = crate::protected_paths::ProtectedPaths::check_mutation(target) {
                return Some(msg);
            }
        }
    }
    None
}

fn suggest_scoped_alternative(command: &str) -> String {
    let cmd = command.trim();
    if cmd.starts_with("find ") && (cmd.contains("./") || cmd.contains(". ")) {
        return format!(
            "find . -maxdepth 1{}",
            cmd.strip_prefix("find .").unwrap_or("")
        );
    }
    if cmd.contains('*') {
        if cmd.starts_with("mv ") {
            return "Use `find . -name 'PATTERN' -exec mv {} dest/ \\;`".to_string();
        }
        if cmd.starts_with("rm ") {
            return "Use `find . -name 'PATTERN' -delete`".to_string();
        }
    }
    "Add `-maxdepth` or specific file patterns to limit scope".to_string()
}

fn preflight_mv(args: &str, workdir: &PathBuf) -> PreflightResult {
    let parts = match try_parse_shlex(args) {
        Some(p) => p,
        None => {
            return PreflightResult {
                risk: RiskLevel::Caution,
                error_guidance: Some("invalid shell quoting in arguments".to_string()),
                dry_run_preview: generate_dry_run_preview(&format!("mv {}", args), workdir),
            };
        }
    };
    if parts.len() < 2 {
        return PreflightResult {
            risk: RiskLevel::Caution,
            error_guidance: Some("mv requires at least source and destination.".to_string()),
            dry_run_preview: generate_dry_run_preview(&format!("mv {}", args), workdir),
        };
    }
    let source = &parts[0];
    let dest = &parts[1];
    let source_path = resolve_path(source, workdir);
    if !source_path.exists() {
        return PreflightResult {
            risk: RiskLevel::Dangerous("source_not_found".to_string()),
            error_guidance: Some(format!("Source '{}' does not exist.", source)),
            dry_run_preview: None,
        };
    }
    let dest_path = resolve_path(dest, workdir);
    let parent = if dest.ends_with('/') || dest_path.is_dir() {
        &dest_path
    } else {
        dest_path.parent().unwrap_or(&dest_path)
    };
    if !parent.exists() {
        return PreflightResult {
            risk: RiskLevel::Dangerous("destination_not_found".to_string()),
            error_guidance: Some(format!(
                "Destination directory '{}' does not exist. Create it with `mkdir -p {}`.",
                parent.display(),
                parent.display()
            )),
            dry_run_preview: None,
        };
    }
    PreflightResult {
        risk: RiskLevel::Caution,
        error_guidance: None,
        dry_run_preview: generate_dry_run_preview(&format!("mv {}", args), workdir),
    }
}

fn preflight_cp(args: &str, workdir: &PathBuf) -> PreflightResult {
    let parts = match try_parse_shlex(args) {
        Some(p) => p,
        None => {
            return PreflightResult {
                risk: RiskLevel::Caution,
                error_guidance: Some("invalid shell quoting in arguments".to_string()),
                dry_run_preview: generate_dry_run_preview(&format!("cp {}", args), workdir),
            };
        }
    };
    if parts.len() < 2 {
        return PreflightResult {
            risk: RiskLevel::Caution,
            error_guidance: Some("cp requires at least source and destination.".to_string()),
            dry_run_preview: generate_dry_run_preview(&format!("cp {}", args), workdir),
        };
    }
    let source = &parts[0];
    let dest = &parts[1];
    let source_path = resolve_path(source, workdir);
    if !source_path.exists() {
        return PreflightResult {
            risk: RiskLevel::Dangerous("source_not_found".to_string()),
            error_guidance: Some(format!("Source '{}' does not exist.", source)),
            dry_run_preview: None,
        };
    }
    if dest.ends_with('/') {
        let dest_path = resolve_path(dest, workdir);
        if !dest_path.exists() {
            return PreflightResult {
                risk: RiskLevel::Dangerous("destination_not_found".to_string()),
                error_guidance: Some(format!(
                    "Destination directory '{}' does not exist.",
                    dest_path.display()
                )),
                dry_run_preview: None,
            };
        }
    }
    PreflightResult {
        risk: RiskLevel::Caution,
        error_guidance: None,
        dry_run_preview: generate_dry_run_preview(&format!("cp {}", args), workdir),
    }
}

fn preflight_rm(args: &str, workdir: &PathBuf) -> PreflightResult {
    if args.contains("*") && !args.contains("--") {
        return PreflightResult {
            risk: RiskLevel::Dangerous("rm with glob".to_string()),
            error_guidance: Some(
                "rm with wildcard (*) detected. Use `find` first to see what would be deleted."
                    .to_string(),
            ),
            dry_run_preview: generate_dry_run_preview(&format!("rm {}", args), workdir),
        };
    }
    if args.contains("-r") || args.contains("-R") || args.contains("--recursive") {
        return PreflightResult {
            risk: RiskLevel::Dangerous("rm -r".to_string()),
            error_guidance: Some(
                "rm -r (recursive delete) is dangerous. Use `ls -R` first to verify contents."
                    .to_string(),
            ),
            dry_run_preview: generate_dry_run_preview(&format!("rm {}", args), workdir),
        };
    }
    let parts = match try_parse_shlex(args) {
        Some(p) => p,
        None => {
            return PreflightResult {
                risk: RiskLevel::Caution,
                error_guidance: Some("invalid shell quoting in arguments".to_string()),
                dry_run_preview: generate_dry_run_preview(&format!("rm {}", args), workdir),
            };
        }
    };
    for file in &parts {
        if file.starts_with('-') {
            continue;
        }
        let path = resolve_path(file, workdir);
        if !path.exists() {
            return PreflightResult {
                risk: RiskLevel::Dangerous("file_not_found".to_string()),
                error_guidance: Some(format!("File '{}' does not exist.", file)),
                dry_run_preview: None,
            };
        }
    }
    PreflightResult {
        risk: RiskLevel::Caution,
        error_guidance: None,
        dry_run_preview: generate_dry_run_preview(&format!("rm {}", args), workdir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands() {
        assert_eq!(classify_command("ls -la"), RiskLevel::Safe);
        assert_eq!(classify_command("cat file.txt"), RiskLevel::Safe);
        assert_eq!(classify_command("pwd"), RiskLevel::Safe);
    }

    #[test]
    fn test_caution_commands() {
        assert!(matches!(classify_command("mv a b"), RiskLevel::Caution));
        assert!(matches!(classify_command("cp a b"), RiskLevel::Caution));
        assert!(matches!(
            classify_command("find . -name '*.rs'"),
            RiskLevel::Safe
        )); // find is read-only
    }

    #[test]
    fn test_dangerous_commands() {
        assert!(matches!(
            classify_command("rm file.txt"),
            RiskLevel::Dangerous(_)
        ));
        assert!(matches!(
            classify_command("find . | while read f; do mv $f dest/; done"),
            RiskLevel::Dangerous(_)
        ));
        assert!(matches!(
            classify_command("git reset --hard"),
            RiskLevel::Dangerous(_)
        ));
    }

    #[test]
    fn test_preflight_mv_source_missing() {
        let workdir = std::env::temp_dir();
        let result = preflight_command("mv nonexistent_file somewhere/", &workdir);
        assert!(!result.can_execute());
        assert!(result
            .error_guidance
            .as_ref()
            .unwrap()
            .contains("does not exist"));
    }

    #[test]
    fn test_preflight_mv_dest_missing() {
        let workdir = std::env::temp_dir();
        let src = workdir.join("preflight_test_src.txt");
        std::fs::write(&src, "test").ok();
        let result = preflight_command(&format!("mv {} nonexistent_dir/", src.display()), &workdir);
        assert!(!result.can_execute());
        assert!(result
            .error_guidance
            .as_ref()
            .unwrap()
            .contains("does not exist"));
        std::fs::remove_file(&src).ok();
    }

    #[test]
    fn test_pipe_destruct_pattern() {
        assert!(matches!(
            classify_command("find . -type f | while read f; do rm \"$f\"; done"),
            RiskLevel::Dangerous(_)
        ));
    }

    #[test]
    fn test_unscoped_find_without_maxdepth() {
        let result = detect_unscoped("find . -name '*.sh'", &PathBuf::from("."));
        assert!(result.is_unscoped || result.estimated_count < UNSCOPED_WARN_THRESHOLD);
    }

    #[test]
    fn test_scoped_find_with_maxdepth() {
        let result = detect_unscoped("find . -maxdepth 1 -name '*.sh'", &PathBuf::from("."));
        assert!(!result.is_unscoped);
    }

    #[test]
    fn test_glob_unscoped_warning() {
        let result = detect_unscoped("mv * dest/", &std::env::temp_dir());
        assert!(result.is_unscoped || result.estimated_count < UNSCOPED_WARN_THRESHOLD);
    }

    #[test]
    fn test_protected_dir_read_allowed() {
        assert!(check_protected_paths("ls sessions/").is_none());
        assert!(check_protected_paths("cat config/orchestrator.toml").is_none());
        assert!(check_protected_paths("rg 'pattern' src/").is_none());
    }

    #[test]
    fn test_git_protected_mutation_blocked() {
        assert!(check_protected_paths("rm -rf .git/").is_some(), ".git/ should be protected");
        assert!(check_protected_paths("mv .git/hooks /tmp/").is_some(), ".git/ should be protected");
        assert!(check_protected_paths("rm -rf .git").is_some(), ".git should be protected");
    }

    #[test]
    fn test_gitignore_protected() {
        assert!(check_protected_paths("rm .gitignore").is_some(), ".gitignore should be protected");
        assert!(check_protected_paths("mv .gitignore backup/").is_some(), ".gitignore should be protected");
    }

    #[test]
    fn test_non_protected_path_allowed() {
        assert!(check_protected_paths("mv file.txt dest/").is_none());
        assert!(check_protected_paths("rm some_file.txt").is_none());
    }

    #[test]
    fn test_dry_run_preview_generated() {
        let workdir = std::env::temp_dir();
        // Caution commands should get dry-run previews
        let result = preflight_command("mv /tmp/file1 /tmp/file2", &workdir);
        assert!(result.dry_run_preview.is_some() || result.error_guidance.is_some());
    }

    #[test]
    fn test_mixed_pipe_destructive_wins() {
        // Even though "| xargs stat" is PIPE_SAFE_READ_ONLY, the later "| xargs rm" must win
        let cmd = "find . | xargs stat | xargs rm";
        assert!(matches!(classify_command(cmd), RiskLevel::Dangerous(_)));
    }

    #[test]
    fn test_safe_pipe_alone_is_safe() {
        let cmd = "find . -name '*.rs' | xargs stat";
        assert!(matches!(classify_command(cmd), RiskLevel::Safe));
    }

    #[test]
    fn test_destructive_pipe_alone_is_dangerous() {
        let cmd = "find . | xargs rm -rf";
        assert!(matches!(classify_command(cmd), RiskLevel::Dangerous(_)));
    }

}
