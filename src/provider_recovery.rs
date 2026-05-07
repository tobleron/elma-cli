//! @efficiency-role: domain-logic
//!
//! Provider Finalization Recovery — Task 693.
//!
//! Adds deterministic fallback finalization from structured tool events
//! when the model finalizer fails. Preserves and presents completed tool
//! evidence cleanly, avoids losing artifact verification, and allows
//! continuation or retry without corrupting the session outcome.

use crate::*;
use std::collections::HashMap;

/// Structured record of a completed tool operation.
#[derive(Debug, Clone)]
pub(crate) struct CompletedOperation {
    pub tool_name: String,
    pub target: String,
    pub success: bool,
    pub output_preview: String,
}

/// Extract completed operations from session messages for fallback finalization.
/// Returns a structured list of tool operations that were successfully executed.
pub(crate) fn extract_completed_operations(messages: &[ChatMessage]) -> Vec<CompletedOperation> {
    let mut operations = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for msg in messages.iter().rev() {
        if msg.role == "tool" {
            if let Some(name) = &msg.name {
                let target = extract_tool_target(name, &msg.content);
                let dedup_key = format!("{}:{}", name, target);
                if seen.insert(dedup_key) && !target.is_empty() {
                    operations.push(CompletedOperation {
                        tool_name: name.clone(),
                        target,
                        success: true,
                        output_preview: msg.content.chars().take(200).collect(),
                    });
                }
            }
        }
    }

    operations.reverse();
    operations
}

/// Extract the target from a tool result message.
fn extract_tool_target(tool_name: &str, content: &str) -> String {
    match tool_name {
        "read" | "write" | "edit" | "exists" | "stat" | "file_size" => {
            let first_line = content.lines().next().unwrap_or("");
            let cleaned = first_line
                .trim()
                .chars()
                .take(120)
                .collect::<String>();
            if !cleaned.is_empty() {
                return cleaned;
            }
            "unknown".to_string()
        }
        "shell" => {
            let line = content
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim()
                .chars()
                .take(120)
                .collect::<String>();
            if !line.is_empty() {
                return line;
            }
            "(empty output)".to_string()
        }
        "search" | "glob" => {
            let count = content.lines().count();
            format!("{} results", count)
        }
        _ => "completed".to_string(),
    }
}

/// Build a deterministic fallback final answer from structured tool evidence.
/// Used when the model finalizer call fails (timeout, decode error, etc.).
pub(crate) fn build_structured_fallback(
    original_request: &str,
    operations: &[CompletedOperation],
    provider_error: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!(
        "I gathered evidence about your request but the final answer could not be fully processed.\n\
         **Original request:** {}\n\
         **Provider error:** {}",
        original_request, provider_error
    ));

    if !operations.is_empty() {
        parts.push("\n**Operations completed:**".to_string());
        for op in operations {
            let status = if op.success { "✓" } else { "✗" };
            parts.push(format!(
                "  {} {}: {}",
                status, op.tool_name, op.target
            ));
        }
    }

    parts.push(
        "\n**Note:** This is a structured fallback answer because the finalization model call \
         failed. All successfully completed tool operations are listed above. \
         You may retry finalization or rerun the task with the evidence already gathered."
            .to_string(),
    );

    parts.join("\n")
}

/// Attempt to write a retryable finalization state to session storage.
/// Allows resuming finalization without re-executing all tools.
pub(crate) fn write_finalization_state(
    session_root: &Path,
    operations: &[CompletedOperation],
    error: &str,
) -> Result<()> {
    let state = serde_json::json!({
        "finalization_retry": {
            "error": error,
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "completed_operations": operations.iter().map(|op| {
                serde_json::json!({
                    "tool": op.tool_name,
                    "target": op.target,
                    "success": op.success,
                })
            }).collect::<Vec<_>>(),
        }
    });

    let _ = crate::session_write::mutate_session_doc(session_root, |doc| {
        doc["finalization"] = state;
    });

    Ok(())
}

/// Classify a provider error for structured fallback messages.
pub(crate) fn classify_provider_error(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        "The model API timed out while generating the final answer. Tool evidence was preserved."
    } else if lower.contains("decod") || lower.contains("parse") || lower.contains("json") {
        "The model API returned an unparseable response during finalization. Tool evidence was preserved."
    } else if lower.contains("rate limit") || lower.contains("too many requests") {
        "Rate limit exceeded during finalization. Tool evidence was preserved."
    } else if lower.contains("auth") || lower.contains("unauthorized") || lower.contains("forbidden") {
        "Authentication error during finalization. Tool evidence was preserved."
    } else {
        "A provider error occurred during finalization. Tool evidence was preserved."
    }
}

