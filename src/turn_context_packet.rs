//! @efficiency-role: domain-logic
//!
//! Minimal Per-Turn Context Packet For Dense Models — Task 701.
//! Task 761: CurrentTurnContext to separate raw user request from injected context.
//!
//! Builds a compact turn packet from raw user request, current objective,
//! required artifacts, successful tool outcomes, failed tool signals,
//! and stop reason. Keeps model-facing context minimal while raw details
//! remain on disk.

use crate::*;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

/// Task 761: Structured context for a single user turn.
/// Keeps the raw user request separate from injected prior evidence,
/// runtime hints, and system guidance so downstream components
/// (artifact extraction, scope extraction, finalization) can distinguish
/// what the user actually asked from what the runtime injected.
#[derive(Debug, Clone)]
pub(crate) struct CurrentTurnContext {
    /// The original user message, verbatim — no injected text.
    pub raw_user_request: String,
    /// Prior-try evidence summary injected for cross-cycle continuity (if any).
    pub prior_relevant_evidence: Option<String>,
    /// Runtime recovery hints (budget, strategy, etc.)
    pub runtime_recovery_hints: Option<String>,
    /// System-level guidance for the model (e.g. "Don't repeat prior steps").
    pub system_guidance: Option<String>,
}

impl CurrentTurnContext {
    pub fn new(raw_user_request: &str) -> Self {
        Self {
            raw_user_request: raw_user_request.to_string(),
            prior_relevant_evidence: None,
            runtime_recovery_hints: None,
            system_guidance: None,
        }
    }

    /// Build the model-facing user message from the separate fields.
    /// Prior evidence is tagged and separated so it's clear to the model
    /// what came from the user vs. what was injected.
    pub fn build_model_message(&self) -> String {
        let mut parts = vec![self.raw_user_request.clone()];
        if let Some(ref evidence) = self.prior_relevant_evidence {
            parts.push(format!(
                "\n\n[Previously gathered in a prior attempt]\n{}\nDo NOT repeat steps already completed. Continue from where you left off.",
                evidence
            ));
        }
        if let Some(ref hints) = self.runtime_recovery_hints {
            parts.push(format!("\n\n[Runtime hints]\n{}", hints));
        }
        if let Some(ref guidance) = self.system_guidance {
            parts.push(format!("\n\n[System guidance]\n{}", guidance));
        }
        parts.join("")
    }

    /// The raw user request alone — for artifact extraction, scope detection, etc.
    /// This ensures artifacts are only derived from what the user actually asked.
    pub fn raw_request(&self) -> &str {
        &self.raw_user_request
    }
}

/// A compact per-turn context packet for model-facing turns.
#[derive(Debug, Clone)]
pub(crate) struct TurnContextPacket {
    /// The original user request (truncated to 400 chars)
    pub objective: String,
    /// Remaining requirement detail
    pub requirement: String,
    /// Artifact files that must be delivered (if any)
    pub required_artifacts: Vec<String>,
    /// Tools that have returned useful results
    pub evidence_gathered: Vec<String>,
    /// Tools that have failed repeatedly
    pub blocked_signals: Vec<String>,
    /// The stop reason that triggered this turn
    pub stop_reason: String,
    /// Allowed next actions
    pub next_action_contract: String,
}

/// Per-session turn counter for packet persistence.
static TURN_COUNTER: LazyLock<Mutex<u32>> = LazyLock::new(|| Mutex::new(0));

/// Reset the turn counter for a new session.
pub(crate) fn reset_turn_counter() {
    if let Ok(mut c) = TURN_COUNTER.lock() {
        *c = 0;
    }
}

/// Build a compact turn context packet.
pub(crate) fn build_turn_context_packet(
    original_request: &str,
    objective: &str,
    required_artifacts: &[String],
    successful_tools: &[String],
    failed_tools: &[String],
    stop_reason: &str,
) -> TurnContextPacket {
    let objective_truncated = truncate_text(original_request, 400);
    let objective_fallback = if objective.is_empty() { "Complete the user's request" } else { objective };

    let next_action = build_next_action_contract(stop_reason, !required_artifacts.is_empty());

    TurnContextPacket {
        objective: objective_truncated,
        requirement: format_remaining_requirement(objective_fallback, required_artifacts),
        required_artifacts: required_artifacts.to_vec(),
        evidence_gathered: summarize_evidence(successful_tools),
        blocked_signals: summarize_failures(failed_tools),
        stop_reason: stop_reason.to_string(),
        next_action_contract: next_action,
    }
}

