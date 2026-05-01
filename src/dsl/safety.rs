//! Shared safety helpers for DSL action execution.
//!
//! These helpers are used by the action executor and by legacy internal edit
//! paths so workspace validation and exact replacement behavior stay aligned.

use crate::dsl::error::{DslErrorCode, RepairObservation};
use crate::dsl::render::render_compact_error;
use crate::program_utils::resolve_workspace_edit_path;
use crate::*;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

static SESSION_READ_FINGERPRINTS: Lazy<Mutex<HashMap<String, HashSet<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static SESSION_EDIT_SNAPSHOT: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandPolicy {
    Strict,
    AskBeforeUnsafe,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandOutcome {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactEditOutcome {
    pub(crate) path: PathBuf,
    pub(crate) summary: String,
    pub(crate) diff: String,
}

pub(crate) fn resolve_workspace_path(workdir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let root = workdir
        .canonicalize()
        .map_err(|err| format!("failed to resolve workspace root: {err}"))?;
    let candidate = resolve_workspace_edit_path(&root, raw_path).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::UnsafePath,
                format!("path {}: {}", raw_path.trim(), err),
            )
            .with_hint("use a project-relative path inside the workspace"),
        )
    })?;

    if candidate.exists() {
        let canon = candidate.canonicalize().map_err(|err| {
            render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::UnsafePath,
                    format!("path {}: {}", raw_path.trim(), err),
                )
                .with_hint("use a project-relative path inside the workspace"),
            )
        })?;
        if !canon.starts_with(&root) {
            return Err(render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::UnsafePath,
                    format!("path {} escapes the workspace root", raw_path.trim()),
                )
                .with_hint("use a project-relative path inside the workspace"),
            ));
        }
        return Ok(canon);
    }

    let mut probe = candidate.as_path();
    loop {
        if probe.exists() {
            let canon = probe.canonicalize().map_err(|err| {
                render_compact_error(
                    &RepairObservation::new(
                        DslErrorCode::UnsafePath,
                        format!("path {}: {}", raw_path.trim(), err),
                    )
                    .with_hint("use a project-relative path inside the workspace"),
                )
            })?;
            if !canon.starts_with(&root) {
                return Err(render_compact_error(
                    &RepairObservation::new(
                        DslErrorCode::UnsafePath,
                        format!("path {} escapes the workspace root", raw_path.trim()),
                    )
                    .with_hint("use a project-relative path inside the workspace"),
                ));
            }
            break;
        }
        let Some(parent) = probe.parent() else {
            break;
        };
        probe = parent;
    }

    Ok(candidate)
}

pub(crate) fn record_session_read(session_key: &str, raw_path: &str) {
    let fingerprint = format!("{}:{}", session_key, raw_path.trim());
    if let Ok(mut guard) = SESSION_READ_FINGERPRINTS.lock() {
        guard
            .entry(session_key.to_string())
            .or_default()
            .insert(fingerprint);
    }
}

pub(crate) fn require_session_read_before_edit(
    session_key: &str,
    raw_path: &str,
) -> Result<(), String> {
    let fingerprint = format!("{}:{}", session_key, raw_path.trim());
    let guard = SESSION_READ_FINGERPRINTS
        .lock()
        .map_err(|_| "failed to access edit read gate".to_string())?;
    let seen = guard
        .get(session_key)
        .map(|set| set.contains(&fingerprint))
        .unwrap_or(false);
    if seen {
        Ok(())
    } else {
        Err(render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("file {} must be read before editing", raw_path.trim()),
            )
            .with_hint("use R path=\"...\" before E"),
        ))
    }
}

pub(crate) fn ensure_session_edit_snapshot(
    session_key: &str,
    session: &SessionPaths,
    workdir: &Path,
    reason: &str,
) -> Result<Option<String>, String> {
    let mut guard = SESSION_EDIT_SNAPSHOT
        .lock()
        .map_err(|_| "failed to access snapshot gate".to_string())?;
    if !guard.insert(session_key.to_string()) {
        return Ok(None);
    }
    let snapshot = crate::snapshot::create_workspace_snapshot(session, workdir, reason, true)
        .map_err(|err| {
            render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::InvalidEdit,
                    format!("snapshot creation failed: {err}"),
                )
                .with_hint("retry the edit after creating a snapshot"),
            )
        })?;
    Ok(Some(snapshot.snapshot_id))
}

