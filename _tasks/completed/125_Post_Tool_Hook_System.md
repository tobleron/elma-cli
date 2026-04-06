# Task 125: Post-Tool Hook System

## Priority
**P2 — Extensibility**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 123 (Pre-Tool Hook System)

## Problem

After a tool executes, no verification occurs. If the model ran `mv *.sh dest/` and it failed silently or moved wrong files, nothing catches the error before the model moves on.

Inspired by Claude Code's `PostToolUseHooks` pattern.

## Scope

### 1. Hook Interface
```rust
pub(crate) trait PostToolHook {
    fn name(&self) -> &str;
    fn after_execute(&self, tool_name: &str, result: &ToolExecutionResult) -> Result<HookVerdict>;
}

pub(crate) enum HookVerdict {
    Pass,
    Warn(String),    // warning to model
    Alert(String),   // serious issue, feed back as error
    TriggerAction,   // e.g., auto-snapshot, rollback
}
```

### 2. Built-in Hooks
- `MutationVerifier` — after `mv`/`rm`: verify expected files actually moved/deleted
- `ErrorPatternDetector` — detects "No such file or directory", "Permission denied" and feeds actionable guidance
- `AutoSnapshotTrigger` — after first successful destructive command in session, create snapshot
- `OutputSizeMonitor` — warns if command produced unexpectedly large output (>100K chars)

### 3. Integration Points
- `src/hook_system.rs` — extend with post-hook support
- `src/tool_calling.rs` — `exec_shell()` runs post-tool hooks after execution
- `src/auto_compact.rs` — snapshot integration

## Design Principles
- **Truthful:** Report actual results, not optimistic summaries
- **Actionable:** Verdicts drive specific next steps, not vague warnings
- **Composable:** Multiple hooks can inspect the same result
- **Non-blocking for reads:** Post-hooks on read/search don't delay responses

## Verification
1. `cargo build` clean
2. Real CLI: `mv` succeeds but moved wrong files → verifier detects mismatch, alerts model
3. Real CLI: `rm` succeeds → auto-snapshot created, snapshot ID logged

## Acceptance Criteria
- [ ] Post-tool hook trait defined
- [ ] Built-in hooks: mutation verifier, error pattern detector, auto-snapshot trigger
- [ ] Hooks execute after tool execution
- [ ] Verdicts (pass/warn/alert/action) drive appropriate follow-up
- [ ] Auto-snapshot triggers on first destructive command success
- [ ] Post-hooks on read-only tools don't delay responses
