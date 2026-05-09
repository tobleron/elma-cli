# Task 644: Dense Coder Model Output Sanitizer And Finalization Guards

**Status:** pending
**Priority:** CRITICAL
**Type:** Model Robustness / Finalization
**Scope:** `src/tool_loop.rs`, `src/final_answer.rs`, `src/text_utils.rs`, `src/json_parser.rs`
**Source:** user request; prior reasoning/tool-call leak tasks; `_knowledge_base` Roo native tool-call parser tests

## Summary

Harden Elma against dense coder models that emit markdown tool-call blocks, XML-ish tags, half-JSON, repeated code fences, or final answers mixed with hidden control text.

## Evidence And Gap

- `tool_loop.rs` already strips `<tool_call>` and `<think>` blocks in several places, proving this failure class is real.
- Sanitization is spread across request paths instead of enforced at a single model-output boundary.
- Dense coder models often produce valid-looking text that is semantically a tool request or internal control block.

## Implementation Plan

1. Add a `ModelOutputEnvelope` normalization stage before tool execution/finalization.
2. Classify output parts as assistant text, reasoning, tool call, rejected control markup, or malformed structured output.
3. Reject internal control markup from final answers with an evidence-backed fallback.
4. Add loop guards for repeated code fences, repeated invalid tool markup, and intent-only responses.
5. Record sanitizer actions in trace/session artifacts and visible notices when they affect the answer.

## Acceptance Criteria

- [ ] All final-answer paths pass through the sanitizer.
- [ ] Tool-call markup cannot appear in user-facing final answers.
- [ ] Sanitizer decisions are auditable in session artifacts.
- [ ] Tests cover partial tags split across streaming chunks.

## Verification Plan

Run parser/finalization fixtures with malformed dense-coder outputs and a real CLI prompt against a dense local model.