#[cfg(test)]
pub(crate) fn clear_session_edit_gates() {
    if let Ok(mut reads) = SESSION_READ_FINGERPRINTS.lock() {
        reads.clear();
    }
    if let Ok(mut snapshots) = SESSION_EDIT_SNAPSHOT.lock() {
        snapshots.clear();
    }
}

/// Default timeout for command execution via the async path (seconds).
pub(crate) const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 30;

/// Execute a validated command asynchronously with a timeout.
///
/// Uses `tokio::process::Command` so the async runtime manages the child process
/// and `tokio::time::timeout` to enforce a bounded wall-clock limit.
///
/// Returns a compact repair observation on timeout or execution failure.
pub(crate) async fn execute_command_policy_async(
    workdir: &Path,
    command: &str,
    policy: CommandPolicy,
    timeout_secs: u64,
) -> Result<CommandOutcome, String> {
    let parts = validate_command(command, policy)?;

    let mut cmd = tokio::process::Command::new(&parts[0]);
    cmd.args(&parts[1..]).current_dir(workdir);
    cmd.kill_on_drop(true);

    let output = match tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::UnsafeCommand,
                    format!("failed to execute {}: {}", parts[0], e),
                )
                .with_hint("use a direct command from the allowlist"),
            ));
        }
        Err(_) => {
            return Err(render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::UnsafeCommand,
                    format!(
                        "command timed out after {} seconds: {}",
                        timeout_secs, parts[0]
                    ),
                )
                .with_hint("use a faster command or split the work"),
            ));
        }
    };

    Ok(CommandOutcome {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}

pub(crate) fn execute_command_policy(
    workdir: &Path,
    command: &str,
    policy: CommandPolicy,
) -> Result<CommandOutcome, String> {
    let parts = validate_command(command, policy)?;

    let mut cmd = std::process::Command::new(&parts[0]);
    cmd.args(&parts[1..]).current_dir(workdir);
    let output = cmd.output().map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::UnsafeCommand,
                format!("failed to execute {}: {}", parts[0], err),
            )
            .with_hint("use a direct command from the allowlist"),
        )
    })?;

    Ok(CommandOutcome {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}

/// Validate a command string against the given policy without executing it.
/// Returns the parsed command parts on success.
pub(crate) fn validate_command(
    command: &str,
    policy: CommandPolicy,
) -> Result<Vec<String>, String> {
    if matches!(policy, CommandPolicy::Disabled) {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::UnsafeCommand, "command execution is disabled")
                .with_hint("use a read-only DSL action instead"),
        ));
    }

    let command = command.trim();
    if command.is_empty() {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::UnsafeCommand, "command is empty")
                .with_hint("provide a verification command"),
        ));
    }

    if command.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        return Err(render_compact_error(
            &RepairObservation::new(
                DslErrorCode::UnsafeCommand,
                "command contains a control character",
            )
            .with_hint("use one direct command line"),
        ));
    }

    let parts = shlex::split(command).ok_or_else(|| {
        render_compact_error(
            &RepairObservation::new(DslErrorCode::UnsafeCommand, "invalid shell quoting")
                .with_hint("use plain arguments without shell operators"),
        )
    })?;
    if parts.is_empty() {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::UnsafeCommand, "command is empty")
                .with_hint("provide a verification command"),
        ));
    }
    if parts.iter().any(|part| is_shell_control_token(part)) {
        return Err(render_compact_error(
            &RepairObservation::new(
                DslErrorCode::UnsafeCommand,
                "shell control operators are not allowed",
            )
            .with_hint("use one direct command without pipes or redirects"),
        ));
    }

    if !is_allowed_command_family(&parts) {
        return Err(render_compact_error(
            &RepairObservation::new(
                DslErrorCode::UnsafeCommand,
                format!("command is not in the strict allowlist: {}", parts[0]),
            )
            .with_hint("use cargo check/test/fmt/clippy, git status/diff, ls, rg, or grep"),
        ));
    }

    Ok(parts)
}

