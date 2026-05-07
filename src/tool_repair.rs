//! @efficiency-role: domain-logic
//!
//! Schema Guided Tool Argument Repair — Task 689 / Task 695.
//!
//! Provides deterministic repair for obvious argument-shape failures:
//! missing filePath/path in read calls, per-tool failure circuits,
//! retry-once with repaired arguments when confidence is high.
//!
//! Task 695: Evidence-derived path recovery — tracks candidate file paths
//! from successful search, glob, ls, write, and shell outputs.

use crate::*;
use std::collections::HashMap;

/// Maximum recent successful tool results to track for context-aware repair.
const MAX_TRACKED_OUTCOMES: usize = 15;

/// Tracks recent successful tool outcomes for context-aware argument repair.
/// When a tool call is missing a required path field, we check the last
/// successful write or read to infer the likely path.
#[derive(Debug, Clone, Default)]
pub(crate) struct ToolOutcomeHistory {
    outcomes: Vec<ToolOutcome>,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolOutcome {
    pub tool_name: String,
    pub path: String,
    pub success: bool,
}

impl ToolOutcomeHistory {
    pub fn record(&mut self, tool_name: &str, args_json: &str, success: bool) {
        let path = extract_path_from_args(args_json);
        if path.is_empty() && !success {
            return;
        }
        self.outcomes.push(ToolOutcome {
            tool_name: tool_name.to_string(),
            path,
            success,
        });
        if self.outcomes.len() > MAX_TRACKED_OUTCOMES {
            self.outcomes.remove(0);
        }
    }

    /// Record a path extracted from a tool result (output content) — Task 695.
    /// Parses search/glob/ls/shell output for file paths to use in argument repair.
    pub fn record_from_result(&mut self, tool_name: &str, result_content: &str, success: bool) {
        if !success {
            return;
        }
        let extracted = extract_path_from_tool_output(tool_name, result_content);
        if let Some(path) = extracted {
            self.outcomes.push(ToolOutcome {
                tool_name: tool_name.to_string(),
                path,
                success: true,
            });
            if self.outcomes.len() > MAX_TRACKED_OUTCOMES {
                self.outcomes.remove(0);
            }
        }
    }

    /// Find the most recent successful path from any write tool call.
    pub fn last_written_path(&self) -> Option<&str> {
        self.outcomes
            .iter()
            .rev()
            .find(|o| o.tool_name == "write" && o.success)
            .map(|o| o.path.as_str())
    }

    /// Find the most recent successful path from any tool call.
    pub fn last_any_path(&self) -> Option<&str> {
        self.outcomes
            .iter()
            .rev()
            .find(|o| o.success && !o.path.is_empty())
            .map(|o| o.path.as_str())
    }

    /// Find the last path from a search or glob result (Task 695).
    pub fn last_search_path(&self) -> Option<&str> {
        self.outcomes
            .iter()
            .rev()
            .find(|o| (o.tool_name == "search" || o.tool_name == "glob" || o.tool_name == "ls")
                  && o.success && !o.path.is_empty())
            .map(|o| o.path.as_str())
    }

    /// Find the most recent successful outcome whose path exists on disk.
    /// Tries each candidate from most recent to oldest.
    pub fn find_existing_path(&self) -> Option<&str> {
        self.outcomes
            .iter()
            .rev()
            .find(|o| o.success && !o.path.is_empty() && candidate_path_exists(&o.path))
            .map(|o| o.path.as_str())
    }

    /// Collect all paths from recent successful search/glob/ls outcomes
    /// that actually exist on disk, up to max_count.
    pub fn get_existing_search_paths(&self, max_count: usize) -> Vec<String> {
        self.outcomes
            .iter()
            .rev()
            .filter(|o| {
                o.success
                    && (o.tool_name == "search" || o.tool_name == "glob" || o.tool_name == "ls")
                    && !o.path.is_empty()
                    && candidate_path_exists(&o.path)
            })
            .take(max_count)
            .map(|o| o.path.clone())
            .collect()
    }

