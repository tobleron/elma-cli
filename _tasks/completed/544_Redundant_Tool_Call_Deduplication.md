# Task 544: Redundant Tool Call Deduplication

**Status:** pending
**Priority:** LOW
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problems:** P8, P9 — High Confidence

## Summary

Two tool calls were issued redundantly in the same session with identical output and zero new informational value:

1. `workspace_info` was called **twice** (session start and mid-session), returning the same directory tree both times
2. `git status` was called **twice** (session start and after second loop start), returning identical output both times

On a 4B model with a constrained context window, redundant tool calls waste token budget and increment the shell caution counter unnecessarily. The `workspace_info` dedup gap is especially notable — the dedup system caught `read:` duplicates but not `workspace_info:` duplicates.

## Evidence

- `terminal_transcript.txt` lines 7–64 (first `workspace_info`) and lines 249–307 (second `workspace_info`)
- `trace_debug.log` line 7: `tool_call: shell command=cd ... && git status` (first)
- `trace_debug.log` line 88: `tool_call: shell command=cd ... && git status` (second)
- Budget counter: `caution: 5/20` after first stagnation, `caution: 6/20` after git status repeat
- No informational difference between first and second calls for either tool

## Root Cause

1. `workspace_info` is not tracked in the dedup/stagnation signal map (only `read:`, `shell:`, `search:` signals appear in trace)
2. `git status` shares the `shell:` signal key with all other shell commands — it's not granular enough to detect same-command repetition
3. The model uses these calls as "re-grounding" after stagnation, which is a reasonable intent but produces zero new data

## Implementation Plan

1. Add `workspace_info` to the dedup signal tracker with the same duplicate-skip logic applied to `read`
2. For `shell` commands, compute a content-hash of the command string as the dedup key (not just `shell:`) — this allows different shell commands while blocking identical repeats
3. When a duplicate is detected, inject a cached result note into the tool response: `[CACHED] Result unchanged from call at T+Ns ago — skipping re-execution`
4. Add an exception: allow re-execution of any tool if more than N minutes have elapsed (staleness window, configurable)

## Success Criteria

- [ ] `workspace_info` called twice in the same session → second call returns cached result, not re-executed
- [ ] `git status` called twice with identical args → second call returns cached result
- [ ] Different shell commands with different args are not blocked by dedup
- [ ] Staleness window allows re-execution after configurable timeout
- [ ] Shell caution counter is not incremented for deduplicated calls

## Verification

```bash
cargo build
cargo test
# Run a session and issue workspace_info twice
# Verify second call shows [CACHED] and does not increment budget
```