/// Render the packet as a concise message for the model.
pub(crate) fn render_turn_context_packet(packet: &TurnContextPacket) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("## Current Objective\n{}", packet.objective));
    parts.push(format!("## Remaining Requirement\n{}", packet.requirement));

    if !packet.evidence_gathered.is_empty() {
        parts.push(format!("## Evidence Gathered\n{}", packet.evidence_gathered.join("\n")));
    }

    if !packet.blocked_signals.is_empty() {
        parts.push(format!("## Blocked Signals\n{}", packet.blocked_signals.join("\n")));
    }

    parts.push(format!("## Next Action Contract\n{}", packet.next_action_contract));

    parts.join("\n\n")
}

/// Persist the packet to the session folder for forensic review.
pub(crate) fn persist_turn_context_packet(session_root: &Path, packet: &TurnContextPacket) {
    let turn_num = {
        if let Ok(mut c) = TURN_COUNTER.lock() {
            *c += 1;
            *c
        } else {
            return;
        }
    };

    let dir = session_root.join("turn_packets");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("turn_{:03}.md", turn_num));

    let rendered = render_turn_context_packet(packet);
    let full = format!(
        "---\nturn: {}\nstop_reason: {}\n---\n\n{}\n",
        turn_num,
        packet.stop_reason,
        rendered
    );
    let _ = std::fs::write(&path, &full);
}

/// Build a continuation message for budget recovery turns.
pub(crate) fn build_continuation_from_packet(
    packet: &TurnContextPacket,
    continuation_count: u32,
    max_continuations: u32,
) -> String {
    format!(
        "## Continue Working (Attempt {}/{})\n\n\
         {}\n\n\
         Continue working. Do NOT provide a final answer until the remaining requirement is satisfied.",
        continuation_count,
        max_continuations,
        render_turn_context_packet(packet)
    )
}

// ── helpers ──────────────────────────────────────────────────────────────

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

fn format_remaining_requirement(objective: &str, artifacts: &[String]) -> String {
    if !artifacts.is_empty() {
        let art_list = artifacts
            .iter()
            .map(|a| format!("- `{}`", a))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{}\n\nRequired artifacts to deliver:\n{}", objective, art_list)
    } else {
        objective.to_string()
    }
}

fn summarize_evidence(tools: &[String]) -> Vec<String> {
    let seen: HashSet<&str> = ["write", "edit", "shell", "glob", "search", "ls", "read", "copy"]
        .iter().copied().collect();
    let mut summary: Vec<String> = tools
        .iter()
        .filter(|t| seen.contains(t.as_str()))
        .map(|t| match t.as_str() {
            "write" => "- Wrote file(s)".to_string(),
            "edit" => "- Edited file(s)".to_string(),
            "shell" => "- Ran shell command(s)".to_string(),
            "glob" => "- Searched with glob".to_string(),
            "search" => "- Searched codebase".to_string(),
            "ls" => "- Listed directory".to_string(),
            "read" => "- Read file(s)".to_string(),
            "copy" => "- Copied file(s)".to_string(),
            _ => format!("- {} results available", t),
        })
        .collect();
    summary.dedup();
    summary
}

fn summarize_failures(tools: &[String]) -> Vec<String> {
    let seen: HashSet<&str> = ["read", "copy", "write", "edit", "shell", "search", "glob"]
        .iter().copied().collect();
    let mut summary: Vec<String> = tools
        .iter()
        .filter(|t| seen.contains(t.as_str()))
        .map(|t| format!("- `{}` failed — do not repeat same call", t))
        .collect();
    summary.dedup();
    summary
}