pub(crate) fn apply_exact_edit(
    workdir: &Path,
    raw_path: &str,
    old: &str,
    new: &str,
) -> Result<ExactEditOutcome, String> {
    if old.is_empty() {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::InvalidEdit, "old text must not be empty")
                .with_hint("include a unique ---OLD block"),
        ));
    }
    let path = resolve_workspace_path(workdir, raw_path)?;
    let current = std::fs::read(&path).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to read file: {err}"),
            )
            .with_hint("read the file again before retrying"),
        )
    })?;
    if current.len() > 8 * 1024 * 1024 {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::InvalidEdit, "file is too large to edit")
                .with_hint("choose a smaller text file"),
        ));
    }
    if current.contains(&0) {
        return Err(render_compact_error(
            &RepairObservation::new(DslErrorCode::InvalidEdit, "binary files are not supported")
                .with_hint("use a text file"),
        ));
    }
    let current = String::from_utf8(current).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("file is not valid UTF-8: {err}"),
            )
            .with_hint("use a text file"),
        )
    })?;
    let matches = current.match_indices(old).collect::<Vec<_>>();
    match matches.len() {
        0 => {
            return Err(render_compact_error(
                &RepairObservation::new(DslErrorCode::InvalidEdit, "old text not found")
                    .with_hint("read the file and retry with an exact ---OLD block"),
            ))
        }
        1 => {}
        _ => {
            return Err(render_compact_error(
                &RepairObservation::new(
                    DslErrorCode::InvalidEdit,
                    "old text appears multiple times",
                )
                .with_hint("use a larger unique ---OLD block"),
            ))
        }
    }

    let updated = current.replacen(old, new, 1);
    let parent = path
        .parent()
        .ok_or_else(|| "edit target has no parent directory".to_string())?;
    std::fs::create_dir_all(parent).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to create parent directory: {err}"),
            )
            .with_hint("choose a writable workspace path"),
        )
    })?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to create temp file: {err}"),
            )
            .with_hint("choose a writable workspace path"),
        )
    })?;
    use std::io::Write;
    tmp.write_all(updated.as_bytes()).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to write temp file: {err}"),
            )
            .with_hint("retry the edit"),
        )
    })?;
    tmp.flush().map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to flush temp file: {err}"),
            )
            .with_hint("retry the edit"),
        )
    })?;
    tmp.persist(&path).map_err(|err| {
        render_compact_error(
            &RepairObservation::new(
                DslErrorCode::InvalidEdit,
                format!("failed to persist edit: {}", err.error),
            )
            .with_hint("retry the edit"),
        )
    })?;

    Ok(ExactEditOutcome {
        path,
        summary: "exact edit applied".to_string(),
        diff: format!("- {}\n+ {}", preview_line(old), preview_line(new)),
    })
}

fn is_shell_control_token(part: &str) -> bool {
    matches!(
        part,
        "|" | "||"
            | "&"
            | "&&"
            | ";"
            | ">"
            | ">>"
            | "<"
            | "("
            | ")"
            | "{"
            | "}"
            | "2>"
            | "2>>"
            | "1>"
            | "1>>"
    )
}

fn is_allowed_command_family(parts: &[String]) -> bool {
    let Some(program) = parts.first().map(|s| s.as_str()) else {
        return false;
    };
    match program {
        "cargo" => matches!(
            parts.get(1).map(|s| s.as_str()),
            Some("check") | Some("test") | Some("fmt") | Some("clippy")
        ),
        "git" => matches!(
            parts.get(1).map(|s| s.as_str()),
            Some("diff") | Some("status")
        ),
        "ls" | "rg" | "grep" => true,
        _ => false,
    }
}

