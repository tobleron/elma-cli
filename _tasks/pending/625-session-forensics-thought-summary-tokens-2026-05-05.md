# Task 625: Session Forensics — Thought Summary & Token Display Fixes

## Type
Bug

## Severity
High

## Scope
System-wide

## Session Evidence
- Session: s_1777932314_635562000 (2026-05-05)
- User request: "what is the square root of 999?"
- Tools executed: shell (python3 math.sqrt(999))
- Primary LLM: 3 retries due to truncation (max_tokens too small)
- Auxiliary LLM: Called successfully, 2000 bytes received, parsed OK
- Summary: NOT displayed on right panel
- Input token progress bar: 0% throughout session

## Problem
1. **Summary not showing**: The auxiliary LLM generates summaries but they don't appear in the UI. The thinking content extraction from inline `<think>` tags may not work correctly, leaving `combined_thinking` empty for models that don't use separate `reasoning_content`.

2. **0% input token progress**: The progress bar uses `input_tokens / 16384` but typical input tokens are ~50-200, giving <1% that shows as 0%.

3. **Primary LLM truncation**: Model response truncated on 2 attempts before succeeding on 3rd (max_tokens went from 256→512→1024).

## Root Cause Hypothesis
1. Likely: `turn.thinking_content` extraction misses think-tag content for models that wrap in `<think>` tags inline
2. Confirmed: Progress bar denominator (16384) is too large for input token estimates
3. Possible: Initial max_tokens (256) is too low for the primary LLM

## Proposed Solution
See individual task files:
- `623-thought-summary-missing-for-think-tag-models.md`
- `624-input-token-progress-bar-zero.md`

## Acceptance Criteria
- [ ] Summaries appear for all model formats (including inline think-tag models)
- [ ] Input token progress bar shows meaningful values
- [ ] First prompt shows summary when thinking content exists

## Verification Plan
- Run elma-cli, send "hello"
- Verify summary appears on right panel after response
- Verify input token progress bar shows > 0%

## Dependencies
Tasks 623, 624.

## Notes
- The primary LLM's truncation is a pre-existing model configuration issue (max_tokens profile setting), not addressed here
- The auxiliary model URL is hardcoded to 192.168.1.186:8084 — should use runtime config
