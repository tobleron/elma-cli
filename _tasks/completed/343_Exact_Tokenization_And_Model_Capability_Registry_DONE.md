# Task 343: Exact Tokenization And Model Capability Registry

**Status:** in_progress
**Source patterns:** Roo-Code model capability handling, Aider model metadata, Goose token tracking
**Revives:** `_tasks/postponed/087_LlamaCpp_Runtime_Token_Telemetry.md`, `_tasks/postponed/088_Objective_Level_Token_Forecasting_And_Budget_Envelopes.md`

## Summary

Replace approximate character-based context budgeting with tokenizer-backed counts where available, plus a model capability registry for context window, output limit, tool support, reasoning controls, and provider quirks.

## Why

`auto_compact.rs` still uses approximate token estimation. That is acceptable as a fallback, but robust agents know model limits explicitly and budget context/output with fewer surprises. This directly improves reliability and context efficiency.

## Implementation Plan

1. Add a `model_capabilities` registry loaded from built-in data plus optional user config.
2. Add tokenizer adapters for supported model families, with char-estimate fallback.
3. Use exact counts in compaction thresholds, transcript token display, and output budgeting.
4. Add provider-specific caps for max output and reasoning controls.
5. Add tests for known token counts and fallback behavior.

## Success Criteria

- [x] Context budgeting uses exact token counts for at least one supported tokenizer.
- [x] Unknown models fall back safely and visibly.
- [x] Output budget is clamped by model/provider capability.
- [x] Compaction triggers are more predictable than char/3.5 estimates.
- [x] Tests cover exact, fallback, and model override paths.

## Anti-Patterns To Avoid

- Do not make tokenizer failures fatal.
- Do not hardcode every model into prompt text.
- Do not put token policy details beyond model name/count/elapsed time in the footer.