fn build_next_action_contract(stop_reason: &str, has_artifacts: bool) -> String {
    match stop_reason {
        "budget_exceeded" | "iteration_limit" | "task_budget_exceeded" => {
            "Use the existing evidence to make progress. Make ONE tool call to advance toward the objective."
                .to_string()
        }
        "repeated_tool_failure" | "repeated_no_new_evidence" | "repeated_same_command" => {
            "Switch to a different approach. Do NOT repeat the failed tool. Use a different strategy."
                .to_string()
        }
        "respond_abuse" => {
            if has_artifacts {
                "Deliver the required artifact(s) first. Then provide a short final answer."
                    .to_string()
            } else {
                "Use a tool to gather evidence before providing an answer. Do NOT call respond yet."
                    .to_string()
            }
        }
        _ => "Make progress toward the objective with a single tool call.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_packet_with_artifacts() {
        let artifacts = vec!["project_tmp/report.md".to_string()];
        let packet = build_turn_context_packet(
            "Create a security report",
            "Write the report",
            &artifacts,
            &["read".to_string()],
            &[],
            "budget_exceeded",
        );
        assert!(packet.objective.contains("Create a security report"));
        assert!(packet.requirement.contains("project_tmp/report.md"));
        assert!(packet.next_action_contract.contains("make progress"));
    }

    #[test]
    fn test_build_packet_respond_abuse_no_artifact() {
        let packet = build_turn_context_packet(
            "Answer a question",
            "",
            &[],
            &[],
            &["read".to_string()],
            "respond_abuse",
        );
        assert!(packet.next_action_contract.contains("gather evidence"));
    }

    #[test]
    fn test_build_packet_with_failures() {
        let packet = build_turn_context_packet(
            "Fix the bug",
            "Find and fix the bug",
            &[],
            &["read".to_string()],
            &["copy".to_string()],
            "repeated_tool_failure",
        );
        assert!(!packet.blocked_signals.is_empty());
        assert!(packet.blocked_signals[0].contains("copy"));
        assert!(packet.next_action_contract.contains("Switch"));
    }

    #[test]
    fn test_render_packet_contains_sections() {
        let packet = build_turn_context_packet(
            "Do something",
            "",
            &[],
            &[],
            &[],
            "iteration_limit",
        );
        let rendered = render_turn_context_packet(&packet);
        assert!(rendered.contains("Current Objective"));
        assert!(rendered.contains("Remaining Requirement"));
        assert!(rendered.contains("Next Action Contract"));
    }

    #[test]
    fn test_build_continuation_from_packet() {
        let packet = build_turn_context_packet(
            "Continue task",
            "",
            &[],
            &[],
            &[],
            "budget_exceeded",
        );
        let msg = build_continuation_from_packet(&packet, 2, 3);
        assert!(msg.contains("Continue Working (Attempt 2/3)"));
        assert!(msg.contains("Current Objective"));
    }

    #[test]
    fn test_truncate_text() {
        let long = "a".repeat(500);
        let truncated = truncate_text(&long, 400);
        assert_eq!(truncated.len(), 400);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_summarize_evidence_dedup() {
        let tools = vec!["write".to_string(), "write".to_string(), "read".to_string()];
        let summary = summarize_evidence(&tools);
        let write_count = summary.iter().filter(|s| s.contains("Wrote")).count();
        assert_eq!(write_count, 1);
    }

    #[test]
    fn test_reset_turn_counter() {
        if let Ok(mut c) = TURN_COUNTER.lock() {
            *c = 100;
        }
        reset_turn_counter();
        if let Ok(c) = TURN_COUNTER.lock() {
            assert_eq!(*c, 0);
        }
    }

    // ── Task 761: CurrentTurnContext tests ──

    #[test]
    fn test_current_turn_context_basic() {
        let ctx = CurrentTurnContext::new("find AGENTS.md");
        assert_eq!(ctx.raw_request(), "find AGENTS.md");
        assert!(ctx.prior_relevant_evidence.is_none());
        let msg = ctx.build_model_message();
        assert_eq!(msg, "find AGENTS.md");
    }

    #[test]
    fn test_current_turn_context_with_evidence() {
        let mut ctx = CurrentTurnContext::new("find AGENTS.md");
        ctx.prior_relevant_evidence = Some("Read /workspace/AGENTS.md".to_string());
        let msg = ctx.build_model_message();
        assert!(msg.contains("find AGENTS.md"));
        assert!(msg.contains("Previously gathered"));
        assert!(msg.contains("Read /workspace/AGENTS.md"));
        assert!(msg.contains("Do NOT repeat"));
    }

    #[test]
    fn test_current_turn_context_raw_request_isolated() {
        let mut ctx = CurrentTurnContext::new("root path has no AGENTS.md ?");
        ctx.prior_relevant_evidence =
            Some("Required artifact: _testing_prompts/01_prompt.txt".to_string());
        let raw = ctx.raw_request();
        // Raw request must never contain injected evidence
        assert_eq!(raw, "root path has no AGENTS.md ?");
        assert!(!raw.contains("_testing_prompts"));
        assert!(!raw.contains("Previously gathered"));
    }

    #[test]
    fn test_current_turn_context_with_hints() {
        let mut ctx = CurrentTurnContext::new("verify completed tasks");
        ctx.runtime_recovery_hints = Some("Narrow the scope to _tasks/completed".to_string());
        ctx.system_guidance = Some("Do not repeat prior searches".to_string());
        let msg = ctx.build_model_message();
        assert!(msg.contains("verify completed tasks"));
        assert!(msg.contains("Runtime hints"));
        assert!(msg.contains("System guidance"));
        let raw = ctx.raw_request();
        assert_eq!(raw, "verify completed tasks");
        assert!(!raw.contains("Runtime hints"));
    }
}