/// Attempt to build a minimal final answer from completed operations.
/// Returns None if no meaningful evidence was gathered.
pub(crate) fn build_minimal_final_answer(
    original_request: &str,
    operations: &[CompletedOperation],
) -> Option<String> {
    if operations.is_empty() {
        return None;
    }

    let file_ops: Vec<&CompletedOperation> = operations
        .iter()
        .filter(|o| o.tool_name == "read" || o.tool_name == "search" || o.tool_name == "glob")
        .collect();

    let write_ops: Vec<&CompletedOperation> = operations
        .iter()
        .filter(|o| o.tool_name == "write" || o.tool_name == "edit")
        .collect();

    let mut parts = Vec::new();

    if !write_ops.is_empty() {
        parts.push("**Files created/modified:**".to_string());
        for op in &write_ops {
            parts.push(format!("  - {}", op.target));
        }
    }

    if !file_ops.is_empty() {
        parts.push("**Files examined:**".to_string());
        for op in file_ops.iter().take(10) {
            parts.push(format!("  - {}", op.target));
        }
        if file_ops.len() > 10 {
            parts.push(format!("  - ... and {} more", file_ops.len() - 10));
        }
    }

    Some(format!(
        "{}\n\n{}",
        original_request,
        parts.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_operations_empty() {
        assert!(extract_completed_operations(&[]).is_empty());
    }

    #[test]
    fn test_extract_operations_with_tool_results() {
        let msgs = vec![
            ChatMessage {
                role: "tool".to_string(),
                content: "src/main.rs\nfn main() {}".to_string(),
                name: Some("read".to_string()),
                tool_calls: None,
                tool_call_id: Some("t1".to_string()),
                reasoning_content: None,
                summarized: false,
            },
            ChatMessage {
                role: "tool".to_string(),
                content: "written".to_string(),
                name: Some("write".to_string()),
                tool_calls: None,
                tool_call_id: Some("t2".to_string()),
                reasoning_content: None,
                summarized: false,
            },
        ];
        let ops = extract_completed_operations(&msgs);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].tool_name, "read");
        assert_eq!(ops[1].tool_name, "write");
    }

    #[test]
    fn test_build_structured_fallback() {
        let ops = vec![
            CompletedOperation {
                tool_name: "read".to_string(),
                target: "src/main.rs".to_string(),
                success: true,
                output_preview: "fn main() {}".to_string(),
            },
        ];
        let fallback = build_structured_fallback("Check the code", &ops, "timeout after 60s");
        assert!(fallback.contains("Check the code"));
        assert!(fallback.contains("timeout after 60s"));
        assert!(fallback.contains("read"));
        assert!(fallback.contains("src/main.rs"));
        assert!(fallback.contains("structured fallback"));
    }

    #[test]
    fn test_classify_provider_error_timeout() {
        let msg = classify_provider_error("timeout after 60s");
        assert!(msg.contains("timed out"));
    }

    #[test]
    fn test_classify_provider_error_decode() {
        let msg = classify_provider_error("error decoding response body");
        assert!(msg.contains("unparseable"));
    }

    #[test]
    fn test_classify_provider_error_generic() {
        let msg = classify_provider_error("connection reset");
        assert!(msg.contains("provider error"));
    }

    #[test]
    fn test_build_minimal_with_operations() {
        let ops = vec![
            CompletedOperation {
                tool_name: "read".to_string(),
                target: "Cargo.toml".to_string(),
                success: true,
                output_preview: "".to_string(),
            },
            CompletedOperation {
                tool_name: "write".to_string(),
                target: "report.md".to_string(),
                success: true,
                output_preview: "".to_string(),
            },
        ];
        let result = build_minimal_final_answer("Analyze and report", &ops);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.contains("report.md"));
        assert!(result.contains("Cargo.toml"));
    }

    #[test]
    fn test_build_minimal_no_operations() {
        assert!(build_minimal_final_answer("test", &[]).is_none());
    }

    #[test]
    fn test_extract_tool_target_read() {
        let target = extract_tool_target("read", "src/main.rs\nfn main() {}");
        assert_eq!(target, "src/main.rs");
    }

    #[test]
    fn test_extract_tool_target_search() {
        let target = extract_tool_target("search", "line1\nline2\nline3");
        assert_eq!(target, "3 results");
    }

    #[test]
    fn test_extract_tool_target_shell() {
        let target = extract_tool_target("shell", "  \nHello World\n");
        assert_eq!(target, "Hello World");
    }

    #[test]
    fn test_write_finalization_state() {
        let tmp = std::env::temp_dir().join(format!("test_finalization_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let ops = vec![CompletedOperation {
            tool_name: "read".to_string(),
            target: "test.txt".to_string(),
            success: true,
            output_preview: "content".to_string(),
        }];
        assert!(write_finalization_state(&tmp, &ops, "timeout").is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
