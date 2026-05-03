# Task 496: Improve Stop Policy For Stagnation With Tool Family Alternatives

**Status:** pending
**Priority:** HIGH
**Source:** Session s_1777735825_94786000 deep trace analysis (2026-05-02)
**Related:** Task 470 (event_log)

## Evidence From Session

Turn 2 trace (`trace_debug.log:42-48`):
```
tool_loop: 1 tool call(s)      ← glob (succeeded)
tool_loop: 1 tool call(s)      ← read (empty path, FAILED)
tool_loop: stagnation run 1 (no new tool signal)
tool_loop: 1 tool call(s)      ← read (empty path, FAILED again)
tool_loop: stagnation run 2 (no new tool signal)
tool_loop: 1 tool call(s)      ← read (empty path, FAILED again)
tool_loop: stopping reason=repeated_tool_failure
```

The model tried `read` 3 times with different path formats, each failing with "empty path". The stop policy correctly detected repeated same-tool-family failures, but:

1. The trace only shows `stagnation run N` with no details about what tool/arguments were tried
2. The stop reason hint is generic: "Check permissions, paths, or command syntax"
3. No alternative tool family is suggested (e.g., shell with `cat`, `search`, `grep`)

## Problems

### Problem 1: No argument-level debugging visibility
The stagnation trace records counts but not what the model actually tried. Without this, debugging tool loop failures requires looking at artifacts separately.

### Problem 2: Stop reason hints don't suggest tool family alternatives
`src/stop_policy.rs:286` suggests `read/search instead of shell` but the reverse is never suggested. When `read` is failing, the hint should suggest `shell cat <path>` as an alternative.

### Problem 3: Repeated failure detection is too aggressive on different-argument calls
The model tried `read` with different path formats each time (the thinking shows it was trying to correct the path format). The stop policy treats all `read` failures as "same tool family" regardless of argument differences. This is both correct (it prevents infinite loops) and too aggressive (it prevents argument correction).

## Fix

### Phase 1: Add tool call argument logging to stagnation trace
- In `stop_policy.rs`, when recording a tool failure for stagnation tracking, also store the tool name and truncated arguments
- In the trace output, include: `stagnation run N (tool=read args=...)`
- Keep only last 3 failed calls in memory to avoid bloat

### Phase 2: Add bidirectional tool family alternatives to stop hints  
- In `stop_policy.rs` `StopReason::RepeatedToolFailure`, add logic:
  - If failed tool is `read` → hint: "Consider using shell with cat/head, or search to find content"
  - If failed tool is `shell` → hint: "Consider using read/search instead of shell"
  - If failed tool is `glob` → hint: "Consider using search tool"
  - Default: "Consider switching tool family or approach"

### Phase 3: Allow 1 extra retry when arguments change
- Track not just tool name but also a short hash of the arguments
- If the arguments differ from previous failures AND the stubbornness counter is ≤ 3, allow one more attempt before stopping
- This lets the model correct argument formats without being cut off

## Implementation Plan

1. Modify `StopPolicy` struct to store `last_failed_tool_args: Vec<String>` alongside `last_failed_tool`
2. Update `record_tool_failure` to capture truncated arguments
3. Update stagnation trace output to include tool + args
4. Modify `StopReason::RepeatedToolFailure` hint generation to be tool-family-aware
5. Add argument-change detection as a stagnation reset condition
6. Add tests for:
   - Same tool, same args → stagnation count increments
   - Same tool, different args → allow 1 extra retry
   - Different tool → stagnation resets
   - Tool-family-specific hints are generated

## Success Criteria

- [ ] Trace shows `stagnation run N (tool=read args={"paths":[...]})`
- [ ] Stop hint suggests alternative tool families (read → shell, shell → read)
- [ ] Argument changes allow 1 extra retry before stopping on repeated_failure
- [ ] Tests pass for stagnation with/without argument changes
