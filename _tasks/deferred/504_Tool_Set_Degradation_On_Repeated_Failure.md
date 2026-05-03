# Task 504: Tool Set Degradation on Repeated Failure

**Status:** pending
**Priority:** MEDIUM
**Estimated effort:** 1-2 days
**Primary surfaces:** `src/stop_policy.rs`, `src/tool_loop.rs`
**Depends on:** None

## Objective

When the model repeatedly produces failing tool calls, the current behavior is to kill the entire tool loop with `repeated_tool_failure`. Instead, degrade the available tool set: remove complex tools, allow the model to keep working with a simpler set, and only kill the loop as a last resort.

## Root Cause

At `stop_policy.rs:182-216`, `record_tool_calls()` tracks total tool call count and repeated shell commands. After too many failures, `stop_policy.record_tool_calls()` returns a `StopOutcome` with `StopReason::StageBudgetExceeded` or similar. The tool loop at `tool_loop.rs:974` immediately exits on any stop outcome.

## Implementation

### Phase 1: Tool Set Degradation Levels

```rust
enum ToolDegradationLevel {
    Full,       // All tools available
    Reduced,    // Remove: edit, write, delete, shell
    Minimal,    // Only: read, respond
}
```

Add a `degradation_level` field to `StopPolicy`. In `record_tool_calls()`, instead of returning a stop outcome on repeated failures, increment the degradation level:

```rust
if self.consecutive_shell_failures >= 3 && self.degradation_level == ToolDegradationLevel::Full {
    self.degradation_level = ToolDegradationLevel::Reduced;
    return Some(self.build_degradation_hint("Removing edit/write/delete/shell due to repeated failures"));
}
if self.consecutive_shell_failures >= 6 {
    self.degradation_level = ToolDegradationLevel::Minimal;
    return Some(self.build_degradation_hint("Only respond and read are available"));
}
```

The degradation hint is injected into `messages` so the model knows tools are restricted.

### Phase 2: Build Tool Definitions From Degradation Level

At `tool_loop.rs:892`, the tool definitions are built:
```rust
tools: Some(crate::tool_calling::build_tool_definitions(&PathBuf::new())),
```

Pass `stop_policy.degradation_level` to `build_tool_definitions()` so it only includes tools matching the current level.

### Phase 3: Inject Degradation Hint Into Messages

When degradation level changes, inject a system message:
```rust
"Injecting hint: Model tool calls are failing repeatedly. Reduced tool set to [respond, read]."
```

This avoids confusing the model when tools suddenly disappear.

## Behavior Change

| Before | After |
|--------|-------|
| 3rd tool failure → loop killed | 3rd failure → remove shell/edit/write/delete, continue |
| Loop stops, user sees incomplete answer | Model continues with read + respond, produces partial answer |
| Force-finalized with 0 evidence | Model can still read files and respond |

## Files to Modify

- `src/stop_policy.rs` — add `ToolDegradationLevel`, modify `record_tool_calls()`
- `src/tool_loop.rs:892` — pass degradation level to `build_tool_definitions()`
- `src/tool_calling.rs` — accept degradation level parameter in `build_tool_definitions()`

## Verification

- Simulate repeated shell failures: model should degrade to reduced tool set, not stop the loop.
- With reduced tools, model can still call `read` and `respond`.
- Only after all levels exhausted and still failing → loop stops.
- `cargo build && cargo test && cargo clippy` passes.
