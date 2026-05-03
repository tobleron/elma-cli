# Task 545: Tool Loop Hard Iteration Cap

**Status:** pending
**Priority:** MEDIUM
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P10 — Medium-High Confidence

## Summary

The tool loop started with `max_iterations=0` in both loops of the session. If `0` is treated as "no cap," then the only termination signals are the stagnation threshold (8) and `respond_abuse`. For a 4B model that stagnates frequently, this means sessions run until emergent behavior terminates them rather than an explicit budget ceiling — burning context window and shell budget unnecessarily.

## Evidence

- `trace_debug.log` line 4: `tool_loop: starting max_iterations=0 stagnation_threshold=8 timeout=30m`
- `trace_debug.log` line 74: `tool_loop: starting max_iterations=0 stagnation_threshold=8 timeout=30m`
- Both loops ran until stagnation (5 consecutive duplicates) triggered `respond_abuse` stop
- 12 total tool calls were made; some were clearly redundant (P8, P9)

## Root Cause

`max_iterations=0` is likely a sentinel for "unlimited" passed from the formula or complexity layer. If the formula assigns no hard cap, the tool loop has no ceiling other than stagnation and timeout. For `DIRECT` and `INVESTIGATE` complexity tiers, a sensible cap should be enforced automatically.

## Implementation Plan

1. Audit the tool loop initialization: locate where `max_iterations` is set and where `0` is passed
2. If `0` means "no cap," replace the sentinel with a named constant `NO_LIMIT` and add a guard:
   - `DIRECT` → max 5 iterations
   - `INVESTIGATE` → max 10 iterations
   - `MULTISTEP` → max 20 iterations
   - `OPEN_ENDED` → max 40 iterations (or configurable)
3. When the hard cap is hit, emit a transcript event: `⚠️ Tool loop hard cap reached (N iterations) — forcing finalization`
4. Make the cap values configurable via `elma.toml` or the model profile config

## Success Criteria

- [ ] No session starts with `max_iterations=0` unless the formula explicitly intends no limit
- [ ] Each complexity tier has a default hard cap
- [ ] Hitting the hard cap surfaces a visible transcript row
- [ ] Cap can be overridden per-session via config

## Verification

```bash
cargo build
cargo test
# Run a DIRECT complexity session and count tool calls
# Verify it stops at or before the hard cap
```
