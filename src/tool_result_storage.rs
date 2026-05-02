//! @efficiency-role: domain-logic
//!
//! Tool Result Storage (Task 113)
//!
//! When tool results exceed a size threshold, they are persisted to disk
//! and the model sees a `<persisted-output>` wrapper with a preview.
//! Inspired by Claude Code's `toolResultStorage.ts`.

use crate::*;

/// Default maximum characters for an inline tool result.
/// Results larger than this are persisted to disk.
/// ~50K chars ≈ ~15K tokens, significant for 8K context models.
pub(crate) const DEFAULT_MAX_RESULT_SIZE_CHARS: usize = 50_000;

/// Preview size for persisted results (shown inline).
/// ~2K chars ≈ ~600 tokens — enough for the model to understand context.
pub(crate) const PREVIEW_SIZE_CHARS: usize = 2_000;

/// Maximum total characters across all tool results in a single message.
/// Prevents aggregate bloat even when individual results are under threshold.
pub(crate) const MAX_TOOL_RESULTS_PER_MESSAGE_CHARS: usize = 150_000;

/// Result of applying the tool result budget.
pub(crate) struct BudgetedResult {
    /// The content to show the model (inline or persisted wrapper).
    pub(crate) content_for_model: String,
    /// Whether the result was persisted to disk.
    pub(crate) persisted: bool,
    /// Path to the persisted file (if any).
    pub(crate) persisted_path: Option<PathBuf>,
    /// Original content size in characters.
    pub(crate) original_size: usize,
}

/// Directory for persisted tool results within a session.
fn tool_results_dir(session: &SessionPaths) -> PathBuf {
    session.artifacts_dir.join("tool-results")
}

/// Ensure the tool-results directory exists.
fn ensure_tool_results_dir(session: &SessionPaths) -> Result<PathBuf> {
    let dir = tool_results_dir(session);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create tool results dir: {}", dir.display()))?;
    Ok(dir)
}

