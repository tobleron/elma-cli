//! @efficiency-role: domain-logic
//!
//! Mutating Request Execution And Verification Contract — Task 699.
//!
//! Detects mutating tasks (edits, writes, backups, replacements) and
//! enforces at least one mutating tool call before finalization.
//! Prevents the model from answering with only a plan when the user
//! requested concrete execution.

use crate::*;
use std::sync::{OnceLock, RwLock};

/// Global flag: whether a mutating tool call has been made this session.
static HAS_MUTATED: OnceLock<RwLock<bool>> = OnceLock::new();

fn has_mutated_flag() -> &'static RwLock<bool> {
    HAS_MUTATED.get_or_init(|| RwLock::new(false))
}

/// Mark that a mutating operation was performed.
pub(crate) fn mark_mutation_performed() {
    if let Ok(mut lock) = has_mutated_flag().write() {
        *lock = true;
    }
}

/// Reset the mutation flag (for new sessions/turns).
pub(crate) fn reset_mutation_flag() {
    if let Ok(mut lock) = has_mutated_flag().write() {
        *lock = false;
    }
}

/// Check if a mutating operation has been performed.
pub(crate) fn mutation_performed() -> bool {
    if let Ok(lock) = has_mutated_flag().read() {
        *lock
    } else {
        false
    }
}

/// Check if a user request describes a mutating task that requires execution.
/// Returns the detected mutation type and target as a string.
pub(crate) fn detect_mutating_request(user_request: &str) -> Option<String> {
    let lower = user_request.to_lowercase();

    // Pattern: "replace X with Y" / "change X to Y" / "update X"
    let edit_keywords = [
        "replace", "substitute", "change", "update", "modify", "edit",
        "rewrite", "refactor", "rename", "fix", "correct",
    ];
    for kw in &edit_keywords {
        if lower.contains(kw) {
            return Some(format!("edit: {}", kw));
        }
    }

    // Pattern: "backup X" / "copy X to Y"
    let backup_keywords = ["backup", "copy", "duplicate", "archive", "mirror"];
    for kw in &backup_keywords {
        if lower.contains(kw) {
            return Some(format!("backup: {}", kw));
        }
    }

    // Pattern: "create X" / "write X" / "generate X"
    let create_keywords = ["create", "write", "generate", "produce", "build"];
    for kw in &create_keywords {
        if lower.contains(kw) {
            return Some(format!("create: {}", kw));
        }
    }

    // Pattern: "remove X" / "delete X" / "clean X"
    let delete_keywords = ["remove", "delete", "clean", "clear", "trash"];
    for kw in &delete_keywords {
        if lower.contains(kw) {
            return Some(format!("delete: {}", kw));
        }
    }

    None
}

/// Check if a tool call is mutating (modifies the workspace).
pub(crate) fn is_mutating_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write" | "edit" | "patch" | "copy" | "r#move" | "mkdir" | "trash"
            | "touch" | "shell"
    )
}

/// Generate a prompt message when a mutating task hasn't performed any mutations.
pub(crate) fn build_mutation_required_message(detected: &str) -> String {
    format!(
        "! You were asked to perform a mutating task ({detected}), \
         but you have not executed any mutating tool calls yet.\n\n\
         You must actually perform the requested operation using one of the available tools \
         (write, edit, shell, patch, copy, mkdir, etc.) before providing your final answer.\n\n\
         If you cannot perform the operation, explain the limitation clearly."
    )
}

/// Check if a final answer is just a plan (no evidence of execution).
pub(crate) fn answer_is_just_a_plan(answer: &str) -> bool {
    let lower = answer.to_lowercase();
    let plan_indicators = [
        "here's my plan",
        "i will",
        "i could",
        "i would",
        "my plan is",
        "the plan is",
        "steps to",
        "first, i would",
        "here's how i would",
        "i suggest",
        "i recommend",
        "proposed approach",
        "here is the plan",
        "i am ready to proceed",
        "ready to proceed",
        "let me know if you",
        "i'd be happy to",
    ];
    let has_plan = plan_indicators.iter().any(|p| lower.contains(p));

    // Only flag as plan if it also doesn't have evidence of execution
    let has_execution = lower.contains("wrote")
        || lower.contains("created")
        || lower.contains("updated")
        || lower.contains("replaced")
        || lower.contains("deleted")
        || lower.contains("backed up")
        || lower.contains("edited")
        || lower.contains("changed");

    has_plan && !has_execution
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static MUT_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_detect_edit_request() {
        let result = detect_mutating_request("Replace all TODO comments with ISSUE tags");
        assert!(result.is_some());
        assert!(result.unwrap().contains("edit"));
    }

    #[test]
    fn test_detect_backup_request() {
        let result = detect_mutating_request("Backup the project source files");
        assert!(result.is_some());
        assert!(result.unwrap().contains("backup"));
    }

    #[test]
    fn test_detect_create_request() {
        let result = detect_mutating_request("Create a security report");
        assert!(result.is_some());
        assert!(result.unwrap().contains("create"));
    }

    #[test]
    fn test_detect_read_only_request() {
        let result = detect_mutating_request("List all files in src directory");
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_read_only_question() {
        let result = detect_mutating_request("What does this function do?");
        assert!(result.is_none());
    }

    #[test]
    fn test_mutation_flag() {
        let _guard = MUT_TEST_MUTEX.lock().unwrap();
        reset_mutation_flag();
        assert!(!mutation_performed());
        mark_mutation_performed();
        assert!(mutation_performed());
        reset_mutation_flag();
        assert!(!mutation_performed());
    }

    #[test]
    fn test_is_mutating_tool() {
        assert!(is_mutating_tool("write"));
        assert!(is_mutating_tool("edit"));
        assert!(is_mutating_tool("shell"));
        assert!(!is_mutating_tool("read"));
        assert!(!is_mutating_tool("search"));
        assert!(!is_mutating_tool("respond"));
    }

    #[test]
    fn test_answer_is_plan() {
        let plan = "Here's my plan: I will look at the files and then make changes. Let me know if you want me to proceed.";
        assert!(answer_is_just_a_plan(plan));
    }

    #[test]
    fn test_answer_is_not_plan_with_execution() {
        let executed = "I have replaced all TODO comments with ISSUE tags as requested. Here's what I changed: src/main.rs:15.";
        assert!(!answer_is_just_a_plan(executed));
    }

    #[test]
    fn test_answer_is_not_plan_direct() {
        let direct = "The answer is 42.";
        assert!(!answer_is_just_a_plan(direct));
    }

    #[test]
    fn test_build_mutation_message() {
        let msg = build_mutation_required_message("edit: replace");
        assert!(msg.contains("edit: replace"));
        assert!(msg.contains("mutating tool"));
    }

    #[test]
    fn test_reset_mutation_flag() {
        let _guard = MUT_TEST_MUTEX.lock().unwrap();
        mark_mutation_performed();
        assert!(mutation_performed());
        reset_mutation_flag();
        assert!(!mutation_performed());
    }
}
