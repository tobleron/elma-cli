# Task 538: Shell Output Truncation Guard

**Status:** pending
**Priority:** MEDIUM
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P2 — Very High Confidence

## Summary

When the model runs `git diff <file> | head -150`, the output is silently truncated at 150 lines if the diff is larger. The model receives no signal that output was cut. During the session, `tool_calling.rs` diff returned exactly 150 lines — the maximum — with no truncation warning. The final risk report cited evidence from this file without flagging that the diff was incomplete.

## Evidence

- `trace_debug.log` line 25: `exit_code=0 lines=150` for `git diff src/tool_calling.rs | head -150`
- Terminal transcript: diff content ends abruptly at exactly 150 lines
- Session final answer: stated facts about `tool_calling.rs` changes without noting partial visibility

## Root Cause

`head -N` does not set a non-zero exit code when output is truncated. The `result_verifier` post-hook only checks `exit_code=0`, which is satisfied. No line-count-vs-limit check exists.

## Implementation Plan

1. In the shell tool's `result_verifier` post-hook, detect when output line count equals the configured limit
2. When at-limit is detected, append a visible annotation to the tool result: `⚠️ Output truncated at N lines — full output may contain additional content`
3. Make the annotation prominent enough that the model includes it in its reasoning (e.g., prefix with `[TRUNCATED]` in the result content string)
4. Apply this guard to all shell commands that pipe through `head`, `tail`, or any line-limiter
5. Consider adding a `truncated: bool` field to `ToolExecutionResult` for machine-readable use downstream

## Success Criteria

- [ ] When shell output exactly hits the line limit, a truncation warning is appended to the result
- [ ] The model's response acknowledges partial visibility when truncation occurs
- [ ] Non-truncated outputs produce no annotation (no false positives)

## Verification

```bash
cargo build
cargo test
# Run: git diff HEAD~10 | head -5 via shell tool
# Verify truncation notice appears in tool result and transcript
```