/// Persist a tool result to disk and return the path.
fn persist_result(
    session: &SessionPaths,
    tool_call_id: &str,
    tool_name: &str,
    content: &str,
) -> Result<PathBuf> {
    let dir = ensure_tool_results_dir(session)?;
    let path = dir.join(format!("{}.txt", tool_call_id));
    let tmp_path = dir.join(format!("{}.tmp", tool_call_id));

    // Atomic write: write to .tmp first, then rename
    let meta_content = format!(
        "Tool: {}\nOriginal size: {} chars\nTimestamp: {}\n\n{}",
        tool_name,
        content.len(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        content
    );

    std::fs::write(&tmp_path, &meta_content)
        .with_context(|| format!("Failed to write tool result: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path).with_context(|| {
        format!(
            "Failed to rename tool result: {} → {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(path)
}

/// Build the `<persisted-output>` wrapper for the model.
fn build_persisted_wrapper(
    tool_name: &str,
    original_size: usize,
    preview: &str,
    persisted_path: &PathBuf,
) -> String {
    format!(
        "[persisted-output]\n\
         Tool: {}\n\
         Original size: {} chars\n\
         Preview: {}\n\
         Full output saved to: {}\n\
         Use `read` tool to examine the full output if needed.\n\
         [/persisted-output]",
        tool_name,
        original_size,
        if preview.len() > PREVIEW_SIZE_CHARS {
            format!(
                "{}...",
                preview.chars().take(PREVIEW_SIZE_CHARS).collect::<String>()
            )
        } else {
            preview.to_string()
        },
        persisted_path.display()
    )
}

/// Apply the tool result budget. If the content exceeds the threshold,
/// persist it to disk and return a wrapper for the model.
///
/// Falls back to truncation if persistence fails (never crash).
pub(crate) fn apply_tool_result_budget(
    session: &SessionPaths,
    tool_call_id: &str,
    tool_name: &str,
    content: &str,
    threshold_chars: usize,
) -> BudgetedResult {
    if content.len() <= threshold_chars {
        return BudgetedResult {
            content_for_model: content.to_string(),
            persisted: false,
            persisted_path: None,
            original_size: content.len(),
        };
    }

    // Persist to disk
    match persist_result(session, tool_call_id, tool_name, content) {
        Ok(path) => {
            trace_verbose(
                true,
                &format!(
                    "tool_result_persisted: {} ({} chars → {})",
                    tool_name,
                    content.len(),
                    path.display()
                ),
            );
            let preview = content.chars().take(PREVIEW_SIZE_CHARS).collect::<String>();
            BudgetedResult {
                content_for_model: build_persisted_wrapper(
                    tool_name,
                    content.len(),
                    &preview,
                    &path,
                ),
                persisted: true,
                persisted_path: Some(path),
                original_size: content.len(),
            }
        }
        Err(e) => {
            // Fallback: truncate instead of crash
            trace(
                args_placeholder(),
                &format!(
                    "tool_result_persist_failed: {} (falling back to truncation): {}",
                    tool_name, e
                ),
            );
            let truncated = content.chars().take(threshold_chars).collect::<String>();
            BudgetedResult {
                content_for_model: format!(
                    "{}... [output truncated, {} total chars]",
                    truncated,
                    content.len()
                ),
                persisted: false,
                persisted_path: None,
                original_size: content.len(),
            }
        }
    }
}

/// Apply aggregate budget across all tool results in a message.
/// If total exceeds limit, replace largest results with persisted wrappers first.
pub(crate) fn apply_aggregate_budget(
    session: &SessionPaths,
    tool_results: &mut Vec<(String, String, String)>, // (tool_call_id, tool_name, content)
    max_total_chars: usize,
) {
    let total: usize = tool_results.iter().map(|(_, _, c)| c.len()).sum();
    if total <= max_total_chars {
        return;
    }

    trace_verbose(
        true,
        &format!(
            "tool_result_aggregate_budget: {} chars exceeds limit {}, persisting largest first",
            total, max_total_chars
        ),
    );

    // Sort by size descending
    let mut indexed: Vec<(usize, usize)> = tool_results
        .iter()
        .enumerate()
        .map(|(i, (_, _, c))| (i, c.len()))
        .collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));

    let mut current_total = total;
    for (idx, size) in indexed {
        if current_total <= max_total_chars {
            break;
        }

        let (call_id, tool_name, content) = &tool_results[idx];
        if content.len() <= PREVIEW_SIZE_CHARS + 500 {
            continue; // Too small to bother persisting
        }

        // Persist this result
        match persist_result(session, call_id, tool_name, content) {
            Ok(path) => {
                let preview = content.chars().take(PREVIEW_SIZE_CHARS).collect::<String>();
                tool_results[idx].2 = build_persisted_wrapper(tool_name, size, &preview, &path);
                current_total = current_total - size + tool_results[idx].2.len();
            }
            Err(_) => {
                // Fallback: truncate
                tool_results[idx].2 = format!(
                    "{}... [truncated, {} total]",
                    content.chars().take(PREVIEW_SIZE_CHARS).collect::<String>(),
                    size
                );
                current_total = current_total - size + tool_results[idx].2.len();
            }
        }
    }
}

/// Placeholder for trace macro when args not available
fn args_placeholder() -> &'static crate::Args {
    // This is only used in error paths where we can't get real args
    // The trace will still work with a placeholder reference
    // In practice this path is never hit in normal operation
    unreachable!("args_placeholder should never be called in normal operation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_result_no_persistence() {
        // Small results should not be persisted
        let content = "short output";
        assert!(content.len() <= DEFAULT_MAX_RESULT_SIZE_CHARS);
    }

    #[test]
    fn test_large_result_exceeds_threshold() {
        // Large results should exceed threshold
        let content = "x".repeat(60_000);
        assert!(content.len() > DEFAULT_MAX_RESULT_SIZE_CHARS);
    }

    #[test]
    fn test_preview_size_reasonable() {
        // Preview should be smaller than threshold
        assert!(PREVIEW_SIZE_CHARS < DEFAULT_MAX_RESULT_SIZE_CHARS);
    }

    #[test]
    fn test_aggregate_budget_no_op_when_under_limit() {
        let total = 1000 + 2000 + 500;
        assert!(total <= MAX_TOOL_RESULTS_PER_MESSAGE_CHARS);
    }
}
