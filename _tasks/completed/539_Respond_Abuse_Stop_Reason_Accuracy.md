# Task 539: Respond Abuse Stop Reason Accuracy

**Status:** pending
**Priority:** HIGH
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P3 — Very High Confidence

## Summary

The tool loop fired `stopping reason=respond_abuse` twice in a session where the model did not misuse the `respond` tool. The stop reason is being triggered by general stagnation (5 consecutive duplicate-skipped calls) rather than actual repeated `respond` calls. This misattributes the stop cause, making it harder to diagnose real stagnation from real respond abuse, and produces misleading telemetry.

## Evidence

- `trace_debug.log` line 69: `stopping reason=respond_abuse`
- `trace_debug.log` line 139: `stopping reason=respond_abuse` (second occurrence)
- Session transcript: zero `respond` tool calls present in either loop
- Both loops terminated after 5x `duplicate skipped` stagnation cycles

## Root Cause

The `respond_abuse` stop condition is shared between two different termination paths:
1. Real respond abuse (model calls respond repeatedly without doing tool work)
2. General stagnation (model keeps issuing duplicates)

Both trigger the same stop label, making the reason misleading.

## Implementation Plan

1. Audit the tool loop stop conditions in the execution pipeline
2. Separate the `respond_abuse` stop from the `stagnation_limit` stop:
   - `stopping reason=respond_abuse` → only when the model issues ≥N `respond` calls in a row
   - `stopping reason=stagnation_limit` → when duplicate-skip count hits the stagnation threshold
3. Add a third case: `stopping reason=tool_failure_stagnation` for when `tool_failures > 0` causes the stagnation
4. Ensure each stop reason is surfaced as a collapsible transcript row (required by AGENTS.md Rule 6)

## Success Criteria

- [ ] `respond_abuse` only fires when the model actually abuses the respond tool
- [ ] Stagnation from duplicate skips fires `stagnation_limit`
- [ ] Tool-failure-induced stagnation fires `tool_failure_stagnation`
- [ ] All stop reasons appear in the transcript as visible (collapsible) rows

## Verification

```bash
cargo build
cargo test
# Run a session that stagnates via duplicate skips (no respond calls)
# Verify stop reason is stagnation_limit, not respond_abuse
# Run a session that actually spams respond
# Verify stop reason is respond_abuse
```
