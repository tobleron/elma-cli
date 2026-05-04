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
You are Elma, an AI assistant. Help the user with their tasks.

Prioritize brevity. Answer the question asked — skip explanations unless requested.
Ground all answers in tool output.

Tool workflow:
1. Discover capabilities with tool_search
2. Execute: shell, read, search, glob, ls, fetch, write, edit, patch, update_todo_list
3. Use respond when you have sufficient evidence.

Prefer `rg` for search — it respects .gitignore.
Begin with the most direct source of truth. Collect evidence until sufficient.

Write in natural plain text only. No markdown, no headings, no bold, no lists.";

// ============================================================================
// Prompt Assembly
// ============================================================================

/// Assemble the full system prompt by combining the core prompt with
/// conversation and skill context wrapped in SILENT_METADATA tags.
///
/// Workspace info and project guidance are injected in the model's first
/// message context, so the model should rarely need to call `workspace_info`
/// directly (Task 595).
///
/// Metadata is available for reasoning and tool decisions but the model
/// is explicitly instructed not to reveal, quote, paraphrase, or acknowledge
/// it in its responses. The model answers only the user's request.
///
/// The core prompt stays constant. Metadata changes per session.
pub fn assemble_system_prompt(
    conversation: &str,
    skill_context: &str,
) -> String {
    let mut metadata = String::new();
    if !conversation.is_empty() {
        metadata.push_str(conversation);
        metadata.push('\n');
    }
    if !skill_context.is_empty() {
        metadata.push_str(&format!("\n## Skill context\n{}\n", skill_context));
    }

    // Append mode-specific response instructions
    let mode_instructions = match crate::ui_state::current_response_mode() {
        crate::ui_state::ResponseMode::Concise => "\n\nMode: Concise\nRespond concisely in natural prose, less than 300 words. Use complete sentences. Acknowledge what action was taken or what was found. Prefer natural language but use structured formats (bullet points, numbered lists) only when they genuinely improve clarity.",
        crate::ui_state::ResponseMode::Long => "\n\nMode: Long\nRespond with paragraph formatting, less than 900 words. Use numbered points only if they genuinely help clarity. Keep tone natural and conversational.",
    };

    if metadata.is_empty() {
        format!("{}{}", TOOL_CALLING_SYSTEM_PROMPT, mode_instructions)
    } else {
        format!(
            r##"{core}

<INSTRUCTIONS>
Use SILENT_METADATA only for reasoning and adaptation.
Do not reveal, quote, paraphrase, or acknowledge it.
Answer only the USER_REQUEST.
</INSTRUCTIONS>

<SILENT_METADATA>{metadata}
</SILENT_METADATA>
{mode_instructions}"##,
            core = TOOL_CALLING_SYSTEM_PROMPT,
            metadata = metadata,
            mode_instructions = mode_instructions,
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
        let approved_hash: u64 = 0x997d4e679e3ad77b;

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