    pub fn clear(&mut self) {
        self.outcomes.clear();
    }
}

/// Maximum consecutive empty read calls before blocking.
const MAX_EMPTY_READ_CALLS: usize = 1;

/// Per-tool argument alias allowlists for schema repair (Task 703).
/// Maps tool name -> list of acceptable argument aliases.
const TOOL_ARG_ALLOWLISTS: &[(&str, &[&str])] = &[
    ("read", &["filePath", "path"]),
    ("write", &["path"]),
    ("edit", &["path", "filePath"]),
    ("copy", &["path", "source", "src", "from"]),
    ("exists", &["path"]),
    ("stat", &["path"]),
    ("trash", &["path"]),
    ("touch", &["path"]),
    ("mkdir", &["path"]),
    ("patch", &["path"]),
    ("glob", &["pattern"]),
];

/// Check if an argument key is a known alias for a tool (Task 703).
pub(crate) fn is_known_tool_arg_alias(tool_name: &str, arg_key: &str) -> bool {
    TOOL_ARG_ALLOWLISTS
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, aliases)| aliases.contains(&arg_key))
        .unwrap_or(false)
}

/// Validate that a copy repair candidate path exists on the filesystem and
/// came from a structured tool output (not arbitrary text). (Task 703)
pub(crate) fn is_valid_copy_repair_path(candidate: &str) -> bool {
    if candidate.is_empty() {
        return false;
    }
    // Reject non-path words commonly hallucinated by models
    let bogus: &[&str] = &["Copied", "ago)", "source", "destination", "target", "backup",
        "File", "files", "directory", "error", "none", "null"];
    if bogus.contains(&candidate.trim()) {
        return false;
    }
    // Must contain a path separator or file extension
    if !candidate.contains('/') && !candidate.contains('.') {
        return false;
    }
    // Check filesystem if it looks plausible
    let path = std::path::Path::new(candidate);
    if path.exists() {
        return true;
    }
    // Also accept if parent directory exists (file may need to be created)
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && parent.exists() {
            return true;
        }
    }
    false
}

/// Track per-tool schema repair attempts and force strategy shift after 2 failed.
/// (Task 703)
static SCHEMA_REPAIR_COUNTS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, usize>>> =
    std::sync::LazyLock::new(Default::default);

/// Record a schema repair attempt for a tool. Returns true if the repair
/// budget is exhausted (>= 3 failed repairs for this tool, i.e., after 2 attempts).
pub(crate) fn record_schema_repair(tool_name: &str) -> bool {
    if let Ok(mut counts) = SCHEMA_REPAIR_COUNTS.lock() {
        let entry = counts.entry(tool_name.to_string()).or_insert(0);
        *entry += 1;
        if *entry >= 3 {
            crate::append_trace_log_line(&format!(
                "[SCHEMA_REPAIR_EXHAUSTED] {}: 2 failed repairs, forcing strategy shift",
                tool_name
            ));
            return true;
        }
    }
    false
}

/// Reset schema repair counters for a tool (on success).
pub(crate) fn reset_schema_repair_count(tool_name: &str) {
    if let Ok(mut counts) = SCHEMA_REPAIR_COUNTS.lock() {
        counts.remove(tool_name);
    }
}

/// Clear all schema repair counters (for testing).
pub(crate) fn clear_all_schema_repair_counts() {
    if let Ok(mut counts) = SCHEMA_REPAIR_COUNTS.lock() {
        counts.clear();
    }
}

/// Build a compact schema correction packet for `read` tool missing `filePath`.
/// Provides the model with the exact required field name and any candidate
/// paths from recent tool outcomes. This avoids verbose JSON schema errors.
pub(crate) fn build_read_schema_correction_packet(
    error_msg: &str,
    candidate_paths: &[String],
) -> String {
    let baseline = "The 'read' tool requires a 'filePath' field with a valid file path.".to_string();

    if candidate_paths.is_empty() {
        format!(
            "{}\n\nUse 'glob', 'search', or 'ls' to discover valid file paths first, then retry read with the correct filePath.",
            baseline
        )
    } else {
        let paths = candidate_paths
            .iter()
            .map(|p| format!("  - `{}`", p))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{}\n\nAvailable paths from recent evidence:\n{}\n\nRetry read with one of these paths using the 'filePath' parameter.",
            baseline, paths
        )
    }
}

