# Task 537: Tool Failure Dedup Collision Fix

**Status:** pending
**Priority:** HIGH
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P1 — Very High Confidence

## Summary

The tool loop dedup/stagnation tracker does not distinguish between a cached success and a cached failure. When `read` returned `success=false` for `Cargo.toml`, the signal key was still marked as "already succeeded," blocking all recovery attempts and forcing the loop to spin until `respond_abuse` fired. The failure was silently swallowed — the user received no indication any tool call failed.

## Evidence

- `trace_debug.log`: `tool_loop: tool_failures=1` on every iteration of second loop
- `session.md` line 80: `TOOL FAIL [read] id=qBvYMlJ4` — first read of Cargo.toml failed
- `trace_debug.log` lines 78–135: every subsequent `read` attempt hit `duplicate skipped (already succeeded) signal=read:` despite the prior failure

## Root Cause

The dedup signal key is set identically for both success and failure outcomes. When a tool fails and retries, the dedup logic sees the same signal and blocks it as a duplicate of "what already ran," regardless of outcome. Success and failure must be tracked separately.

## Implementation Plan

1. Locate the dedup/stagnation tracker in the tool loop (likely in `src/tool_calling.rs` or the execution pipeline)
2. Change the dedup map to track `(signal_key, outcome)` pairs, not just `signal_key`
3. On `success=false`, allow re-execution of the same tool with different parameters, or up to N retries before marking permanently failed
4. Surface permanent tool failures as a visible transcript row (collapsible) with the tool name, error, and retry count
5. Add a test: tool that fails once should be retried, not deduped

## Success Criteria

- [ ] A failed tool call does not block retries of the same tool
- [ ] Permanent failures (≥N retries) are surfaced in the transcript as a visible row
- [ ] `tool_failures` counter is reflected in the stop reason, not hidden in trace only
- [ ] Existing dedup behavior for successful tool calls is unchanged

## Verification

```bash
cargo build
cargo test
# Run a session where a tool deliberately returns success=false on first call
# Verify retry occurs and failure is visible in transcript
```
