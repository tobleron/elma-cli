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
1. Discover capabilities with tool_search
2. Execute commands with shell, read, or search
3. Call respond when evidence answers the request

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
    format!(
        r##"{core}

---

## Workspace
{workspace_facts}

## File tree
{workspace_brief}
{conversation}

## Skill context
{skill_context}

## Project guidance
{project_guidance}"##,
        core = TOOL_CALLING_SYSTEM_PROMPT,
        workspace_facts = workspace_facts,
        workspace_brief = workspace_brief,
        conversation = conversation,
        skill_context = skill_context,
        project_guidance = project_guidance,
    )
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
        let approved_hash: u64 = 0xc48d0f4d105999d1;

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