/// Check if a validation error message indicates a missing `filePath` on `read`.
pub(crate) fn is_read_missing_filepath_error(tool_name: &str, error_msg: &str) -> bool {
    if tool_name != "read" {
        return false;
    }
    let lower = error_msg.to_lowercase();
    lower.contains("filepath") && (lower.contains("missing") || lower.contains("required"))
}

/// Track empty read validation failures per session turn.
static EMPTY_READ_VALIDATION_COUNT: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(0));

/// Record an empty read validation failure. Returns the current count.
pub(crate) fn record_empty_read_validation_failure() -> usize {
    if let Ok(mut count) = EMPTY_READ_VALIDATION_COUNT.lock() {
        *count += 1;
        *count
    } else {
        0
    }
}

/// Reset the empty read validation failure counter.
pub(crate) fn reset_empty_read_validation_failures() {
    if let Ok(mut count) = EMPTY_READ_VALIDATION_COUNT.lock() {
        *count = 0;
    }
}

/// Check if empty read validation has exceeded the stagnation threshold.
pub(crate) fn is_empty_read_validation_stagnation() -> bool {
    if let Ok(count) = EMPTY_READ_VALIDATION_COUNT.lock() {
        *count >= 2
    } else {
        false
    }
}

/// Canonicalize an absolute workspace path to a workspace-relative path (Task 695).
/// If the path is already relative or cannot be made relative, returns it unchanged.
pub(crate) fn canonicalize_to_relative(path: &str, workspace_root: &Path) -> String {
    let p = Path::new(path);
    if !p.is_absolute() {
        return path.to_string();
    }

    // Try to strip the workspace root prefix
    if let Ok(canonical) = p.canonicalize() {
        if let Ok(ws_canonical) = workspace_root.canonicalize() {
            if let Ok(relative) = canonical.strip_prefix(&ws_canonical) {
                return relative.to_string_lossy().to_string();
            }
        }
    }

    // Fallback: just return the original (the caller will handle validation)
    path.to_string()
}

/// Check if a read call has empty/absent arguments and should be blocked (Task 695).
/// Returns true if the call should be blocked and replaced with a fallback.
pub(crate) fn should_block_empty_read(
    tool_name: &str,
    args_json: &str,
    empty_read_count: usize,
) -> bool {
    if tool_name != "read" {
        return false;
    }
    let path = extract_path_from_args(args_json);
    if path.is_empty() && empty_read_count >= MAX_EMPTY_READ_CALLS {
        return true;
    }
    // Also block absolute paths that can't be made relative
    if !path.is_empty() && Path::new(&path).is_absolute() {
        return true;
    }
    false
}

/// Generate a fallback strategy hint for blocked empty read calls (Task 695).
pub(crate) fn empty_read_fallback_hint(last_search_path: Option<&str>) -> String {
    match last_search_path {
        Some(path) => format!(
            "The 'read' tool was called with an empty or invalid path. \
             Use 'shell cat {}' instead, or 'read' with a valid relative path.",
            path
        ),
        None => format!(
            "The 'read' tool was called with an empty or invalid path. \
             Use 'shell cat <path>' instead of 'read'. Example: shell command='cat src/main.rs'"
        ),
    }
}

