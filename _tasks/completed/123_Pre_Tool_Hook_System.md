# Task 123: Pre-Tool Hook System

## Priority
**P2 — Extensibility**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection), Task 122 (Safe Shell Wrappers)

## Problem

Safety checks are currently hardcoded in `exec_shell()`. An extensible hook system would allow audit logging, permission checks, safety guards, and custom behavior injection without modifying tool code.

Inspired by Claude Code's `PreToolUseHooks` pattern.

## Scope

### 1. Hook Interface
```rust
pub(crate) trait PreToolHook {
    fn name(&self) -> &str;
    /// Returns Ok(allow) or Err(block_with_message)
    fn before_execute(&self, tool_name: &str, args: &str) -> Result<HookDecision>;
}

pub(crate) enum HookDecision {
    Allow,
    Block(String),    // reason
    Modify(String),   // new command
    RequireConfirm,   // ask user
}
```

### 2. Built-in Hooks
- `DestructiveCommandDetector` — flags mv/rm/find-pipe patterns
- `PathProtector` — blocks access to sessions/, config/, src/
- `UnscopedGlobDetector` — warns about find . / rm * patterns
- `CommandBudgetChecker` — enforces session budget

### 3. Hook Execution Order
Hooks run sequentially before tool execution. First hook to block/modify wins.

### 4. Integration Points
- `src/hook_system.rs` (new) — hook registry + execution
- `src/tool_calling.rs` — `exec_shell()` runs pre-tool hooks
- Hook registration at startup

## Design Principles
- **Extensible:** Users can add custom hooks without modifying core code
- **Ordered:** Safety hooks run before convenience hooks
- **Composable:** Multiple hooks can inspect/modify the same command
- **Transparent:** Each hook logs its decision to trace

## Verification
1. `cargo build` clean
2. `cargo test` — hook ordering, decision propagation, custom hook registration
3. Real CLI: add custom hook at runtime, verify it intercepts commands

## Acceptance Criteria
- [ ] Hook trait defined with before_execute interface
- [ ] Built-in hooks: destructive detection, path protection, unscoped glob, budget
- [ ] Hooks execute in defined order before tool execution
- [ ] Hook decisions (allow/block/modify/confirm) respected by executor
- [ ] Custom hooks can be registered at startup
- [ ] Each hook decision logged to trace