fn preview_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(80)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit_gate_test_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[test]
    fn rejects_absolute_path() {
        let root = tempfile::tempdir().unwrap();
        let err = resolve_workspace_path(root.path(), "/etc/passwd").unwrap_err();
        assert!(err.contains("UNSAFE_PATH"));
    }

    #[test]
    fn rejects_parent_escape() {
        let root = tempfile::tempdir().unwrap();
        let err = resolve_workspace_path(root.path(), "../escape").unwrap_err();
        assert!(err.contains("UNSAFE_PATH"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let symlink_path = root.path().join("link");
        symlink(outside.path(), &symlink_path).unwrap();
        let err = resolve_workspace_path(root.path(), "link/file.txt").unwrap_err();
        assert!(err.contains("UNSAFE_PATH"));
    }

    #[test]
    fn allows_missing_parent_safely() {
        let root = tempfile::tempdir().unwrap();
        let resolved = resolve_workspace_path(root.path(), "nested/file.txt").unwrap();
        assert!(resolved.ends_with("nested/file.txt"));
    }

    #[test]
    fn strict_command_allows_known_command() {
        let root = tempfile::tempdir().unwrap();
        let outcome = execute_command_policy(root.path(), "ls", CommandPolicy::Strict).unwrap();
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[test]
    fn strict_command_rejects_pipeline() {
        let root = tempfile::tempdir().unwrap();
        let err = execute_command_policy(root.path(), "cargo test | cat", CommandPolicy::Strict)
            .unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn strict_command_rejects_disallowed_family() {
        let root = tempfile::tempdir().unwrap();
        let err =
            execute_command_policy(root.path(), "rm -rf .", CommandPolicy::Strict).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn exact_edit_rejects_missing_old() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file.txt");
        std::fs::write(&file, "hello world").unwrap();
        let err = apply_exact_edit(root.path(), "file.txt", "missing", "new").unwrap_err();
        assert!(err.contains("INVALID_EDIT"));
        let unchanged = std::fs::read_to_string(&file).unwrap();
        assert_eq!(unchanged, "hello world");
    }

    #[test]
    fn exact_edit_rejects_duplicate_old() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file.txt");
        std::fs::write(&file, "alpha beta alpha").unwrap();
        let err = apply_exact_edit(root.path(), "file.txt", "alpha", "gamma").unwrap_err();
        assert!(err.contains("INVALID_EDIT"));
    }

    #[test]
    fn exact_edit_updates_file_atomically() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file.txt");
        std::fs::write(&file, "hello world").unwrap();
        let outcome = apply_exact_edit(root.path(), "file.txt", "world", "there").unwrap();
        assert!(outcome.summary.contains("applied"));
        let updated = std::fs::read_to_string(&file).unwrap();
        assert_eq!(updated, "hello there");
    }

    #[test]
    fn edit_gate_requires_prior_read() {
        let _guard = edit_gate_test_lock().lock().unwrap();
        clear_session_edit_gates();
        let err =
            require_session_read_before_edit("session-edit-gate-requires", "file.txt").unwrap_err();
        assert!(err.contains("INVALID_EDIT"));
    }

    #[test]
    fn edit_gate_accepts_recorded_read() {
        let _guard = edit_gate_test_lock().lock().unwrap();
        clear_session_edit_gates();
        record_session_read("session-edit-gate-accepts", "file.txt");
        assert!(require_session_read_before_edit("session-edit-gate-accepts", "file.txt").is_ok());
    }

    #[test]
    fn validate_command_rejects_empty() {
        let err = validate_command("", CommandPolicy::Strict).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn validate_command_rejects_control_chars() {
        // Embedded null byte should be rejected even after trim
        let err = validate_command("ls\0extra", CommandPolicy::Strict).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn validate_command_accepts_known() {
        let parts = validate_command("ls -la", CommandPolicy::Strict).unwrap();
        assert_eq!(parts, vec!["ls", "-la"]);
    }

    #[test]
    fn validate_command_rejects_pipeline() {
        let err = validate_command("ls | cat", CommandPolicy::Strict).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn validate_command_rejects_disallowed() {
        let err = validate_command("rm -rf .", CommandPolicy::Strict).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[test]
    fn validate_command_disabled_policy() {
        let err = validate_command("ls", CommandPolicy::Disabled).unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[tokio::test]
    async fn execute_command_async_success() {
        let root = tempfile::tempdir().unwrap();
        let outcome = execute_command_policy_async(root.path(), "ls", CommandPolicy::Strict, 10)
            .await
            .unwrap();
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[tokio::test]
    async fn execute_command_async_rejects_disallowed() {
        let root = tempfile::tempdir().unwrap();
        let err = execute_command_policy_async(root.path(), "rm -rf .", CommandPolicy::Strict, 10)
            .await
            .unwrap_err();
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[tokio::test]
    async fn execute_command_async_timeout() {
        // Use a very short timeout to trigger timeout on a "sleep 1" command
        // but only if the allowlist allows it. Since sleep isn't allowed,
        // test that policy validation happens before execution.
        let root = tempfile::tempdir().unwrap();
        let err = execute_command_policy_async(root.path(), "sleep 60", CommandPolicy::Strict, 1)
            .await
            .unwrap_err();
        // Should fail at policy validation (sleep is not in the allowlist)
        assert!(err.contains("UNSAFE_COMMAND"));
    }

    #[tokio::test]
    async fn validate_command_does_not_execute() {
        // validate_command should not execute anything, just validate syntax+policies
        let parts = validate_command("grep -r foo src", CommandPolicy::Strict).unwrap();
        assert_eq!(parts, vec!["grep", "-r", "foo", "src"]);
    }
}