/// Extract file paths from tool output content (Task 695).
/// Parses results from search, glob, ls, and shell tools for path-like strings.
fn extract_path_from_tool_output(tool_name: &str, content: &str) -> Option<String> {
    match tool_name {
        "search" | "glob" => {
            let mut paths: Vec<&str> = content
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    !t.is_empty()
                        && (t.contains('/') || t.contains(".rs") || t.contains(".md")
                            || t.contains(".toml") || t.contains(".json"))
                        && !t.starts_with("error")
                        && !t.starts_with("[")
                        && !t.starts_with("Tool result")
                })
                .collect();
            paths.truncate(3);
            paths.first().map(|s| s.trim().to_string())
        }
        "ls" => {
            let first_file = content
                .lines()
                .find(|l| {
                    let t = l.trim();
                    !t.is_empty() && !t.starts_with("total") && t.contains('.')
                })
                .map(|l| {
                    // Extract filename from `ls -la` output (last whitespace-delimited token)
                    l.split_whitespace().last().unwrap_or("").to_string()
                });
            first_file.filter(|s| !s.is_empty())
        }
        "shell" => {
            // Extract first path-like token from non-error shell output
            let first_path = content
                .lines()
                .find(|l| {
                    let t = l.trim();
                    t.contains('/') && !t.contains("error") && !t.contains("command not found")
                })
                .map(|l| l.trim().split_whitespace().next().unwrap_or("").to_string());
            first_path.filter(|s| !s.is_empty())
        }
        _ => None,
    }
}

