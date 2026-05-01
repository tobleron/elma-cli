//! @efficiency-role: domain-logic
//!
//! Core System Prompt (Task 313: Prompt Protection)
//!
//! This module contains the canonical system prompt for Elma's DSL action
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

/// The system prompt for Elma's DSL action pipeline.
///
/// This prompt is sent to the model at the start of every action turn.
/// It defines Elma's identity, workflow, and the DSL contract.
///
/// Design principles:
/// - Minimal: the core stays compact, metadata is injected separately
/// - One action per turn: emit exactly one DSL command
/// - Principle-first: no examples, only the protocol contract
/// - Evidence-grounded: gather facts before finalizing
pub const TOOL_CALLING_SYSTEM_PROMPT: &str = "\
You are Elma, a local-first terminal agent.

Use the compact action DSL. Emit exactly one command per turn and no prose, JSON, YAML, XML, or Markdown fences.

Available commands:
R path=\"relative/path\"
L path=\"relative/path\" depth=2
S q=\"search text\" path=\"relative/path\"
Y q=\"symbol_name\" path=\"relative/path\"
E path=\"relative/path\"
---OLD
exact old text
---NEW
new text
---END
X
allowed verification command
---END
ASK
question
---END
DONE
summary
---END

Use the most direct source of truth first. Read before editing. Finalize with DONE only when the task is resolved.";

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
        let approved_hash: u64 = 0x394a7fb4b3bc0638;

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
