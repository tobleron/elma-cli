# Task 616: Preemptive shell fallback when read tool fails validation

## Type

Bug (Small-Model Robustness)

## Severity

High

## Scope

Tool-specific (read tool / tool_loop validation error handling)

## Session Evidence

**Session:** `s_1777843822_776972000`, multiple turns
**Model:** Huihui-Qwen3.5-4B

The 4B model fails to construct valid `read` tool calls repeatedly. Evidence from `trace_debug.log`:

Turn "Ok, show me the first 5 lines of GEMINI.md":
- Iteration 1: `[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing` → failure
- Iteration 2: duplicate skipped (same: `read:GEMINI.md`)
- Iteration 3: duplicate skipped (same: `read:GEMINI.md`)
- Iteration 4: duplicate skipped (same: `read:GEMINI.md`)
- Iteration 5: model gives up on read, uses `shell: head -n 5 GEMINI.md` → FAILS (wrong path)
- Iteration 6: glob finds the file
- Iteration 7: budget exhausted

Turn "did you do it?" (checking if edit was done):
- 6 iterations, ALL 6 were `read` failures with missing filePath
- Duplicate suppression works (5 skipped) but the model NEVER recovers
- Shell fallback never attempted

The existing format hint at `src/tool_loop.rs:1413` says:
```
"read" => "The 'read' tool requires a filePath argument. Use 'shell cat <path>' instead."
```
But this hint is only injected AFTER the first failure. The model, being too small to parse the hint, continues calling `read` incorrectly.

## Problem

The 4B model consistently fails to construct valid `read` tool calls (missing `filePath` parameter). The current system:
1. Detects the validation failure
2. Injects a hint suggesting `shell cat <path>` instead
3. The duplicate suppression blocks repeated identical calls (Task 607 fix)
4. But the model never follows the hint — it retries `read` with different (still wrong) arguments

Budget iterations are wasted while the model fails to self-correct. In the "did you do it?" turn, ALL 6 iterations were completely wasted — the model achieved nothing.

## Root Cause Hypothesis

**Confirmed:** The model cannot parse error messages or format hints to construct valid tool calls. The hint system assumes the model can read and understand the error, but a 4B model cannot reliably do this. A deterministic fallback is needed.

## Proposed Solution

When the `read` tool fails with a validation error and the model retries it (even with different arguments), after 2 failures within the same turn, automatically convert the read attempt to an equivalent shell command:

```
read: GEMINI.md  →  shell: cat GEMINI.md
read: filePath=GEMINI.md, limit=5  →  shell: head -n 5 GEMINI.md
```

The conversion should:
1. Extract the path from the failed read arguments (even partial)
2. If no path is found, use the path from the previous successful tool (e.g., from a `glob` or `ls` result)
3. Construct the equivalent shell command
4. Execute it as if the model had called `shell` instead

Implementation in `src/tool_loop.rs` around line 1302 (where `execute_tool_call` is called):
```rust
if tc.function.name == "read" && read_failures_in_turn >= 2 {
    // Convert to shell fallback
    let fallback_cmd = build_read_fallback_command(&tc.function.arguments);
    // execute as shell
}
```

Files to change:
- `src/tool_loop.rs` — add read→shell fallback logic after repeated validation failures

## Acceptance Criteria

- [ ] When `read` fails with validation error 2+ times in the same turn, the system auto-converts to an equivalent shell command
- [ ] The conversion preserves the model's intent (head/cat with the right path)
- [ ] A trace message is emitted when fallback is activated
- [ ] Replaying turn "Ok, show me the first 5 lines of GEMINI.md" results in successful file reading within 2-3 iterations (not 6)

## Verification Plan

- Unit test: `build_read_fallback_command` with various argument combinations
- Integration test: simulate a turn with repeated read failures, verify fallback activates
- Replay session: verify the "read GEMINI.md" turns succeed faster

## Dependencies

- Task 607 (duplicate suppression) — prevents identical-arg retries, but this task handles different-arg retries
- Task 618 (budget limits) — this fix reduces the need for larger budgets

## Notes

This is a deterministic safety net — not a prompt fix. It acknowledges that 4B models simply cannot construct valid JSON tool calls reliably, and provides a code-level workaround. The philosophy: if the model TRIES to read a file but can't express it in the tool schema, help it out instead of letting it fail.