/// Extract a path-like value from tool call arguments JSON.
pub(crate) fn extract_path_from_args(args_json: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(args_json) {
        if let Some(obj) = val.as_object() {
            for key in &["path", "paths", "filePath", "pattern"] {
                if let Some(v) = obj.get(*key) {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() {
                            return s.to_string();
                        }
                    }
                    if let Some(arr) = v.as_array() {
                        if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                            if !first.is_empty() {
                                return first.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Check if a candidate path exists on disk relative to the workspace root.
/// Accepts both workspace-relative and absolute paths.
fn candidate_path_exists(candidate: &str) -> bool {
    if candidate.is_empty() {
        return false;
    }
    let path = std::path::Path::new(candidate);
    if path.is_absolute() {
        path.exists()
    } else {
        // Try relative to current dir (workspace root)
        path.exists()
            || std::env::current_dir()
                .map(|cwd| cwd.join(candidate).exists())
                .unwrap_or(false)
    }
}

/// Attempt to repair missing required path fields in a tool call.
/// Returns Some(repaired_args_json) if repair was possible, None otherwise.
pub(crate) fn repair_tool_call_args(
    tool_name: &str,
    args_json: &str,
    history: &ToolOutcomeHistory,
) -> Option<String> {
    let needs_path = match tool_name {
        "read" | "write" | "exists" | "edit" | "copy" | "trash" | "touch" | "file_size"
        | "stat" | "patch" | "mkdir" => true,
        _ => false,
    };
    if !needs_path {
        return None;
    }

    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(args_json) {
        if let Some(obj) = val.as_object_mut() {
            if tool_name == "read" {
                let has_file_path = obj
                    .get("filePath")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.trim().is_empty());
                let has_path = obj
                    .get("path")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.trim().is_empty());
                // Validate that model-supplied path aliases exist on disk
                if !has_file_path && has_path {
                    if let Some(alias_path) = obj
                        .get("path")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.to_string())
                    {
                        if candidate_path_exists(&alias_path) {
                            obj.insert(
                                "filePath".to_string(),
                                serde_json::Value::String(alias_path.clone()),
                            );
                            if let Ok(repaired) = serde_json::to_string(&val) {
                                crate::append_trace_log_line(&format!(
                                    "[TOOL_ARG_REPAIR] read: mapped path alias to filePath='{}' (exists on disk)",
                                    alias_path
                                ));
                                return Some(repaired);
                            }
                        } else {
                            crate::append_trace_log_line(&format!(
                                "[TOOL_ARG_REPAIR] read: rejected path alias '{}' (not found on disk)",
                                alias_path
                            ));
                        }
                    }
                }
            }
        }
    }

    let existing_path = extract_path_from_args(args_json);
    if !existing_path.is_empty() {
        return None;
    }

    // Find first valid candidate from history.
    // History paths are trusted (they come from actual tool outputs, not model hallucination).
    let candidate = if tool_name == "copy" {
        let written = history.last_written_path();
        if written.map_or(false, |p| is_valid_copy_repair_path(p)) {
            written
        } else {
            history.last_search_path().filter(|p| is_valid_copy_repair_path(p))
        }
    } else {
        history.last_search_path()
            .or_else(|| history.last_written_path())
            .or_else(|| history.last_any_path())
    }?;
    if candidate.is_empty() || (tool_name == "copy" && !is_valid_copy_repair_path(candidate)) {
        return None;
    }

    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(args_json) {
        if let Some(obj) = val.as_object_mut() {
            let target_key = match tool_name {
                "read" => "filePath",
                _ => "path",
            };
            obj.insert(target_key.to_string(), serde_json::Value::String(candidate.to_string()));
            if let Ok(repaired) = serde_json::to_string(&val) {
                let source = if tool_name == "copy" { "search/glob evidence" } else { "evidence" };
                crate::append_trace_log_line(&format!(
                    "[TOOL_ARG_REPAIR] {}: injected {}='{}' from {} (exists on disk)",
                    tool_name, target_key, candidate, source
                ));
                return Some(repaired);
            }
        }
    }

    None
}

/// Per-tool failure circuit breaker.
/// Tracks consecutive failures per tool and prevents repeated
/// identical errors from consuming budget iterations.
#[derive(Debug, Clone)]
pub(crate) struct ToolFailureCircuit {
    /// Per-tool consecutive identical failure count
    consecutive_identical: HashMap<String, usize>,
    /// Per-tool total failure count (never resets on error change)
    total_failures: HashMap<String, usize>,
    /// Per-tool last error text for change detection
    last_errors: HashMap<String, String>,
    /// Circuit open state: tool_name -> is_open
    open_circuits: HashMap<String, bool>,
}

impl Default for ToolFailureCircuit {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolFailureCircuit {
    pub fn new() -> Self {
        Self {
            consecutive_identical: HashMap::new(),
            total_failures: HashMap::new(),
            last_errors: HashMap::new(),
            open_circuits: HashMap::new(),
        }
    }

    /// Record a tool failure. Returns true if the circuit is now open
    /// (should stop retrying this tool).
    pub fn record_failure(&mut self, tool_name: &str, error_signal: &str) -> bool {
        let total = self.total_failures.entry(tool_name.to_string()).or_insert(0);
        *total += 1;

        let last = self
            .last_errors
            .insert(tool_name.to_string(), error_signal.to_string());
        let is_same_error = last.as_deref() == Some(error_signal);

        if is_same_error {
            let identical = self
                .consecutive_identical
                .entry(tool_name.to_string())
                .or_insert(0);
            *identical += 1;

            // Open circuit after 3 consecutive identical failures
            if *identical >= 3 {
                self.open_circuits.insert(tool_name.to_string(), true);
                crate::append_trace_log_line(&format!(
                    "[TOOL_CIRCUIT_OPEN] {}: circuit opened after {} consecutive identical failures",
                    tool_name, identical
                ));
                return true;
            }
        } else {
            // Error changed — reset identical counter but keep total
            self.consecutive_identical
                .insert(tool_name.to_string(), 1);
        }

        // Open circuit after 5 total failures regardless of error type
        if *total >= 5 {
            self.open_circuits.insert(tool_name.to_string(), true);
            crate::append_trace_log_line(&format!(
                "[TOOL_CIRCUIT_OPEN] {}: circuit opened after {} total failures",
                tool_name, total
            ));
            return true;
        }

        false
    }

    /// Record a tool success — resets the failure counter and closes the circuit.
    pub fn record_success(&mut self, tool_name: &str) {
        self.consecutive_identical.remove(tool_name);
        self.total_failures.remove(tool_name);
        self.last_errors.remove(tool_name);
        self.open_circuits.remove(tool_name);
    }

    /// Check if a tool's circuit is open (repeated failures detected).
    pub fn is_open(&self, tool_name: &str) -> bool {
        self.open_circuits.get(tool_name).copied().unwrap_or(false)
    }

    /// Get the total failure count for a tool.
    pub fn failure_count(&self, tool_name: &str) -> usize {
        self.total_failures.get(tool_name).copied().unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.consecutive_identical.clear();
        self.total_failures.clear();
        self.last_errors.clear();
        self.open_circuits.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_from_args_path_field() {
        let args = r#"{"path": "src/main.rs", "limit": 50}"#;
        assert_eq!(extract_path_from_args(args), "src/main.rs");
    }

    #[test]
    fn test_extract_path_from_args_paths_field() {
        let args = r#"{"paths": ["src/main.rs", "src/lib.rs"]}"#;
        assert_eq!(extract_path_from_args(args), "src/main.rs");
    }

    #[test]
    fn test_extract_path_from_args_filepath_field() {
        let args = r#"{"filePath": "Cargo.toml"}"#;
        assert_eq!(extract_path_from_args(args), "Cargo.toml");
    }

    #[test]
    fn test_extract_path_from_args_empty() {
        let args = r#"{"name": "test"}"#;
        assert_eq!(extract_path_from_args(args), "");
    }

    #[test]
    fn test_extract_path_from_args_invalid_json() {
        let args = "not json";
        assert_eq!(extract_path_from_args(args), "");
    }

    #[test]
    fn test_outcome_history_records_write() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "output.txt", "content": "data"}"#, true);
        assert_eq!(history.last_written_path(), Some("output.txt"));
    }

    #[test]
    fn test_outcome_history_last_any_path() {
        let mut history = ToolOutcomeHistory::default();
        history.record("read", r#"{"path": "readme.md"}"#, true);
        assert_eq!(history.last_any_path(), Some("readme.md"));
    }

    #[test]
    fn test_outcome_history_prefers_write() {
        let mut history = ToolOutcomeHistory::default();
        history.record("read", r#"{"path": "readme.md"}"#, true);
        history.record("write", r#"{"path": "output.txt", "content": "data"}"#, true);
        assert_eq!(history.last_written_path(), Some("output.txt"));
    }

    #[test]
    fn test_repair_missing_path_from_write_history() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "report.md", "content": "data"}"#, true);
        let repaired = repair_tool_call_args("read", r#"{"limit": 50}"#, &history);
        assert!(repaired.is_some());
        let repaired = repaired.unwrap();
        assert!(repaired.contains("report.md"));
        assert!(repaired.contains("filePath"));
    }

    #[test]
    fn test_repair_maps_read_path_alias_to_filepath() {
        let history = ToolOutcomeHistory::default();
        let repaired = repair_tool_call_args("read", r#"{"path": "src/main.rs"}"#, &history);
        assert!(repaired.is_some());
        let repaired = repaired.unwrap();
        assert!(repaired.contains("\"filePath\""));
        assert!(repaired.contains("src/main.rs"));
    }

    #[test]
    fn test_repair_no_history_returns_none() {
        let history = ToolOutcomeHistory::default();
        let repaired = repair_tool_call_args("read", r#"{"limit": 50}"#, &history);
        assert!(repaired.is_none());
    }

    #[test]
    fn test_repair_existing_path_not_needed() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "report.md", "content": "data"}"#, true);
        let repaired = repair_tool_call_args("read", r#"{"filePath": "existing.rs"}"#, &history);
        assert!(repaired.is_none());
    }

    #[test]
    fn test_repair_skips_non_path_tools() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "report.md", "content": "data"}"#, true);
        let repaired = repair_tool_call_args("respond", r#"{}"#, &history);
        assert!(repaired.is_none());
    }

    #[test]
    fn test_failure_circuit_opens_after_3_identical() {
        let mut circuit = ToolFailureCircuit::new();
        assert!(!circuit.record_failure("read", "missing filePath"));
        assert!(!circuit.record_failure("read", "missing filePath"));
        assert!(circuit.record_failure("read", "missing filePath"));
        assert!(circuit.is_open("read"));
    }

    #[test]
    fn test_failure_circuit_opens_after_5_total() {
        let mut circuit = ToolFailureCircuit::new();
        for _ in 0..4 {
            circuit.record_failure("shell", "error_a");
            circuit.record_failure("shell", "error_b");
        }
        assert!(circuit.record_failure("shell", "error_c"));
        assert!(circuit.is_open("shell"));
    }

    #[test]
    fn test_failure_circuit_resets_on_success() {
        let mut circuit = ToolFailureCircuit::new();
        circuit.record_failure("read", "missing filePath");
        circuit.record_failure("read", "missing filePath");
        circuit.record_success("read");
        assert!(!circuit.is_open("read"));
        assert_eq!(circuit.failure_count("read"), 0);
    }

    #[test]
    fn test_failure_circuit_resets_on_error_change() {
        let mut circuit = ToolFailureCircuit::new();
        circuit.record_failure("read", "missing filePath");
        circuit.record_failure("read", "missing filePath");
        // Different error: identical counter resets, total keeps going
        assert!(!circuit.record_failure("read", "wrong type"));
        // Total failures is 3, but consecutive identical was reset
        assert_eq!(circuit.failure_count("read"), 3);
    }

    #[test]
    fn test_clear_histories() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "f.txt"}"#, true);
        assert!(history.last_written_path().is_some());
        history.clear();
        assert!(history.last_written_path().is_none());

        let mut circuit = ToolFailureCircuit::new();
        circuit.record_failure("read", "err");
        assert!(circuit.failure_count("read") > 0);
        circuit.clear();
        assert_eq!(circuit.failure_count("read"), 0);
    }

    #[test]
    fn test_repair_injects_path_for_mkdir() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "project_tmp/report.md", "content": "data"}"#, true);
        let repaired = repair_tool_call_args("mkdir", r#"{}"#, &history);
        assert!(repaired.is_some());
        let repaired = repaired.unwrap();
        assert!(repaired.contains("project_tmp/report.md"));
    }

    #[test]
    fn test_extract_path_with_pattern_field() {
        let args = r#"{"pattern": "src/**/*.rs", "path": "src/"}"#;
        assert_eq!(extract_path_from_args(args), "src/");
    }

    // ── Task 695: Evidence-derived path recovery ──

    #[test]
    fn test_record_from_result_search() {
        let mut history = ToolOutcomeHistory::default();
        history.record_from_result("search", "src/main.rs\nsrc/lib.rs\nsrc/util.rs", true);
        assert!(history.last_search_path().is_some());
    }

    #[test]
    fn test_record_from_result_glob() {
        let mut history = ToolOutcomeHistory::default();
        history.record_from_result("glob", "src/main.rs\nCargo.toml", true);
        assert!(history.last_search_path().is_some());
    }

    #[test]
    fn test_record_from_result_shell_with_path() {
        let mut history = ToolOutcomeHistory::default();
        history.record_from_result("shell", "src/main.rs", true);
        assert!(history.last_any_path().is_some());
    }

    #[test]
    fn test_record_from_result_failure_ignored() {
        let mut history = ToolOutcomeHistory::default();
        history.record_from_result("search", "src/main.rs", false);
        assert!(history.last_search_path().is_none());
    }

    #[test]
    fn test_canonicalize_relative_stays_relative() {
        let result = canonicalize_to_relative("src/main.rs", Path::new("/workspace"));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn test_canonicalize_empty_stays_empty() {
        let result = canonicalize_to_relative("", Path::new("/workspace"));
        assert_eq!(result, "");
    }

    #[test]
    fn test_should_block_first_empty_read_not_blocked() {
        assert!(!should_block_empty_read("read", r#"{}"#, 0));
    }

    #[test]
    fn test_should_block_second_empty_read_blocked() {
        assert!(should_block_empty_read("read", r#"{}"#, 1));
    }

    #[test]
    fn test_should_block_non_read_not_blocked() {
        assert!(!should_block_empty_read("shell", r#"{}"#, 5));
    }

    #[test]
    fn test_should_block_read_with_valid_path_not_blocked() {
        assert!(!should_block_empty_read("read", r#"{"filePath": "main.rs"}"#, 0));
    }

    #[test]
    fn test_empty_read_fallback_hint_with_path() {
        let hint = empty_read_fallback_hint(Some("src/main.rs"));
        assert!(hint.contains("src/main.rs"));
        assert!(hint.contains("shell cat"));
    }

    #[test]
    fn test_empty_read_fallback_hint_without_path() {
        let hint = empty_read_fallback_hint(None);
        assert!(hint.contains("shell cat"));
    }

    #[test]
    fn test_last_search_path_prefers_search_over_write() {
        let mut history = ToolOutcomeHistory::default();
        history.record("write", r#"{"path": "report.md"}"#, true);
        history.record_from_result("search", "src/main.rs\nsrc/lib.rs", true);
        assert_eq!(history.last_search_path(), Some("src/main.rs"));
    }

    // ── Task 703: Per-tool allowlists and copy repair validation ──

    #[test]
    fn test_is_known_tool_arg_alias_read() {
        assert!(is_known_tool_arg_alias("read", "filePath"));
        assert!(is_known_tool_arg_alias("read", "path"));
        assert!(!is_known_tool_arg_alias("read", "content"));
    }

    #[test]
    fn test_is_known_tool_arg_alias_copy() {
        assert!(is_known_tool_arg_alias("copy", "source"));
        assert!(is_known_tool_arg_alias("copy", "src"));
        assert!(is_known_tool_arg_alias("copy", "path"));
        assert!(!is_known_tool_arg_alias("copy", "random"));
    }

    #[test]
    fn test_is_valid_copy_repair_path_rejects_bogus() {
        assert!(!is_valid_copy_repair_path("Copied"));
        assert!(!is_valid_copy_repair_path("ago)"));
        assert!(!is_valid_copy_repair_path(""));
    }

    #[test]
    fn test_is_valid_copy_repair_path_accepts_real() {
        assert!(is_valid_copy_repair_path("src/main.rs"));
        assert!(is_valid_copy_repair_path("Cargo.toml"));
    }

    #[test]
    fn test_schema_repair_counter_exhausts_after_3() {
        clear_all_schema_repair_counts();
        // First two calls should still be within budget
        assert!(!record_schema_repair("schema_test_tool"));
        assert!(!record_schema_repair("schema_test_tool"));
        // Third call exceeds budget (after 2 attempts)
        assert!(record_schema_repair("schema_test_tool"));
        clear_all_schema_repair_counts();
    }

    #[test]
    fn test_schema_repair_counter_resets() {
        clear_all_schema_repair_counts();
        assert!(!record_schema_repair("reset_tool"));
        reset_schema_repair_count("reset_tool");
        assert!(!record_schema_repair("reset_tool"));
    }

    // ── Task 708: Read Schema Correction Packet ──

    #[test]
    fn test_build_read_schema_correction_packet_with_candidates() {
        let candidates = vec!["src/main.rs".to_string(), "Cargo.toml".to_string()];
        let packet = build_read_schema_correction_packet("filePath: field is required", &candidates);
        assert!(packet.contains("filePath"));
        assert!(packet.contains("src/main.rs"));
        assert!(packet.contains("Cargo.toml"));
        assert!(packet.contains("Available paths"));
    }

    #[test]
    fn test_build_read_schema_correction_packet_no_candidates() {
        let packet = build_read_schema_correction_packet("filePath: field is required", &[]);
        assert!(packet.contains("filePath"));
        assert!(packet.contains("glob"));
        assert!(!packet.contains("Available paths"));
    }

    #[test]
    fn test_is_read_missing_filepath_error_positive() {
        assert!(is_read_missing_filepath_error("read", "filePath: field is required"));
        assert!(is_read_missing_filepath_error("read", "filePath: missing field"));
    }

    #[test]
    fn test_is_read_missing_filepath_error_wrong_tool() {
        assert!(!is_read_missing_filepath_error("write", "filePath: missing field"));
        assert!(!is_read_missing_filepath_error("shell", "filePath: missing field"));
    }

    #[test]
    fn test_is_read_missing_filepath_error_other_error() {
        assert!(!is_read_missing_filepath_error("read", "limit: invalid value"));
    }

    #[test]
    fn test_empty_read_validation_counters() {
        reset_empty_read_validation_failures();
        assert!(!is_empty_read_validation_stagnation());
        assert_eq!(record_empty_read_validation_failure(), 1);
        assert!(!is_empty_read_validation_stagnation());
        assert_eq!(record_empty_read_validation_failure(), 2);
        assert!(is_empty_read_validation_stagnation());
        reset_empty_read_validation_failures();
        assert!(!is_empty_read_validation_stagnation());
    }
}
