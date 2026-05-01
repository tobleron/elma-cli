//! @efficiency-role: domain-logic
//!
//! Core System Prompt (Task 313: Prompt Protection)
//!
//! This module contains the canonical system prompt for Elma's tool-calling
//! pipeline. It is the single source of truth for how the model is instructed.
//!
//! ╔══════════════════════════════════════════════════════════╗
//! ║  DO NOT MODIFY THIS FILE without explicit user approval ║
//! ║  The prompt is the result of extensive iteration and    ║
//! ║  performance tuning. Any change must be reviewed by     ║
//! ║  the user and validated against scenario tests.         ║
//! ╚══════════════════════════════════════════════════════════╝
//!
//! Rationale: The system prompt is the most sensitive piece of context
//! sent to the model. Small changes can cause large behavioral shifts.
//! This file is protected by:
//! - CODEOWNERS (requires human review)
//! - AGENTS.md Rule 8 (explicit behavioral constraint)
//! - Build-time hash verification (test_prompt_unchanged)
//!
//! To propose a change:
//! 1. Update TOOL_CALLING_SYSTEM_PROMPT below
//! 2. Update the hash in test_prompt_unchanged
//! 3. Run scenario tests to verify behavior
//! 4. Get explicit user approval before merging

// ============================================================================
// Core System Prompt
// ============================================================================

/// The system prompt for Elma's tool-calling pipeline.
///
/// This prompt is sent to the model at the start of every tool-calling turn.
/// It defines Elma's identity, workflow, and evidence-grounding principles.
///
/// Design principles:
/// - Minimal: ~60 tokens for the core, metadata injected separately
/// - No negations: all instructions are positive ("do X" not "don't do Y")
/// - Principle-first: no examples, only reasoning principles
/// - Clear workflow: discover → execute → respond
/// - Evidence-grounded: all answers must come from tool output
pub const TOOL_CALLING_SYSTEM_PROMPT: &str = "\
You are Elma, a local-first terminal agent.

Understand the user's request and take action. Deliver direct answers for conversational queries. Use tools to gather evidence for factual requests.

Tool workflow:
1. Discover extra capabilities with tool_search
2. Execute commands: shell (terminal), read (view files), search (ripgrep), glob (file patterns), ls (directory tree), fetch (web), write (create), edit (modify), patch (multi-file), update_todo_list (tasks)
3. Use respond for interim status updates (loops)
4. Use summary when you have enough evidence that the user request, inquiry, or task is resolved and accomplished

Prefer `rg` for text search and file listing — it respects .gitignore and skips hidden files automatically.

Begin with the most direct source of truth. Collect evidence until you have sufficient information. Ground all answers in tool output.";

// ============================================================================
// Prompt Assembly
// ============================================================================

/// Assemble the full system prompt by combining the core prompt with
/// workspace metadata (facts, file tree, conversation, skill context,
/// project guidance).
///
/// The core prompt stays constant. Metadata changes per session.
/// The `---` separator creates a clear boundary between instructions
/// (above) and metadata (below).
pub fn assemble_system_prompt(
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &str,
    skill_context: &str,
    project_guidance: &str,
) -> String {
    let mut extra = String::new();
    if !workspace_facts.is_empty() {
        extra.push_str(&format!("\n## Workspace\n{}\n", workspace_facts));
    }
    if !workspace_brief.is_empty() {
        extra.push_str(&format!("\n## File tree\n{}\n", workspace_brief));
    }
    if !conversation.is_empty() {
        extra.push_str(conversation);
        extra.push('\n');
    }
    if !skill_context.is_empty() {
        extra.push_str(&format!("\n## Skill context\n{}\n", skill_context));
    }
    if !project_guidance.is_empty() {
        extra.push_str(&format!(
            "\n## Project guidance\n{}\n",
            project_guidance
        ));
    }

    if extra.is_empty() {
        TOOL_CALLING_SYSTEM_PROMPT.to_string()
    } else {
        format!(
            r##"{core}

---{extra}"##,
            core = TOOL_CALLING_SYSTEM_PROMPT,
            extra = extra,
        )
    }
}

// ============================================================================
// Build-Time Verification
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Verify the core system prompt has not been modified.
    ///
    /// This test acts as a safety net: if an agent or developer changes
    /// TOOL_CALLING_SYSTEM_PROMPT without updating the hash, this test
    /// will fail and require explicit review.
    ///
    /// To update: run `cargo test` with the new hash from the failure message.
    #[test]
    fn test_prompt_unchanged() {
        let mut hasher = DefaultHasher::new();
        TOOL_CALLING_SYSTEM_PROMPT.hash(&mut hasher);
        let current_hash = hasher.finish();

        // This hash represents the approved version of the prompt.
        // Update it ONLY after user review and scenario validation.
        let approved_hash: u64 = 0x54da215bbfee1019;

        // If this assertion fails, the prompt has been modified.
        // See the module documentation for the change process.
        if current_hash != approved_hash {
            panic!(
                "TOOL_CALLING_SYSTEM_PROMPT has been modified (hash: 0x{:016x}).\n\
                 This requires explicit user approval.\n\
                 See src/prompt_core.rs for the change process.\n\
                 Approved hash: 0x{:016x}",
                current_hash, approved_hash
            );
        }
    }
}
