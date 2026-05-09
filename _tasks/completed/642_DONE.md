# Task 642: Reasoning Visibility Redaction And Thinking Model UI Contract

**Status:** pending
**Priority:** MEDIUM
**Type:** Model Robustness / UI Internals
**Scope:** `src/tool_loop.rs`, `src/thinking_content.rs`, `src/claude_ui/`, `src/text_utils.rs`
**Source:** old postponed task U008, user request to support modern thinking models

## Summary

Create a formal contract for reasoning/thinking visibility across models that emit `reasoning_content`, `<think>` tags, empty summaries, or interleaved content/tool calls.

## Evidence And Gap

- `tool_loop.rs` handles `reasoning_content`, `reasoning`, `thought`, and `<think>` blocks in multiple streaming paths.
- Prior completed tasks addressed thought-summary gaps, but there is no single policy that separates raw reasoning artifacts, displayed summaries, redaction, and final-answer sanitation.
- Modern thinking models and dense coder models differ in whether reasoning appears in separate fields or main content.

## Implementation Plan

1. Define a `ReasoningVisibilityPolicy` with hidden, summarized, expanded, and raw-debug modes.
2. Normalize reasoning extraction in one adapter used by both tool-loop and final-answer streaming.
3. Preserve raw reasoning only in configured debug artifacts; display concise summaries by default.
4. Ensure final answers never leak raw thinking/tool-call markup.
5. Add fixtures for DeepSeek/Qwen-style tags, OpenAI-compatible `reasoning_content`, and models that emit empty assistant deltas before tool calls.

## Acceptance Criteria

- [ ] Reasoning handling is centralized and tested.
- [ ] `/reasoning` toggles a policy, not scattered render flags.
- [ ] Final answers are clean under all fixture formats.
- [ ] Session artifacts make it clear whether raw reasoning was saved, summarized, or redacted.

## Verification Plan

Run streaming parser fixtures and an interactive prompt on a thinking model with `--show-thinking` both enabled and disabled.

